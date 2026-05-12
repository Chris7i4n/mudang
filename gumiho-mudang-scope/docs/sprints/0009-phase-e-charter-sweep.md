# Sprint 0009 — Phase E: Charter sweep and shim retirement

> **Source of truth**: [`CHARTER.md` § 2 "Single-operator posture"](../CHARTER.md#2-who-scope-serves) + [§ 3 invariant 8](../CHARTER.md#3-core-invariants--must-never-break) + [`REFACTOR-STATUS.md` § Compat shims outstanding](../REFACTOR-STATUS.md#compat-shims-outstanding).
> **Phase**: E acceptance gate (no R-move). **Final sprint of the refactor.** Sprint 0008 (R6) merges first; this sprint then closes Phase E + the refactor as a whole.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Close the refactor by sweeping the codebase for every compatibility shim, transitional alias, version-detector, dual-read path, deprecated CLI surface, and `// legacy …` reader arm — and removing them. No new R-moves land. The deliverable is a clean tree that holds CHARTER § 2 "Single-operator posture" verbatim and CHARTER § 3 invariant 8 with no exceptions.

Phase E ships **last** so that every preceding phase has converged on its final shape. After this sprint closes, [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) is eligible.

## R-moves shipped this sprint

- **None.** This sprint is acceptance-only. R-moves are charter-policy reads against the closed codebase.

## Prerequisites

- Sprints 0001–0008 `shipped`.
- [`REFACTOR-STATUS.md` § Compat shims outstanding](../REFACTOR-STATUS.md#compat-shims-outstanding) populated.
- [`REFACTOR-STATUS.md` § Stubs outstanding](../REFACTOR-STATUS.md#stubs-outstanding) empty (every stub retired by its scheduled R-move).

## What lands this sprint

### Chunk 1 — Shim table closure

Iterate every row currently in [`REFACTOR-STATUS.md` § Compat shims outstanding](../REFACTOR-STATUS.md#compat-shims-outstanding). For each:

1. Open the cited file / line.
2. Confirm the shim is removable (i.e. the downstream R-move that retires it has shipped). If not, escalate via § 3 ambiguity protocol.
3. Delete the shim. Adjust call sites. Run gates.
4. Strike the row from the table; append a log entry in [`REFACTOR-STATUS.md` § Log](../REFACTOR-STATUS.md#log) with the closing commit.

The chunk closes when the table is empty.

### Chunk 2 — Full-codebase grep pass

After Chunk 1 strikes every known row, sweep for unrecorded shims that slipped past the audits. The greps below are the canonical pass — every hit not justifiable as charter-aligned (e.g. a "legacy" in a regex test fixture is fine; a "legacy" in production code is not) gets removed or escalated.

Canonical greps (run from repo root, exclude `target/`, `tests/integration/`, `.scope/`):

- `legacy` — transitional reader arms, fallback fields, "legacy" string literals
- `deprecated` — CLI aliases, deprecation warnings
- `backward` / `back-compat` / `compat` — explicit compat shims
- `pre-R[0-9]` / `pre-fix` — version-detector branches
- `_deprecated` / `_legacy` — column / field naming
- `version: u32` or `schema_version` fields in stored shapes
- `INSERT OR IGNORE` — silent dedup that hides shape drift (only valid where the dedup is the intended semantics, never as a shim)
- `if old_shape` / `or_else.*as_str` patterns suggesting dual-read
- `pub type X = Y` aliases — every type alias is reviewed against "does the rename serve clarity or hide a transition?"
- `pub use foo::Bar` re-exports — every re-export ditto

Each hit gets one of three dispositions, recorded in the chunk-2 commit message:

- **Charter-aligned** (e.g. test fixture that asserts a deprecation error message exists for an old user input). No action.
- **Shim — remove now.** Delete; run gates.
- **Shim — escalate.** Cannot be removed without a follow-up R-move. Block the sprint close; escalate via § 3 ambiguity protocol so the charter / refactor plan is amended before close.

### Chunk 3 — `just gate-charter` recipe

Wire the canonical greps into a `just gate-charter` recipe under `justfile`. Add to `just gate-refactor` aggregation.

Output format: zero hits ⇒ exit 0; any hit ⇒ exit 1 with the hits listed and a pointer to [`REFACTOR-STATUS.md` § Compat shims outstanding](../REFACTOR-STATUS.md#compat-shims-outstanding).

This gate is the mechanical successor to Chunk 2's manual pass — every subsequent commit on `main` runs it.

### Chunk 4 — Codex review + sprint close

Two codex passes fire here per `sprints/README.md § 9 — Role 1`:

1. **Sprint-scope review** — canonical command, `--base main`, `--title "sprint 0009 — charter sweep"`, gpt-5.5 medium. Prompt asks codex to grep for the canonical shim patterns INDEPENDENTLY of the `just gate-charter` recipe (codex is not allowed to read the recipe before the sweep). Any P0 / P1 blocks the close.
2. **Full-refactor-scope review** — canonical command with `-c model_reasoning_effort="high"` override (authorised in `README.md § 9 — Why these flags`), `--base <pre-refactor-baseline>` (the commit immediately preceding sprint 0000's first commit; recorded in `REFACTOR-STATUS.md` log), `--title "Refactor close"`. Prompt focus: whole-refactor acceptance set in `ARCHITECTURAL-REFACTOR.md § Acceptance for the refactor as a whole`. Override recorded in PR body.

Both reports attach to the PR body. When both return green, transition Phase E in [`REFACTOR-STATUS.md` § Phases](../REFACTOR-STATUS.md#phases) to `shipped` and append the closing log entry recording refactor close.

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and [`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-atomic-phase-shipment):

- **Branch**: `refactor/sprint-0009-charter-sweep`, cut from `main` after sprint 0008 merged.
- **Base**: `main` directly — acceptance-only sprint, no integration branch.
- **Open**: no R-move row flips (sprint is acceptance-only). Append log entry noting branch name.
- **Codex review**: two passes per Chunk 4 above.
- **Close**: flip the **Phase E** row in the phase snapshot table to `shipped` in the same commit as the closing chunk-4 transition. Append final log entry recording the refactor as a whole is `shipped`.
- **Merge**: squash-merge or rebase-merge to `main`. `POST-REFACTOR-PLAN.md` queue becomes eligible immediately; first post-refactor branch follows its own naming (not `refactor/…`).

## Acceptance

The refactor closes when **all** the following hold simultaneously, mirroring [`ARCHITECTURAL-REFACTOR.md` § Acceptance for the refactor as a whole](../ARCHITECTURAL-REFACTOR.md#acceptance-for-the-refactor-as-a-whole):

1. [`REFACTOR-STATUS.md` § Compat shims outstanding](../REFACTOR-STATUS.md#compat-shims-outstanding) is empty.
2. [`REFACTOR-STATUS.md` § Stubs outstanding](../REFACTOR-STATUS.md#stubs-outstanding) is empty.
3. `just gate-charter` runs clean in CI.
4. Both codex passes (sprint-scope + full-refactor-scope) surface no P0 / P1 findings.
5. `REFACTOR-STATUS.md` shows **every** R-move (R0–R12) and **every** phase (A–E) as `shipped`.
6. Every universal rule in the inventory tables ([`CHARTER.md` §5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these) hard limits and [`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)) is in class 1, class 2, or the **explicit class-3 list of three** (B1, C2, E3). No other rule is delegated to discipline.
7. Every active language plugin's `docs/languages/<name>.md` has **zero** `NEEDS REVIEW` entries.
8. Every active framework plugin's `docs/frameworks/<name>.md` — none adopted at refactor close — has, when adopted, an explicit decision in every row of the 15-category walkthrough ([`FRAMEWORK-PLAYBOOK.md` Step 4](../FRAMEWORK-PLAYBOOK.md#step-4--gotcha-catalogue)). Framework adoption is post-refactor work; this gate is forward-looking.
9. Full benchmark suite shows **< 10% regression** from pre-refactor baseline. Baseline = commit immediately preceding sprint 0001's first commit; post-refactor measurement taken on the commit that closes this sprint.
10. `scope audit confidence` runs against the reference fixture corpus and produces a parseable precision report per `(kind, tier, producer, pattern_id)`.
11. CI pipeline includes the malformed-source gate (R6, from sprint 0008), the typed-trait audit (R12), and the immutable-source check (R9).
12. Sprint acceptance from sprints 0001–0008 each remain demonstrated on `main` (no regression introduced by the sweep itself).
13. [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) becomes eligible the moment this sprint's closing commit lands on `main`. The first post-refactor sprint is **not** part of this document — planned separately against the closed architecture.

## Out of scope for this sprint

- Anything in [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md).
- Any new R-move. If chunk 2 surfaces a problem that needs a new R-move, the sprint blocks; the R-move is added to `ARCHITECTURAL-REFACTOR.md` and assigned to a re-opened earlier sprint (or a sprint 0008.5 inserted via § 3 ambiguity protocol).
- Performance work beyond the < 10 % regression check in § Acceptance #9. The benchmark is a measurement, not an optimisation target for this sprint.
- Documentation rewrites beyond struck-shim log entries and the `just gate-charter` recipe.
