# Sprint 0006 — Priority 1: multi-labeller verdict aggregation

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(i) Multi-labeller verdict aggregation**.
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Specify and implement the aggregation surface for realistic multi-labeller pipelines (LSP fast-path for `calls`, LLM for everything else, human reviewer for diffs) that produce conflicting verdicts on the same `edge_id`. Aggregation policy lives **in the runner** (labeller-crate ecosystem from sprint 0005), not in Scope — Scope's `--label` reads the **aggregated** JSONL output. No new flag on the `scope audit confidence` subcommand.

## Scope owned this sprint

- **Priority 1 (i)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0004 shipped — `labeller_id` field present in v2 records.
- Sprint 0005 shipped — at least two reference labellers exist (LSP + LLM) so aggregation has a real surface to consume.

## Charter alignment

- **Hard limits** — preserved. No new flag on `scope audit confidence`; aggregation policy lives outside Scope (single-binary posture intact).
- **Soft expansion zone** — `CHARTER.md` §6.
- **Invariants** — auditor-immutability preserved; aggregator emits a JSONL file, never writes to `graph.db`.

## Deliverables

### Priority 1 (i) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] Multi-source JSONL format documented in [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md): multiple labellers' outputs concatenated or merged, each record carrying `labeller_id`. Schema unchanged from sprint 0004 — this is a usage pattern, not a schema bump.
- [ ] Aggregation policy options documented and one default selected:
  - Priority order (e.g. human > LLM > LSP).
  - Quorum (n-of-m agree → use; disagreement → flag for review).
  - Per-labeller confidence weight (`labeller_id` → trust score applied to `confidence_proposed`).
  - Hybrid (LSP fast-path when available; fall back to LLM; defer to human on confirmed disagreement).
- [ ] Disagreement diagnostics surfaced: when labellers disagree on `kind_proposed` or `target_proposed`, the disagreement is itself signal — captured in the aggregated JSONL output and downstream in the precision report.
- [ ] Aggregator implementation lives in the labeller workspace (sprint 0005), not in Scope.
- [ ] **No new flag** on `scope audit confidence`; `--label aggregated.jsonl` flow unchanged.

### Priority 1 (i) implementation deliverables

- [ ] New aggregator binary `scope-audit-aggregator` (or library + tiny CLI) in the labeller workspace. Reads N input JSONL streams, writes one aggregated JSONL.
- [ ] Aggregation policy is configurable via runner-side config (TOML or CLI flags on the aggregator).
- [ ] Test fixture: three labellers (LSP/LLM/human) disagreeing on the same `edge_id`; assert correct aggregation output under each documented policy.
- [ ] Disagreement diagnostics: a counter or per-record flag visible in the precision report so an operator can see "N edges with cross-labeller disagreement this audit".

---

## Ambiguities resolved before this sprint opens

- **Default policy choice** — BACKLOG (i) lists options without naming a default. Halt and decide on `main` before opening (likely `hybrid` per the BACKLOG hybrid sketch).
- **Disagreement surface** — whether it lives in `coverage_summary` (sprint 0004 (h)) or a new top-level `aggregation_summary` field. If unclear, halt and amend [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) first.

---

## CI gates activated in this sprint

- [ ] Optionally: **aggregated-JSONL well-formedness** — `--label` accepts aggregator output, all records have non-empty `labeller_id`. `planned → active` if added.

## Glossary terms touched

`aggregator`, `quorum policy`, `disagreement diagnostics` — confirm in [`GLOSSARY.md`](../GLOSSARY.md); add via glossary's channel if missing.

## Reporting

- **Branch**: `selfcorrect/sprint-0006-multi-labeller-aggregation`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint).

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** — aggregation policy documented in [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) matches the aggregator's actual default; disagreement-diagnostic fields documented match the report shape. Scope's CLI surface unchanged (no new flag on `scope audit confidence`). Enforcement-map: refinement only if R8 entry's contract widens.

## Out of scope for this sprint

- Continuous re-audit in CI — sprint 0007 (c).
- Aggregator-driven trust-score learning — future work; this sprint accepts static config.
- Any modification to the v2 schema fields.
