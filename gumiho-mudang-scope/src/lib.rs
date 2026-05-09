//! Syntactic engine for gumiho-mudang.
//!
//! Migrated from the legacy `scope` codebase. Owns tree-sitter parsing,
//! per-language plugins, the SQLite graph, and FTS5 search. No LSP, no
//! semantic resolution — those live in `gumiho-mudang-lsp` and the
//! orchestration layer in `gumiho-mudang-cli`.
//!
//! Pending migration. Skeleton only.
