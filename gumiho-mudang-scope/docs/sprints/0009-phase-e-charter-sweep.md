# Sprint 0009 — Phase E: Charter sweep and shim retirement

> **Source of truth**: [`CHARTER.md` § 2 "Single-operator posture"](../CHARTER.md#2-who-scope-serves) + [§ 3 invariant 8](../CHARTER.md#3-core-invariants--must-never-break) + [`REFACTOR-STATUS.md` § Compat shims outstanding](../REFACTOR-STATUS.md#compat-shims-outstanding).
> **Phase**: E. Single-sprint phase. **Final sprint of the refactor.**
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Close the refactor by sweeping the codebase for every compatibility shim, transitional alias, version-detector, dual-read path, deprecated CLI surface, and `// legacy …` reader arm — and removing them. No new R-moves land. The deliverable is a clean tree that holds CHARTER § 2 "Single-operator posture" verbatim and CHARTER § 3 invariant 8 with no exceptions.

Phase E ships **last** so that every preceding phase has converged on its final shape. After this sprint closes, [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) is eligible.

## R-moves shipped this sprint

- **None.** This sprint is acceptance-only. R-moves are charter-policy reads against the closed codebase.

## Prerequisites

- Sprints 0001–0007 `shipped`.
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

External review by `codex review --base main` (gpt-5.5, medium). The prompt asks codex to grep for the canonical shim patterns INDEPENDENTLY of the `just gate-charter` recipe (codex is not allowed to read the recipe before the sweep). Any P0 / P1 blocks the close.

When codex returns green, transition Phase E in [`REFACTOR-STATUS.md` § Phases](../REFACTOR-STATUS.md#phases) to `shipped` and append the closing log entry.

## Acceptance

The refactor closes when **all** the following hold:

1. [`REFACTOR-STATUS.md` § Compat shims outstanding](../REFACTOR-STATUS.md#compat-shims-outstanding) is empty.
2. `just gate-charter` runs clean in CI.
3. [`REFACTOR-STATUS.md` § Stubs outstanding](../REFACTOR-STATUS.md#stubs-outstanding) is empty.
4. Codex review against this branch surfaces no P0 / P1 charter violations.
5. The acceptance criteria from sprints 0001–0007 each remain demonstrated on `main` (no regression introduced by the sweep itself).
6. [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) becomes eligible the moment this sprint's closing commit lands on `main`.

## Out of scope for this sprint

- Anything in [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md).
- Any new R-move. If chunk 2 surfaces a problem that needs a new R-move, the sprint blocks; the R-move is added to `ARCHITECTURAL-REFACTOR.md` and assigned to a re-opened earlier sprint (or a sprint 0008.5 inserted via § 3 ambiguity protocol).
- Performance work. The benchmark gate from sprint 0008 already pins regression < 10 %; sprint 0008 must not regress further but does not optimise.
- Documentation rewrites beyond struck-shim log entries and the `just gate-charter` recipe.
