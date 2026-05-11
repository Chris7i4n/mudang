//! Core types shared across scope sub-crates.
//!
//! `Symbol` and `Edge` are the type backbone of parser output,
//! language-plugin signatures, indexer pipeline, graph storage, and
//! search results. They live in `scope-core` so that
//! `gumiho-mudang-edit` (phase E) can depend on `scope-core` alone
//! without inheriting SQLite via `scope-graph`.

use serde::{Deserialize, Serialize};

/// A code symbol extracted from source and stored in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Unique identifier: `"{file_path}::{name}::{kind}"`.
    pub id: String,
    /// The symbol name (e.g. `PaymentService`, `processPayment`).
    pub name: String,
    /// The kind of symbol (function, class, method, etc.).
    pub kind: String,
    /// File path relative to project root, always forward slashes.
    pub file_path: String,
    /// First line of the symbol definition (1-based).
    pub line_start: u32,
    /// Last line of the symbol definition (1-based).
    pub line_end: u32,
    /// Full type signature where available.
    pub signature: Option<String>,
    /// Extracted doc comment.
    pub docstring: Option<String>,
    /// Parent symbol ID (e.g. class ID for a method).
    pub parent_id: Option<String>,
    /// Source language.
    pub language: String,
    /// JSON blob with modifiers, parameters, return type, etc.
    pub metadata: String,
}

/// A relationship between two symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source symbol ID.
    pub from_id: String,
    /// Target symbol ID (may reference external symbols not in the index).
    pub to_id: String,
    /// Edge kind: calls, imports, extends, implements, instantiates, references, references_type.
    pub kind: String,
    /// File where this edge was observed.
    pub file_path: String,
    /// Line number where the edge was observed.
    pub line: Option<u32>,
}
