//! `Summary` view — typed shape for `scope summary`.
//!
//! Two variants:
//! - [`Summary::Symbol`] — one-line view of a class / method / function
//!   / interface / enum / const / type.
//! - [`Summary::File`] — file-level view (symbol count + top-level
//!   names).
//!
//! The enum serialises with `#[serde(tag = "summary_kind")]`. JSON
//! consumers branch on `"symbol"` / `"file"`.

use serde::Serialize;

/// `scope summary` JSON output — sum type over the two summary
/// shapes.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "summary_kind", rename_all = "lowercase")]
pub enum Summary<'a> {
    Symbol(SymbolSummary<'a>),
    File(FileSummary<'a>),
}

/// Symbol-level summary — one-line view.
#[derive(Debug, Clone, Serialize)]
pub struct SymbolSummary<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub file_path: &'a str,
    pub line_start: u32,
    pub line_end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<&'a str>,
    pub callers: usize,
    pub outgoing_calls: usize,
    pub methods: usize,
}

/// File-level summary.
#[derive(Debug, Clone, Serialize)]
pub struct FileSummary<'a> {
    pub file_path: &'a str,
    pub symbol_count: usize,
    pub top_level: Vec<String>,
}
