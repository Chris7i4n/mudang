# Sprint 0004 — Priority 1: schema_version "1" → "2" bundle (verdict types + report coverage + audit-history DB)

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-items **(g) Richer auditor verdict types**, **(h) Per-group coverage surfaced on the precision report**, **(j) Audit-history persistence in the DB**.
> **Phase**: B (multi-deliverable atomic). BACKLOG (g) mandates "bump lands together with sub-item (h) and sub-item (j)" — therefore all three ship in one sprint, not three.
> Sprint is the phase. Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Bump the audit-sample JSONL `schema_version` from `"1"` to `"2"` across three coordinated entry points so the auditor surface ships its qualitative-signal upgrade atomically. The bundle is mandated by [`BACKLOG.md`](../BACKLOG.md): "The bump lands together with sub-item (h) (coverage surfacing on the report side) and sub-item (j) (DB storage shape), because all three are entry points for the same qualitative-signal surface." Splitting the bundle would create a half-upgraded surface that the labeller crates (sprint 0005) cannot target cleanly.

## Scope owned this sprint

- **Priority 1 (g)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))
- **Priority 1 (h)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))
- **Priority 1 (j)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprints 0001 + 0002 + 0003 shipped. Loop architecture doc + corpus policy + lang_version coverage in place.

## Charter alignment

- **Hard limits** — none crossed.
- **Soft expansion zone** — `CHARTER.md` §6.
- **Invariants** — auditor-immutability ([`CHARTER.md` § Core invariants](../CHARTER.md#3-core-invariants--must-never-break)) **extended**, not broken: BACKLOG (j) carves a writable namespace for audit-derived rows that never mutates source-derived rows (`edges`, `symbols`, `file_hashes`). The new `edge_audit_history` table is writable; the source-derived schema stays frozen during audit. [`AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule](../AUDIT-LABEL-SCHEMA.md#auditor-immutability-rule) gains a paragraph carving out the writable namespace explicitly.
- **Single-operator posture** — schema bump v1 → v2: `--label` accepts both versions, treating new fields as `null` when absent (per BACKLOG (g) explicit clause). Removing or repurposing existing `"1"` fields stays charter-grade.

## Deliverables

### Priority 1 (g) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) `schema_version` bumped `"1"` → `"2"`.
- [ ] New `"2"` record fields added (each `null` on emit, populated by capable labellers; partial population tolerated): `evidence`, `target_proposed`, `kind_proposed`, `confidence_proposed`, `reasoning_text`, `lang_version_evidence`, `labeller_id`.
- [ ] `--label` accepts both `schema_version: "1"` and `"2"` inputs; `"1"` records treat new fields as `null`.
- [ ] Migration note added to [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) explaining the bump and backward acceptance.

### Priority 1 (h) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] Per-row report fields added: `skipped_count: usize`, `labelled_count: usize` (alias for current `sample_size`), `coverage_ratio: f64 = labelled_count / (labelled_count + skipped_count)`.
- [ ] Top-level `coverage_summary` object added: `records_total`, `records_labelled`, `records_skipped`, `distinct_groups_with_coverage`, `distinct_groups_fully_skipped`.
- [ ] `COVERAGE_LIMITATION_NOTE` in `gumiho-mudang-cli/src/commands/audit.rs` and the inline comment on `compute_precision_report` are removed (the gap they flag closes here).

### Priority 1 (j) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] New table `edge_audit_history` added in [`scope-graph/src/sql/schema.sql`](../../scope-graph/src/sql/schema.sql) with columns `(audit_id, edge_id, labelled_at, labeller_id, label, target_proposed, kind_proposed, confidence_proposed, evidence_json)`. Append-only.
- [ ] Indices created: `(edge_id, audit_id)` and `(labeller_id, audit_id)`.
- [ ] Sibling auditor-immutability rule enforced: `--label` writes to `edge_audit_history` only; **never** mutates `edges` / `symbols` / `file_hashes`. New CI gate or audit script enforces this.
- [ ] [`AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule](../AUDIT-LABEL-SCHEMA.md#auditor-immutability-rule) gains the writable-namespace paragraph.
- [ ] New subcommand `scope audit history` implemented (read-side surface), three forms — drill-down workflow over the new table:
  - **`scope audit history`** (no subcommand) — default aggregate dashboard: headline (latest `audit_id`, overall precision, `records_total`); top-N patterns regressing (precision delta vs previous audit); top-N edges flapping (most `correct↔incorrect` label flips across audits).
  - **`scope audit history edge <edge_id>`** — drill: chronological label timeline for one edge (columns: `audit_id`, `labelled_at`, `labeller_id`, `label`, `target_proposed`, `kind_proposed`, `confidence_proposed`).
  - **`scope audit history pattern <pattern_id>`** — drill: precision-over-time for one pattern plus the edges currently labelled `incorrect` driving the regression.
  - **Deferred to sprint 0006** under sub-item (i): `scope audit history labeller <id>` and `scope audit history agreement-matrix`. Rationale: charter single-operator posture (CHARTER §3 invariant 1) — a single human labeller renders both views sparse-to-empty until multi-labeller pipelines exist. Both views logged in [`BACKLOG.md` § Priority 1 (j)](../BACKLOG.md#priority-1--self-correction-cycle) so the surface is not lost; sprint 0006 picks them up alongside (i)'s aggregation policy where cross-labeller density makes them meaningful.

### Implementation deliverables (cross-cutting)

- [ ] `SampleRecord` struct updated in `gumiho-mudang-cli/src/commands/audit.rs` (or post-Priority-3 location) to carry v2 fields.
- [ ] `PrecisionReport` / `ReportRow` updated with coverage fields.
- [ ] `SCHEMA_VERSION` constant bumped.
- [ ] DB migration: wipe-and-reindex absorbs the schema impact per single-operator posture ([`CHARTER.md` § Single-operator posture](../CHARTER.md#single-operator-posture)). No dual-read path.
- [ ] [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) R0 / R8 entries updated: schema closure expanded; new immutability sub-rule for the audit-history namespace registered.

---

## Ambiguities resolved before this sprint opens

- **Single namespace name** — `edge_audit_history` per BACKLOG (j) ("or analogous"). If a clash with an existing schema name emerges, halt and choose on `main` first.
- **`scope audit history` flag surface** — confirmed before sprint open. Subcommand-per-view layout chosen over flat flags (extensibility for view-specific `--since`, `--limit`, `--json`; mutually-exclusive flags would force ad-hoc validation otherwise). Three views ship in sprint 0004 (default dashboard + `edge <id>` + `pattern <id>`); two views (`labeller <id>` + `agreement-matrix`) defer to sprint 0006 (i) under single-operator-posture reasoning. Decision logged in [`BACKLOG.md` § Priority 1 (j)](../BACKLOG.md#priority-1--self-correction-cycle) and [`BACKLOG.md` § Priority 1 (i)](../BACKLOG.md#priority-1--self-correction-cycle); commit `db4c3ac` on `main`.
- **Retention policy** — BACKLOG (j) says "accumulates indefinitely; eviction is a future sprint". No eviction this sprint.

---

## CI gates activated in this sprint

- [ ] **edge_audit_history-source-immutability** — audit-script gate verifying `--label` writes touch only `edge_audit_history`, never `edges` / `symbols` / `file_hashes`. `planned → active`. New row in [`CI-GATES.md`](../CI-GATES.md).
- [ ] Possibly: **schema_v2 round-trip** — both `"1"` and `"2"` JSONL inputs are accepted by `--label` and produce identical-shape reports. `planned → active`.

## Glossary terms touched

- `schema_version` ([`GLOSSARY.md` § Schema](../GLOSSARY.md#schema)) — refined.
- `auditor-immutability` — refined; writable-namespace clause added.
- New: `labeller_id`, `audit-history table`, `coverage_ratio` — halt and add via glossary's commit channel before resuming if absent.

## Reporting

- **Branch**: `selfcorrect/sprint-0004-schema-v2-bundle`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint). Review focus must include the writable-namespace carveout.

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** — `SCHEMA_VERSION` const value matches `schema_version` in [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md); `SampleRecord` field set ⊆ documented v2 fields; new `edge_audit_history` columns match the schema doc; `coverage_summary` fields match the report doc. Two CI gates flipped `planned → active` (edge_audit_history-source-immutability + schema_v2 round-trip). Enforcement-map updated for R0 + R8 + new immutability sub-rule. Wipe-and-reindex documented as the migration path; no dual-read code.

## Out of scope for this sprint

- Multi-labeller aggregation policy — sprint 0006 owns (i).
- Reference labeller crates — sprint 0005 owns (b).
- Confidence re-stamping policy — sprint 0009 owns (k); writes nothing automatic to source-derived tables here.
- `producer_captured_args` field bump on schema → that is **Priority 2** (c) territory, not Priority 1.
- Eviction / retention of `edge_audit_history` rows.
