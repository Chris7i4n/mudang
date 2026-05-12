# Sprint 0007 — Phase D: Confidence audit subcommand

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R8](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand).
> **Phase**: D — second of two sprints. Phase D atomic close lands on `refactor/phase-d` integration branch in this sprint's close commit.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Land the `scope audit confidence` subcommand — a **precision** report per `(kind, tier, producer, pattern_id)`. R8 is the symptom-side safety net for the detection-class rules (A1–A3, B2 in the inventory) that the trait-shape audit ([sprint 0004, R12](./0004-phase-b-trait-closure-and-audits.md)) cannot catch when a determined plugin author uses correctly-named helpers or runtime-resolved spawns.

This is the second of two Phase D sprints. The sprint-close commit also flips the **Phase D** row in [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) to `shipped` and merges `refactor/phase-d` (carrying R10 from sprint 0006 + R8 from this sprint) to `main`.

## R-moves shipped this sprint

- **R8 — Confidence audit subcommand** ([§ R8](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand))

R10 was shipped in [sprint 0006](./0006-phase-d-typed-output-schema.md).

## Prerequisites

- Sprint 0006 `shipped`: this sprint cuts from the head of `refactor/phase-d` (post-sprint-0006-merge). R8's subcommand emits typed output, which means it depends on the R10 output-struct surface.
- Phase C `shipped`: R8 samples precision **per `producer`** and **per `pattern_id`**, and framework predicates contribute their own rows.
- Phase B `shipped`: R8's tier targets assume the post-R3 status column and the post-R0 `producer` / `pattern_id` columns.

## Charter alignment

- **Universal language boundaries** ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  - **A1, A2, A3, B2** (detectable per the inventory) — R8 is the symptom-side detection that catches what the trait-shape audit cannot ([`ARCHITECTURAL-REFACTOR.md` § Why detectable, not mechanical](../ARCHITECTURAL-REFACTOR.md#why-detectable-not-mechanical-for-trait-shape-rules)).
- **Honest framing** ([`ARCHITECTURAL-REFACTOR.md` § R8 → What R8 measures and what it does not](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand)): R8 measures **precision only**. Recall regressions are caught by integration-test snapshots and per-framework doc walkthroughs, not by this subcommand. The subcommand's help text and report header must state this verbatim.

## Deliverables

### R8 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand))

- [ ] `scope audit confidence` subcommand exists, runs against the reference fixture corpus, and produces a parseable precision report per `(kind, tier, producer, pattern_id)`.
- [ ] Tier targets enforced: `high ≥ 95%`, `medium ≥ 70%`, `low` has no minimum. Offenders are identifiable to specific plugins and patterns via `(producer, pattern_id)`.
- [ ] Help text and report header **both** state: *"precision report; recall is measured by integration-test snapshots, not this subcommand."*
- [ ] The reference fixture corpus is committed and version-controlled at `gumiho-mudang-scope/scope-core/tests/fixtures/reference/<language_slug>/` per the pre-Phase-D ambiguity #1 resolution.
- [ ] Sampling protocol per the pre-Phase-D ambiguity #2 resolution: default `N = 30` per `(kind, confidence)` cell, `--sample-size N` override, `--seed N` for reproducibility (default fixed compile-time constant).
- [ ] Manual labelling pipeline per the pre-Phase-D ambiguity #3 resolution: two-phase workflow `scope audit confidence --emit-sample <path>` → maintainer fills `label` slot → `scope audit confidence --label <path>`.
- [ ] Output format per the pre-Phase-D ambiguity #4 resolution: default `--format json` with top-level `schema_version: "1"`, plus `--format tsv` for shell pipelines.

---

## Ambiguities to clarify before code lands

All four pre-Phase-D ambiguities were resolved on `main` in commit `cdf24bb` ahead of sprint 0006 — see [`ARCHITECTURAL-REFACTOR.md` § R8 → Operational shape](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand) and the sprint 0006 doc's restated summary. Sprint 0007 inherits the resolutions; no new ambiguities open here.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **Confidence audit** (`just audit-confidence`) — `planned` → `active`. Per `CI-GATES.md`, this fails the build when precision is below the tier target.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`Confidence`, `status`, orthogonality, cleanest-signal filter](../GLOSSARY.md#confidence-and-status-orthogonal)
- [`Producer`, `pattern_id`](../GLOSSARY.md#refactor-types)
- [`scope audit confidence`, `scope audit coverage` (planned)](../GLOSSARY.md#subcommands)
- [Gate, Gate status](../GLOSSARY.md#ci-gates)

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and [`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0007-confidence-audit`, cut from `refactor/phase-d` after sprint 0006 merged.
- **Open**: flip R8 row in [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to `in-progress`. Append log entry noting branch name.
- **Codex review**: before the sprint-close commit, run the canonical command from [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint) with:
  - `--base refactor/phase-d`
  - `--title "sprint 0007 — R8"`
  - Prompt focus: R8 acceptance bullets, precision-only framing (recall caught elsewhere), tier targets enforcement (`high ≥ 95%`, `medium ≥ 70%`), help-text + report-header disclaimer parity, reference corpus structure, sampling-protocol reproducibility (seed), manual-label workflow, JSON `schema_version: "1"` shape, TSV column parity.

  Attach report to PR body; address blockers.
- **Close**: flip R8 to `shipped`. **In the same commit**, flip the **Phase D** row in the phase snapshot table to `shipped`.
- **Merge**: sprint 0007 close commit merges to `refactor/phase-d`; `refactor/phase-d` then merges to `main` (Phase D atomic close).

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. The Confidence audit CI gate is `active` in `CI-GATES.md` and CI.
3. `REFACTOR-STATUS.md` shows R8 `shipped`; Phase D `shipped` after `refactor/phase-d` merges to `main`.
4. `scope audit confidence --help` exists and prints the precision-only disclaimer.
5. The first run of `scope audit confidence` against the reference corpus produces a report that the human reviews; any offenders are either acknowledged (with confidence downgrade or pattern fix) or accepted with rationale recorded.

## Out of scope for this sprint

- `scope audit coverage` subcommand — explicitly post-refactor ([`POST-REFACTOR-PLAN.md` § Items deliberately deferred](../POST-REFACTOR-PLAN.md#items-deliberately-deferred-beyond-this-plan)).
- LLM-assisted labelling (`--labeller <executable>`) — deferred per ambiguity #3 resolution.
- Malformed-source harness — sprint 0008 (R6).
- Per-language depth feature work — post-refactor.
- Performance regression measurement — handled by the refactor-as-a-whole acceptance criterion enforced at Phase E close.
