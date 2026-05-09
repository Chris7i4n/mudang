use crate::instance::ServerState;

#[derive(Debug, Clone)]
pub struct ServerStatusEntry {
    pub id: String,
    pub name: String,
    pub state: ServerState,
}

static KNOWN_SHORT: &[(&str, &str)] = &[
    ("typescript", "ts"),
    ("gopls", "go"),
    ("rust-analyzer", "rs"),
    ("clangd", "c++"),
    ("sourcekit-lsp", "swift"),
    ("bash-language-server", "sh"),
    ("lua-language-server", "lua"),
    ("kotlin-lsp", "kt"),
    ("kotlin-language-server", "kt"),
    ("pyright", "pyr"),
    ("pylsp", "py"),
    ("svelte", "svel"),
    ("ruby-lsp", "ruby"),
    ("dart", "dart"),
    ("vue", "vue"),
];

pub fn short_server_id(id: &str) -> &str {
    for (full, short) in KNOWN_SHORT {
        if *full == id {
            return short;
        }
    }
    // Fallback: first segment before "-", max 4 chars
    let segment = id.split('-').next().unwrap_or(id);
    if segment.len() > 4 {
        &segment[..4]
    } else {
        segment
    }
}

pub fn status_symbol(state: ServerState) -> &'static str {
    match state {
        ServerState::Running => "●",
        ServerState::Starting => "◌",
        ServerState::Error => "↺",
        ServerState::Failed => "✕",
        ServerState::Stopped => "○",
    }
}

pub fn format_status_widget(entries: &[ServerStatusEntry]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }

    let parts: Vec<String> = entries
        .iter()
        .map(|e| {
            let short = short_server_id(&e.id);
            let symbol = status_symbol(e.state);
            format!("{short} {symbol}")
        })
        .collect();

    Some(format!("LSP  {}", parts.join("  ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_server_id_known() {
        assert_eq!(short_server_id("rust-analyzer"), "rs");
        assert_eq!(short_server_id("typescript"), "ts");
        assert_eq!(short_server_id("gopls"), "go");
    }

    #[test]
    fn test_short_server_id_fallback() {
        assert_eq!(short_server_id("unknown-server"), "unkn");
        assert_eq!(short_server_id("foo"), "foo");
    }

    #[test]
    fn test_status_symbol() {
        assert_eq!(status_symbol(ServerState::Running), "●");
        assert_eq!(status_symbol(ServerState::Starting), "◌");
        assert_eq!(status_symbol(ServerState::Error), "↺");
        assert_eq!(status_symbol(ServerState::Failed), "✕");
        assert_eq!(status_symbol(ServerState::Stopped), "○");
    }

    #[test]
    fn test_format_widget_empty() {
        assert_eq!(format_status_widget(&[]), None);
    }

    #[test]
    fn test_format_widget() {
        let entries = vec![
            ServerStatusEntry {
                id: "rust-analyzer".into(),
                name: "Rust".into(),
                state: ServerState::Running,
            },
            ServerStatusEntry {
                id: "typescript".into(),
                name: "TypeScript".into(),
                state: ServerState::Starting,
            },
        ];
        let result = format_status_widget(&entries).unwrap();
        assert_eq!(result, "LSP  rs ●  ts ◌");
    }
}
