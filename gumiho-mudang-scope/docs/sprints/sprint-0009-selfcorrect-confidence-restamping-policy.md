# Sprint 0009 — Priority 1: confidence re-stamping policy from accumulated audit signal

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(k) Confidence re-stamping policy from accumulated audit signal**.
> **Phase**: A (single-sprint). Merges directly to `main`. **Ships last in Priority 1** per BACKLOG (k): "This policy is the riskiest piece in Priority 1 and ships last. Premature automation here can pollute the index with stamps that lag the actual extractor behaviour by audit-cycle epochs."
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Decide and implement the policy that converts accumulated `edge_audit_history` signal into Scope's confidence stamps — closing the actuator side of the self-correction loop. Sub-item (k) is the **policy decision that gates the actuator turning on**: the riskiest piece in Priority 1, so it ships last and only after every earlier sprint has demonstrated stable signal.

## Scope owned this sprint

- **Priority 1 (k)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprints 0001 + 0002 + 0003 + 0004 + 0005 + 0006 + 0007 all shipped.
- Sprint 0008 shipped **or** explicitly deferred (BACKLOG (f) is gated on corpus size; if not met, (k) may still open since it operates on `edge_audit_history` directly, not on suggester output — confirm in plan review).
- `edge_audit_history` table has accumulated enough audit cycles for the threshold-detection logic to have signal (sprint plan defines "enough").

## Charter alignment

- **Hard limits** — re-stamping is a soft-expansion act. The audit-trail invariant is non-negotiable (BACKLOG (k): "every automatic re-stamp is logged in a dedicated audit-trail file ... The audit trail is non-optional; without it, the loop becomes opaque").
- **Soft expansion zone** — `CHARTER.md` §6.
- **Invariants** — extractor-source-as-truth invariant is the one this sprint touches most carefully. Whichever policy is chosen, the audit-trail file makes every re-stamp traceable back to its triggering audit signal.

## Deliverables

### Priority 1 (k) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] Policy decision committed to a governing doc (likely `SELF-CORRECTION-CYCLE.md` from sprint 0001, or a new dedicated doc):
  - **Automatic downgrade** — N consecutive sub-target audits → next index run stamps `medium` instead of `high`. Closes the loop; introduces indirection between extractor source and emitted confidence.
  - **Flag-for-review** — same threshold → audit report surfaces "pattern_id X is sub-target; manual review recommended"; human edits extractor source. Extractor source stays canonical; slow.
  - **Hybrid** — automatic for tier-internal moves (`high → medium`); manual-only for cross-tier (`medium → low` or any upgrade).
- [ ] Threshold values defined (N consecutive audits, margin below tier target).
- [ ] **Audit-trail file** created and populated on every automatic re-stamp. Path + format defined (likely `gumiho-mudang-scope/audit-trail/restamps.jsonl` or analogous). Append-only.
- [ ] The audit trail records: triggering audit IDs, `(producer, pattern_id)` tuple, previous confidence, new confidence, timestamp.
- [ ] Indexer (R0 / R1) reads the audit-trail file at index time and applies re-stamps. Edge-emit code path documented.

### Priority 1 (k) implementation deliverables

- [ ] New module / sub-crate handling the policy (location decided in plan review — likely `scope-audit` if Priority 3 (a) has shipped; otherwise `scope-core::audit`).
- [ ] Indexer integration: edge-emit consults the audit-trail file; emitted confidence is the **lower of** the extractor's naive stamp and the audit-trail's recorded downgrade.
- [ ] CI gate ensuring audit-trail file is never deleted without a recorded rationale.
- [ ] [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) R0 / R8 entries updated: confidence-stamp closure now reads from two sources (extractor + audit trail).
- [ ] Doc update in `SELF-CORRECTION-CYCLE.md` (sprint 0001) — "Actuator" section gains the re-stamping flow.

---

## Ambiguities resolved before this sprint opens

- **Policy choice** (automatic / flag / hybrid) — halt and decide on `main` before opening. This is the single biggest ambiguity in Priority 1; § 3 ambiguity protocol applies.
- **Threshold values** — N consecutive audits, margin in percentage points. Decide on `main` first.
- **Upgrade vs. downgrade** — BACKLOG (k) flags `low → medium` upgrades as "dangerous". Hybrid policy explicitly forbids automatic upgrades; if any other policy is chosen, halt and decide upgrade behaviour on `main`.
- **Charter-amendment-grade?** — re-stamping introduces a second source of truth for confidence stamps. Confirm in plan review whether this requires a [`CHARTER.md`](../CHARTER.md) invariant amendment.

---

## CI gates activated in this sprint

- [ ] **audit-trail-append-only** — verifies the audit-trail file is never rewritten or shortened across commits. `planned → active`. New row in [`CI-GATES.md`](../CI-GATES.md).
- [ ] **restamp-traceability** — every confidence stamp downstream of the indexer matches either (a) the extractor's naive output or (b) a recorded audit-trail entry. `planned → active`. New row in [`CI-GATES.md`](../CI-GATES.md).

## Glossary terms touched

`audit-trail file`, `confidence re-stamping`, `actuator`, `tier-internal move`, `cross-tier move` — confirm / add in [`GLOSSARY.md`](../GLOSSARY.md) via its own channel before opening.

## Reporting

- **Branch**: `selfcorrect/sprint-0009-confidence-restamping-policy`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint). Focus must verify the audit-trail invariant and the extractor-source-as-truth boundary.

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** — audit-trail file path documented in `SELF-CORRECTION-CYCLE.md` matches the path the indexer reads; restamp record schema in code matches the schema in the doc; new policy choice (automatic/flag/hybrid) documented matches what the indexer implements. Two CI gates `active` (audit-trail-append-only + restamp-traceability). Enforcement-map R0 / R8 updated. Audit-trail file exists and is exercised by an integration test. Priority 1 actuator declared closed in the sprint PR body — every Priority 1 sub-item (a) through (k) is `shipped`.

## Out of scope for this sprint

- Automatic upgrades of confidence — explicitly out unless the chosen policy is the "automatic" variant and the human decision recorded so on `main`.
- Retroactive re-stamping of existing index rows — BACKLOG charter §2 wipe-and-reindex is the canonical migration path. The audit-trail file affects the **next** index run, not the current one.
- Eviction of `edge_audit_history` rows — deferred per BACKLOG (j).
- Priority 2 / Priority 3 work — independent surfaces.
