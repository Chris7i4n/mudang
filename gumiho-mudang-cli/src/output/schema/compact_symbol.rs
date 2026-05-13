//! `CompactSymbol<'a>` — borrowed projection of `Symbol` for `--compact`.
//!
//! Strips `id`, `docstring`, `parent_id`, `language`, `metadata`. The
//! remaining six fields are what LLM agents actually need: identity
//! (`name`, `kind`, `signature`) and location (`file_path`,
//! `line_start`, `line_end`).
//!
//! The struct borrows from a parent `Symbol`; no allocation. The
//! conversion is `CompactSymbol::from(&symbol)`.

use gumiho_mudang_scope::graph::Symbol;
use serde::Serialize;

/// Compact projection of a `Symbol` — six fields, all borrowed.
#[derive(Debug, Clone, Serialize)]
pub struct CompactSymbol<'a> {
    /// The symbol name.
    pub name: &'a str,
    /// Symbol kind ("class", "method", etc.).
    pub kind: &'a str,
    /// Type signature where available. Serialises as JSON `null` when
    /// the symbol has no signature — the before R10 `serde_json::json!()`
    /// output always emitted the key, so the typed shape preserves the
    /// same wire contract .
    pub signature: Option<&'a str>,
    /// File path, forward-slash normalized.
    pub file_path: &'a str,
    /// First line of the definition (1-based).
    pub line_start: u32,
    /// Last line of the definition (1-based).
    pub line_end: u32,
}

impl<'a> From<&'a Symbol> for CompactSymbol<'a> {
    fn from(symbol: &'a Symbol) -> Self {
        Self {
            name: &symbol.name,
            kind: &symbol.kind,
            signature: symbol.signature.as_deref(),
            file_path: &symbol.file_path,
            line_start: symbol.line_start,
            line_end: symbol.line_end,
        }
    }
}
