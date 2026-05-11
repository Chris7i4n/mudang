//! Scope's indexing pipeline.
//!
//! Orchestrates full and incremental indexing, owns the file-hash
//! table, the SHA-256 pipeline, the embedding text builder, and the
//! file watcher.

pub mod embedder;
pub mod indexer;
pub mod watcher;
