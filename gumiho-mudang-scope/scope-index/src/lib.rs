//! Scope's indexing pipeline.
//!
//! Orchestrates full and incremental indexing, owns the file-hash
//! table, the SHA-256 pipeline, and the file watcher. The embedding
//! text builder lives in `scope-search` (its sole consumer) per TODO
//! 0006 § Affected code § embedder.rs.

pub mod indexer;
pub mod watcher;
