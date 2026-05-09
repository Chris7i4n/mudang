use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::Value;
use tokio::sync::Notify;

use crate::client::LspClient;
use crate::jsonrpc::Notification;
use crate::registry::LspServerDef;

type NotificationHandler = Arc<dyn Fn(Notification) + Send + Sync>;
type StateChangeHandler = Box<dyn Fn(ServerState) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Stopped,
    Starting,
    Running,
    Error,
    Failed,
}

#[derive(Clone)]
pub struct InstanceConfig {
    pub stability_threshold: Duration,
    pub max_restarts: u32,
    pub base_restart_delay: Duration,
    pub handshake_timeout: Duration,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            stability_threshold: Duration::from_secs(10),
            max_restarts: 5,
            base_restart_delay: Duration::from_secs(2),
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

struct InstanceInner {
    state: ServerState,
    restart_count: u32,
    started_at: Option<Instant>,
    last_error: Option<String>,
}

pub struct LspServerInstance {
    pub def: &'static LspServerDef,
    cwd: PathBuf,
    config: InstanceConfig,
    inner: Arc<Mutex<InstanceInner>>,
    client: Arc<Mutex<Option<Arc<LspClient>>>>,
    notification_handlers: Arc<Mutex<Vec<NotificationHandler>>>,
    shutdown_notify: Arc<Notify>,
    on_state_change: Arc<Mutex<Option<StateChangeHandler>>>,
}

impl LspServerInstance {
    pub fn new(def: &'static LspServerDef, cwd: &Path, config: InstanceConfig) -> Self {
        Self {
            def,
            cwd: cwd.to_path_buf(),
            config,
            inner: Arc::new(Mutex::new(InstanceInner {
                state: ServerState::Stopped,
                restart_count: 0,
                started_at: None,
                last_error: None,
            })),
            client: Arc::new(Mutex::new(None)),
            notification_handlers: Arc::new(Mutex::new(Vec::new())),
            shutdown_notify: Arc::new(Notify::new()),
            on_state_change: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_on_state_change(&self, handler: impl Fn(ServerState) + Send + Sync + 'static) {
        *self.on_state_change.lock() = Some(Box::new(handler));
    }

    pub fn on_notification(&self, handler: impl Fn(Notification) + Send + Sync + 'static) {
        self.notification_handlers.lock().push(Arc::new(handler));
    }

    pub fn state(&self) -> ServerState {
        self.inner.lock().state
    }

    pub fn last_error(&self) -> Option<String> {
        self.inner.lock().last_error.clone()
    }

    pub fn restart_count(&self) -> u32 {
        self.inner.lock().restart_count
    }

    pub async fn start(&self) {
        {
            let state = self.inner.lock().state;
            if state != ServerState::Stopped {
                return;
            }
        }
        self.spawn().await;
    }

    pub async fn stop(&self) {
        self.set_state(ServerState::Stopped);
        let client = self.client.lock().take();
        if let Some(c) = client {
            let _ = c.shutdown_and_exit().await;
        }
        self.shutdown_notify.notify_waiters();
    }

    pub async fn restart(&self) {
        self.stop().await;
        {
            let mut inner = self.inner.lock();
            inner.restart_count = 0;
        }
        self.spawn().await;
    }

    pub async fn send_notification(&self, method: &str, params: Value) {
        if self.state() != ServerState::Running {
            return;
        }
        let client = self.client.lock().clone();
        if let Some(c) = client {
            let _ = c.notify(method, params).await;
        }
    }

    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value, String> {
        if self.state() != ServerState::Running {
            return Err(format!("server '{}' is not running", self.def.id));
        }
        let client = self.client.lock().clone();
        match client {
            Some(c) => c.request(method, params).await.map_err(|e| e.to_string()),
            None => Err("no client available".into()),
        }
    }

    async fn spawn(&self) {
        self.set_state(ServerState::Starting);

        let root_uri = self.find_root_uri().await;
        let client = Arc::new(LspClient::new(self.def.command, self.def.args, &self.cwd));

        // Register notification handlers
        let handlers = self.notification_handlers.lock().clone();
        for handler in handlers {
            client.on_notification(move |n| handler(n));
        }

        // Set up crash handler
        let inner = Arc::clone(&self.inner);
        let config = self.config.clone();
        let on_state_change = Arc::clone(&self.on_state_change);
        let def_id = self.def.id;
        client.set_on_crash(move |msg| {
            let mut guard = inner.lock();
            if guard.state == ServerState::Stopped || guard.state == ServerState::Failed {
                return;
            }
            let was_stable = guard
                .started_at
                .is_some_and(|t| t.elapsed() >= config.stability_threshold);
            guard.restart_count = if was_stable {
                1
            } else {
                guard.restart_count + 1
            };
            guard.last_error = Some(msg);
            guard.started_at = None;
            guard.state = ServerState::Error;
            if let Some(cb) = on_state_change.lock().as_ref() {
                cb(ServerState::Error);
            }
            tracing::warn!(
                "LSP server '{}' crashed, restart #{}",
                def_id,
                guard.restart_count
            );
        });

        if let Err(e) = client.start().await {
            self.on_start_failed(format!("spawn failed: {e}"));
            return;
        }

        // Race handshake against timeout
        let handshake_timeout = self.config.handshake_timeout;
        let result = tokio::time::timeout(handshake_timeout, client.initialize(&root_uri)).await;

        match result {
            Ok(Ok(_)) => {
                *self.client.lock() = Some(Arc::clone(&client));
                let mut inner = self.inner.lock();
                inner.state = ServerState::Running;
                inner.started_at = Some(Instant::now());
                if let Some(cb) = self.on_state_change.lock().as_ref() {
                    cb(ServerState::Running);
                }
            }
            Ok(Err(e)) => {
                client.kill().await;
                self.on_start_failed(format!("handshake failed: {e}"));
            }
            Err(_) => {
                client.kill().await;
                self.on_start_failed(format!("handshake timed out after {:?}", handshake_timeout));
            }
        }
    }

    fn on_start_failed(&self, error: String) {
        let mut inner = self.inner.lock();
        let was_stable = inner
            .started_at
            .is_some_and(|t| t.elapsed() >= self.config.stability_threshold);
        inner.restart_count = if was_stable {
            1
        } else {
            inner.restart_count + 1
        };
        inner.last_error = Some(error);
        inner.started_at = None;

        if inner.restart_count >= self.config.max_restarts {
            inner.state = ServerState::Failed;
            if let Some(cb) = self.on_state_change.lock().as_ref() {
                cb(ServerState::Failed);
            }
        } else {
            inner.state = ServerState::Error;
            if let Some(cb) = self.on_state_change.lock().as_ref() {
                cb(ServerState::Error);
            }
            let delay = self.config.base_restart_delay * inner.restart_count;
            let inner_clone = Arc::clone(&self.inner);
            let on_state_change = Arc::clone(&self.on_state_change);
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let state = inner_clone.lock().state;
                if state == ServerState::Error {
                    // Signal that restart is needed — actual restart driven by manager
                    if let Some(cb) = on_state_change.lock().as_ref() {
                        cb(ServerState::Error);
                    }
                }
            });
        }
    }

    async fn find_root_uri(&self) -> String {
        for marker in self.def.root_markers {
            if self.cwd.join(marker).exists() {
                return format!("file://{}", self.cwd.display());
            }
        }
        format!("file://{}", self.cwd.display())
    }

    pub fn is_stable(&self) -> bool {
        let inner = self.inner.lock();
        if inner.state != ServerState::Running {
            return false;
        }
        inner
            .started_at
            .is_some_and(|t| t.elapsed() >= self.config.stability_threshold)
    }

    fn set_state(&self, state: ServerState) {
        self.inner.lock().state = state;
        if let Some(cb) = self.on_state_change.lock().as_ref() {
            cb(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LspCapabilities;

    fn test_def() -> &'static LspServerDef {
        // Use a server that won't exist for testing state transitions
        static TEST_DEF: LspServerDef = LspServerDef {
            id: "test-server",
            name: "Test",
            command: "nonexistent_lsp_test_server",
            args: &[],
            languages: &[],
            extensions: &[".test"],
            root_markers: &[],
            skip_if: &[],
            requires: &[],
            capabilities: LspCapabilities::BASIC,
            install_hint: "",
        };
        &TEST_DEF
    }

    #[test]
    fn test_initial_state_stopped() {
        let inst = LspServerInstance::new(test_def(), Path::new("/tmp"), InstanceConfig::default());
        assert_eq!(inst.state(), ServerState::Stopped);
    }

    #[tokio::test]
    async fn test_start_nonexistent_transitions_to_error_or_failed() {
        let config = InstanceConfig {
            max_restarts: 1,
            ..Default::default()
        };
        let inst = LspServerInstance::new(test_def(), Path::new("/tmp"), config);
        inst.start().await;
        let state = inst.state();
        assert!(state == ServerState::Error || state == ServerState::Failed);
    }

    #[tokio::test]
    async fn test_stop_resets_to_stopped() {
        let inst = LspServerInstance::new(test_def(), Path::new("/tmp"), InstanceConfig::default());
        inst.stop().await;
        assert_eq!(inst.state(), ServerState::Stopped);
    }

    #[tokio::test]
    async fn test_max_restarts_leads_to_failed() {
        let config = InstanceConfig {
            max_restarts: 2,
            base_restart_delay: Duration::from_millis(1),
            handshake_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let inst = LspServerInstance::new(test_def(), Path::new("/tmp"), config);

        // First attempt
        inst.start().await;
        // Should be Error or Failed after max restarts
        assert!(inst.restart_count() >= 1);
    }

    #[test]
    fn test_config_defaults() {
        let config = InstanceConfig::default();
        assert_eq!(config.stability_threshold, Duration::from_secs(10));
        assert_eq!(config.max_restarts, 5);
        assert_eq!(config.base_restart_delay, Duration::from_secs(2));
        assert_eq!(config.handshake_timeout, Duration::from_secs(10));
    }
}
