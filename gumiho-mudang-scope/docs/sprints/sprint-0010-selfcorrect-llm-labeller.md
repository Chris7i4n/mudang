# Sprint 0010 — Priority 1: LLM labeller

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(b₂) `scope-audit-labeller-llm`** (per the sprint 0005 prep amendment).
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Ship `scope-audit-labeller-llm` — a provider-agnostic LLM wrapper that consumes a v2 JSONL sample file and emits a v2 JSONL verdict file with `evidence`, `target_proposed`, `kind_proposed`, `confidence_proposed`, `reasoning_text` populated where the model commits to a verdict, `null` where it abstains. Lives in the `gumiho-mudang-labeller/` sibling workspace; never touches the Scope binary.

## Scope owned this sprint

- **Priority 1 (b₂)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0005 shipped — `scope-audit-labeller-core` crate + `Labeller` trait + v2 JSONL helpers exist; row in [`SELF-CORRECTION-STATE.md` § Snapshot](../SELF-CORRECTION-STATE.md#snapshot) shows (b₁) `shipped`.
- Sibling workspace `gumiho-mudang-labeller/` is the build context for this sprint; root `Cargo.lock` is not touched.

## Charter alignment

- **Hard limits** — sprint deliverables live exclusively in the sibling workspace. Mechanical enforcement is the workspace-isolation gate shipped in sprint 0005 (R14).
- **Soft expansion zone** — self-correction-cycle surface.
- **Invariants** — preserves invariants 1 + 6 by construction (no Scope binary change).

## Deliverables

### Priority 1 (b₂) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] `scope-audit-labeller-llm` crate exists under `gumiho-mudang-labeller/scope-audit-labeller-llm/` and implements the `Labeller` trait from `scope-audit-labeller-core`.
- [ ] Provider-agnostic — the LLM transport is a trait on the crate side; at least one provider impl ships (e.g. Anthropic Messages API). Provider implementations sit behind cargo features so users select what they want without dragging every SDK into the build.
- [ ] Prompt template documented in the crate's README. Inputs to the template: `kind`, `from`, `to`, `confidence`, `source_snippet`, optional `producer_captured_args` (future), `lang`, `lang_version`. Outputs: structured verdict mapped onto the v2 labeller-fillable fields.
- [ ] `labeller_id` written by this labeller is `llm:<provider>:<model-id>` (e.g. `llm:anthropic:claude-sonnet-4-6`).
- [ ] Retry + rate-limit handling at the transport layer — bounded attempts, exponential backoff, surfaces "abstained" (`null` verdict) on persistent failure rather than corrupting output.
- [ ] Integration test against a mocked provider transport: reads a fixture v2 JSONL, applies a canned model response per record, asserts a v2-conformant labelled output stream with the expected `labeller_id` stamp and the seven labeller-fillable fields populated as the mock prescribes.

### Priority 1 (b₂) implementation deliverables

- [ ] Cargo features per provider (at minimum: `anthropic`). Default features chosen to keep the dependency surface minimal for users who only want one provider.
- [ ] Diagnostic output (stderr) per record on transport error so the operator sees what failed and why; the JSONL output itself stays clean.
- [ ] Bench against the reference fixture corpus committed in sprint 0002 — order-of-magnitude budget for throughput (records / minute, model-dependent). Not a gate; numbers recorded in PR body.
- [ ] Cross-link from [`SELF-CORRECTION-CYCLE.md`](../SELF-CORRECTION-CYCLE.md) — extend the "Labeller workspace" subsection from sprint 0005 with the LLM labeller as the first concrete implementation.

---

## Ambiguities resolved before this sprint opens

If a provider transport raises a contractual question this sprint cannot resolve from existing docs (e.g. how to map streaming responses onto the line-by-line JSONL contract, or how to attribute per-record cost in the report), halt; consult; amend [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) or [`SELF-CORRECTION-CYCLE.md`](../SELF-CORRECTION-CYCLE.md) on `main` before resuming.

---

## CI gates activated in this sprint

None on the Scope workspace. The labeller workspace may carry its own CI (mock-provider integration test, no-network unit tests); listed in `gumiho-mudang-labeller/`'s own README, not in [`CI-GATES.md`](../CI-GATES.md) which inventories Scope-side gates.

## Glossary terms touched

`provider-agnostic LLM wrapper`, `prompt template`, `labeller_id` (already in glossary; extended usage). Confirm in [`GLOSSARY.md`](../GLOSSARY.md); add via the glossary's commit channel before resuming if missing.

## Reporting

- **Branch**: `selfcorrect/sprint-0010-llm-labeller`
- **Base**: `main`
- **Codex review**: canonical command. Focus must verify: (i) no Scope-crate dependency edge added; (ii) prompt template emits the labeller-fillable fields the v2 schema names, not a labeller-invented shape; (iii) provider-feature gating actually keeps unselected providers out of the build.

## Definition of done

All Deliverables bullets checked. Sibling-workspace `cargo test --workspace` green. Scope workspace `Cargo.lock` unchanged. labeller-workspace-isolation gate (R14, shipped in sprint 0005) still green. No new Scope-side R-entry — sprint touches no Scope enforcement surface.

## Out of scope for this sprint

- `scope-audit-labeller-lsp` — sprint 0011 owns (b₃).
- `scope-audit-labeller-hybrid` — sprint 0012 owns (b₄).
- Multi-labeller aggregation — sprint 0006 owns (i).
- Continuous re-audit in CI — sprint 0007 owns (c).
- Any change to the v2 schema or the `Labeller` trait shape — schema is frozen; trait is frozen at sprint 0005 close.
