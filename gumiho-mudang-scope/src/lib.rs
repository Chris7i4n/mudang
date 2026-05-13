//! Syntactic engine for gumiho-mudang.
//!
//! Façade crate. The implementation lives in five sub-crates nested
//! under this directory; this crate re-exports their public surface
//! as `gumiho_mudang_scope::{graph, indexer, parser, searcher,
//! workspace_graph, …}` so CLI callers reach the engine through a
//! single dependency.
//!
//! Decomposition map (see `docs/todos/0006-split-scope-crate.md`):
//!
//! - `scope-core` — tree-sitter parser, language plugins, `Symbol`
//!   and edge types (`RawEdge`), project / workspace config.
//! - `scope-index` — full / incremental indexing pipeline, file-hash
//!   table, SHA-256 ingestion, file watcher.
//! - `scope-graph` — SQLite-backed graph storage and recursive
//!   queries (`find_refs`, `find_impact`, `find_deps`,
//!   `find_call_paths`, `find_flow_paths`).
//! - `scope-search` — FTS5 search, embedding text builder.
//! - `scope-workspace` — federated workspace facade.

pub use scope_core::{config, languages, parser, types, workspace};
pub use scope_core::{RawEdge, Symbol};
pub use scope_graph::graph;
pub use scope_index::{indexer, watcher};
pub use scope_search::{embedder, searcher};
pub use scope_workspace::workspace_graph;
