//! [`Labeller`] trait and [`run_labeller`] driver.
//!
//! Concrete labellers (`scope-audit-labeller-noop` in this sprint;
//! `-llm` / `-lsp` / `-hybrid` in sprints 0010-0012) implement [`Labeller`]
//! and call [`run_labeller`] to drive a reader / writer pair.

use std::io::{BufRead, Write};

use crate::io::{read_records, write_record, ParseError};
use crate::record::SampleRecord;

/// One concrete labeller. Stateless or stateful, the trait commits to
/// per-record application: `label_one` consumes one record and returns
/// one record. Composability (e.g. the hybrid composer in sprint 0012)
/// stacks labellers by holding inner ones and forwarding records.
pub trait Labeller {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Apply this labeller to a single record. The returned record carries
    /// any of the seven labeller-fillable fields the labeller chose to
    /// populate plus [`Self::labeller_id`] stamped into the `labeller_id`
    /// field; other fields are passed through.
    fn label_one(&mut self, record: SampleRecord) -> Result<SampleRecord, Self::Error>;

    /// The stable identifier this labeller writes into the `labeller_id`
    /// field of every record it processes. Convention: `<kind>:<recipe>`
    /// where `kind` is one of `noop` / `human` / `llm` / `lsp` / `hybrid`.
    fn labeller_id(&self) -> &str;
}

/// Tallies for one `run_labeller` invocation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunStats {
    /// Records the labeller successfully processed (a `label_one` `Ok`).
    pub records_labelled: usize,
    /// Records the iterator skipped because they were blank / comment lines.
    /// Always `0` from `run_labeller`'s perspective — the iterator drops them
    /// before the labeller sees a row. Retained as a column for future
    /// telemetry that may pipe its own counters in.
    pub records_skipped: usize,
}

/// Top-level run error: either the input stream rejects parse, or the
/// labeller surfaces an error on a record.
#[derive(Debug, thiserror::Error)]
pub enum RunError<E: std::error::Error + Send + Sync + 'static> {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("labeller error: {0}")]
    Labeller(#[source] E),
    #[error("io error writing record: {0}")]
    Io(#[from] std::io::Error),
}

/// Drive `labeller` across every record in `reader`, writing labelled
/// records to `writer`. Errors propagate eagerly — the first parse or
/// labeller error aborts the run. Callers wanting best-effort behaviour
/// should consume [`read_records`] directly and handle errors per-record.
pub fn run_labeller<L, R, W>(
    labeller: &mut L,
    reader: R,
    mut writer: W,
) -> Result<RunStats, RunError<L::Error>>
where
    L: Labeller,
    R: BufRead,
    W: Write,
{
    let mut stats = RunStats::default();
    for parsed in read_records(reader) {
        let record = parsed?;
        let labelled = labeller.label_one(record).map_err(RunError::Labeller)?;
        write_record(&mut writer, &labelled)?;
        stats.records_labelled += 1;
    }
    // Explicit flush before reporting success. `BufWriter`'s `Drop` impl
    // swallows late flush errors (e.g. broken pipe, full filesystem) so a
    // caller that uses a buffered writer would otherwise see `Ok(stats)`
    // while the final bytes never reached the underlying writer.
    writer.flush()?;
    Ok(stats)
}
