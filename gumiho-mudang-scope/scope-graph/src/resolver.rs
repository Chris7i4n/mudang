//! Phase A resolver stub.
//!
//! **STUB:phase-a-resolver — retired by R3 (sprint 0003).**
//!
//! Converts a `RawEdge` to an `InsertableEdge` by name-looking-up
//! `to_id` against the workspace symbols table. Assigns:
//!   * `Status::Resolved`  — exactly one row matches.
//!   * `Status::Ambiguous` — more than one row matches.
//!   * `Status::Dangling`  — zero rows match.
//!
//! This stub does **not** consult `LanguageWorkspaceContext` (which
//! lands in R4), and is **not** a baseline for R8's audit. R3
//! replaces both the call sites and this module wholesale per
//! `docs/REFACTOR-STATUS.md` § Stubs outstanding. No Phase B sprint
//! may extend or patch this stub in place — only the retiring
//! R-move (R3) replaces it.

use anyhow::Result;
use rusqlite::Connection;
use scope_core::{InsertableEdge, RawEdge, Status};

/// Phase A resolver. See module docs.
pub fn resolve_stub(conn: &Connection, raw: RawEdge) -> Result<InsertableEdge> {
    let to_id = raw.to_id();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM symbols WHERE id = ?1",
        rusqlite::params![to_id],
        |row| row.get(0),
    )?;

    let status = match count {
        0 => Status::Dangling,
        1 => Status::Resolved,
        _ => Status::Ambiguous,
    };

    Ok(InsertableEdge::__phase_a_new(raw, status))
}

/// Batch helper for `insert_file_data`-style call sites that hand the
/// stub a vector of `RawEdge`s prepared by the extractor.
pub fn resolve_stub_batch(conn: &Connection, raws: Vec<RawEdge>) -> Result<Vec<InsertableEdge>> {
    raws.into_iter()
        .map(|raw| resolve_stub(conn, raw))
        .collect()
}
