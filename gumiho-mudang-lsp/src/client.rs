use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex as SyncMutex;
use serde_json::Value;
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex as AsyncMutex};

use crate::jsonrpc::{self, CodecError, IncomingMessage, Notification};

type NotificationHandler = Arc<dyn Fn(Notification) + Send + Sync>;
type PendingMap = HashMap<i64, oneshot::Sender<Result<Value, ClientError>>>;
type CrashHandler = Box<dyn Fn(String) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("client not started")]
    NotStarted,
    #[error("spawn failed: {0}")]
    SpawnFailed(String),
    #[error("request timeout after {0:?}")]
    Timeout(Duration),
    #[error("request failed: {0}")]
    RequestFailed(String),
    #[error("codec error: {0}")]
    Codec(#[from] CodecError),
    #[error("server crashed: {0}")]
    Crashed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct LspClient {
    command: String,
    args: Vec<String>,
    cwd: String,
    next_id: AtomicI64,
    pending: Arc<SyncMutex<PendingMap>>,
    writer: Arc<AsyncMutex<Option<tokio::process::ChildStdin>>>,
    child: AsyncMutex<Option<Child>>,
    notification_handlers: Arc<SyncMutex<Vec<NotificationHandler>>>,
    reader_handle: SyncMutex<Option<tokio::task::JoinHandle<()>>>,
    on_crash: Arc<SyncMutex<Option<CrashHandler>>>,
}

impl LspClient {
    pub fn new(command: &str, args: &[&str], cwd: &Path) -> Self {
        Self {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: cwd.to_string_lossy().to_string(),
            next_id: AtomicI64::new(1),
            pending: Arc::new(SyncMutex::new(HashMap::new())),
            writer: Arc::new(AsyncMutex::new(None)),
            child: AsyncMutex::new(None),
            notification_handlers: Arc::new(SyncMutex::new(Vec::new())),
            reader_handle: SyncMutex::new(None),
            on_crash: Arc::new(SyncMutex::new(None)),
        }
    }

    pub fn set_on_crash(&self, handler: impl Fn(String) + Send + Sync + 'static) {
        *self.on_crash.lock() = Some(Box::new(handler));
    }

    pub fn on_notification(&self, handler: impl Fn(Notification) + Send + Sync + 'static) {
        self.notification_handlers.lock().push(Arc::new(handler));
    }

    pub async fn start(&self) -> Result<(), ClientError> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .current_dir(&self.cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let mut child = cmd
            .spawn()
            .map_err(|e| ClientError::SpawnFailed(format!("{}: {e}", self.command)))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ClientError::SpawnFailed("no stdout".into()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ClientError::SpawnFailed("no stdin".into()))?;

        *self.writer.lock().await = Some(stdin);

        let pending = Arc::clone(&self.pending);
        let handlers = Arc::clone(&self.notification_handlers);
        let on_crash = Arc::clone(&self.on_crash);

        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                match jsonrpc::read_message(&mut reader).await {
                    Ok(IncomingMessage::Response(resp)) => {
                        if let Some(id) = resp.id {
                            let sender = pending.lock().remove(&id);
                            if let Some(tx) = sender {
                                let result = if let Some(err) = resp.error {
                                    Err(ClientError::RequestFailed(err.message))
                                } else {
                                    Ok(resp.result.unwrap_or(Value::Null))
                                };
                                let _ = tx.send(result);
                            }
                        }
                    }
                    Ok(IncomingMessage::Notification(notif)) => {
                        let handlers = handlers.lock().clone();
                        for h in &handlers {
                            h(notif.clone());
                        }
                    }
                    Ok(IncomingMessage::Request(_)) => {}
                    Err(CodecError::ConnectionClosed) => {
                        if let Some(cb) = on_crash.lock().as_ref() {
                            cb("connection closed".to_string());
                        }
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });

        *self.reader_handle.lock() = Some(reader_handle);
        *self.child.lock().await = Some(child);

        Ok(())
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, ClientError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        self.pending.lock().insert(id, tx);

        {
            let mut writer_guard = self.writer.lock().await;
            let writer = writer_guard.as_mut().ok_or(ClientError::NotStarted)?;
            jsonrpc::send_request(writer, id, method, Some(params)).await?;
        }

        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(ClientError::Crashed("response channel dropped".into())),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err(ClientError::Timeout(Duration::from_secs(30)))
            }
        }
    }

    pub async fn notify(&self, method: &str, params: Value) -> Result<(), ClientError> {
        let mut writer_guard = self.writer.lock().await;
        let writer = writer_guard.as_mut().ok_or(ClientError::NotStarted)?;
        jsonrpc::send_notification(writer, method, Some(params)).await?;
        Ok(())
    }

    pub async fn initialize(&self, root_uri: &str) -> Result<Value, ClientError> {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "dynamicRegistration": false,
                        "didSave": true
                    },
                    "publishDiagnostics": {
                        "relatedInformation": true
                    },
                    "hover": { "dynamicRegistration": false },
                    "definition": { "dynamicRegistration": false },
                    "references": { "dynamicRegistration": false },
                    "documentSymbol": {
                        "dynamicRegistration": false,
                        "hierarchicalDocumentSymbolSupport": true
                    },
                    "callHierarchy": { "dynamicRegistration": false }
                }
            }
        });

        let result = self.request("initialize", params).await?;
        self.notify("initialized", serde_json::json!({})).await?;
        Ok(result)
    }

    pub async fn shutdown_and_exit(&self) -> Result<(), ClientError> {
        let _ = self.request("shutdown", serde_json::json!(null)).await;
        let _ = self.notify("exit", serde_json::json!(null)).await;
        self.kill().await;
        Ok(())
    }

    pub async fn kill(&self) {
        *self.writer.lock().await = None;
        if let Some(handle) = self.reader_handle.lock().take() {
            handle.abort();
        }
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            let _ = child.kill().await;
        }
        *child_guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_client_spawn_nonexistent_binary() {
        let client = LspClient::new("nonexistent_lsp_binary_xyz", &[], &PathBuf::from("/tmp"));
        let err = client.start().await.unwrap_err();
        assert!(matches!(err, ClientError::SpawnFailed(_)));
    }
}
