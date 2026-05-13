# Sprint 0007 — Priority 1: continuous re-audit in CI

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(c) Continuous re-audit in CI**.
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Wire a per-PR run of `scope audit confidence --label committed-sample.jsonl` against the committed labelled corpus, with a precision diff printed in the PR body (e.g. `rust.calls.method: 96% → 94% (-2pp, 2 new failures)`). Catches extractor regressions before merge. Sample size capped for CPU budget; full audit still runs nightly.

## Scope owned this sprint

- **Priority 1 (c)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0002 shipped — corpus accumulation policy in place.
- Sprint 0004 shipped — coverage report fields exist so precision diff is honest.
- Sprint 0005 shipped — at least one labeller has produced a real committed sample file in the corpus (otherwise the diff has no baseline).
- Sprint 0006 shipped — aggregated JSONL is the canonical committed-sample shape.

## Charter alignment

- **Hard limits** — none crossed.
- **Soft expansion zone** — `CHARTER.md` §6.
- **Invariants** — preserves auditor-immutability; the CI run consumes the committed JSONL, never mutates the index.

## Deliverables

### Priority 1 (c) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] Per-PR CI job runs `scope audit confidence --label <committed-sample.jsonl>` against the committed corpus from sprint 0002.
- [ ] Sample size is capped for CPU budget (cap value chosen in plan review; default e.g. 500 records).
- [ ] Precision diff is computed against the previous run (baseline stored in CI cache, on `main`, or recomputed against the parent commit's `graph.db` — decide in plan review).
- [ ] Diff is printed in the PR body in the format `<lang>.<kind>.<tier>: <prev>% → <curr>% (<delta>, N new failures)`.
- [ ] Nightly full-audit job runs without the sample cap (against the full committed corpus).

### Priority 1 (c) implementation deliverables

- [ ] New `justfile` recipe `just audit-ci` invoking the capped per-PR audit and emitting the diff to stdout / a file the CI step posts.
- [ ] New `justfile` recipe `just audit-nightly` invoking the uncapped audit.
- [ ] GitHub Actions workflow (or whatever CI host is canonical — see [`CI-GATES.md`](../CI-GATES.md)) wiring both jobs.
- [ ] PR comment / body update step (bot or GitHub Actions native) posting the diff.
- [ ] Baseline-snapshot mechanism (per plan-review decision).

---

## Ambiguities resolved before this sprint opens

- **Sample cap** — pick a number in plan review. If unclear, halt and decide on `main` first.
- **Diff baseline storage** — CI artifact cache vs. committed snapshot vs. recompute. Halt and decide first.
- **Threshold for failing the PR** — does any precision drop fail the build, or only drops below the [`ENFORCEMENT-MAP.md` R8](../ENFORCEMENT-MAP.md) tier targets? Halt and amend R8 or [`CI-GATES.md`](../CI-GATES.md) on `main` if unclear.

---

## CI gates activated in this sprint

- [ ] **per-PR precision diff** — `planned → active`. Runs `just audit-ci` and posts diff. New row in [`CI-GATES.md`](../CI-GATES.md).
- [ ] **nightly full audit** — `planned → active`. Runs `just audit-nightly`. New row in [`CI-GATES.md`](../CI-GATES.md).
- [ ] **tier-target enforcement** — if the existing R8 gate (`check_tier_gate`, `HIGH_TIER_MIN`, `MEDIUM_TIER_MIN`) is already wired to fail builds, confirm it triggers in the new per-PR job too. Otherwise queue a follow-up. [`ENFORCEMENT-MAP.md` R8](../ENFORCEMENT-MAP.md).

## Glossary terms touched

`precision diff`, `nightly audit`, `sample cap` — confirm in [`GLOSSARY.md`](../GLOSSARY.md); add via glossary's channel if missing.

## Reporting

- **Branch**: `selfcorrect/sprint-0007-continuous-reaudit-ci`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint).

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** — every new `just` recipe (`audit-ci`, `audit-nightly`) is referenced in [`CI-GATES.md`](../CI-GATES.md) and exists in [`justfile`](../../../justfile); diff format documented matches the format the CI step prints. Two CI gates flipped `planned → active`. Enforcement-map R8 refinement if the gate's "Where in the tree" or "CI gates" lines shift.

## Out of scope for this sprint

- ML-driven patch suggester — sprint 0008 (f).
- Confidence re-stamping policy — sprint 0009 (k).
- Aggregator policy changes — frozen at sprint 0006 close.
