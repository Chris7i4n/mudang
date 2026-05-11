//! Syntactic engine for gumiho-mudang.
//!
//! Façade crate. The implementation lives in five sub-crates nested
//! under this directory; this crate re-exports their public surface so
//! existing consumers (`gumiho-mudang-cli`, the future composer, third
//! parties) continue to compile against `gumiho_mudang_scope::*`
//! without source changes.
//!
//! Decomposition map (see `docs/todos/0006-split-scope-crate.md`):
//!
//! - `scope-core` — tree-sitter parser, language plugins, `Symbol` /
//!   `Edge` types, project / workspace config.
//! - `scope-index` — full / incremental indexing pipeline, file-hash
//!   table, SHA-256 ingestion, file watcher.
//! - `scope-graph` — SQLite-backed graph storage and recursive
//!   queries (`find_refs`, `find_impact`, `find_deps`,
//!   `find_call_paths`, `find_flow_paths`).
//! - `scope-search` — FTS5 search, embedding text builder. (Phase D
//!   adds LanceDB + a `Searcher` trait split.)
//! - `scope-workspace` — federated workspace facade.

// Backwards-compatible `core` module: pre-split callers reach for
// `gumiho_mudang_scope::core::{parser, graph, indexer, searcher,
// embedder, workspace_graph, watcher}`. Synthesise that namespace from
// the sub-crates so existing imports continue to resolve.
pub mod core {
    pub use scope_core::parser;
    pub use scope_graph::graph;
    pub use scope_index::{indexer, watcher};
    pub use scope_search::{embedder, searcher};
    pub use scope_workspace::workspace_graph;
}

pub use scope_core::{config, languages, parser, types};
pub use scope_core::{Edge, Symbol};
pub use scope_graph::graph;
pub use scope_index::{indexer, watcher};
pub use scope_search::{embedder, searcher};
pub use scope_workspace::workspace_graph;

// Sub-crate roots for fully qualified access
// (e.g. `gumiho_mudang_scope::scope_core::parser::...`).
pub use scope_core as scope_core_crate;
pub use scope_graph as scope_graph_crate;
pub use scope_index as scope_index_crate;
pub use scope_search as scope_search_crate;
pub use scope_workspace as scope_workspace_crate;
