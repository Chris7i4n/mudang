/// `scope audit` — confidence and coverage audits over the indexed graph.
///
/// Subcommands:
///   confidence  — precision report per (kind, tier, producer, pattern_id)
///                 against the reference fixture corpus.
///
/// `scope audit coverage` is explicitly post-refactor — see
/// `POST-REFACTOR-PLAN.md` § Items deliberately deferred.
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::collections::BTreeMap;
use std::path::Path;

use gumiho_mudang_scope::core::graph::{AuditEdgeRow, Graph};

/// Default sample size per `(kind, confidence)` cell.
///
/// Per pre-Phase-D ambiguity #2 resolution: N = 30 hits the 0.05-margin
/// statistical sweet spot for high-tier precision (95% target).
/// Overridable via `--sample-size`.
pub const DEFAULT_SAMPLE_SIZE: usize = 30;

/// Default PRNG seed for stratified sampling.
///
/// Pinned at compile time so the audit is reproducible across runs:
/// `scope audit confidence` with no `--seed` samples the same edges
/// from the same indexed graph. The literal value is arbitrary;
/// reproducibility comes from it being a const, not from the specific
/// bits. Overridable via `--seed`.
pub const DEFAULT_SEED: u64 = 0xA5C0_DE17_5EED_0001;

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
#[derive(Args, Debug)]
pub struct ConfidenceArgs {
    /// Number of edges to sample per `(kind, confidence)` cell.
    ///
    /// Defaults to 30 — the pre-Phase-D ambiguity #2 resolution
    /// statistical sweet spot for high-tier (95% target) precision.
    /// Cells with fewer than this many edges are taken in full.
    #[arg(long, default_value_t = DEFAULT_SAMPLE_SIZE)]
    pub sample_size: usize,

    /// PRNG seed for the deterministic stratified sampler.
    ///
    /// Same seed + same indexed graph + same `--sample-size` =>
    /// identical sample set. Defaults to a pinned compile-time
    /// constant so unsupplied runs are reproducible. Override when
    /// you want to sample a different subset of the same graph.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    pub seed: u64,
}

pub fn run(args: &AuditArgs, project_root: &Path) -> Result<()> {
    match &args.command {
        AuditCommands::Confidence(c) => run_confidence(c, project_root),
    }
}

fn run_confidence(args: &ConfidenceArgs, project_root: &Path) -> Result<()> {
    let db_path = project_root.join(".scope").join("graph.db");
    if !db_path.exists() {
        anyhow::bail!(
            "no index found at {}. Run `scope index` first.",
            db_path.display()
        );
    }

    let graph = Graph::open(&db_path)
        .with_context(|| format!("failed to open index at {}", db_path.display()))?;

    let rows = graph.list_edges_for_audit()?;
    let total = rows.len();
    let sample = sample_stratified(rows, args.sample_size, args.seed);
    let cells = count_cells(&sample);

    println!("# scope audit confidence");
    println!("# {PRECISION_ONLY_DISCLAIMER}");
    println!("# {SCHEMA_DOC_POINTER}");
    println!("# sampled {} edge(s) across {} (kind, confidence) cell(s)", sample.len(), cells);
    println!("# (from {total} edge(s) in the index; sample_size={}, seed={:#x})", args.sample_size, args.seed);
    println!();
    anyhow::bail!(
        "audit confidence: sampling engine wired (chunk 3); two-phase labelling \
         (--emit-sample / --label), JSON/TSV writers, and tier gate land in \
         sprint 0007 chunks 4-6. See `docs/sprints/0007-phase-d-confidence-audit.md`."
    )
}

/// Deterministic stratified sampler.
///
/// Groups input rows by `(kind, confidence)` then takes up to
/// `sample_size` per cell using a seeded partial Fisher-Yates shuffle.
/// Output is deterministic in two ways:
/// 1. Cell visit order is sorted (BTreeMap iteration) — independent of
///    HashMap hashing randomness.
/// 2. Within each cell, the partial Fisher-Yates draws are driven by
///    a seeded `xorshift64` PRNG, so the same `(rows, sample_size, seed)`
///    triple always yields the same output.
///
/// Cells with `len() <= sample_size` are taken in full (every edge
/// labeled). Cells larger than `sample_size` return a uniform random
/// subset.
pub fn sample_stratified(
    rows: Vec<AuditEdgeRow>,
    sample_size: usize,
    seed: u64,
) -> Vec<AuditEdgeRow> {
    if sample_size == 0 {
        return Vec::new();
    }

    let mut by_cell: BTreeMap<(String, String), Vec<AuditEdgeRow>> = BTreeMap::new();
    for r in rows {
        by_cell
            .entry((r.kind.clone(), r.confidence.clone()))
            .or_default()
            .push(r);
    }

    let mut state = if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed };
    let mut out = Vec::new();
    for (_cell_key, mut cell_rows) in by_cell {
        let take = sample_size.min(cell_rows.len());
        let len = cell_rows.len();
        for i in 0..take {
            let span = len - i;
            let j = i + (xorshift64(&mut state) as usize % span);
            cell_rows.swap(i, j);
        }
        cell_rows.truncate(take);
        out.extend(cell_rows);
    }
    out
}

fn count_cells(sample: &[AuditEdgeRow]) -> usize {
    let mut cells: BTreeMap<(&str, &str), ()> = BTreeMap::new();
    for r in sample {
        cells.insert((r.kind.as_str(), r.confidence.as_str()), ());
    }
    cells.len()
}

/// xorshift64 — a deterministic 64-bit PRNG.
///
/// Reference: Marsaglia, "Xorshift RNGs" (J. Stat. Softw., 2003).
/// Period 2^64 - 1, passes the canonical xorshift test cycle. R8 needs
/// a *deterministic* draw, not a *cryptographic* one, so xorshift is
/// the cheapest correct fit and avoids pulling in the `rand` crate.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_row(edge_id: i64, kind: &str, confidence: &str) -> AuditEdgeRow {
        AuditEdgeRow {
            edge_id,
            from_id: format!("f{edge_id}"),
            to_id: format!("t{edge_id}"),
            kind: kind.to_string(),
            confidence: confidence.to_string(),
            producer: "test".to_string(),
            pattern_id: format!("p{edge_id}"),
            file_path: "src/test.rs".to_string(),
            line: Some(edge_id as u32),
            args_text: None,
        }
    }

    #[test]
    fn sample_takes_all_when_below_threshold() {
        let rows = vec![
            mk_row(1, "calls", "high"),
            mk_row(2, "calls", "high"),
            mk_row(3, "calls", "high"),
        ];
        let out = sample_stratified(rows, 30, DEFAULT_SEED);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn sample_caps_at_size_per_cell() {
        let rows: Vec<_> = (1..=100).map(|i| mk_row(i, "calls", "high")).collect();
        let out = sample_stratified(rows, 10, DEFAULT_SEED);
        assert_eq!(out.len(), 10);
    }

    #[test]
    fn sample_stratifies_per_cell() {
        let mut rows = Vec::new();
        for i in 1..=50 {
            rows.push(mk_row(i, "calls", "high"));
        }
        for i in 51..=70 {
            rows.push(mk_row(i, "imports", "medium"));
        }
        let out = sample_stratified(rows, 10, DEFAULT_SEED);
        // 10 from calls/high cell + 10 from imports/medium cell.
        assert_eq!(out.len(), 20);

        let calls_high = out
            .iter()
            .filter(|r| r.kind == "calls" && r.confidence == "high")
            .count();
        let imports_medium = out
            .iter()
            .filter(|r| r.kind == "imports" && r.confidence == "medium")
            .count();
        assert_eq!(calls_high, 10);
        assert_eq!(imports_medium, 10);
    }

    #[test]
    fn sample_is_reproducible_under_same_seed() {
        let rows: Vec<_> = (1..=200).map(|i| mk_row(i, "calls", "high")).collect();
        let a = sample_stratified(rows.clone(), 30, DEFAULT_SEED);
        let b = sample_stratified(rows, 30, DEFAULT_SEED);
        let a_ids: Vec<_> = a.iter().map(|r| r.edge_id).collect();
        let b_ids: Vec<_> = b.iter().map(|r| r.edge_id).collect();
        assert_eq!(a_ids, b_ids);
    }

    #[test]
    fn sample_differs_under_different_seed() {
        let rows: Vec<_> = (1..=200).map(|i| mk_row(i, "calls", "high")).collect();
        let a = sample_stratified(rows.clone(), 30, 0x1111_1111_1111_1111);
        let b = sample_stratified(rows, 30, 0xFFFF_FFFF_FFFF_FFFF);
        let a_ids: Vec<_> = a.iter().map(|r| r.edge_id).collect();
        let b_ids: Vec<_> = b.iter().map(|r| r.edge_id).collect();
        assert_ne!(a_ids, b_ids);
    }

    #[test]
    fn sample_visits_cells_in_deterministic_order() {
        let rows = vec![
            mk_row(10, "imports", "low"),
            mk_row(11, "calls", "high"),
            mk_row(12, "extends", "medium"),
        ];
        let out = sample_stratified(rows, 30, DEFAULT_SEED);
        // BTreeMap key order: (calls, high) < (extends, medium) < (imports, low).
        let keys: Vec<_> = out
            .iter()
            .map(|r| (r.kind.as_str(), r.confidence.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("calls", "high"),
                ("extends", "medium"),
                ("imports", "low"),
            ]
        );
    }

    #[test]
    fn sample_zero_size_returns_empty() {
        let rows: Vec<_> = (1..=10).map(|i| mk_row(i, "calls", "high")).collect();
        let out = sample_stratified(rows, 0, DEFAULT_SEED);
        assert!(out.is_empty());
    }
}
