use std::path::Path;

use crate::registry::LspServerDef;

pub fn is_installed(def: &LspServerDef) -> bool {
    if which::which(def.command).is_err() {
        return false;
    }
    for bin in def.requires {
        if which::which(bin).is_err() {
            return false;
        }
    }
    true
}

pub fn detect_installed(servers: &[LspServerDef]) -> Vec<&LspServerDef> {
    servers.iter().filter(|s| is_installed(s)).collect()
}

pub async fn filter_for_project<'a>(
    servers: &[&'a LspServerDef],
    project_root: &Path,
) -> Vec<&'a LspServerDef> {
    let mut result = Vec::new();
    for def in servers {
        if def.skip_if.is_empty() {
            result.push(*def);
            continue;
        }
        let mut skip = false;
        for marker in def.skip_if {
            if project_root.join(marker).exists() {
                skip = true;
                break;
            }
        }
        if !skip {
            result.push(*def);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::KNOWN_SERVERS;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_known_binary() {
        // `ls` or `cat` should exist on any unix system
        let fake_def = LspServerDef {
            id: "test",
            name: "Test",
            command: "ls",
            args: &[],
            languages: &[],
            extensions: &[],
            root_markers: &[],
            skip_if: &[],
            requires: &[],
            capabilities: crate::types::LspCapabilities::BASIC,
            install_hint: "",
        };
        assert!(is_installed(&fake_def));
    }

    #[test]
    fn test_filter_out_missing() {
        let fake_def = LspServerDef {
            id: "nonexistent",
            name: "Nonexistent",
            command: "absolutely_nonexistent_binary_xyz_123",
            args: &[],
            languages: &[],
            extensions: &[],
            root_markers: &[],
            skip_if: &[],
            requires: &[],
            capabilities: crate::types::LspCapabilities::BASIC,
            install_hint: "",
        };
        let servers = [fake_def];
        let result = detect_installed(&servers);
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_skip_if_marker() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("deno.json"), "{}").unwrap();

        let ts_server = KNOWN_SERVERS.iter().find(|s| s.id == "typescript").unwrap();
        let servers: Vec<&LspServerDef> = vec![ts_server];
        let filtered = filter_for_project(&servers, tmp.path()).await;
        assert!(filtered.is_empty());
    }
}
