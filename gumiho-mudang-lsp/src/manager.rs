use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::Value;

use crate::diagnostics::DiagnosticRegistry;
use crate::instance::{InstanceConfig, LspServerInstance, ServerState};
use crate::registry::LspServerDef;
use crate::status::ServerStatusEntry;

pub struct LspServerManager {
    instances: HashMap<String, Arc<LspServerInstance>>,
    config: InstanceConfig,
    diagnostics: Arc<Mutex<DiagnosticRegistry>>,
    opened_files: HashMap<String, String>, // uri -> server_id
}

impl LspServerManager {
    pub fn new(config: InstanceConfig) -> Self {
        Self {
            instances: HashMap::new(),
            config,
            diagnostics: Arc::new(Mutex::new(DiagnosticRegistry::new())),
            opened_files: HashMap::new(),
        }
    }

    pub fn diagnostics(&self) -> Arc<Mutex<DiagnosticRegistry>> {
        Arc::clone(&self.diagnostics)
    }

    pub async fn start(&mut self, def: &'static LspServerDef, cwd: &Path) {
        if self.instances.contains_key(def.id) {
            return;
        }

        let inst = Arc::new(LspServerInstance::new(def, cwd, self.config.clone()));

        // Wire diagnostics notification
        let diag_registry = Arc::clone(&self.diagnostics);
        inst.on_notification(move |notif| {
            if notif.method == "textDocument/publishDiagnostics" {
                if let Some(params) = notif.params {
                    let uri = params.get("uri").and_then(|v| v.as_str()).map(String::from);
                    let diagnostics = params.get("diagnostics").cloned();
                    if let (Some(uri), Some(diags_val)) = (uri, diagnostics) {
                        let diags: Vec<crate::types::Diagnostic> =
                            serde_json::from_value(diags_val).unwrap_or_default();
                        diag_registry.lock().register(uri, diags);
                    }
                }
            }
        });

        inst.start().await;
        self.instances.insert(def.id.to_string(), inst);
    }

    pub async fn stop(&mut self, id: &str) {
        if let Some(inst) = self.instances.get(id) {
            inst.stop().await;
        }
    }

    pub async fn stop_all(&mut self) {
        for inst in self.instances.values() {
            inst.stop().await;
        }
        self.instances.clear();
        self.opened_files.clear();
    }

    pub async fn restart(&mut self, id: &str) {
        if let Some(inst) = self.instances.get(id) {
            inst.restart().await;
        }
    }

    pub async fn transition(&mut self, defs: &[&'static LspServerDef], new_cwd: &Path) {
        self.stop_all().await;
        for def in defs {
            self.start(def, new_cwd).await;
        }
    }

    // File sync

    fn get_instance_for_file(&self, file_path: &Path) -> Option<&Arc<LspServerInstance>> {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();

        self.instances.values().find(|inst| {
            inst.state() == ServerState::Running && inst.def.extensions.contains(&ext.as_str())
        })
    }

    fn to_uri(file_path: &Path) -> String {
        format!(
            "file://{}",
            file_path
                .canonicalize()
                .unwrap_or_else(|_| file_path.to_path_buf())
                .display()
        )
    }

    pub async fn open_file(&mut self, file_path: &Path, content: &str) {
        let inst = match self.get_instance_for_file(file_path) {
            Some(i) => Arc::clone(i),
            None => return,
        };
        let uri = Self::to_uri(file_path);
        if self.opened_files.get(&uri) == Some(&inst.def.id.to_string()) {
            return;
        }
        let language_id = inst.def.languages.first().unwrap_or(&"plaintext");
        inst.send_notification(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": content
                }
            }),
        )
        .await;
        self.opened_files.insert(uri, inst.def.id.to_string());
    }

    pub async fn change_file(&mut self, file_path: &Path, content: &str) {
        let uri = Self::to_uri(file_path);
        if !self.opened_files.contains_key(&uri) {
            self.open_file(file_path, content).await;
            return;
        }
        let inst = match self.get_instance_for_file(file_path) {
            Some(i) => Arc::clone(i),
            None => return,
        };
        inst.send_notification(
            "textDocument/didChange",
            serde_json::json!({
                "textDocument": { "uri": uri, "version": 1 },
                "contentChanges": [{ "text": content }]
            }),
        )
        .await;
    }

    pub async fn save_file(&mut self, file_path: &Path) {
        let uri = Self::to_uri(file_path);
        let server_id = match self.opened_files.get(&uri) {
            Some(id) => id.clone(),
            None => return,
        };
        if let Some(inst) = self.instances.get(&server_id) {
            inst.send_notification(
                "textDocument/didSave",
                serde_json::json!({ "textDocument": { "uri": uri } }),
            )
            .await;
        }
    }

    pub async fn close_file(&mut self, file_path: &Path) {
        let uri = Self::to_uri(file_path);
        let server_id = match self.opened_files.remove(&uri) {
            Some(id) => id,
            None => return,
        };
        if let Some(inst) = self.instances.get(&server_id) {
            inst.send_notification(
                "textDocument/didClose",
                serde_json::json!({ "textDocument": { "uri": uri } }),
            )
            .await;
        }
    }

    pub async fn send_request(
        &self,
        file_path: &Path,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let inst = self
            .get_instance_for_file(file_path)
            .ok_or_else(|| format!("no LSP server for {:?}", file_path))?;
        inst.send_request(method, params).await
    }

    pub fn get_server_def_for_file(&self, file_path: &Path) -> Option<&'static LspServerDef> {
        self.get_instance_for_file(file_path).map(|i| i.def)
    }

    pub fn status(&self) -> Vec<ServerStatusEntry> {
        let mut entries: Vec<ServerStatusEntry> = self
            .instances
            .values()
            .map(|inst| ServerStatusEntry {
                id: inst.def.id.to_string(),
                name: inst.def.name.to_string(),
                state: inst.state(),
            })
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.instances
            .get(id)
            .is_some_and(|i| i.state() == ServerState::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_status_empty() {
        let mgr = LspServerManager::new(InstanceConfig::default());
        assert!(mgr.status().is_empty());
    }

    #[test]
    fn test_get_server_def_no_match() {
        let mgr = LspServerManager::new(InstanceConfig::default());
        assert!(mgr.get_server_def_for_file(Path::new("test.xyz")).is_none());
    }

    #[tokio::test]
    async fn test_open_file_no_server() {
        let mut mgr = LspServerManager::new(InstanceConfig::default());
        // Should not panic
        mgr.open_file(Path::new("/tmp/test.unknown"), "content")
            .await;
    }

    #[test]
    fn test_is_running_false_when_empty() {
        let mgr = LspServerManager::new(InstanceConfig::default());
        assert!(!mgr.is_running("rust-analyzer"));
    }
}
