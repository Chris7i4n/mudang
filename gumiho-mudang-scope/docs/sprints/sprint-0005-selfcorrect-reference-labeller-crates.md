# Sprint 0005 — Priority 1: reference labeller crates (external workspace)

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(b) Reference labeller crates**.
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Ship three reference labeller crates **external to the Scope workspace** that implement the v2 JSONL contract from sprint 0004: `scope-audit-labeller-llm` (provider-agnostic LLM wrapper), `scope-audit-labeller-lsp` (per-language LSP cross-check via `tower-lsp` clients), `scope-audit-labeller-hybrid` (LLM-first, human-reviews-diffs). Living in a separate workspace keeps Scope's surface minimal — these consume only the published schema.

## Scope owned this sprint

- **Priority 1 (b)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0004 shipped — `schema_version: "2"` and `labeller_id` field landed on `main`.
- Sprint 0002 shipped — corpus accumulation policy in place.

## Charter alignment

- **Hard limits** — Scope's own crate inventory does not gain LLM or LSP dependencies. The labeller crates live in a sibling workspace ([`CHARTER.md` § Hard limits](../CHARTER.md#5-hard-limits--scope-will-never-cross-these) — Scope stays a single-binary, no-network analyser).
- **Soft expansion zone** — `CHARTER.md` §6 self-correction surface.
- **Invariants** — auditor-immutability preserved; labellers produce JSONL files, never write to `graph.db` directly.

## Deliverables

### Priority 1 (b) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] `scope-audit-labeller-llm` crate exists in a sibling workspace (separate `Cargo.toml`, separate `target/`), provider-agnostic LLM wrapper, consumes a v2 JSONL sample file and emits a v2 JSONL verdict file.
- [ ] `scope-audit-labeller-lsp` crate exists, performs per-language LSP cross-checks via `tower-lsp` clients for Rust (rust-analyzer) and TypeScript (tsserver) at minimum; other languages stubbed or surfaced as TODO.
- [ ] `scope-audit-labeller-hybrid` crate exists, LLM-first, human-reviews-diffs flow.
- [ ] Each crate's README documents: input JSONL contract (v2), output JSONL contract (v2 + populated verdict fields), how the crate is invoked, environment variables / config.
- [ ] None of the labeller crates declare a path or workspace dependency back on Scope crates; they consume only the **schema documented in [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md)**.

### Priority 1 (b) implementation deliverables

- [ ] Decide and document the sibling-workspace location (sibling git repo, or a `labellers/` directory at the repo root with its own `Cargo.toml` workspace — decide in plan review).
- [ ] Each labeller writes its own `labeller_id` into emitted JSONL records.
- [ ] Integration test per labeller against a sealed fixture (LLM mocked; LSP cross-checked against a vendored mini-fixture).
- [ ] Cross-link from `SELF-CORRECTION-CYCLE.md` (sprint 0001 output) — "Reference labeller crates" section.

---

## Ambiguities resolved before this sprint opens

- **Workspace location** — sibling repo vs. `labellers/` directory. If unclear at planning, halt; decide on `main` before opening the branch.
- **LSP language coverage** — at least Rust + TypeScript per "per-language LSP cross-check"; other languages may stub. If a labeller must cover all seven, halt and re-scope.

---

## CI gates activated in this sprint

None on the Scope workspace. The labeller workspace may carry its own CI but it is out-of-tree from this initiative's gate inventory.

## Glossary terms touched

`labeller_id`, `LSP cross-check`, `hybrid labeller` — confirm present in [`GLOSSARY.md`](../GLOSSARY.md); add via the glossary's commit channel if missing before opening the sprint.

## Reporting

- **Branch**: `selfcorrect/sprint-0005-reference-labeller-crates`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint). Focus must verify the workspace separation — no Scope-crate dependency edge added.

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** — `SELF-CORRECTION-CYCLE.md` cross-links the labeller workspace; [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) v2 contract is the only schema reference the labeller READMEs cite (no drift). Scope workspace `Cargo.lock` unchanged by this sprint (no Scope dependency gained). Enforcement-map: `n/a — no enforcement surface touched in Scope`.

## Out of scope for this sprint

- Multi-labeller aggregation policy — sprint 0006 owns (i).
- Per-PR continuous re-audit in CI — sprint 0007 owns (c).
- Any change to the v2 schema — that surface is frozen at sprint 0004's close until a future bump.
- ML-driven patch suggester — sprint 0008 owns (f).
