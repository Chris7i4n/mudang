# Sprint 0008 — Priority 1: ML-driven extractor patch suggester

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(f) ML-driven extractor patch suggester**.
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Trigger gate**: BACKLOG (f) explicitly gates this sprint on corpus-size heuristic — "1000+ samples across ≥4 languages". Sprint **does not open** until the heuristic is met.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Build the long-horizon analyser that reads labelled failures, locates the offending pattern in the extractor source, and proposes a code patch (branch on an AST shape, downgrade the confidence stamp, add a guard). The human review gate stays mandatory — this sprint ships a **suggester**, not an applier.

## Scope owned this sprint

- **Priority 1 (f)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0004 shipped — v2 schema + `edge_audit_history` table; suggester reads from `target_proposed`, `kind_proposed`, `confidence_proposed`.
- Sprint 0005 shipped — labellers populating qualitative verdicts.
- Sprint 0007 shipped — continuous re-audit accumulates regression signal.
- **Trigger met**: ≥1000 labelled samples across ≥4 languages exist in the committed corpus (per BACKLOG (f)).

## Charter alignment

- **Hard limits** — preserved. Suggester lives in the labeller workspace, not in Scope (Scope stays a single-binary analyser without ML deps).
- **Soft expansion zone** — `CHARTER.md` §6.
- **Invariants** — human review gate is non-bypassable; suggester emits a proposal artefact, never opens a PR autonomously.

## Deliverables

### Priority 1 (f) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] Trigger gate confirmed: corpus size verified ≥1000 across ≥4 languages; verification command (e.g. `just corpus-stats`) shows the count in the sprint PR.
- [ ] Suggester reads labelled failures from `edge_audit_history` (via `scope audit history`) and the v2 JSONL corpus.
- [ ] Suggester locates the offending pattern in the extractor source (Tree-sitter `.scm` query, Rust extractor module, or generic-extractor pattern).
- [ ] Suggester proposes one of: branch-on-AST-shape patch, confidence-tier downgrade patch, guard insertion. Output is a unified diff or structured proposal artefact.
- [ ] Human review gate documented as non-bypassable in `SELF-CORRECTION-CYCLE.md` (sprint 0001 doc).
- [ ] Integration test: feed a known-failing pattern, assert the suggester produces a reasonable patch proposal.

### Priority 1 (f) implementation deliverables

- [ ] New crate `scope-audit-patch-suggester` (or analogous) in the labeller workspace.
- [ ] Model / heuristic choice documented (BACKLOG (f) does not name one — decide in plan review: small-LLM-prompted, classic ML, rule-based, hybrid).
- [ ] Output format: unified diff against extractor source; structured `proposal.json` with `extractor_path`, `pattern_id`, `proposed_change_kind`, `rationale`, `confidence_estimate`.
- [ ] Reviewer workflow documented: how a human picks up a proposal, validates against fresh fixtures, applies if accepted.

---

## Ambiguities resolved before this sprint opens

- **Trigger verification** — confirm BACKLOG (f) heuristic (1000+ samples × ≥4 langs) on `main`. If not met, sprint does not open; queue corpus growth instead.
- **Model architecture choice** — decide on `main` before opening: prompted LLM vs. classic ML vs. rule-based. Halt under § 3 if unclear.
- **Proposal artefact format** — unified diff vs. structured JSON vs. both. Decide on `main` first.

---

## CI gates activated in this sprint

None on Scope. The suggester workspace may carry its own CI but is out-of-tree from Scope's gate inventory.

## Glossary terms touched

`patch suggester`, `proposal artefact`, `pattern_id` — confirm / add in [`GLOSSARY.md`](../GLOSSARY.md) via its own channel.

## Reporting

- **Branch**: `selfcorrect/sprint-0008-ml-patch-suggester`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint). Focus must verify the human-gate non-bypass.

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** — `SELF-CORRECTION-CYCLE.md` "Analyser" section names the suggester crate and matches its output contract. No automatic PR opening; no automatic extractor source edits. Enforcement-map: `n/a — no enforcement surface touched in Scope`.

## Out of scope for this sprint

- Automatic patch application — explicitly out per BACKLOG (f) ("This is a suggester, not an applier").
- Confidence re-stamping policy — sprint 0009 (k).
- Any change to Scope's own crate inventory.
