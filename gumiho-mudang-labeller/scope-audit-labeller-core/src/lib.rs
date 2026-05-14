//! Shared core for Scope audit labellers.
//!
//! Consumes only the v2 JSONL contract documented in
//! `gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md`. No path or workspace
//! dependency on any Scope crate — the contract is the schema doc; the
//! types here are a faithful duplicate maintained in the labeller workspace.
//!
//! The split:
//!
//! - [`SampleRecord`] — the v2 wire shape. Field order matches the schema
//!   doc § Record schema table so `serde_json::to_string` emits the canonical
//!   ordering.
//! - [`Labeller`] — trait every concrete labeller implements. One method
//!   per record; composable.
//! - [`read_records`] / [`write_record`] — JSONL IO helpers. Read tolerates
//!   blank lines and `#` comments and enforces `schema_version == "2"` per
//!   record (rejects on mismatch with the same diagnostic Scope's CLI emits
//!   in `label_pass`).
//! - [`run_labeller`] — drives a labeller across a reader / writer pair and
//!   returns per-run counts.

pub mod io;
pub mod record;
pub mod runner;

pub use io::{read_records, write_record, ParseError, RecordIter};
pub use record::{SampleRecord, SCHEMA_VERSION};
pub use runner::{run_labeller, Labeller, RunError, RunStats};
