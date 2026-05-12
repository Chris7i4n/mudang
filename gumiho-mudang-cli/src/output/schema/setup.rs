//! `scope setup` view — `data` payload inside the `JsonOutput<T>`
//! envelope.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SetupResult {
    pub initialized: bool,
    pub indexed: bool,
    pub preloaded: bool,
    pub claude_md_updated: bool,
    pub skill_installed: bool,
    pub scope_dir: &'static str,
}
