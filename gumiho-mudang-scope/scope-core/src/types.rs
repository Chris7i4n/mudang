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

/// Legacy `Edge` name re-aliased to [`crate::edge::RawEdge`] (R1).
///
/// Plugin return types and the façade re-export keep the historical
/// name; production code holds either a `RawEdge` (extractor output)
/// or an `InsertableEdge` (resolver output). External `Edge { … }`
/// struct-literal construction is a compile error because the
/// underlying `RawEdge` has `pub(crate)` fields; the only entry point
/// is [`Edge::builder`] (provided as an inherent fn on `RawEdge`).
pub type Edge = crate::edge::RawEdge;
