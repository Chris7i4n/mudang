//! `scope-audit-labeller-llm` binary.
//!
//! Reads v2 JSONL records from stdin, applies an LLM-backed labeller,
//! writes labelled records to stdout. Provider is selected at compile
//! time via cargo features; this sprint ships the `deepseek` feature
//! (default). The provider needs `DEEPSEEK_API_KEY` in the environment.
//!
//! Pipeline integration mirrors `scope-audit-labeller-noop` — same
//! stdin / stdout contract documented in
//! `gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md`.

use std::io::{stdin, stdout, BufReader, BufWriter};
use std::process::ExitCode;

use scope_audit_labeller_core::run_labeller;
use scope_audit_labeller_llm::LlmLabeller;

#[cfg(feature = "deepseek")]
use scope_audit_labeller_llm::providers::deepseek::DeepSeekProvider;

fn main() -> ExitCode {
    let reader = BufReader::new(stdin().lock());
    let writer = BufWriter::new(stdout().lock());

    #[cfg(feature = "deepseek")]
    {
        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => p,
            Err(err) => {
                eprintln!("scope-audit-labeller-llm: {err}");
                return ExitCode::from(2);
            }
        };
        let mut labeller = LlmLabeller::new(provider);
        match run_labeller(&mut labeller, reader, writer) {
            Ok(stats) => {
                eprintln!(
                    "scope-audit-labeller-llm: labelled {} records",
                    stats.records_labelled
                );
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("scope-audit-labeller-llm: {err}");
                ExitCode::FAILURE
            }
        }
    }

    #[cfg(not(feature = "deepseek"))]
    {
        // Mark the imports as used so this branch compiles when every
        // provider feature is disabled. The binary is not meaningfully
        // useful in that configuration — flag a clear error.
        let _ = (reader, writer);
        let _phantom: Option<LlmLabeller<scope_audit_labeller_llm::MockProvider>> = None;
        eprintln!(
            "scope-audit-labeller-llm: built with no provider feature enabled; \
             nothing to do. Rebuild with --features deepseek."
        );
        ExitCode::from(2)
    }
}
