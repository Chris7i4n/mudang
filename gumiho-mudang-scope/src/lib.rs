//! Syntactic engine for gumiho-mudang.
//!
//! Tree-sitter parsing, per-language plugins, the SQLite graph,
//! and FTS5 search. No LSP, no semantic resolution — those live in
//! `gumiho-mudang-lsp`. User-facing commands and output formatting
//! live in `gumiho-mudang-cli`.

pub mod config;
pub mod core;
pub mod languages;
