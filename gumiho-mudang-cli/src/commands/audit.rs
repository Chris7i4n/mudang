/// `scope audit` — confidence and coverage audits over the indexed graph.
///
/// Subcommands:
///   confidence  — precision report per (kind, tier, producer, pattern_id)
///                 against the reference fixture corpus.
///
/// `scope audit coverage` is explicitly post-refactor — see
/// `POST-REFACTOR-PLAN.md` § Items deliberately deferred.
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
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

/// Minimum precision the **high** tier must hit, per R8 acceptance
/// (`docs/ARCHITECTURAL-REFACTOR.md` § R8). The CI gate enforces this:
/// any high-tier row whose `precision < HIGH_TIER_MIN` is a build
/// failure. The number itself is the pre-Phase-D ambiguity #2 anchor —
/// `high` exists to mean "this edge is almost certainly correct" and
/// 95% is the operational floor for that claim.
pub const HIGH_TIER_MIN: f64 = 0.95;

/// Minimum precision the **medium** tier must hit, per R8 acceptance.
/// Below this, the producer either downgrades the stamp to `low` or
/// fixes the pattern.
pub const MEDIUM_TIER_MIN: f64 = 0.70;

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

/// One row of the precision report — emitted as one element of the
/// JSON `report` array and as one TSV row.
///
/// Columns mirror the pre-Phase-D ambiguity #4 resolution:
/// `(kind, tier, producer, pattern_id, sample_size, correct_count, precision)`.
/// `tier` is the same string as the sample record's `confidence` field
/// (`"high"` / `"medium"` / `"low"`); the report uses the *tier-target*
/// vocabulary because R8's tier targets are stated against the tier
/// name, not against the bare confidence stamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportRow {
    pub kind: String,
    pub tier: String,
    pub producer: String,
    pub pattern_id: String,
    pub sample_size: usize,
    pub correct_count: usize,
    /// `correct_count / sample_size`, clamped at f64 precision.
    /// Always in `[0.0, 1.0]`.
    pub precision: f64,
}

/// Full precision report — the top-level shape emitted by
/// `scope audit confidence --label --format json`.
///
/// The `schema_version` is the report-side contract (distinct from the
/// sample-side contract in `docs/AUDIT-LABEL-SCHEMA.md`, though both
/// happen to be locked at "1" today). `disclaimer` is the verbatim
/// precision-only framing — see [`PRECISION_ONLY_DISCLAIMER`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecisionReport {
    pub schema_version: String,
    pub disclaimer: String,
    pub report: Vec<ReportRow>,
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

    /// Report output format. Applies only to `--label`; ignored by
    /// `--emit-sample` (the sample file is always JSONL per
    /// `docs/AUDIT-LABEL-SCHEMA.md`) and by the default summary.
    ///
    /// - `json` (default): top-level object with `schema_version: "1"`,
    ///   the precision-only disclaimer, and a `report` array of rows
    ///   `(kind, tier, producer, pattern_id, sample_size, correct_count, precision)`.
    /// - `tsv`: same columns as the JSON `report` array, tab-separated,
    ///   one header line then one row per group — pipeable into shell tools.
    #[arg(long, value_enum, default_value_t = ReportFormat::Json)]
    pub format: ReportFormat,
}

/// Precision report serialisation format.
///
/// JSON is the contract per pre-Phase-D ambiguity #4. TSV is the shell-
/// pipeline convenience surface with the same column set.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Tsv,
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
    args: &ConfidenceArgs,
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

    // Reject incomplete labelling: every row must carry a true/false
    // verdict before precision can be computed. Mixing labelled and
    // unlabelled rows in one report would mis-state the denominator —
    // either we'd inflate precision by ignoring nulls or deflate it by
    // counting them as wrong. Either choice is dishonest. Mechanical
    // refusal until the labeller finishes is the only honest path.
    let unlabelled = records.iter().filter(|r| r.label.is_none()).count();
    if unlabelled > 0 {
        anyhow::bail!(
            "{}: {unlabelled} of {} record(s) have label=null; complete labelling before re-running --label. \
             Each record's `label` must be `true` (correct) or `false` (incorrect) per docs/AUDIT-LABEL-SCHEMA.md.",
            in_path.display(),
            records.len(),
        );
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

    let report = compute_precision_report(&records);
    write_report(&report, args.format, &mut std::io::stdout().lock())?;
    check_tier_gate(&report)?;
    Ok(())
}

/// Enforce the R8 tier targets against a computed precision report.
///
/// Returns `Ok(())` if every row passes its tier's minimum, or an error
/// listing every offender (kind, tier, producer, pattern_id, precision)
/// otherwise. The error message is multi-line and intentionally verbose:
/// the operator needs to see every failing pattern to decide whether to
/// downgrade the confidence stamp or fix the pattern, not just the first
/// offender.
///
/// Tier targets per `docs/ARCHITECTURAL-REFACTOR.md` § R8 (verbatim):
/// - `high ≥ 95%`
/// - `medium ≥ 70%`
/// - `low` has no minimum
///
/// Unknown tier strings produce an error — better to fail loudly than
/// silently accept a tier that was never reviewed against a target.
pub fn check_tier_gate(report: &PrecisionReport) -> Result<()> {
    use std::fmt::Write as _;
    let mut failures: Vec<&ReportRow> = Vec::new();
    for row in &report.report {
        let min = match row.tier.as_str() {
            "high" => HIGH_TIER_MIN,
            "medium" => MEDIUM_TIER_MIN,
            "low" => continue,
            other => anyhow::bail!(
                "unknown tier {other:?} in report row (kind={}, producer={}, pattern_id={}); \
                 expected `high` / `medium` / `low` per docs/ARCHITECTURAL-REFACTOR.md § R8",
                row.kind,
                row.producer,
                row.pattern_id
            ),
        };
        if row.precision < min {
            failures.push(row);
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    let mut msg = String::new();
    let _ = writeln!(
        msg,
        "tier gate: {} row(s) below precision target (high >= {:.0}%, medium >= {:.0}%, low no minimum):",
        failures.len(),
        HIGH_TIER_MIN * 100.0,
        MEDIUM_TIER_MIN * 100.0
    );
    for row in &failures {
        let min = if row.tier == "high" {
            HIGH_TIER_MIN
        } else {
            MEDIUM_TIER_MIN
        };
        let _ = writeln!(
            msg,
            "  {} / {} / {} / {} -> precision {:.4} (target {:.4}; {}/{} correct)",
            row.kind,
            row.tier,
            row.producer,
            row.pattern_id,
            row.precision,
            min,
            row.correct_count,
            row.sample_size,
        );
    }
    msg.push_str(
        "Remediation: either downgrade the confidence stamp at the producer \
         (so the pattern lands in a lower tier with a lower target) or fix \
         the pattern so the labelled precision rises.",
    );
    Err(anyhow::anyhow!(msg))
}

/// Group records by `(kind, tier, producer, pattern_id)` and compute
/// precision per group.
///
/// `tier` is taken from each record's `confidence` field — they are the
/// same string vocabulary (`"high"` / `"medium"` / `"low"`) per the
/// schema. Group iteration order is sorted (`BTreeMap`) so the report
/// is byte-for-byte deterministic given the same input.
///
/// `precision = correct_count as f64 / sample_size as f64`. The caller
/// must ensure every record carries a non-null label before calling —
/// `label_pass` enforces that at the parse boundary.
pub fn compute_precision_report(records: &[SampleRecord]) -> PrecisionReport {
    let mut groups: BTreeMap<(String, String, String, String), (usize, usize)> =
        BTreeMap::new();
    for r in records {
        let key = (
            r.kind.clone(),
            r.confidence.clone(),
            r.producer.clone(),
            r.pattern_id.clone(),
        );
        let entry = groups.entry(key).or_insert((0, 0));
        entry.0 += 1; // sample_size
        if r.label == Some(true) {
            entry.1 += 1; // correct_count
        }
    }

    let rows = groups
        .into_iter()
        .map(|((kind, tier, producer, pattern_id), (n, k))| {
            let precision = if n == 0 { 0.0 } else { k as f64 / n as f64 };
            ReportRow {
                kind,
                tier,
                producer,
                pattern_id,
                sample_size: n,
                correct_count: k,
                precision,
            }
        })
        .collect();

    PrecisionReport {
        schema_version: SCHEMA_VERSION.to_string(),
        disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
        report: rows,
    }
}

/// Write the precision report to `out` in the requested format.
///
/// JSON: pretty-printed (two-space indent) so a human can read the
/// report directly and so diffs over time stay reviewable.
/// `serde_json::to_writer_pretty` streams without an intermediate
/// `Value`.
///
/// TSV: a single header line (`kind\ttier\t...\tprecision`) then one
/// row per `ReportRow`. Precision is rendered with four decimal places
/// — enough resolution to distinguish 0.95 from 0.9499 (the tier
/// boundary) without flooding shell output with float noise.
pub fn write_report<W: Write>(
    report: &PrecisionReport,
    format: ReportFormat,
    out: &mut W,
) -> Result<()> {
    match format {
        ReportFormat::Json => {
            serde_json::to_writer_pretty(&mut *out, report)?;
            writeln!(out)?;
        }
        ReportFormat::Tsv => {
            writeln!(
                out,
                "kind\ttier\tproducer\tpattern_id\tsample_size\tcorrect_count\tprecision"
            )?;
            for row in &report.report {
                writeln!(
                    out,
                    "{}\t{}\t{}\t{}\t{}\t{}\t{:.4}",
                    row.kind,
                    row.tier,
                    row.producer,
                    row.pattern_id,
                    row.sample_size,
                    row.correct_count,
                    row.precision,
                )?;
            }
        }
    }
    Ok(())
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

    // -- Chunk 5: precision report + JSON / TSV writers --

    fn labelled(
        edge_id: i64,
        kind: &str,
        confidence: &str,
        producer: &str,
        pattern_id: &str,
        label: bool,
    ) -> SampleRecord {
        SampleRecord {
            schema_version: SCHEMA_VERSION.to_string(),
            edge_id: edge_id.to_string(),
            kind: kind.to_string(),
            confidence: confidence.to_string(),
            producer: producer.to_string(),
            pattern_id: pattern_id.to_string(),
            from: format!("f{edge_id}"),
            to: format!("t{edge_id}"),
            source_snippet: String::new(),
            lang_version: None,
            label: Some(label),
        }
    }

    #[test]
    fn precision_report_groups_by_full_key() {
        let records = vec![
            labelled(1, "calls", "high", "rust", "rust.calls.method", true),
            labelled(2, "calls", "high", "rust", "rust.calls.method", true),
            labelled(3, "calls", "high", "rust", "rust.calls.method", false),
            labelled(4, "calls", "high", "rust", "rust.calls.fn", true),
            labelled(5, "imports", "medium", "rust", "rust.imports.use", false),
        ];
        let report = compute_precision_report(&records);
        assert_eq!(report.schema_version, "1");
        assert_eq!(report.report.len(), 3);

        // Find each group; assert math.
        let g1 = report
            .report
            .iter()
            .find(|r| r.pattern_id == "rust.calls.method")
            .unwrap();
        assert_eq!(g1.sample_size, 3);
        assert_eq!(g1.correct_count, 2);
        assert!((g1.precision - 2.0 / 3.0).abs() < 1e-9);

        let g2 = report
            .report
            .iter()
            .find(|r| r.pattern_id == "rust.calls.fn")
            .unwrap();
        assert_eq!(g2.sample_size, 1);
        assert_eq!(g2.correct_count, 1);
        assert_eq!(g2.precision, 1.0);

        let g3 = report
            .report
            .iter()
            .find(|r| r.pattern_id == "rust.imports.use")
            .unwrap();
        assert_eq!(g3.sample_size, 1);
        assert_eq!(g3.correct_count, 0);
        assert_eq!(g3.precision, 0.0);
    }

    #[test]
    fn precision_report_rows_are_sorted_deterministically() {
        let records = vec![
            labelled(1, "imports", "low", "python", "p.imports.from", true),
            labelled(2, "calls", "high", "rust", "rust.calls.method", true),
            labelled(3, "calls", "high", "rust", "rust.calls.fn", true),
            labelled(4, "extends", "medium", "ts", "ts.extends.class", true),
        ];
        let report = compute_precision_report(&records);
        // BTreeMap key sort: (kind, tier, producer, pattern_id) ascending.
        let keys: Vec<_> = report
            .report
            .iter()
            .map(|r| (r.kind.as_str(), r.tier.as_str(), r.pattern_id.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("calls", "high", "rust.calls.fn"),
                ("calls", "high", "rust.calls.method"),
                ("extends", "medium", "ts.extends.class"),
                ("imports", "low", "p.imports.from"),
            ]
        );
    }

    #[test]
    fn precision_report_disclaimer_is_verbatim() {
        let report = compute_precision_report(&[]);
        assert_eq!(report.disclaimer, PRECISION_ONLY_DISCLAIMER);
    }

    #[test]
    fn write_report_json_carries_schema_disclaimer_rows() {
        let report = PrecisionReport {
            schema_version: "1".to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            report: vec![ReportRow {
                kind: "calls".to_string(),
                tier: "high".to_string(),
                producer: "rust".to_string(),
                pattern_id: "rust.calls.method".to_string(),
                sample_size: 30,
                correct_count: 29,
                precision: 29.0 / 30.0,
            }],
        };
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Json, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"schema_version\": \"1\""));
        assert!(s.contains(PRECISION_ONLY_DISCLAIMER));
        assert!(s.contains("\"sample_size\": 30"));
        assert!(s.contains("\"correct_count\": 29"));
        assert!(s.contains("\"precision\":"));
        // Pretty-printed: contains a newline (not a single-line blob).
        assert!(s.contains('\n'));
    }

    #[test]
    fn write_report_tsv_has_header_and_one_row_per_group() {
        let report = PrecisionReport {
            schema_version: "1".to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            report: vec![
                ReportRow {
                    kind: "calls".to_string(),
                    tier: "high".to_string(),
                    producer: "rust".to_string(),
                    pattern_id: "rust.calls.method".to_string(),
                    sample_size: 30,
                    correct_count: 29,
                    precision: 29.0 / 30.0,
                },
                ReportRow {
                    kind: "imports".to_string(),
                    tier: "medium".to_string(),
                    producer: "python".to_string(),
                    pattern_id: "p.imports.from".to_string(),
                    sample_size: 12,
                    correct_count: 9,
                    precision: 0.75,
                },
            ],
        };
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Tsv, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<_> = s.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert_eq!(
            lines[0],
            "kind\ttier\tproducer\tpattern_id\tsample_size\tcorrect_count\tprecision"
        );
        assert_eq!(
            lines[1],
            "calls\thigh\trust\trust.calls.method\t30\t29\t0.9667"
        );
        assert_eq!(
            lines[2],
            "imports\tmedium\tpython\tp.imports.from\t12\t9\t0.7500"
        );
    }

    // -- Chunk 6: tier gate --

    fn report_with_rows(rows: Vec<ReportRow>) -> PrecisionReport {
        PrecisionReport {
            schema_version: "1".to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            report: rows,
        }
    }

    fn row(kind: &str, tier: &str, pattern_id: &str, n: usize, k: usize) -> ReportRow {
        ReportRow {
            kind: kind.to_string(),
            tier: tier.to_string(),
            producer: "rust".to_string(),
            pattern_id: pattern_id.to_string(),
            sample_size: n,
            correct_count: k,
            precision: if n == 0 { 0.0 } else { k as f64 / n as f64 },
        }
    }

    #[test]
    fn tier_gate_passes_when_every_row_meets_target() {
        let report = report_with_rows(vec![
            row("calls", "high", "p1", 20, 19),   // 0.95 — exactly at boundary
            row("imports", "medium", "p2", 10, 7), // 0.70 — exactly at boundary
            row("extends", "low", "p3", 5, 0),    // low: no minimum
        ]);
        check_tier_gate(&report).expect("gate should pass at exact boundaries");
    }

    #[test]
    fn tier_gate_fails_on_high_tier_below_95() {
        let report = report_with_rows(vec![row("calls", "high", "p1", 100, 94)]); // 0.94
        let err = check_tier_gate(&report).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("tier gate"));
        assert!(msg.contains("p1"));
        assert!(msg.contains("0.9400"));
        assert!(msg.contains("Remediation"));
    }

    #[test]
    fn tier_gate_fails_on_medium_tier_below_70() {
        let report = report_with_rows(vec![row("imports", "medium", "p2", 10, 6)]); // 0.60
        let err = check_tier_gate(&report).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("medium"));
        assert!(msg.contains("p2"));
        assert!(msg.contains("0.6000"));
    }

    #[test]
    fn tier_gate_does_not_fail_on_low_tier_at_zero_precision() {
        let report = report_with_rows(vec![row("calls", "low", "p3", 10, 0)]); // 0.0
        check_tier_gate(&report).expect("low tier has no minimum");
    }

    #[test]
    fn tier_gate_reports_every_offender_not_just_first() {
        let report = report_with_rows(vec![
            row("calls", "high", "p1", 100, 90),    // 0.90 fail
            row("imports", "medium", "p2", 10, 5),  // 0.50 fail
            row("extends", "high", "p3", 100, 100), // 1.00 pass
            row("calls", "high", "p4", 100, 80),    // 0.80 fail
        ]);
        let err = check_tier_gate(&report).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("3 row(s)"), "expected 3 failures: {msg}");
        for pattern in ["p1", "p2", "p4"] {
            assert!(msg.contains(pattern), "missing {pattern}: {msg}");
        }
        assert!(!msg.contains("p3"));
    }

    #[test]
    fn tier_gate_rejects_unknown_tier_string() {
        let report = report_with_rows(vec![row("calls", "ultra", "p1", 10, 10)]);
        let err = check_tier_gate(&report).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown tier"));
        assert!(msg.contains("\"ultra\""));
    }

    #[test]
    fn tier_gate_constants_match_r8_targets() {
        assert_eq!(HIGH_TIER_MIN, 0.95);
        assert_eq!(MEDIUM_TIER_MIN, 0.70);
    }

    #[test]
    fn write_report_tsv_uses_four_decimal_precision() {
        // Picks values either side of the high-tier 0.95 boundary so a
        // future format-string change that loses resolution fails here.
        let report = PrecisionReport {
            schema_version: "1".to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            report: vec![
                ReportRow {
                    kind: "calls".to_string(),
                    tier: "high".to_string(),
                    producer: "rust".to_string(),
                    pattern_id: "p1".to_string(),
                    sample_size: 200,
                    correct_count: 190, // 0.9500
                    precision: 190.0 / 200.0,
                },
                ReportRow {
                    kind: "calls".to_string(),
                    tier: "high".to_string(),
                    producer: "rust".to_string(),
                    pattern_id: "p2".to_string(),
                    sample_size: 10000,
                    correct_count: 9499, // 0.9499
                    precision: 9499.0 / 10000.0,
                },
            ],
        };
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Tsv, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\t0.9500\n"));
        assert!(s.contains("\t0.9499\n"));
    }
}
