/// `scope audit` — confidence and coverage audits over the indexed graph.
///
/// Subcommands:
///   confidence  — precision report per (kind, tier, producer, pattern_id)
///                 against the reference fixture corpus.
///
/// `scope audit coverage` is explicitly post-refactor — see
/// `POST-REFACTOR-PLAN.md` § Items deliberately deferred.
use anyhow::Result;
use clap::{Args, Subcommand};

/// Disclaimer printed in `--help` and as the first line of every report.
///
/// Stating this verbatim in both surfaces is a sprint 0007 deliverable
/// (see `docs/sprints/0007-phase-d-confidence-audit.md` → Deliverables).
pub const PRECISION_ONLY_DISCLAIMER: &str =
    "precision report; recall is measured by integration-test snapshots, not this subcommand.";

/// Pointer printed alongside the disclaimer so external labeller authors
/// discover the JSONL contract from the CLI itself.
pub const SCHEMA_DOC_POINTER: &str =
    "Sample-file schema: docs/AUDIT-LABEL-SCHEMA.md (schema_version \"1\").";

/// `scope audit` — top-level dispatch for audit subcommands.
#[derive(Args, Debug)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub command: AuditCommands,
}

/// Audit subcommands.
#[derive(Subcommand, Debug)]
pub enum AuditCommands {
    /// precision report; recall is measured by integration-test snapshots, not this subcommand.
    ///
    /// Samples edges per (kind, confidence) cell, asks an external
    /// labeller (human, LLM, LSP cross-check, hybrid) to fill labels
    /// via a JSONL sample file, then computes precision per
    /// (kind, tier, producer, pattern_id) and enforces tier targets
    /// (high >= 95%, medium >= 70%, low has no minimum).
    ///
    /// Sample file format: see `docs/AUDIT-LABEL-SCHEMA.md`. Scope spawns
    /// no labeller subprocess — the JSONL file is the contract for any
    /// external labeller plugged into the loop.
    ///
    /// Examples:
    ///   scope audit confidence
    ///   scope audit confidence --emit-sample sample.jsonl
    ///   scope audit confidence --label sample.jsonl --format tsv
    Confidence(ConfidenceArgs),
}

/// Arguments for `scope audit confidence`.
///
/// Sprint 0007 lands the clap surface and the precision-only disclaimer
/// in chunk 2. Flags (`--sample-size`, `--seed`, `--emit-sample`,
/// `--label`, `--format`) land in subsequent chunks as their behavior
/// arrives.
#[derive(Args, Debug)]
pub struct ConfidenceArgs {}

pub fn run(args: &AuditArgs, _project_root: &std::path::Path) -> Result<()> {
    match &args.command {
        AuditCommands::Confidence(_confidence_args) => run_confidence(),
    }
}

fn run_confidence() -> Result<()> {
    println!("# scope audit confidence");
    println!("# {PRECISION_ONLY_DISCLAIMER}");
    println!("# {SCHEMA_DOC_POINTER}");
    println!();
    anyhow::bail!(
        "audit confidence: execution logic lands in sprint 0007 chunks 3-6 \
         (sampling, two-phase labelling, JSON/TSV writers, tier gate). \
         The clap surface and disclaimer wiring are in place; see \
         `docs/sprints/0007-phase-d-confidence-audit.md` for the chunk plan."
    )
}
