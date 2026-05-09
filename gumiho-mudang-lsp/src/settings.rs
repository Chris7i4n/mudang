use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::registry::LspServerDef;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspManagerSettings {
    pub enabled_servers: Vec<String>,
}

fn settings_path(cwd: &Path) -> std::path::PathBuf {
    cwd.join(".athena").join("lsp.json")
}

pub async fn load_settings(cwd: &Path) -> LspManagerSettings {
    let path = settings_path(cwd);
    match tokio::fs::read_to_string(&path).await {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => LspManagerSettings::default(),
    }
}

pub async fn save_settings(cwd: &Path, settings: &LspManagerSettings) -> std::io::Result<()> {
    let dir = cwd.join(".athena");
    tokio::fs::create_dir_all(&dir).await?;
    let json = serde_json::to_string_pretty(settings).unwrap();
    tokio::fs::write(settings_path(cwd), format!("{json}\n")).await
}

pub fn resolve_enabled<'a>(
    settings: &LspManagerSettings,
    available: &[&'a LspServerDef],
) -> Vec<&'a LspServerDef> {
    let enabled: std::collections::HashSet<&str> = settings
        .enabled_servers
        .iter()
        .map(|s| s.as_str())
        .collect();
    available
        .iter()
        .filter(|def| enabled.contains(def.id))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_save_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let settings = LspManagerSettings {
            enabled_servers: vec!["rust-analyzer".into(), "gopls".into()],
        };
        save_settings(tmp.path(), &settings).await.unwrap();
        let loaded = load_settings(tmp.path()).await;
        assert_eq!(loaded.enabled_servers, settings.enabled_servers);
    }

    #[tokio::test]
    async fn test_missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let loaded = load_settings(tmp.path()).await;
        assert!(loaded.enabled_servers.is_empty());
    }

    #[test]
    fn test_resolve_enabled_filters() {
        use crate::registry::KNOWN_SERVERS;
        let settings = LspManagerSettings {
            enabled_servers: vec!["rust-analyzer".into()],
        };
        let available: Vec<&LspServerDef> = KNOWN_SERVERS.iter().collect();
        let resolved = resolve_enabled(&settings, &available);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id, "rust-analyzer");
    }

    #[tokio::test]
    async fn test_malformed_json_returns_default() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".athena");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("lsp.json"), "not json at all")
            .await
            .unwrap();
        let loaded = load_settings(tmp.path()).await;
        assert!(loaded.enabled_servers.is_empty());
    }
}
