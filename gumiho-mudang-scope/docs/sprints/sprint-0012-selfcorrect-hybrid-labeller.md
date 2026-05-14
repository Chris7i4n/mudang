# Sprint 0012 — Priority 1: hybrid labeller

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(b₄) `scope-audit-labeller-hybrid`** (per the sprint 0005 prep amendment).
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Ship `scope-audit-labeller-hybrid` — an orchestrating labeller that runs LLM-first, then surfaces diffs against the human reviewer's expected verdicts. Composes over `scope-audit-labeller-llm` once shipped (sprint 0010); can dev-spike against `scope-audit-labeller-noop` while (b₂) is in flight. Lives in the `gumiho-mudang-labeller/` sibling workspace.

## Scope owned this sprint

- **Priority 1 (b₄)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0005 shipped — `scope-audit-labeller-core` + `Labeller` trait + noop reference exist.
- Sprint 0010 shipped — `scope-audit-labeller-llm` exists if the hybrid intends to compose over real LLM verdicts at merge time. (Sprint can dev-spike against noop; final ship needs the LLM crate.)

## Charter alignment

- **Hard limits** — same boundary as the LLM + LSP labellers: orchestration logic lives in the sibling workspace.
- **Soft expansion zone** — self-correction-cycle surface.
- **Invariants** — preserves invariants 1 + 6 by construction.

## Deliverables

### Priority 1 (b₄) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] `scope-audit-labeller-hybrid` crate exists under `gumiho-mudang-labeller/scope-audit-labeller-hybrid/` and implements the `Labeller` trait.
- [ ] Flow: read v2 JSONL → invoke inner LLM labeller → surface each (record, LLM verdict) pair to a human-review surface → emit final v2 JSONL stamped with the **human's** verdict where the human committed, the **LLM's** verdict where the human deferred, abstention (`null`) where both abstain.
- [ ] Human-review surface — initial implementation is interactive CLI (one record at a time, accept LLM verdict or override). Future surfaces (TUI, editor integration) are out of scope here.
- [ ] `labeller_id` written by the hybrid is `hybrid:<recipe>` where recipe encodes the inner labeller + version (e.g. `hybrid:llm-first-human-review:anthropic-claude-sonnet-4-6`). The inner labeller's `labeller_id` is preserved in `evidence` so the audit history can distinguish "human agreed with LLM" from "human overrode LLM".
- [ ] Integration test against the noop labeller as inner — fixture record stream, scripted human input, asserts the final verdict matches the script.

### Priority 1 (b₄) implementation deliverables

- [ ] Cargo feature `with-llm` toggling the dependency on `scope-audit-labeller-llm`. When disabled, the hybrid uses any inner `Labeller` impl provided at construction time (covers dev-spiking against noop).
- [ ] Disagreement metric in the run summary (printed to stderr): N records, M human overrides, K LLM-trusted, L double-abstentions. Counts only; aggregation policy (which verdict wins under priority/quorum) is sprint 0006's domain (i).
- [ ] Cross-link from [`SELF-CORRECTION-CYCLE.md`](../SELF-CORRECTION-CYCLE.md) — extend the "Labeller workspace" subsection with the hybrid composer as the third concrete impl.

---

## Ambiguities resolved before this sprint opens

If the human-review interactive surface raises a contract question that the v2 schema cannot answer (e.g. how to record "human deferred" vs "human abstained"), halt; consult; amend [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) on `main` before resuming.

---

## CI gates activated in this sprint

None on the Scope workspace.

## Glossary terms touched

`hybrid labeller`, `LLM-first`, `human-reviews-diffs`. Confirm in [`GLOSSARY.md`](../GLOSSARY.md); add via the glossary's commit channel before resuming if missing.

## Reporting

- **Branch**: `selfcorrect/sprint-0012-hybrid-labeller`
- **Base**: `main`
- **Codex review**: canonical command. Focus must verify: (i) no Scope-crate dependency edge added; (ii) the hybrid's `labeller_id` recipe is round-trippable (a reader can recover which inner labeller produced each verdict from the stamped string); (iii) the inner labeller's `labeller_id` is preserved in `evidence` per acceptance.

## Definition of done

All Deliverables bullets checked. Sibling-workspace `cargo test --workspace` green. Scope workspace `Cargo.lock` unchanged. labeller-workspace-isolation gate (R14) still green. No new Scope-side R-entry.

## Out of scope for this sprint

- Multi-labeller aggregation across many labellers in a single run — sprint 0006 owns (i). The hybrid composes exactly **one** inner labeller; sprint 0006's surface composes many.
- Editor / TUI human-review surface — initial impl is CLI; richer surfaces are individual follow-up sprints.
- Any change to the v2 schema or the `Labeller` trait shape.
