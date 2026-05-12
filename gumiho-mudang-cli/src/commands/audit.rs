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
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use gumiho_mudang_scope::core::graph::{AuditEdgeRow, AuditFreshness, Graph};

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

/// Wire-format schema version emitted by `--emit-sample` and accepted by
/// `--label`. Locked at `"1"` per the contract in
/// `docs/AUDIT-LABEL-SCHEMA.md`. Bumping this is charter-grade and lands
/// via the POST-REFACTOR-PLAN.md § Priority 2 audit (which bundles the
/// `producer_captured_args` field addition with the `args_text` cap drop).
pub const SCHEMA_VERSION: &str = "1";

/// JSONL record per `docs/AUDIT-LABEL-SCHEMA.md` (schema_version "1").
///
/// Field order in this struct matches the order declared in the schema
/// table so `serde_json::to_string` emits a deterministic key order in
/// the JSONL output. `edge_id` is round-tripped as a string (per the
/// schema doc) even though the DB column is `i64`; the conversion
/// happens at the (de)serialisation boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SampleRecord {
    pub schema_version: String,
    pub edge_id: String,
    pub kind: String,
    pub confidence: String,
    pub producer: String,
    pub pattern_id: String,
    pub from: String,
    pub to: String,
    pub source_snippet: String,
    pub lang_version: Option<String>,
    pub label: Option<bool>,
}

impl SampleRecord {
    /// Build a record from a sampled edge row plus the snippet read
    /// from disk. `lang_version` is `null` for sprint 0007 (populated
    /// by a future sprint when the seven per-language detectors land
    /// atomically — see `docs/AUDIT-LABEL-SCHEMA.md`). `label` is
    /// `null` on emit; the external labeller fills it.
    pub fn from_row(row: &AuditEdgeRow, source_snippet: String) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            edge_id: row.edge_id.to_string(),
            kind: row.kind.clone(),
            confidence: row.confidence.clone(),
            producer: row.producer.clone(),
            pattern_id: row.pattern_id.clone(),
            from: row.from_id.clone(),
            to: row.to_id.clone(),
            source_snippet,
            lang_version: None,
            label: None,
        }
    }
}

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

    /// Emit a JSONL sample file at `<PATH>` for an external labeller
    /// (human / LLM / LSP cross-check / hybrid).
    ///
    /// File format: `docs/AUDIT-LABEL-SCHEMA.md` (schema_version "1").
    /// Each line is one sampled edge with `label: null` to be filled in
    /// out-of-band, then read back via `--label <PATH>`.
    ///
    /// Mutually exclusive with `--label`.
    ///
    /// Drift gate: Scope refuses to emit the sample if any source file
    /// referenced by the sample no longer hashes to the value recorded
    /// at index time (auditor immutability rule — see
    /// `docs/AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule). The
    /// only remediation is `scope index` then re-emit.
    #[arg(long, value_name = "PATH", conflicts_with = "label")]
    pub emit_sample: Option<PathBuf>,

    /// Read a labelled JSONL sample file from `<PATH>` and produce the
    /// precision report.
    ///
    /// Records with `schema_version` other than "1" are rejected; the
    /// remediation is to re-emit the sample at the current schema.
    ///
    /// Mutually exclusive with `--emit-sample`.
    ///
    /// Drift gate: identical to `--emit-sample`. Editing source between
    /// emit and label invalidates the measurement and aborts the audit.
    /// The only remediation is `scope index` then re-emit + re-label.
    #[arg(long, value_name = "PATH", conflicts_with = "emit_sample")]
    pub label: Option<PathBuf>,
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

    match (&args.emit_sample, &args.label) {
        (Some(out_path), None) => emit_sample(&graph, args, project_root, out_path),
        (None, Some(in_path)) => label_pass(&graph, args, project_root, in_path),
        (None, None) => default_summary(&graph, args),
        // Unreachable because clap's `conflicts_with` prevents both being set.
        (Some(_), Some(_)) => unreachable!(
            "clap conflicts_with should prevent --emit-sample and --label together"
        ),
    }
}

/// Default surface (no `--emit-sample`, no `--label`): print sampling
/// summary then bail with a chunk-plan pointer. The JSON/TSV writers
/// (chunk 5) and tier gate (chunk 6) land before this path becomes
/// useful on its own.
fn default_summary(graph: &Graph, args: &ConfidenceArgs) -> Result<()> {
    let rows = graph.list_edges_for_audit()?;
    let total = rows.len();
    let sample = sample_stratified(rows, args.sample_size, args.seed);
    let cells = count_cells(&sample);

    println!("# scope audit confidence");
    println!("# {PRECISION_ONLY_DISCLAIMER}");
    println!("# {SCHEMA_DOC_POINTER}");
    println!(
        "# sampled {} edge(s) across {} (kind, confidence) cell(s)",
        sample.len(),
        cells
    );
    println!(
        "# (from {total} edge(s) in the index; sample_size={}, seed={:#x})",
        args.sample_size, args.seed
    );
    println!();
    anyhow::bail!(
        "audit confidence: sampling engine + two-phase labelling wired \
         (chunks 3-4). Use `--emit-sample <path>` then `--label <path>`. \
         JSON/TSV writers and tier gate land in sprint 0007 chunks 5-6. \
         See `docs/sprints/0007-phase-d-confidence-audit.md`."
    )
}

/// `--emit-sample <PATH>`: sample, drift-check, then write JSONL.
///
/// Step order matters:
/// 1. Sample first (purely in-memory, no side effects on failure).
/// 2. Drift-check the distinct files referenced by the sample. Abort
///    on any drift — the snippet we ship would otherwise misrepresent
///    what the extractor saw.
/// 3. Read each row's source snippet from the real file and write
///    JSONL per `docs/AUDIT-LABEL-SCHEMA.md` (schema_version "1").
fn emit_sample(
    graph: &Graph,
    args: &ConfidenceArgs,
    project_root: &Path,
    out_path: &Path,
) -> Result<()> {
    let rows = graph.list_edges_for_audit()?;
    let total = rows.len();
    let sample = sample_stratified(rows, args.sample_size, args.seed);

    enforce_freshness(graph, project_root, &sample)?;

    let file = File::create(out_path)
        .with_context(|| format!("failed to create sample file {}", out_path.display()))?;
    let mut writer = BufWriter::new(file);
    for row in &sample {
        let snippet = read_source_snippet(project_root, &row.file_path, row.line)?;
        let record = SampleRecord::from_row(row, snippet);
        let line = serde_json::to_string(&record)
            .with_context(|| format!("serialise edge_id={}", record.edge_id))?;
        writeln!(writer, "{line}")?;
    }
    writer.flush()?;

    eprintln!(
        "wrote {} record(s) to {} (sampled from {total} edge(s); sample_size={}, seed={:#x})",
        sample.len(),
        out_path.display(),
        args.sample_size,
        args.seed
    );
    Ok(())
}

/// `--label <PATH>`: parse a labelled JSONL file, drift-check, hold
/// records for the precision report (chunks 5-6).
///
/// Chunk 4 lands the read-path mechanics:
/// - One record per line; empty lines and `#`-prefixed comments skipped.
/// - Every record must declare `schema_version: "1"`; unknown versions
///   are a hard error with re-emit remediation.
/// - Drift gate runs against the files referenced by the records'
///   `edge_id`s (joined back through the index, since the schema does
///   not carry `file_path`).
///
/// The actual precision computation, JSON / TSV writers, and tier gate
/// land in chunks 5-6.
fn label_pass(
    graph: &Graph,
    _args: &ConfidenceArgs,
    project_root: &Path,
    in_path: &Path,
) -> Result<()> {
    let file = File::open(in_path)
        .with_context(|| format!("failed to open sample file {}", in_path.display()))?;
    let reader = BufReader::new(file);

    let mut records = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line =
            line.with_context(|| format!("{}: read line {}", in_path.display(), idx + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let record: SampleRecord = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "{}: line {}: invalid JSON record (schema: docs/AUDIT-LABEL-SCHEMA.md)",
                in_path.display(),
                idx + 1
            )
        })?;
        if record.schema_version != SCHEMA_VERSION {
            anyhow::bail!(
                "{}: line {}: unknown schema_version {:?}; this scope build understands {:?} only. \
                 Re-emit the sample with `scope audit confidence --emit-sample <new-path>` against the current index.",
                in_path.display(),
                idx + 1,
                record.schema_version,
                SCHEMA_VERSION
            );
        }
        records.push(record);
    }

    // Join record edge_ids back to file_paths via the index. The schema
    // intentionally omits file_path (the labeller does not need it), so
    // the index is the truth source for the drift gate at label time.
    let edge_ids: BTreeSet<i64> = records
        .iter()
        .filter_map(|r| r.edge_id.parse::<i64>().ok())
        .collect();
    let all_rows = graph.list_edges_for_audit()?;
    let referenced_rows: Vec<AuditEdgeRow> = all_rows
        .into_iter()
        .filter(|r| edge_ids.contains(&r.edge_id))
        .collect();
    enforce_freshness(graph, project_root, &referenced_rows)?;

    anyhow::bail!(
        "audit confidence --label: read {} record(s) and verified source freshness. \
         Precision report, JSON/TSV writers, and tier gate land in sprint 0007 chunks 5-6. \
         See `docs/sprints/0007-phase-d-confidence-audit.md`.",
        records.len()
    )
}

/// Hard mechanical enforcement of the auditor immutability rule.
///
/// Collects the distinct `file_path`s referenced by `rows`, then asks
/// [`Graph::check_audit_freshness`] to re-hash each one and compare
/// against the indexer's stored digest. Any drift — modified, missing,
/// or unknown-to-index — produces a hard error with the re-index
/// remediation message. There is no `--allow-drift` escape, per
/// `docs/AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule.
fn enforce_freshness(graph: &Graph, project_root: &Path, rows: &[AuditEdgeRow]) -> Result<()> {
    let files: BTreeSet<&str> = rows.iter().map(|r| r.file_path.as_str()).collect();
    let file_vec: Vec<&str> = files.into_iter().collect();
    let report = graph.check_audit_freshness(project_root, &file_vec)?;
    if !report.is_clean() {
        return Err(drift_error(&report));
    }
    Ok(())
}

/// Read one line from the on-disk source file for the JSONL
/// `source_snippet` field.
///
/// The drift gate (which runs first) guarantees the on-disk content
/// hashes equal to the indexer's `file_hashes.hash`, so the snippet
/// the labeller sees is byte-identical to what the extractor saw.
///
/// `line` is 1-based (tree-sitter's `start_position().row + 1` per
/// `scope-core/src/parser.rs`). An absent line (e.g. resolution edges
/// without a site) yields the empty string — schema allows empty but
/// not null for this field.
fn read_source_snippet(
    project_root: &Path,
    file_path: &str,
    line: Option<u32>,
) -> Result<String> {
    let Some(line) = line else {
        return Ok(String::new());
    };
    if line == 0 {
        return Ok(String::new());
    }
    let abs = project_root.join(file_path);
    let content = std::fs::read_to_string(&abs)
        .with_context(|| format!("read source for snippet: {}", abs.display()))?;
    let idx = (line as usize) - 1;
    Ok(content.lines().nth(idx).unwrap_or("").to_string())
}

/// Format the [`AuditFreshness`] report into a hard-error anyhow value
/// with the re-index remediation.
fn drift_error(report: &AuditFreshness) -> anyhow::Error {
    use std::fmt::Write as _;
    let mut msg = String::new();
    msg.push_str(
        "source drift detected — the audit subject (indexed graph + on-disk source) no longer matches.\n",
    );
    msg.push_str(
        "The auditor immutability rule (docs/AUDIT-LABEL-SCHEMA.md § Auditor immutability rule) \
         forbids labelling against drifted source: the labeller would judge against text the \
         extractor never saw.\n",
    );
    if !report.modified.is_empty() {
        let _ = writeln!(msg, "\n  modified ({}):", report.modified.len());
        for p in &report.modified {
            let _ = writeln!(msg, "    {p}");
        }
    }
    if !report.missing.is_empty() {
        let _ = writeln!(msg, "\n  missing ({}):", report.missing.len());
        for p in &report.missing {
            let _ = writeln!(msg, "    {p}");
        }
    }
    if !report.unknown.is_empty() {
        let _ = writeln!(msg, "\n  unknown to index ({}):", report.unknown.len());
        for p in &report.unknown {
            let _ = writeln!(msg, "    {p}");
        }
    }
    msg.push_str(
        "\nRemediation: run `scope index` to refresh the snapshot, then re-run the audit. \
         There is no `--allow-drift` escape.",
    );
    anyhow::anyhow!(msg)
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

    // -- Chunk 4: JSONL record + schema versioning --

    #[test]
    fn sample_record_serialises_with_schema_v1() {
        let row = mk_row(42, "calls", "high");
        let rec = SampleRecord::from_row(&row, "format_name(&user.name)".to_string());
        let json = serde_json::to_string(&rec).unwrap();
        // schema_version locked at "1"; edge_id stringified; lang_version/label null on emit.
        assert!(json.contains("\"schema_version\":\"1\""));
        assert!(json.contains("\"edge_id\":\"42\""));
        assert!(json.contains("\"source_snippet\":\"format_name(&user.name)\""));
        assert!(json.contains("\"lang_version\":null"));
        assert!(json.contains("\"label\":null"));
    }

    #[test]
    fn sample_record_round_trips_through_jsonl() {
        let row = mk_row(7, "imports", "medium");
        let original = SampleRecord::from_row(&row, "use std::fs;".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: SampleRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn sample_record_keeps_field_order_for_deterministic_jsonl() {
        let row = mk_row(1, "calls", "high");
        let rec = SampleRecord::from_row(&row, "x".to_string());
        let json = serde_json::to_string(&rec).unwrap();
        // serde_derive emits struct fields in declaration order; assert the schema's
        // documented order so a future field reorder fails this test.
        let expected_order = [
            "\"schema_version\"",
            "\"edge_id\"",
            "\"kind\"",
            "\"confidence\"",
            "\"producer\"",
            "\"pattern_id\"",
            "\"from\"",
            "\"to\"",
            "\"source_snippet\"",
            "\"lang_version\"",
            "\"label\"",
        ];
        let mut last_pos = 0usize;
        for key in expected_order {
            let pos = json.find(key).unwrap_or_else(|| panic!("missing {key}"));
            assert!(
                pos > last_pos,
                "field {key} out of order in {json}"
            );
            last_pos = pos;
        }
    }

    #[test]
    fn sample_record_label_accepts_true_false_null() {
        let cases = [
            ("\"label\":null", None),
            ("\"label\":true", Some(true)),
            ("\"label\":false", Some(false)),
        ];
        for (substr, expected) in cases {
            let row = mk_row(99, "calls", "high");
            let mut rec = SampleRecord::from_row(&row, String::new());
            rec.label = expected;
            let json = serde_json::to_string(&rec).unwrap();
            assert!(json.contains(substr), "expected {substr} in {json}");
            let parsed: SampleRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.label, expected);
        }
    }

    #[test]
    fn schema_version_constant_locked_at_one() {
        // The schema is contract-grade per docs/AUDIT-LABEL-SCHEMA.md.
        // Bumping is charter-grade and must land via POST-REFACTOR-PLAN
        // § Priority 2; this assertion is the canary against drive-by edits.
        assert_eq!(SCHEMA_VERSION, "1");
    }
}
