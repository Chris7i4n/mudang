//! Semantic oracle for gumiho-mudang.
//!
//! Migrated from the standalone `gumiho-lsp` crate. Owns LSP server
//! lifecycle (spawn, handshake, crash recovery, restart with stability
//! threshold), capability probing, file sync (didOpen/didChange/...), and
//! generic request dispatch. Knows nothing about gumiho-mudang's schema
//! or routing — translation lives in the CLI orchestration layer.
//!
//! Pending migration. Skeleton only.
