//! `scope-audit-labeller-noop` — binary entry point.
//!
//! Streams JSONL records from stdin to stdout, applying [`NoopLabeller`]
//! to each record. Errors (parse, IO) are reported on stderr and produce
//! a non-zero exit.

use std::io::{stdin, stdout, BufReader, BufWriter};
use std::process::ExitCode;

use scope_audit_labeller_core::run_labeller;
use scope_audit_labeller_noop::NoopLabeller;

fn main() -> ExitCode {
    let mut labeller = NoopLabeller;
    let reader = BufReader::new(stdin().lock());
    let writer = BufWriter::new(stdout().lock());
    match run_labeller(&mut labeller, reader, writer) {
        Ok(stats) => {
            eprintln!("noop: labelled {} record(s)", stats.records_labelled);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("noop: {err}");
            ExitCode::FAILURE
        }
    }
}
