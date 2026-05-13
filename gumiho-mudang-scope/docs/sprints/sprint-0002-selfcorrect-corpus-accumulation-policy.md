# Sprint 0002 — Priority 1: labelled corpus accumulation policy

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(e) Labelled corpus accumulation policy**.
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Establish the on-disk policy for committed labelled JSONL corpora under `scope-core/tests/fixtures/reference/<lang>/audit-samples/`. Records sample provenance per commit (labeller used, date), retains old samples until the underlying fixture is removed, and treats stable precision over time as itself the signal. Defines the contract every later sprint (b, c, f, i, j, k) relies on when reading or writing labelled samples.

## Scope owned this sprint

- **Priority 1 (e)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0001 shipped — Priority 1 (a) row `shipped`; `SELF-CORRECTION-CYCLE.md` exists.

## Charter alignment

- **Hard limits** — none crossed.
- **Soft expansion zone** — `CHARTER.md` §6 (regression-asset accumulation).
- **Invariants** — preserves auditor-immutability and single-operator posture (committed corpus is the operator's regression baseline, not a multi-tenant store).

## Deliverables

### Priority 1 (e) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] Directory layout `scope-core/tests/fixtures/reference/<lang>/audit-samples/*.jsonl` is established and documented.
- [ ] Per-commit provenance is recorded for every sample file (labeller used, date) — either in-file (front-matter or sidecar `provenance.json`) or in a per-directory `MANIFEST.md`.
- [ ] Retention rule documented: samples kept until the underlying fixture is removed.
- [ ] Stable-precision-over-time is documented as the longitudinal signal.

### Priority 1 (e) implementation deliverables

- [ ] Create the directory tree under `scope-core/tests/fixtures/reference/<lang>/audit-samples/` for each currently supported language (Rust, Go, Python, TypeScript, Java, C#, Ruby). Empty directories may use `.gitkeep`.
- [ ] Add `gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md` section "Corpus accumulation policy" (or new dedicated doc, decision in plan review) documenting the rules.
- [ ] Add provenance record shape: minimum fields `labeller_id`, `labelled_at`, `scope_commit`, `sample_count`.
- [ ] Cross-link policy from `SELF-CORRECTION-CYCLE.md` (sprint 0001 output).

---

## Ambiguities resolved before this sprint opens

If the provenance record shape conflicts with the in-record `labeller_id` field that sprint 0004 adds to schema_version "2", halt and amend [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) on `main` first.

---

## CI gates activated in this sprint

None planned. A future gate validating provenance freshness against committed JSONL may be queued in [`CI-GATES.md`](../CI-GATES.md) as `planned` — not flipped active here.

## Glossary terms touched

`labelled corpus`, `provenance record` — confirm in [`GLOSSARY.md`](../GLOSSARY.md); add via glossary's own channel if missing.

## Reporting

- **Branch**: `selfcorrect/sprint-0002-corpus-accumulation-policy`
- **Base**: `main`
- **Open / Close**: per [`README.md` § 4](./README.md#4-reporting-hooks).
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint).

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** (every doc this sprint touched matches code; corpus-directory layout matches the policy doc). Doc-only sprint; enforcement-map: `n/a — no enforcement surface touched`. Codex review attached to PR.

## Out of scope for this sprint

- Populating the corpus directories with real samples — sprint 0007 onward consumes them.
- Eviction policy ("future sprint adds eviction" per BACKLOG (j)) — explicitly deferred.
- Schema bump v1 → v2 — owned by sprint 0004.
