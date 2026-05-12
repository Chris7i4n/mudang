//! `scope index` view types.
//!
//! Three single-shot output shapes (full / incremental-up-to-date /
//! incremental-result) and one event-stream sum type (`IndexEvent` —
//! `start` / `reindex` / `stop`) used while `scope index --watch` is
//! running.
//!
//! These wrap the `scope-index` stats types (`IndexStats`,
//! `IncrementalStats`, `LanguageStats`) without requiring `Serialize`
//! upstream. The CLI owns the JSON shape.

use serde::Serialize;

/// Output for a full index run.
#[derive(Debug, Clone, Serialize)]
pub struct IndexFullResult<'a> {
    pub command: &'static str,
    pub mode: &'static str,
    pub file_count: usize,
    pub symbol_count: usize,
    pub edge_count: usize,
    pub duration_secs: f64,
    pub languages: Vec<IndexLanguageStat<'a>>,
}

/// Per-language row in `IndexFullResult`.
#[derive(Debug, Clone, Serialize)]
pub struct IndexLanguageStat<'a> {
    pub language: &'a str,
    pub file_count: usize,
    pub symbol_count: usize,
}

/// Short output when an incremental index finds nothing to do.
#[derive(Debug, Clone, Serialize)]
pub struct IndexIncrementalUpToDate {
    pub command: &'static str,
    pub mode: &'static str,
    pub up_to_date: bool,
}

/// Output for an incremental index run that actually re-indexed.
#[derive(Debug, Clone, Serialize)]
pub struct IndexIncrementalResult<'a> {
    pub command: &'static str,
    pub mode: &'static str,
    pub up_to_date: bool,
    pub modified: &'a [String],
    pub added: &'a [String],
    pub deleted: &'a [String],
    pub symbol_count: usize,
    pub edge_count: usize,
    pub duration_secs: f64,
}

/// `scope index --watch --json` event stream — one event per line of
/// stdout. Discriminated by `event` tag.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum IndexEvent<'a> {
    Start(IndexStartEvent<'a>),
    Reindex(IndexReindexEvent),
    Stop(IndexStopEvent),
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexStartEvent<'a> {
    pub project: &'a str,
    pub languages: &'a [String],
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexReindexEvent {
    pub files_changed: usize,
    pub symbols_added: usize,
    pub symbols_removed: usize,
    pub edges_added: usize,
    pub edges_removed: usize,
    pub duration_ms: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexStopEvent {
    pub total_reindexes: u64,
    pub total_files_processed: u64,
    pub uptime_seconds: u64,
    pub timestamp: String,
}
