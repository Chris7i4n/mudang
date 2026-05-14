# Sprint 0011 — Priority 1: LSP labeller

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(b₃) `scope-audit-labeller-lsp`** (per the sprint 0005 prep amendment).
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Ship `scope-audit-labeller-lsp` — a per-language LSP cross-check labeller using `tower-lsp` clients to drive real language servers (rust-analyzer, tsserver) over a v2 JSONL sample stream. For each record, query the language server for the ground-truth target (goto-definition, type-of, references) and compare against Scope's `to` / `kind` / `confidence`. Emit a v2 verdict populating `target_proposed`, `kind_proposed`, `evidence` with the LSP response details. Lives in the `gumiho-mudang-labeller/` sibling workspace; never touches the Scope binary.

## Scope owned this sprint

- **Priority 1 (b₃)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0005 shipped — `scope-audit-labeller-core` + `Labeller` trait + v2 JSONL helpers exist.
- Language servers available at test time (rust-analyzer, tsserver) — vendored or assumed installed in the runner.

## Charter alignment

- **Hard limits** — LSP machinery requires running a language server, which Scope itself is forbidden from doing ([`CHARTER.md` § Hard limits](../CHARTER.md#5-hard-limits--scope-will-never-cross-these): "No toolchain required" + "Invoking the language's compiler or interpreter"). The cross-check is **why the workspace is separate** — putting `tower-lsp` + spawn-language-server in the Scope binary would directly cross a hard limit. The sibling workspace boundary is what makes this labeller legal.
- **Soft expansion zone** — self-correction-cycle surface.
- **Invariants** — invariant 6 ("No network calls") still holds for Scope; LSP servers may run over stdio transport which is not a network call, but even if a server uses a TCP socket the boundary stays outside Scope.

## Deliverables

### Priority 1 (b₃) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] `scope-audit-labeller-lsp` crate exists under `gumiho-mudang-labeller/scope-audit-labeller-lsp/` and implements the `Labeller` trait.
- [ ] Per-language LSP client modules — Rust (rust-analyzer) and TypeScript (tsserver) at minimum. Other languages may stub with a `null` verdict + `labeller_id` reasoning recording the abstention.
- [ ] LSP transport via `tower-lsp` client — server spawn + initialize + `textDocument/definition` (or equivalent) per record, then cross-check against Scope's `to`.
- [ ] `labeller_id` written by this labeller is `lsp:<server>:<version>` (e.g. `lsp:rust-analyzer:2025.5.12`).
- [ ] `evidence` populated with `{"resolver": "<server>", "target_uri": "...", "definition_range": [...]}` matching the convention in [`AUDIT-LABEL-SCHEMA.md` § `evidence`](../AUDIT-LABEL-SCHEMA.md).
- [ ] Server lifecycle managed: one server process per language reused across records (not per-record spawn); clean shutdown on drop.
- [ ] Integration test against a mini-fixture vendored into the labeller workspace — small Rust + TS file pair, one record per language, asserts the labeller's `target_proposed` matches the expected goto-def result.

### Priority 1 (b₃) implementation deliverables

- [ ] Cargo features per language server (at minimum: `rust-analyzer`, `typescript`). Default features chosen to keep deps minimal.
- [ ] Server-availability probe at startup — if a required server is not on `PATH`, emit a diagnostic and yield abstentions for that language rather than crashing the stream.
- [ ] Per-record timeout — bounded latency budget; on timeout, abstain (`null` verdict).
- [ ] Cross-link from [`SELF-CORRECTION-CYCLE.md`](../SELF-CORRECTION-CYCLE.md) — extend the "Labeller workspace" subsection with the LSP labeller's role as the higher-fidelity cross-check.

---

## Ambiguities resolved before this sprint opens

If LSP server behaviour exposes a contract gap (e.g. `definition` returning multiple locations and the v2 schema has one `target_proposed` slot), halt; consult; amend [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) on `main` before resuming.

---

## CI gates activated in this sprint

None on the Scope workspace. Labeller-workspace CI (server-availability fixture test) lives in the sibling workspace's own configuration.

## Glossary terms touched

`LSP cross-check`, `tower-lsp client`, `server-availability probe`. Confirm in [`GLOSSARY.md`](../GLOSSARY.md); add via the glossary's commit channel before resuming if missing.

## Reporting

- **Branch**: `selfcorrect/sprint-0011-lsp-labeller`
- **Base**: `main`
- **Codex review**: canonical command. Focus must verify: (i) no Scope-crate dependency edge added; (ii) the language-server spawn happens entirely inside the labeller crate; (iii) the `evidence` shape matches the schema doc's convention.

## Definition of done

All Deliverables bullets checked. Sibling-workspace `cargo test --workspace` green when target language servers are available; abstentions when not. Scope workspace `Cargo.lock` unchanged. labeller-workspace-isolation gate (R14) still green. No new Scope-side R-entry.

## Out of scope for this sprint

- `scope-audit-labeller-llm` — sprint 0010 owns (b₂).
- `scope-audit-labeller-hybrid` — sprint 0012 owns (b₄).
- LSP coverage for languages beyond Rust + TS — those land via individual follow-up sprints when the trigger justifies.
- Any change to the v2 schema or the `Labeller` trait shape.
