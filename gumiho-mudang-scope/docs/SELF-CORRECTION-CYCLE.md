# Self-correction cycle

The closed loop that converts R8 audit signal into automated extractor improvement. Names every contract surface, the mandatory human review gate, and the rollback path when an analyser-suggested patch regresses precision elsewhere.

Source of truth for **the loop shape**. Every Priority 1 sprint (0001–0009, see [`BACKLOG.md` § Priority 1 — Self-correction cycle](./BACKLOG.md#priority-1--self-correction-cycle)) links into this doc instead of restating the pipeline.

State-tracking lives in [`SELF-CORRECTION-STATE.md`](./SELF-CORRECTION-STATE.md). Doc-↔-code drift is mechanically prevented by the **doc-sync gate** (see [Extending the doc-sync gate](#extending-the-doc-sync-gate) below).

---

## Purpose

R8 ships the **sensor**: `scope audit confidence` measures per-`(producer, pattern_id)` precision against a labelled fixture corpus and fails the build when any tier falls below target ([`ENFORCEMENT-MAP.md` § R8](./ENFORCEMENT-MAP.md#r8--confidence-audit-subcommand)). R8 alone does not close the loop: when a tier drops, a human still has to read the labelled samples, find the failing pattern, and patch the extractor by hand.

This doc names the **actuator** — the surfaces, transitions, and gates that turn R8 signal into extractor improvement without losing the auditor-immutability invariant ([`CHARTER.md` § 3 Core invariants](./CHARTER.md#3-core-invariants--must-never-break)) or the single-operator posture ([`CHARTER.md` § Single-operator posture](./CHARTER.md#single-operator-posture)).

---

## Pipeline

```
                    ┌────────────────────────────┐
                    │  source code                │
                    │  (operator working tree)    │
                    └──────────┬─────────────────┘
                               │
                               │ scope index
                               ▼
                    ┌────────────────────────────┐
                    │  graph.db                   │
                    │  (edges, symbols, …;       │
                    │   source-derived; immutable │
                    │   during audit)             │
                    └──────────┬─────────────────┘
                               │
                               │ scope audit confidence
                               │   --emit-sample
                               ▼
                    ┌────────────────────────────┐
                    │  sample.jsonl (v2)          │
                    │  schema_version: "2"        │
                    │  one row per audited edge   │
                    └──────────┬─────────────────┘
                               │
                               │ labeller(s) — LLM / LSP /
                               │   hybrid / human
                               │   (sprint 0005 (b), sprint 0006 (i))
                               ▼
                    ┌────────────────────────────┐
                    │  labelled.jsonl (v2,        │
                    │  optionally aggregated)     │
                    │  carries: label,            │
                    │  target_proposed,           │
                    │  kind_proposed,             │
                    │  confidence_proposed,       │
                    │  evidence, reasoning_text,  │
                    │  labeller_id                │
                    └──────────┬─────────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            │                  │                  │
            ▼                  ▼                  ▼
   scope audit         edge_audit_history   precision report
   confidence            (audit-derived       (--format json/tsv;
   --label                writable;             coverage_summary;
   (precision check)      sprint 0004 (j))      sprint 0004 (h))
                               │
                               │ scope audit history
                               ▼
                    ┌────────────────────────────┐
                    │  patch suggester            │
                    │  (sprint 0008 (f);          │
                    │  reads history; proposes    │
                    │  extractor diff)            │
                    └──────────┬─────────────────┘
                               │
                               │ proposal artefact
                               ▼
                    ┌────────────────────────────┐
                    │  HUMAN REVIEW GATE          │ ◀── non-bypassable
                    │  (mandatory; not optional)  │
                    └──────────┬─────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
              ▼                ▼                ▼
        accept patch      reject patch    restamp policy
        (edit extractor   (record in PR    (sprint 0009 (k);
        source; commit)    body; tracked    audit-trail file;
                           in BACKLOG)      indexer reads at
                                            next index run)
                               │
                               ▼
                    ┌────────────────────────────┐
                    │  next `scope index` run     │
                    │  emits corrected edges      │
                    │  (existing rows: wipe-and-  │
                    │  reindex per CHARTER §2)    │
                    └─────────────────────────────┘
```

---

## Contract surfaces

Each surface has a single owner sprint. Other sprints reference it; never duplicate.

| Surface | Owner | Shape |
|---|---|---|
| Sensor (`scope audit confidence`) | R8 (shipped) | Per-tier precision check; fails build on tier-target miss |
| Sample JSONL emitter | R8 (shipped) | One row per audited edge; `schema_version: "2"` from sprint 0004 |
| Sample schema | sprint 0004 (g) | `SampleRecord` struct ↔ [`AUDIT-LABEL-SCHEMA.md`](./AUDIT-LABEL-SCHEMA.md); gated by doc-sync |
| Labelled corpus on disk | sprint 0002 (e) | `scope-core/tests/fixtures/reference/<lang>/audit-samples/*.jsonl` + provenance |
| Labellers | sprint 0005 (b) | External workspace; LLM / LSP / hybrid crates; consume v2 schema |
| Aggregator | sprint 0006 (i) | Runner-side; merges multi-labeller verdicts; emits single aggregated JSONL |
| Coverage report | sprint 0004 (h) | `coverage_summary` top-level + per-row `skipped_count` / `coverage_ratio` |
| Audit-history table | sprint 0004 (j) | `edge_audit_history` — append-only; sibling auditor-immutability rule |
| `scope audit history` | sprint 0004 (j) | Read-side surface: per-edge timeline, per-`pattern_id` trend, per-labeller agreement |
| Continuous re-audit in CI | sprint 0007 (c) | Per-PR precision diff + nightly full audit |
| Patch suggester | sprint 0008 (f) | Reads `edge_audit_history`; proposes extractor diff; **never** opens PR autonomously |
| Restamp policy | sprint 0009 (k) | Audit-trail file → indexer reads at next index run |
| Doc-sync gate | sprint 0001 (this doc) | `scripts/gate_doc_sync.sh` — narrow-grep gate against doc-↔-code drift |

---

## Mandatory human review gate

**Non-bypassable.** Every analyser-suggested patch reaches the operator as a proposal artefact, not as an applied change. The operator:

1. Reviews the suggested diff against the extractor source.
2. Validates against fresh fixtures the suggester did not see.
3. Either applies the patch (a regular `feat`/`fix` commit on `main`), records the rejection (PR body + [`BACKLOG.md`](./BACKLOG.md) entry if the failure mode is worth tracking), or escalates to a [`sprints/README.md` § 3 ambiguity protocol](./sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc) consultation if the proposal exposes a rule gap.

The suggester **never**:

- Opens a PR autonomously.
- Edits extractor source files directly.
- Mutates `graph.db` source-derived rows (`edges`, `symbols`, `file_hashes`).

The suggester **may**:

- Append to the audit-trail file (sprint 0009 (k)) when the policy is the automatic-downgrade variant — and only within the bounds that policy commits permit.
- Write proposal artefacts to a scratch location for human pickup.

---

## Rollback path

When an analyser-suggested patch is merged and then proves to regress precision elsewhere:

1. The continuous re-audit gate (sprint 0007 (c)) catches the regression on the next PR / nightly run.
2. The audit-trail file (sprint 0009 (k)) records the original triggering signal and the merged commit SHA.
3. Rollback options, in order of cheapness:
   - **Revert the extractor commit** — standard `git revert`; next `scope index` run re-emits the previous edges. Wipe-and-reindex if persisted rows are now wrong.
   - **Downgrade the affected `pattern_id`** via the audit-trail file (sprint 0009 (k) hybrid policy variant) — leaves the patch in place but lowers the confidence stamp until further signal arrives.
   - **Escalate as `pattern_id` quarantine** — temporarily stop emitting the edge until a fresh patch lands. Recorded in the audit-trail file with explicit rationale; never silent.
4. Rollback decision is recorded in the PR body of the reverting / patching commit. The rollback path itself is never silent — every step leaves a trace in the audit-trail file or in `BACKLOG.md`.

---

## Auditor-immutability invariant (extended for the loop)

The closed loop introduces **two distinct namespaces** in `graph.db`:

- **Source-derived** — `edges`, `symbols`, `file_hashes`, every other table populated by `scope index`. **Immutable during audit.** Wipe-and-reindex per [`CHARTER.md` § 2](./CHARTER.md#2-single-operator-posture) is the only migration path.
- **Audit-derived** — `edge_audit_history` (sprint 0004 (j)) plus any future tables sprint 0009 (k) adds. **Writable by `scope audit confidence --label`.** Never mutates source-derived rows.

Two namespaces, two distinct mechanical enforcement gates. The CI gate added in sprint 0004 verifies `--label` writes touch only `edge_audit_history`. The source-derived auditor-immutability rule ([`AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule](./AUDIT-LABEL-SCHEMA.md#auditor-immutability-rule)) gains the writable-namespace paragraph in sprint 0004's commit.

The indexer (sprint 0009 (k)) reads the audit-trail file at index time to apply confidence re-stamps. The extractor source stays canonical for **what the stamp means**; the audit-trail file captures **the cycle-driven correction** on top.

---

## Extending the doc-sync gate

The doc-sync gate (`scripts/gate_doc_sync.sh`, recipe `just gate-doc-sync`, [`ENFORCEMENT-MAP.md` § R13](./ENFORCEMENT-MAP.md)) is the **mechanical** half of preventing doc-↔-code drift across the self-correction loop. Every later sprint (0002–0009) ships code and docs that move in lockstep; the gate enforces it.

The gate is modelled on [`scripts/gate_charter.sh`](../../scripts/gate_charter.sh): a single shell script with **named, narrow check functions**, each targeting one specific drift shape. Adding a new check is cheap.

### How to add a check (per-sprint recipe)

When a later sprint introduces a new code-↔-doc pair that must stay in sync, the sprint's implementation commit edits `scripts/gate_doc_sync.sh` to add **one** new check function. Pattern:

```bash
# Check N — <one-line drift shape this catches>
#
# Rationale: <which doc, which code surface, why drift would matter>.
check_<short_name>() {
    local doc_value code_value
    doc_value=$(grep -oE '<doc pattern>' "$DOC_PATH" | head -1)
    code_value=$(grep -oE '<code pattern>' "$CODE_PATH" | head -1)
    if [[ "$doc_value" != "$code_value" ]]; then
        fail_block "<short_name>" \
                   "doc says '$doc_value'; code says '$code_value'" \
                   "$DOC_PATH ↔ $CODE_PATH"
    fi
}
```

Then invoke it from `main()` alongside the other checks. The check stays narrow: it asserts **one** drift shape, never a loose substring scan that could fire on charter-aligned prose.

### Sprint-by-sprint additions expected

| Sprint | Sub-item | New check(s) the sprint commit adds |
|---|---|---|
| 0002 | (e) | Corpus directory layout exists for every supported `LanguageId` arm (no extras, no missing) |
| 0003 | (d) | Per-language detector module presence matches the list in [`CHARTER.md` § 7](./CHARTER.md#7-per-language-scope-and-non-scope) |
| 0004 | (g) | `SCHEMA_VERSION` const value ↔ `schema_version` in [`AUDIT-LABEL-SCHEMA.md`](./AUDIT-LABEL-SCHEMA.md); `SampleRecord` field set ⊆ documented v2 fields |
| 0004 | (h) | `coverage_summary` struct field set ↔ documented coverage fields |
| 0004 | (j) | `edge_audit_history` SQL columns ↔ documented columns |
| 0006 | (i) | Documented default aggregation policy ↔ aggregator's hard-coded default |
| 0007 | (c) | Every `audit-ci` / `audit-nightly` recipe in [`justfile`](../../justfile) referenced in [`CI-GATES.md`](./CI-GATES.md) |
| 0009 | (k) | Audit-trail file path documented ↔ path the indexer reads |

Each addition is **one commit on the owning sprint's branch**, in the same commit that ships the code-↔-doc pair. The gate flip from `planned → active` for the new check happens in the same commit per [`sprints/README.md` § 7](./sprints/README.md#7-ci-gate-activation).

### When the gate is the wrong tool

The gate catches **drift between named code surfaces and named doc passages**. It does not:

- Catch semantic drift ("the doc says X is fast; the code is slow"). That is acceptance-test territory.
- Catch missing documentation for a code surface. That is reviewer / `ENFORCEMENT-MAP.md` § 7.5 territory.
- Catch out-of-band rule amendments. That is the [`sprints/README.md` § 3](./sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc) protocol.

If a sprint's drift shape does not fit "one named code value ↔ one named doc value" or "one named code surface ↔ one named directory layout", the gate is the wrong place. Surface it in the sprint plan and escalate per § 3 before opening the branch.

---

## See also

- [`BACKLOG.md` § Priority 1 — Self-correction cycle](./BACKLOG.md#priority-1--self-correction-cycle) — sub-item (a) through (k) catalogue.
- [`SELF-CORRECTION-STATE.md`](./SELF-CORRECTION-STATE.md) — sprint state tracking.
- [`AUDIT-LABEL-SCHEMA.md`](./AUDIT-LABEL-SCHEMA.md) — JSONL contract surface (current `schema_version: "1"`; bumped to `"2"` in sprint 0004).
- [`ENFORCEMENT-MAP.md` § R8](./ENFORCEMENT-MAP.md) — confidence-audit sensor; [`§ R13`](./ENFORCEMENT-MAP.md) — doc-sync gate.
- [`CI-GATES.md`](./CI-GATES.md) — gate inventory including doc-sync.
- [`CHARTER.md` § 6 Soft expansion zone](./CHARTER.md#6-soft-expansion-zone--scope-expands-freely-here) — the surface this initiative expands.
