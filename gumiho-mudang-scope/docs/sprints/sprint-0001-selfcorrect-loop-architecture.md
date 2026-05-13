# Sprint 0001 — Priority 1: self-correction loop architecture document

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(a) Loop architecture document**.
> **Phase**: A (single-sprint). Sprint is the phase. Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Formalise the closed self-correction pipeline in a new governing doc `docs/SELF-CORRECTION-CYCLE.md`. Names every contract surface (R8 sensor → labelled corpus → analyser → extractor patch suggestion → human review → next index run), the mandatory human gate, and the rollback path when an analyser-suggested patch regresses precision. Every downstream Priority-1 sprint links into this doc instead of restating the loop shape.

## Scope owned this sprint

- **Priority 1 (a)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Eligibility gate in [`BACKLOG.md` § Eligibility](../BACKLOG.md#eligibility) holds.
- R8 sensor shipped (signal source the loop reads).

## Charter alignment

- **Hard limits** — none crossed; doc-only.
- **Soft expansion zone** — `CHARTER.md` §6 self-correction surface formalisation.
- **Per-language IN/OUT** — none touched.
- **Invariants** — preserves auditor-immutability ([`CHARTER.md` § Core invariants](../CHARTER.md#3-core-invariants--must-never-break)); the new doc states the rule explicitly for the closed loop.

## Deliverables

### Priority 1 (a) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] New file `gumiho-mudang-scope/docs/SELF-CORRECTION-CYCLE.md` exists.
- [ ] Doc names every contract surface in the pipeline: R8 audit signal · labelled corpus · analyser (ML / LLM / heuristic) · extractor patch suggestion · human review · merge · next index run.
- [ ] Mandatory human review gate is documented as non-bypassable.
- [ ] Rollback path is documented for the case where an analyser-suggested patch regresses precision elsewhere.
- [ ] [`BACKLOG.md` Priority 1 (a)](../BACKLOG.md#priority-1--self-correction-cycle) is cross-linked from the new doc; the new doc is cross-linked from [`gumiho-mudang-scope/docs/README.md`](../README.md) doc index.

### Priority 1 (a) implementation deliverables

- [ ] Draft `SELF-CORRECTION-CYCLE.md` with sections: Purpose · Pipeline diagram · Contract surfaces · Human gate · Rollback · Out of scope.
- [ ] Cross-link from [`CHARTER.md` § 6](../CHARTER.md#6-soft-expansion-zone--scope-expands-freely-here) soft-expansion row covering the self-correction surface.
- [ ] Cross-link from [`gumiho-mudang-scope/docs/README.md` § Where to put a new note](../README.md#where-to-put-a-new-note).

---

## Ambiguities resolved before this sprint opens

None expected. If sprint surfaces a charter ambiguity (e.g. whether automatic re-stamping is permitted under invariants), halt per § 3 and amend [`CHARTER.md`](../CHARTER.md) on `main` first.

---

## CI gates activated in this sprint

- [ ] **doc-sync** (`just gate-doc-sync`) — `planned` → `active`. New narrow-grep gate modelled on `scripts/gate_charter.sh`. Checks: `SCHEMA_VERSION` const ↔ `schema_version` in [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md); `SampleRecord` / `ReportRow` field names ⊆ documented schema fields; every `### R<n>` in [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) references existing file paths; every `active` row in [`CI-GATES.md`](../CI-GATES.md) has its recipe in [`justfile`](../../../justfile); every markdown link `](*.md)` across `docs/` resolves; `HIGH_TIER_MIN` / `MEDIUM_TIER_MIN` const values match the percentages cited in the R8 doc text. New row in [`CI-GATES.md`](../CI-GATES.md). New R-entry in [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) (next free R-ID) registering the technique. Sprint ships the script + recipe + CI wiring in the same commit that flips the row to `active`.

## Glossary terms touched

None new. If the loop pipeline introduces a term ("analyser", "patch suggester", "audit cycle") not in [`GLOSSARY.md`](../GLOSSARY.md), halt and add via the glossary's own commit channel first.

## Reporting

- **Branch**: `selfcorrect/sprint-0001-loop-architecture`
- **Base**: `main`
- **Open**: append log entry in active state-tracking doc (or PR body if none) noting Priority 1 (a) `unstarted → in-progress`.
- **Codex review**: `codex review -c model="gpt-5.5" -c model_reasoning_effort="medium" --base main --title "sprint 0001 — selfcorrect loop architecture"`.
- **Close**: flip Priority 1 (a) row to `shipped` with commit SHA.
- **Merge**: squash-merge or rebase-merge (initiative-wide choice locked at sprint 0001 open).

## Definition of done

All bullets in **Deliverables** checked. **doc-sync gate green** on the closing commit (every doc the sprint touched is reflected in code paths, and vice versa). New R-entry registered in [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) for the doc-sync technique. Codex review report attached to PR body.

## Out of scope for this sprint

- Sprints 0002–0009 deliverables.
- Any code change to the audit pipeline.
- Charter or playbook amendments beyond the cross-link.
