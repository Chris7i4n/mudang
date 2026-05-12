//! `scope source` view — JSON payload for `--json` output.

use serde::Serialize;

/// Typed `data` payload for `scope source --json`.
#[derive(Debug, Clone, Serialize)]
pub struct SourceView<'a> {
    pub symbol: &'a str,
    pub kind: &'a str,
    pub file_path: &'a str,
    pub line_start: u32,
    pub line_end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<&'a str>,
    pub source: String,
}
