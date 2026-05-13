//! Scope's search backend. Today: FTS5 over the SQLite graph.
//!
//! Future (mudang Phase D) will add a LanceDB vector-search backend
//! and a `Searcher` trait splitting the two. Trait + LanceDB
//! landing is governed by
//! `docs/todos/0004-onnx-and-lancedb-distinction.md`.

pub mod embedder;
pub mod searcher;
