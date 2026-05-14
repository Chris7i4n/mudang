/// `scope audit` — confidence and coverage audits over the indexed graph.
///
/// Subcommands:
///   confidence  — precision report per (kind, tier, producer, pattern_id)
///                 against the reference fixture corpus.
///
/// `scope audit coverage` is explicitly — see
/// `BACKLOG.md` § Items deliberately deferred.
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use gumiho_mudang_scope::graph::{AuditEdgeRow, AuditFreshness, Graph};
use gumiho_mudang_scope::workspace::lang_version::detect_lang_version;

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
pub const PRECISION_ONLY_DISCLAIMER: &str =
    "precision report; recall is measured by integration-test snapshots, not this subcommand.";

/// Pointer printed alongside the disclaimer so external labeller authors
/// discover the JSONL contract from the CLI itself.
pub const SCHEMA_DOC_POINTER: &str =
    "Sample-file schema: docs/AUDIT-LABEL-SCHEMA.md (schema_version \"2\").";

/// Wire-format schema version emitted by `--emit-sample` and accepted
/// by `--label`. Per `docs/AUDIT-LABEL-SCHEMA.md` § Record schema and
/// the single-operator-posture invariant
/// ([`CHARTER.md` § 3](../../gumiho-mudang-scope/docs/CHARTER.md#3-core-invariants--must-never-break)),
/// there is exactly one accepted version on read; a future bump wipes
/// any committed corpus + re-emits at the new version. No dual-read
/// shim.
pub const SAMPLE_SCHEMA_VERSION: &str = "2";

/// Report-side schema version (distinct from the sample-side contract;
/// see `docs/AUDIT-LABEL-SCHEMA.md`). Bumped to `"2"` in sprint 0004 CP3
/// alongside the per-row coverage fields (`labelled_count`,
/// `skipped_count`, `coverage_ratio`) and the top-level
/// `coverage_summary` object. The bump also retires
/// `coverage_limitation_note` — the gap it disclosed closes here.
pub const REPORT_SCHEMA_VERSION: &str = "2";

/// Minimum precision the **high** tier must hit, per R8 acceptance
/// (`docs/ENFORCEMENT-MAP.md` § R8). The CI gate enforces this:
/// any high-tier row whose `precision < HIGH_TIER_MIN` is a build
/// failure. The number itself is the pre-Phase-D ambiguity #2 anchor —
/// `high` exists to mean "this edge is almost certainly correct" and
/// 95% is the operational floor for that claim.
pub const HIGH_TIER_MIN: f64 = 0.95;

/// Minimum precision the **medium** tier must hit, per R8 acceptance.
/// Below this, the producer either downgrades the stamp to `low` or
/// fixes the pattern.
pub const MEDIUM_TIER_MIN: f64 = 0.70;

/// JSONL record per `docs/AUDIT-LABEL-SCHEMA.md` (schema_version "2").
///
/// Field order in this struct matches the order declared in the schema
/// table so `serde_json::to_string` emits a deterministic key order in
/// the JSONL output. `edge_id` is round-tripped as a string (per the
/// schema doc) even though the DB column is `i64`; the conversion
/// happens at the (de)serialisation boundary.
///
/// No `#[serde(default)]` on the nullable fields — every record must
/// carry every key physically (with `null` where applicable). The
/// presence check in `label_pass` enforces that rule; serde defaults
/// would silently let a labeller-side serializer that drops nulls
/// produce records that violate the contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub evidence: Option<serde_json::Value>,
    pub target_proposed: Option<String>,
    pub kind_proposed: Option<String>,
    pub confidence_proposed: Option<String>,
    pub reasoning_text: Option<String>,
    pub lang_version_evidence: Option<String>,
    pub labeller_id: Option<String>,
}

impl SampleRecord {
    /// Build a record from a sampled edge row plus the snippet read
    /// from disk and the per-file `lang_version` produced by the
    /// indexer-side detector matrix (see
    /// `scope_core::workspace::lang_version::detect_lang_version` and
    /// `BACKLOG.md` § Priority 1 sub-item (d)). `lang_version` is
    /// `None` only when no manifest could be resolved for the file's
    /// language (e.g. snippet from outside the project root, or
    /// unsupported extension). `label` is `null` on emit; the
    /// external labeller fills it. All v2 fields emit `null`; capable
    /// labellers populate them.
    pub fn from_row(
        row: &AuditEdgeRow,
        source_snippet: String,
        lang_version: Option<String>,
    ) -> Self {
        Self {
            schema_version: SAMPLE_SCHEMA_VERSION.to_string(),
            edge_id: row.edge_id.to_string(),
            kind: row.kind.clone(),
            confidence: row.confidence.clone(),
            producer: row.producer.clone(),
            pattern_id: row.pattern_id.clone(),
            from: row.from_id.clone(),
            to: row.to_id.clone(),
            source_snippet,
            lang_version,
            label: None,
            evidence: None,
            target_proposed: None,
            kind_proposed: None,
            confidence_proposed: None,
            reasoning_text: None,
            lang_version_evidence: None,
            labeller_id: None,
        }
    }
}

/// One row of the precision report — emitted as one element of the
/// JSON `report` array and as one TSV row.
///
/// Columns (post-sprint-0004 v2 shape):
/// `(kind, tier, producer, pattern_id, sample_size, labelled_count,
/// skipped_count, coverage_ratio, correct_count, precision)`.
/// `labelled_count` is an explicit alias for `sample_size`; both ship
/// side-by-side so the report is self-documenting next to
/// `skipped_count` — an operator no longer has to mentally compute the
/// total. `tier` is the same string as the sample record's `confidence`
/// field (`"high"` / `"medium"` / `"low"`); the report uses the
/// *tier-target* vocabulary because R8's tier targets are stated
/// against the tier name, not against the bare confidence stamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportRow {
    pub kind: String,
    pub tier: String,
    pub producer: String,
    pub pattern_id: String,
    /// Number of labelled records in this group (`label = true | false`).
    /// Equal to `labelled_count`; kept for backward shape compatibility
    /// with downstream tooling that already reads `sample_size`. The
    /// precision denominator.
    pub sample_size: usize,
    /// Explicit alias for `sample_size`. Side-by-side with
    /// `skipped_count` so the row reads self-documenting.
    pub labelled_count: usize,
    /// Number of records in this group whose label was `null`. With
    /// `labelled_count` this gives the full coverage picture per group.
    pub skipped_count: usize,
    /// `labelled_count / (labelled_count + skipped_count)`. Always in
    /// `[0.0, 1.0]`. `0.0` when every record in this group was skipped
    /// (in which case `precision` is `None`).
    pub coverage_ratio: f64,
    /// Number of labelled records the labeller marked `true`.
    pub correct_count: usize,
    /// `correct_count / sample_size`. `None` when `sample_size == 0`
    /// (every record in this group was skipped — no precision is
    /// measurable). Otherwise always in `[0.0, 1.0]`.
    pub precision: Option<f64>,
}

/// Top-level coverage summary for a precision report.
///
/// Surfaces the labelling-coverage shape across **all** sampled records
/// (not just the ones that contributed to a precision number). Lets an
/// operator answer two questions at a glance:
///
/// 1. How much of the sample got actually labelled? (`records_labelled
///    / records_total`)
/// 2. How many groups have zero coverage? (`distinct_groups_fully_skipped`)
///
/// `distinct_groups_with_coverage` counts groups with at least one
/// labelled record (precision computable). `distinct_groups_fully_skipped`
/// counts groups present in the sample where every record was skipped
/// (precision = None on the row).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageSummary {
    pub records_total: usize,
    pub records_labelled: usize,
    pub records_skipped: usize,
    pub distinct_groups_with_coverage: usize,
    pub distinct_groups_fully_skipped: usize,
}

/// Full precision report — the top-level shape emitted by
/// `scope audit confidence --label --format json`.
///
/// The `schema_version` is the report-side contract (distinct from the
/// sample-side contract in `docs/AUDIT-LABEL-SCHEMA.md`). Sprint 0004 CP3
/// bumped this to `"2"`, retiring the `coverage_limitation_note` carve
/// and adding the structured `coverage_summary` object together with
/// the per-row `labelled_count` / `skipped_count` / `coverage_ratio`
/// fields. `disclaimer` is the verbatim precision-only framing — see
/// [`PRECISION_ONLY_DISCLAIMER`]. `sample_schema_doc` carries the inline
/// pointer to the sample-file contract so external labeller authors
/// (LLM / LSP / hybrid) can discover it directly from the report rather
/// than from out-of-band docs — see [`SCHEMA_DOC_POINTER`] and
/// `docs/AUDIT-LABEL-SCHEMA.md`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrecisionReport {
    pub schema_version: String,
    pub disclaimer: String,
    pub sample_schema_doc: String,
    pub coverage_summary: CoverageSummary,
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
        (Some(_), Some(_)) => {
            unreachable!("clap conflicts_with should prevent --emit-sample and --label together")
        }
    }
}

/// Default surface (no `--emit-sample`, no `--label`): the operator-
/// exploration mode. Prints a summary of what a full audit would
/// sample against the current index, plus a usage hint pointing at
/// the two-phase flow that produces the actual precision report.
/// Exits success — the no-flag invocation is a documented `--help`
/// example and must work as an introspection surface.
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
    println!("# This is the introspection surface — no precision report is produced");
    println!("# without a labelled sample. To produce one:");
    println!("#");
    println!("#   1. emit:  scope audit confidence --emit-sample <path>");
    println!("#   2. label: external labeller fills `label` per record");
    println!("#             (see docs/AUDIT-LABEL-SCHEMA.md § External labeller examples)");
    println!("#   3. read:  scope audit confidence --label <path> [--format tsv]");
    println!("#");
    println!("# The report then enforces high >= 95% / medium >= 70% / low any precision.");
    Ok(())
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

    // Refuse to clobber an existing file at the sample destination
    // : the previous `File::create` would have
    // truncated any pre-existing file at `out_path` — including an
    // indexed source file the operator misnamed. Snippets for other
    // edges in that file would then read the truncated content and
    // emit empty / wrong `source_snippet`s, and the indexed source
    // would be silently corrupted. `OpenOptions::new().write(true)
    // .create_new(true)` errors with `AlreadyExists` if anything is
    // at the path; the operator removes it or picks a new path.
    // There is no `--force` flag — silent overwrite of the working
    // tree from an audit subcommand is the same class of dishonesty
    // the auditor immutability rule (AUDIT-LABEL-SCHEMA.md § Auditor
    // immutability rule) forbids.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out_path)
        .with_context(|| {
            format!(
                "failed to create sample file {} (refusing to overwrite an existing path; \
                 remove the file or pick a new path)",
                out_path.display()
            )
        })?;
    let mut writer = BufWriter::new(file);
    for row in &sample {
        let snippet = read_source_snippet(project_root, &row.file_path, row.line)?;
        let lang_version = detect_lang_version(project_root, &project_root.join(&row.file_path));
        let record = SampleRecord::from_row(row, snippet, lang_version);
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
/// records for the precision report.
///
/// Read-path mechanics:
/// - One record per line; empty lines and `#`-prefixed comments skipped.
/// - Every record must declare `schema_version: "1"`; unknown versions
///   are a hard error with re-emit remediation.
/// - Drift gate runs against the files referenced by the records'
///   `edge_id`s (joined back through the index, since the schema does
///   not carry `file_path`).
///
/// Precision computation, JSON / TSV writers, and tier gate run after
/// this read-path completes.
fn label_pass(
    graph: &Graph,
    args: &ConfidenceArgs,
    project_root: &Path,
    in_path: &Path,
) -> Result<()> {
    let file = File::open(in_path)
        .with_context(|| format!("failed to open sample file {}", in_path.display()))?;
    let reader = BufReader::new(file);

    let mut records: Vec<(usize, SampleRecord)> = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("{}: read line {}", in_path.display(), idx + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Pre-typed key-presence check. Per
        // `docs/AUDIT-LABEL-SCHEMA.md` every field on a schema_version
        // "2" record is required-with-value (null is a valid value for
        // the nullable fields, but the *key* must still be present).
        // Serde silently treats an absent `Option<T>` field as `None`,
        // which would otherwise let a labeller-side serializer that
        // drops nulls produce records that violate the round-trip
        // contract without any audit-side error. The typed parse
        // handles values; this pre-check handles presence.
        const REQUIRED_FIELDS: &[&str] = &[
            "schema_version",
            "edge_id",
            "kind",
            "confidence",
            "producer",
            "pattern_id",
            "from",
            "to",
            "source_snippet",
            "lang_version",
            "label",
            "evidence",
            "target_proposed",
            "kind_proposed",
            "confidence_proposed",
            "reasoning_text",
            "lang_version_evidence",
            "labeller_id",
        ];
        let raw: serde_json::Value = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "{}: line {}: invalid JSON record (schema: docs/AUDIT-LABEL-SCHEMA.md)",
                in_path.display(),
                idx + 1
            )
        })?;
        let raw_obj = raw.as_object().ok_or_else(|| {
            anyhow::anyhow!(
                "{}: line {}: JSON record must be an object per docs/AUDIT-LABEL-SCHEMA.md",
                in_path.display(),
                idx + 1
            )
        })?;
        let missing: Vec<&&str> = REQUIRED_FIELDS
            .iter()
            .filter(|k| !raw_obj.contains_key(**k))
            .collect();
        if !missing.is_empty() {
            let names: Vec<String> = missing.iter().map(|k| format!("`{k}`")).collect();
            anyhow::bail!(
                "{}: line {}: required field(s) {} missing; per docs/AUDIT-LABEL-SCHEMA.md every \
                 schema_version \"{}\" record must carry every field explicitly (with `null` where \
                 applicable). A missing key is not the same as an explicit `null` value and is \
                 rejected to surface labeller-side serializer bugs.",
                in_path.display(),
                idx + 1,
                names.join(", "),
                SAMPLE_SCHEMA_VERSION
            );
        }
        let record: SampleRecord = serde_json::from_value(raw).with_context(|| {
            format!(
                "{}: line {}: record does not match SampleRecord schema (schema: docs/AUDIT-LABEL-SCHEMA.md)",
                in_path.display(),
                idx + 1
            )
        })?;
        if record.schema_version != SAMPLE_SCHEMA_VERSION {
            anyhow::bail!(
                "{}: line {}: unknown schema_version {:?}; this scope build emits and accepts {:?} only \
                 (single-operator posture: no dual-read shim). Re-emit the sample with \
                 `scope audit confidence --emit-sample <new-path>` against the current index.",
                in_path.display(),
                idx + 1,
                record.schema_version,
                SAMPLE_SCHEMA_VERSION
            );
        }
        records.push((idx + 1, record));
    }

    // Partial labelling: `label=null` records are *skipped*, not rejected.
    //
    // A hard-rejection of nulls would contradict the documented
    // LSP-cross-check labeller flow in `docs/AUDIT-LABEL-SCHEMA.md`
    // (which explicitly says "leave undecided; --label tolerates
    // partial coverage" for edge kinds the LSP cannot classify).
    //
    // Resolution: tolerate partial coverage, but stay honest about the
    // denominator. `compute_precision_report` filters records with
    // `label.is_none()` *before* group accumulation, so:
    // - `sample_size` per row = number of LABELLED records in the group
    //   (the precision denominator)
    // - `correct_count` per row = number of `label = true` records
    // - precision = correct_count / sample_size — meaningful even when
    //   the labeller covered only part of the sample
    // - groups where every record was skipped (`sample_size == 0`)
    //   do not appear in the report (no denominator => no precision)
    //
    // This matches the schema doc's stated semantics and preserves the
    // Priority 2 honesty principle: the denominator is never inflated
    // by counting null records as judged. A future schema bump may add
    // a sibling `skipped_count` column to surface the partial-coverage
    // ratio per group; that lands in BACKLOG.md § Priority 1.
    let unlabelled = records.iter().filter(|(_, r)| r.label.is_none()).count();
    if unlabelled == records.len() {
        anyhow::bail!(
            "{}: every record has label=null; no labelling has been performed. \
             A labeller must fill at least one record's `label` field before --label can produce a report.",
            in_path.display(),
        );
    }

    // edge_id integrity gate : every record's
    // `edge_id` must (a) parse as i64 — the on-wire representation per
    // docs/AUDIT-LABEL-SCHEMA.md is a string solely for JSON-number-
    // safety reasons but the underlying DB column is `i64` — and
    // (b) resolve to a row in the current index. Records that fail
    // either check would otherwise be silently dropped from the drift
    // gate (which joins by edge_id) while still contributing to the
    // precision math. That is the same dishonesty the auditor
    // immutability rule (AUDIT-LABEL-SCHEMA.md § Auditor immutability
    // rule) forbids in source-drift form, in a different shape: here
    // the *sample file* drifts from the index, not the source. Hard
    // mechanical rejection, no escape flag — re-emit the sample
    // against the current index.
    let all_rows = graph.list_edges_for_audit()?;
    let indexed: BTreeSet<i64> = all_rows.iter().map(|r| r.edge_id).collect();
    let mut unparseable: Vec<(usize, String)> = Vec::new();
    let mut unknown: Vec<(usize, i64)> = Vec::new();
    let mut parsed_ids: BTreeSet<i64> = BTreeSet::new();
    // Duplicate-detection map: edge_id -> Vec<line_no>. Any entry with
    // len > 1 is a duplicate, which would otherwise collapse in the
    // `parsed_ids` set but still double-count in the precision report
    // .
    let mut by_id: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (line_no, record) in &records {
        match record.edge_id.parse::<i64>() {
            Err(_) => unparseable.push((*line_no, record.edge_id.clone())),
            Ok(id) => {
                by_id.entry(id).or_default().push(*line_no);
                if !indexed.contains(&id) {
                    unknown.push((*line_no, id));
                } else {
                    parsed_ids.insert(id);
                }
            }
        }
    }
    let duplicates: Vec<(i64, Vec<usize>)> = by_id
        .into_iter()
        .filter(|(_, lines)| lines.len() > 1)
        .collect();
    if !unparseable.is_empty() || !unknown.is_empty() || !duplicates.is_empty() {
        use std::fmt::Write as _;
        let mut msg = String::new();
        msg.push_str(
            "sample-file integrity check failed: every record's `edge_id` must parse as i64, \
             resolve to a row in the current index, and appear at most once. ",
        );
        if !unparseable.is_empty() {
            let _ = writeln!(
                msg,
                "\n  {} record(s) with non-integer edge_id:",
                unparseable.len()
            );
            for (line_no, raw) in &unparseable {
                let _ = writeln!(
                    msg,
                    "    {}: line {}: edge_id = {:?}",
                    in_path.display(),
                    line_no,
                    raw
                );
            }
        }
        if !unknown.is_empty() {
            let _ = writeln!(
                msg,
                "\n  {} record(s) whose edge_id is not in the current index:",
                unknown.len()
            );
            for (line_no, id) in &unknown {
                let _ = writeln!(
                    msg,
                    "    {}: line {}: edge_id = {}",
                    in_path.display(),
                    line_no,
                    id
                );
            }
        }
        if !duplicates.is_empty() {
            let _ = writeln!(
                msg,
                "\n  {} edge_id(s) repeated across multiple records (would double-count in the precision report):",
                duplicates.len()
            );
            for (id, lines) in &duplicates {
                let lines_csv: Vec<String> = lines.iter().map(|n| n.to_string()).collect();
                let _ = writeln!(
                    msg,
                    "    {}: edge_id = {} on lines {}",
                    in_path.display(),
                    id,
                    lines_csv.join(", ")
                );
            }
        }
        msg.push_str(
            "\nRemediation: re-emit the sample against the current index \
             (`scope audit confidence --emit-sample <new-path>`) and re-label, \
             ensuring each edge_id appears at most once. \
             There is no `--allow-integrity-skip` escape — silently dropping or \
             collapsing these records while still counting them in the precision \
             report would violate AUDIT-LABEL-SCHEMA.md § Auditor immutability rule.",
        );
        return Err(anyhow::anyhow!(msg));
    }

    // Source-drift gate runs BEFORE the tamper gate: if a source
    // file changed or was deleted between
    // `--emit-sample` and `--label`, the tamper gate would re-derive
    // the source_snippet from the now-different file and report a
    // sample-tamper error with re-emit remediation, when the *correct*
    // diagnosis is source drift with `scope index` remediation. Order
    // matters: drift gate first surfaces the right error class, then
    // the tamper gate (which reads source_snippet from disk) runs
    // against files whose content is byte-identical to what the
    // indexer saw at index time.
    let referenced_rows: Vec<AuditEdgeRow> = all_rows
        .iter()
        .filter(|r| parsed_ids.contains(&r.edge_id))
        .cloned()
        .collect();
    // Freshness gate runs **before** the lang_version recomputation
    // in the tamper check below: the detector reads manifests from
    // disk, and a working-tree drift since index time would let a
    // since-edited `Cargo.toml` / `tsconfig.json` shadow the indexed
    // truth. Failing fresh here guarantees the detector below sees
    // the same workspace state the indexer did.
    enforce_freshness(graph, project_root, &referenced_rows)?;

    // Report-key + endpoint + snippet tamper gate (after the round,
    // extended in round 4 to cover `from` / `to` / `source_snippet`).
    // The precision report groups rows by (kind, tier, producer,
    // pattern_id); the labeller READS `from` / `to` / `source_snippet`
    // to make a verdict. Any of those rewritten in the JSONL = the
    // labeller judged a different edge / context than the one the
    // report will credit. Per the auditor-independence principle the
    // labeller fills `label` only; rewriting any other non-`label`
    // field is sample drift (sibling to source drift, already cleared
    // above) and gets the same hard mechanical rejection treatment.
    //
    // With the drift gate ahead of this, file reads inside the loop
    // are guaranteed to succeed (file content matches the indexer's
    // hash); we propagate any unexpected read failure (race with rm,
    // permission flip mid-audit) instead of masking it as a snippet
    // diff.
    let mut indexed_by_id: HashMap<i64, &AuditEdgeRow> = HashMap::new();
    for row in &all_rows {
        indexed_by_id.insert(row.edge_id, row);
    }
    // (field-name, sample value, indexed value) — one entry per axis that diverges.
    type FieldDiff = (&'static str, String, String);
    let mut tampered: Vec<(usize, Vec<FieldDiff>)> = Vec::new();
    for (line_no, record) in &records {
        let id = record
            .edge_id
            .parse::<i64>()
            .expect("edge_id parse already validated above");
        let indexed_row = indexed_by_id
            .get(&id)
            .expect("edge_id presence already validated above");
        let mut diffs: Vec<FieldDiff> = Vec::new();
        if record.kind != indexed_row.kind {
            diffs.push(("kind", record.kind.clone(), indexed_row.kind.clone()));
        }
        if record.confidence != indexed_row.confidence {
            diffs.push((
                "confidence",
                record.confidence.clone(),
                indexed_row.confidence.clone(),
            ));
        }
        if record.producer != indexed_row.producer {
            diffs.push((
                "producer",
                record.producer.clone(),
                indexed_row.producer.clone(),
            ));
        }
        if record.pattern_id != indexed_row.pattern_id {
            diffs.push((
                "pattern_id",
                record.pattern_id.clone(),
                indexed_row.pattern_id.clone(),
            ));
        }
        if record.from != indexed_row.from_id {
            diffs.push(("from", record.from.clone(), indexed_row.from_id.clone()));
        }
        if record.to != indexed_row.to_id {
            diffs.push(("to", record.to.clone(), indexed_row.to_id.clone()));
        }
        // Drift gate already verified file content == indexer's hash;
        // read errors here are unexpected (concurrent rm / permission
        // race) and propagate rather than silently degrading to an
        // empty-string snippet diff.
        let expected_snippet =
            read_source_snippet(project_root, &indexed_row.file_path, indexed_row.line)?;
        if record.source_snippet != expected_snippet {
            diffs.push((
                "source_snippet",
                record.source_snippet.clone(),
                expected_snippet,
            ));
        }
        // `lang_version` field — the schema's reserved per-project
        // lang_version slot, populated by the indexer-side detector
        // matrix (BACKLOG.md § Priority 1 sub-item (d)). The labeller
        // fills `label` only; rewriting `lang_version` is sample
        // tamper on the auditor-immutability rule. Recompute via the
        // same detector entry point used on emit and compare; any
        // mismatch is a rewrite.
        let expected_lang_version =
            detect_lang_version(project_root, &project_root.join(&indexed_row.file_path));
        if record.lang_version != expected_lang_version {
            diffs.push((
                "lang_version",
                format!("{:?}", record.lang_version),
                format!("{:?}", expected_lang_version),
            ));
        }
        if !diffs.is_empty() {
            tampered.push((*line_no, diffs));
        }
    }
    if !tampered.is_empty() {
        use std::fmt::Write as _;
        let mut msg = String::new();
        let _ = writeln!(
            msg,
            "sample-file tamper check failed: {} record(s) carry non-`label` fields \
             (kind / confidence / producer / pattern_id / from / to / source_snippet / lang_version) \
             that disagree with the indexed edge. The labeller may set `label` only; rewriting any \
             other field invalidates the audit because the labeller's verdict then applies to a \
             different edge / context than the one the precision report will credit.",
            tampered.len()
        );
        for (line_no, diffs) in &tampered {
            let _ = writeln!(msg, "\n  {}: line {}:", in_path.display(), line_no);
            for (field, sample, indexed_val) in diffs {
                let _ = writeln!(
                    msg,
                    "    {field}: sample = {sample:?}, indexed = {indexed_val:?}"
                );
            }
        }
        msg.push_str(
            "\nRemediation: re-emit the sample against the current index \
             (`scope audit confidence --emit-sample <new-path>`) and re-label, \
             modifying only the `label` field of each record. \
             There is no `--allow-tampering-skip` escape — silently substituting \
             labeller-supplied report keys for indexed values would hide the drift \
             and produce a precision report with mis-stratified rows, violating \
             AUDIT-LABEL-SCHEMA.md § Auditor immutability rule.",
        );
        return Err(anyhow::anyhow!(msg));
    }

    let records_only: Vec<SampleRecord> = records.into_iter().map(|(_, r)| r).collect();
    let report = compute_precision_report(&records_only);
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
/// Tier targets per `docs/ENFORCEMENT-MAP.md` § R8 (verbatim):
/// - `high ≥ 95%`
/// - `medium ≥ 70%`
/// - `low` has no minimum
///
/// Unknown tier strings produce an error — better to fail loudly than
/// silently accept a tier that was never reviewed against a target.
pub fn check_tier_gate(report: &PrecisionReport) -> Result<()> {
    use std::fmt::Write as _;
    // `(row, measured_precision)` — we only fire on rows where a
    // precision number actually exists. Fully-skipped rows
    // (`precision = None`) have nothing to enforce against; the
    // coverage gap is surfaced through `coverage_summary` instead.
    let mut failures: Vec<(&ReportRow, f64)> = Vec::new();
    for row in &report.report {
        let min = match row.tier.as_str() {
            "high" => HIGH_TIER_MIN,
            "medium" => MEDIUM_TIER_MIN,
            "low" => continue,
            other => anyhow::bail!(
                "unknown tier {other:?} in report row (kind={}, producer={}, pattern_id={}); \
                 expected `high` / `medium` / `low` per docs/ENFORCEMENT-MAP.md § R8",
                row.kind,
                row.producer,
                row.pattern_id
            ),
        };
        let Some(p) = row.precision else {
            continue;
        };
        if p < min {
            failures.push((row, p));
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
    for (row, p) in &failures {
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
            p,
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

/// Per-group accumulator: `(labelled_count, correct_count, skipped_count)`.
type GroupAcc = (usize, usize, usize);

/// Group records by `(kind, tier, producer, pattern_id)` and compute
/// precision plus per-group coverage.
///
/// `tier` is taken from each record's `confidence` field — they are the
/// same string vocabulary (`"high"` / `"medium"` / `"low"`) per the
/// schema. Group iteration order is sorted (`BTreeMap`) so the report
/// is byte-for-byte deterministic given the same input.
///
/// `label.is_none()` records are **counted** against this group's
/// `skipped_count`, not dropped. A group whose every record was skipped
/// still appears in the report with `precision = None` and
/// `coverage_ratio = 0.0` — surfacing the coverage gap is the whole
/// point of the post-bump shape ([`BACKLOG.md` § Priority 1 sub-item
/// (h)]).
///
/// `precision = Some(correct_count as f64 / labelled_count as f64)`
/// when `labelled_count > 0`; `None` otherwise.
pub fn compute_precision_report(records: &[SampleRecord]) -> PrecisionReport {
    let mut groups: BTreeMap<(String, String, String, String), GroupAcc> = BTreeMap::new();
    for r in records {
        let key = (
            r.kind.clone(),
            r.confidence.clone(),
            r.producer.clone(),
            r.pattern_id.clone(),
        );
        let entry = groups.entry(key).or_insert((0, 0, 0));
        match r.label {
            Some(true) => {
                entry.0 += 1; // labelled_count
                entry.1 += 1; // correct_count
            }
            Some(false) => {
                entry.0 += 1; // labelled_count
            }
            None => {
                entry.2 += 1; // skipped_count
            }
        }
    }

    let mut records_labelled = 0usize;
    let mut records_skipped = 0usize;
    let mut distinct_groups_with_coverage = 0usize;
    let mut distinct_groups_fully_skipped = 0usize;

    let rows: Vec<ReportRow> = groups
        .into_iter()
        .map(
            |((kind, tier, producer, pattern_id), (labelled, correct, skipped))| {
                records_labelled += labelled;
                records_skipped += skipped;
                if labelled > 0 {
                    distinct_groups_with_coverage += 1;
                } else {
                    distinct_groups_fully_skipped += 1;
                }
                let denom = labelled + skipped;
                // denom > 0 always — a group cannot exist with zero records;
                // it materialised because at least one record fell into it.
                let coverage_ratio = labelled as f64 / denom as f64;
                let precision = if labelled > 0 {
                    Some(correct as f64 / labelled as f64)
                } else {
                    None
                };
                ReportRow {
                    kind,
                    tier,
                    producer,
                    pattern_id,
                    sample_size: labelled,
                    labelled_count: labelled,
                    skipped_count: skipped,
                    coverage_ratio,
                    correct_count: correct,
                    precision,
                }
            },
        )
        .collect();

    let coverage_summary = CoverageSummary {
        records_total: records_labelled + records_skipped,
        records_labelled,
        records_skipped,
        distinct_groups_with_coverage,
        distinct_groups_fully_skipped,
    };

    PrecisionReport {
        schema_version: REPORT_SCHEMA_VERSION.to_string(),
        disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
        sample_schema_doc: SCHEMA_DOC_POINTER.to_string(),
        coverage_summary,
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
/// TSV: 3-line `#`-prefixed preamble (disclaimer, sample-file schema
/// pointer, coverage-summary line) then a header then one row per
/// `ReportRow`. Precision is rendered with four decimal places — enough
/// resolution to distinguish 0.95 from 0.9499 (the tier boundary)
/// without flooding shell output with float noise. Fully-skipped rows
/// (`precision = None`) render an empty cell so awk/cut/Miller can
/// pattern-match the missing-precision case via `$N == ""`.
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
            // Preamble: `#`-prefixed comments carrying the disclaimer,
            // sample-file schema doc pointer, and the structured
            // coverage_summary line. Both surfaces (JSON
            // `coverage_summary` object, TSV preamble line) carry the
            // same numbers so the operator gets the full coverage
            // picture from either format. Standard TSV consumers
            // (awk / cut / Miller / csvkit) either ignore `#` lines
            // via a `--comment` flag or can be teed through
            // `grep -v '^#'` before parsing.
            writeln!(out, "# {}", report.disclaimer)?;
            writeln!(out, "# {}", report.sample_schema_doc)?;
            let cs = &report.coverage_summary;
            writeln!(
                out,
                "# coverage_summary: total={} labelled={} skipped={} groups_with_coverage={} groups_fully_skipped={}",
                cs.records_total,
                cs.records_labelled,
                cs.records_skipped,
                cs.distinct_groups_with_coverage,
                cs.distinct_groups_fully_skipped,
            )?;
            writeln!(
                out,
                "kind\ttier\tproducer\tpattern_id\tsample_size\tlabelled_count\tskipped_count\tcoverage_ratio\tcorrect_count\tprecision"
            )?;
            for row in &report.report {
                let precision_cell = match row.precision {
                    Some(p) => format!("{p:.4}"),
                    None => String::new(),
                };
                writeln!(
                    out,
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{}\t{}",
                    row.kind,
                    row.tier,
                    row.producer,
                    row.pattern_id,
                    row.sample_size,
                    row.labelled_count,
                    row.skipped_count,
                    row.coverage_ratio,
                    row.correct_count,
                    precision_cell,
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
fn read_source_snippet(project_root: &Path, file_path: &str, line: Option<u32>) -> Result<String> {
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

    let mut state = if seed == 0 {
        0x9E37_79B9_7F4A_7C15
    } else {
        seed
    };
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
            vec![("calls", "high"), ("extends", "medium"), ("imports", "low"),]
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
    fn sample_record_serialises_with_schema_v2() {
        let row = mk_row(42, "calls", "high");
        let rec = SampleRecord::from_row(&row, "format_name(&user.name)".to_string(), None);
        let json = serde_json::to_string(&rec).unwrap();
        // schema_version "2"; edge_id stringified; nullable fields null on emit.
        assert!(json.contains("\"schema_version\":\"2\""));
        assert!(json.contains("\"edge_id\":\"42\""));
        assert!(json.contains("\"source_snippet\":\"format_name(&user.name)\""));
        assert!(json.contains("\"lang_version\":null"));
        assert!(json.contains("\"label\":null"));
        // v2 fields all null on emit.
        assert!(json.contains("\"evidence\":null"));
        assert!(json.contains("\"target_proposed\":null"));
        assert!(json.contains("\"kind_proposed\":null"));
        assert!(json.contains("\"confidence_proposed\":null"));
        assert!(json.contains("\"reasoning_text\":null"));
        assert!(json.contains("\"lang_version_evidence\":null"));
        assert!(json.contains("\"labeller_id\":null"));
    }

    #[test]
    fn sample_record_round_trips_through_jsonl() {
        let row = mk_row(7, "imports", "medium");
        let original = SampleRecord::from_row(&row, "use std::fs;".to_string(), None);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: SampleRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn sample_record_keeps_field_order_for_deterministic_jsonl() {
        let row = mk_row(1, "calls", "high");
        let rec = SampleRecord::from_row(&row, "x".to_string(), None);
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
            "\"evidence\"",
            "\"target_proposed\"",
            "\"kind_proposed\"",
            "\"confidence_proposed\"",
            "\"reasoning_text\"",
            "\"lang_version_evidence\"",
            "\"labeller_id\"",
        ];
        let mut last_pos = 0usize;
        for key in expected_order {
            let pos = json.find(key).unwrap_or_else(|| panic!("missing {key}"));
            assert!(pos > last_pos, "field {key} out of order in {json}");
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
            let mut rec = SampleRecord::from_row(&row, String::new(), None);
            rec.label = expected;
            let json = serde_json::to_string(&rec).unwrap();
            assert!(json.contains(substr), "expected {substr} in {json}");
            let parsed: SampleRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.label, expected);
        }
    }

    #[test]
    fn sample_schema_version_constant_at_two() {
        // The schema is contract-grade per docs/AUDIT-LABEL-SCHEMA.md.
        // Bumping is charter-grade and must land via the BACKLOG sprint
        // that owns the bumped surface; this assertion is the canary
        // against drive-by edits. Single-operator posture
        // (CHARTER.md § 3): exactly one accepted version on read; a
        // future bump wipes the committed corpus + re-emits, never
        // dual-read.
        assert_eq!(SAMPLE_SCHEMA_VERSION, "2");
    }

    #[test]
    fn report_schema_version_constant_at_two() {
        // CP3 (sprint 0004 sub-item (h)) bumps the report version to
        // "2" together with the per-group coverage surface (labelled /
        // skipped / coverage_ratio per row + coverage_summary top-level).
        assert_eq!(REPORT_SCHEMA_VERSION, "2");
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
        record(edge_id, kind, confidence, producer, pattern_id, Some(label))
    }

    fn skipped(
        edge_id: i64,
        kind: &str,
        confidence: &str,
        producer: &str,
        pattern_id: &str,
    ) -> SampleRecord {
        record(edge_id, kind, confidence, producer, pattern_id, None)
    }

    fn record(
        edge_id: i64,
        kind: &str,
        confidence: &str,
        producer: &str,
        pattern_id: &str,
        label: Option<bool>,
    ) -> SampleRecord {
        SampleRecord {
            schema_version: SAMPLE_SCHEMA_VERSION.to_string(),
            edge_id: edge_id.to_string(),
            kind: kind.to_string(),
            confidence: confidence.to_string(),
            producer: producer.to_string(),
            pattern_id: pattern_id.to_string(),
            from: format!("f{edge_id}"),
            to: format!("t{edge_id}"),
            source_snippet: String::new(),
            lang_version: None,
            label,
            evidence: None,
            target_proposed: None,
            kind_proposed: None,
            confidence_proposed: None,
            reasoning_text: None,
            lang_version_evidence: None,
            labeller_id: None,
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
        assert_eq!(report.schema_version, "2");
        assert_eq!(report.report.len(), 3);

        let g1 = report
            .report
            .iter()
            .find(|r| r.pattern_id == "rust.calls.method")
            .unwrap();
        assert_eq!(g1.sample_size, 3);
        assert_eq!(g1.labelled_count, 3);
        assert_eq!(g1.skipped_count, 0);
        assert_eq!(g1.correct_count, 2);
        assert_eq!(g1.coverage_ratio, 1.0);
        assert!((g1.precision.unwrap() - 2.0 / 3.0).abs() < 1e-9);

        let g2 = report
            .report
            .iter()
            .find(|r| r.pattern_id == "rust.calls.fn")
            .unwrap();
        assert_eq!(g2.sample_size, 1);
        assert_eq!(g2.labelled_count, 1);
        assert_eq!(g2.skipped_count, 0);
        assert_eq!(g2.correct_count, 1);
        assert_eq!(g2.coverage_ratio, 1.0);
        assert_eq!(g2.precision, Some(1.0));

        let g3 = report
            .report
            .iter()
            .find(|r| r.pattern_id == "rust.imports.use")
            .unwrap();
        assert_eq!(g3.sample_size, 1);
        assert_eq!(g3.labelled_count, 1);
        assert_eq!(g3.skipped_count, 0);
        assert_eq!(g3.correct_count, 0);
        assert_eq!(g3.precision, Some(0.0));

        // Fully-labelled corpus: coverage_summary is the trivial case
        // (records_total = records_labelled, no skipped, no fully-skipped
        // groups). The non-trivial cases live in the dedicated tests
        // below.
        let cs = &report.coverage_summary;
        assert_eq!(cs.records_total, 5);
        assert_eq!(cs.records_labelled, 5);
        assert_eq!(cs.records_skipped, 0);
        assert_eq!(cs.distinct_groups_with_coverage, 3);
        assert_eq!(cs.distinct_groups_fully_skipped, 0);
    }

    #[test]
    fn precision_report_groups_count_skipped_records_per_group() {
        // A group with a mix of labelled and skipped records:
        // - labelled_count tracks `label != null`
        // - skipped_count tracks `label == null`
        // - coverage_ratio = labelled / (labelled + skipped)
        // - precision uses only labelled records as the denominator
        let records = vec![
            labelled(1, "calls", "high", "rust", "p1", true),
            labelled(2, "calls", "high", "rust", "p1", true),
            skipped(3, "calls", "high", "rust", "p1"),
            skipped(4, "calls", "high", "rust", "p1"),
        ];
        let report = compute_precision_report(&records);
        let g = &report.report[0];
        assert_eq!(g.labelled_count, 2);
        assert_eq!(g.skipped_count, 2);
        assert_eq!(g.coverage_ratio, 0.5);
        assert_eq!(g.precision, Some(1.0));
        assert_eq!(g.correct_count, 2);

        let cs = &report.coverage_summary;
        assert_eq!(cs.records_total, 4);
        assert_eq!(cs.records_labelled, 2);
        assert_eq!(cs.records_skipped, 2);
        assert_eq!(cs.distinct_groups_with_coverage, 1);
        assert_eq!(cs.distinct_groups_fully_skipped, 0);
    }

    #[test]
    fn precision_report_emits_fully_skipped_groups_with_precision_none() {
        // A group with zero labelled records still appears in the
        // report — the operator must see the coverage gap. Precision is
        // None (no measurement possible); coverage_ratio is 0.0;
        // distinct_groups_fully_skipped counts the group.
        let records = vec![
            skipped(1, "calls", "high", "rust", "p_skip"),
            skipped(2, "calls", "high", "rust", "p_skip"),
            labelled(3, "imports", "medium", "rust", "p_ok", true),
        ];
        let report = compute_precision_report(&records);
        assert_eq!(report.report.len(), 2);

        let skip_row = report
            .report
            .iter()
            .find(|r| r.pattern_id == "p_skip")
            .unwrap();
        assert_eq!(skip_row.labelled_count, 0);
        assert_eq!(skip_row.skipped_count, 2);
        assert_eq!(skip_row.sample_size, 0);
        assert_eq!(skip_row.coverage_ratio, 0.0);
        assert_eq!(skip_row.precision, None);
        assert_eq!(skip_row.correct_count, 0);

        let cs = &report.coverage_summary;
        assert_eq!(cs.records_total, 3);
        assert_eq!(cs.records_labelled, 1);
        assert_eq!(cs.records_skipped, 2);
        assert_eq!(cs.distinct_groups_with_coverage, 1);
        assert_eq!(cs.distinct_groups_fully_skipped, 1);
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

    fn make_row(
        kind: &str,
        tier: &str,
        producer: &str,
        pattern_id: &str,
        labelled: usize,
        skipped: usize,
        correct: usize,
    ) -> ReportRow {
        let total = labelled + skipped;
        let coverage_ratio = if total == 0 {
            0.0
        } else {
            labelled as f64 / total as f64
        };
        let precision = if labelled > 0 {
            Some(correct as f64 / labelled as f64)
        } else {
            None
        };
        ReportRow {
            kind: kind.to_string(),
            tier: tier.to_string(),
            producer: producer.to_string(),
            pattern_id: pattern_id.to_string(),
            sample_size: labelled,
            labelled_count: labelled,
            skipped_count: skipped,
            coverage_ratio,
            correct_count: correct,
            precision,
        }
    }

    fn make_summary(
        records_total: usize,
        records_labelled: usize,
        records_skipped: usize,
        groups_cov: usize,
        groups_skip: usize,
    ) -> CoverageSummary {
        CoverageSummary {
            records_total,
            records_labelled,
            records_skipped,
            distinct_groups_with_coverage: groups_cov,
            distinct_groups_fully_skipped: groups_skip,
        }
    }

    #[test]
    fn write_report_json_carries_schema_disclaimer_rows() {
        let report = PrecisionReport {
            schema_version: "2".to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            sample_schema_doc: SCHEMA_DOC_POINTER.to_string(),
            coverage_summary: make_summary(30, 30, 0, 1, 0),
            report: vec![make_row(
                "calls",
                "high",
                "rust",
                "rust.calls.method",
                30,
                0,
                29,
            )],
        };
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Json, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"schema_version\": \"2\""));
        assert!(s.contains(PRECISION_ONLY_DISCLAIMER));
        assert!(s.contains("\"sample_size\": 30"));
        assert!(s.contains("\"labelled_count\": 30"));
        assert!(s.contains("\"skipped_count\": 0"));
        assert!(s.contains("\"coverage_ratio\":"));
        assert!(s.contains("\"correct_count\": 29"));
        assert!(s.contains("\"precision\":"));
        assert!(s.contains("\"coverage_summary\""));
        assert!(s.contains("\"records_total\": 30"));
        // Pretty-printed: contains a newline (not a single-line blob).
        assert!(s.contains('\n'));
    }

    #[test]
    fn write_report_tsv_has_header_and_one_row_per_group() {
        let report = PrecisionReport {
            schema_version: "2".to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            sample_schema_doc: SCHEMA_DOC_POINTER.to_string(),
            coverage_summary: make_summary(42, 42, 0, 2, 0),
            report: vec![
                make_row("calls", "high", "rust", "rust.calls.method", 30, 0, 29),
                make_row("imports", "medium", "python", "p.imports.from", 12, 0, 9),
            ],
        };
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Tsv, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<_> = s.lines().collect();
        // 3 preamble (`#` disclaimer + `#` schema-doc pointer + `#`
        // coverage-summary) + 1 header + 2 rows.
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0], format!("# {PRECISION_ONLY_DISCLAIMER}"));
        assert_eq!(lines[1], format!("# {SCHEMA_DOC_POINTER}"));
        assert_eq!(
            lines[2],
            "# coverage_summary: total=42 labelled=42 skipped=0 groups_with_coverage=2 groups_fully_skipped=0"
        );
        assert_eq!(
            lines[3],
            "kind\ttier\tproducer\tpattern_id\tsample_size\tlabelled_count\tskipped_count\tcoverage_ratio\tcorrect_count\tprecision"
        );
        assert_eq!(
            lines[4],
            "calls\thigh\trust\trust.calls.method\t30\t30\t0\t1.0000\t29\t0.9667"
        );
        assert_eq!(
            lines[5],
            "imports\tmedium\tpython\tp.imports.from\t12\t12\t0\t1.0000\t9\t0.7500"
        );
    }

    #[test]
    fn write_report_tsv_renders_empty_cell_for_skipped_only_groups() {
        // Fully-skipped group: precision = None. TSV cell must be
        // empty so `awk -F'\t' '$10 == ""'` matches the
        // missing-precision case cleanly.
        let report = PrecisionReport {
            schema_version: "2".to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            sample_schema_doc: SCHEMA_DOC_POINTER.to_string(),
            coverage_summary: make_summary(2, 0, 2, 0, 1),
            report: vec![make_row("calls", "high", "rust", "p_skip", 0, 2, 0)],
        };
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Tsv, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let last = s.lines().last().unwrap();
        // 10 columns: trailing cell is empty (line ends with the
        // `0\t` for correct_count then nothing).
        assert!(
            last.ends_with("\t0\t"),
            "fully-skipped row must end with empty precision cell: {last:?}"
        );
        let cols: Vec<&str> = last.split('\t').collect();
        assert_eq!(cols.len(), 10);
        assert_eq!(cols[9], "");
    }

    #[test]
    fn write_report_json_carries_sample_schema_doc_field() {
        let report = compute_precision_report(&[]);
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Json, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // Field name + AUDIT-LABEL-SCHEMA.md path both present.
        assert!(s.contains("\"sample_schema_doc\""));
        assert!(s.contains("docs/AUDIT-LABEL-SCHEMA.md"));
    }

    #[test]
    fn write_report_tsv_preamble_uses_hash_prefix_for_consumers() {
        // Standard TSV consumers (awk / Miller / csvkit) recognise `#`
        // comment prefix; assert the preamble lines are `#`-prefixed so
        // a future format change that loses the prefix fails here.
        let report = compute_precision_report(&[]);
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Tsv, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        for line in s.lines().take(3) {
            assert!(
                line.starts_with("# "),
                "preamble line must start with `# `: {line:?}"
            );
        }
    }

    #[test]
    fn write_report_carries_coverage_summary_on_both_surfaces() {
        // The coverage_summary is the operator-facing headline that
        // contextualises every precision number. It must appear on both
        // JSON (top-level object) and TSV (preamble line) so any
        // consumer reads it as part of the report header.
        let records = vec![
            labelled(1, "calls", "high", "rust", "p1", true),
            labelled(2, "calls", "high", "rust", "p1", true),
            skipped(3, "calls", "high", "rust", "p1"),
            skipped(4, "imports", "medium", "rust", "p_skip"),
        ];
        let report = compute_precision_report(&records);
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Json, &mut buf).unwrap();
        let json = String::from_utf8(buf).unwrap();
        assert!(json.contains("\"coverage_summary\""));
        assert!(json.contains("\"records_total\": 4"));
        assert!(json.contains("\"records_labelled\": 2"));
        assert!(json.contains("\"records_skipped\": 2"));
        assert!(json.contains("\"distinct_groups_with_coverage\": 1"));
        assert!(json.contains("\"distinct_groups_fully_skipped\": 1"));
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Tsv, &mut buf).unwrap();
        let tsv = String::from_utf8(buf).unwrap();
        let third = tsv.lines().nth(2).unwrap();
        assert!(
            third.starts_with("# coverage_summary:"),
            "third preamble line must carry coverage_summary: {third:?}"
        );
        assert!(third.contains("total=4"));
        assert!(third.contains("labelled=2"));
        assert!(third.contains("skipped=2"));
        assert!(third.contains("groups_with_coverage=1"));
        assert!(third.contains("groups_fully_skipped=1"));
    }

    #[test]
    fn precision_report_carries_coverage_summary() {
        // Empty input: all summary counters zero. The struct still
        // ships on the report so external consumers can parse a
        // well-shaped object regardless of input.
        let report = compute_precision_report(&[]);
        let cs = &report.coverage_summary;
        assert_eq!(cs.records_total, 0);
        assert_eq!(cs.records_labelled, 0);
        assert_eq!(cs.records_skipped, 0);
        assert_eq!(cs.distinct_groups_with_coverage, 0);
        assert_eq!(cs.distinct_groups_fully_skipped, 0);
    }

    #[test]
    fn precision_report_carries_sample_schema_doc_pointer() {
        let report = compute_precision_report(&[]);
        assert_eq!(report.sample_schema_doc, SCHEMA_DOC_POINTER);
        assert!(report.sample_schema_doc.contains("AUDIT-LABEL-SCHEMA.md"));
        assert!(report.sample_schema_doc.contains("schema_version"));
    }

    // -- Chunk 6: tier gate --

    fn report_with_rows(rows: Vec<ReportRow>) -> PrecisionReport {
        PrecisionReport {
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            sample_schema_doc: SCHEMA_DOC_POINTER.to_string(),
            coverage_summary: make_summary(0, 0, 0, 0, 0),
            report: rows,
        }
    }

    fn row(kind: &str, tier: &str, pattern_id: &str, n: usize, k: usize) -> ReportRow {
        make_row(kind, tier, "rust", pattern_id, n, 0, k)
    }

    #[test]
    fn tier_gate_passes_when_every_row_meets_target() {
        let report = report_with_rows(vec![
            row("calls", "high", "p1", 20, 19), // 0.95 — exactly at boundary
            row("imports", "medium", "p2", 10, 7), // 0.70 — exactly at boundary
            row("extends", "low", "p3", 5, 0),  // low: no minimum
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
            schema_version: REPORT_SCHEMA_VERSION.to_string(),
            disclaimer: PRECISION_ONLY_DISCLAIMER.to_string(),
            sample_schema_doc: SCHEMA_DOC_POINTER.to_string(),
            coverage_summary: make_summary(10200, 10200, 0, 2, 0),
            report: vec![
                make_row("calls", "high", "rust", "p1", 200, 0, 190), // 0.9500
                make_row("calls", "high", "rust", "p2", 10000, 0, 9499), // 0.9499
            ],
        };
        let mut buf = Vec::new();
        write_report(&report, ReportFormat::Tsv, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\t0.9500\n"));
        assert!(s.contains("\t0.9499\n"));
    }
}
