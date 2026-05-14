# Sprint 0005 — Priority 1: labeller workspace scaffolding + shared core

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(b₁) Workspace scaffolding + shared `scope-audit-labeller-core` crate** (after the (b) split landed on `main` in the sprint 0005 prep amendment — see [`SELF-CORRECTION-STATE.md` § Log](../SELF-CORRECTION-STATE.md#log)).
> **Phase**: A (single-sprint). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Stand up the sibling cargo workspace `gumiho-mudang-labeller/` at the repo root, excluded from the root Scope workspace, and ship the shared `scope-audit-labeller-core` crate that defines the `Labeller` trait + v2 JSONL read/write helpers consuming only [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md). A no-op reference impl `scope-audit-labeller-noop` proves the trait + IO loop end-to-end so the three concrete labellers (sprints 0010/0011/0012) inherit a stable contract without writing their own JSONL machinery.

This sprint **does not** ship LLM, LSP, or hybrid labellers. Those have their own sprint slots downstream.

## Scope owned this sprint

- **Priority 1 (b₁)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprint 0004 shipped — `schema_version: "2"` is the v2 contract this workspace consumes; row in [`SELF-CORRECTION-STATE.md` § Snapshot](../SELF-CORRECTION-STATE.md#snapshot) shows (g)/(h)/(j) all `shipped`.
- Sprint 0005 prep amendment landed on `main` — BACKLOG (b) split + physical-location decision. Log entry in [`SELF-CORRECTION-STATE.md` § Log](../SELF-CORRECTION-STATE.md#log) records the commit.

## Charter alignment

- **Hard limits** ([`CHARTER.md` § Hard limits](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)) — the **mechanical** enforcement of "Network calls during query" + "No toolchain" lives in this sprint. Cargo workspace exclusion is the build-system fact that prevents labeller deps from ever entering the Scope binary; the gate that proves it is added in this sprint.
- **Soft expansion zone** ([`CHARTER.md` § Soft expansion](../CHARTER.md#6-soft-expansion-zone--scope-expands-freely-here)) — labeller surface is not a Scope feature; the soft-expansion row this sprint touches is the self-correction-cycle surface, not a per-language one.
- **Per-language IN/OUT** ([`CHARTER.md` § Per-language scope](../CHARTER.md#7-per-language-scope-and-non-scope)) — none. Labellers are language-agnostic at this layer.
- **Invariants** ([`CHARTER.md` § Core invariants](../CHARTER.md#3-core-invariants--must-never-break)) — preserves invariant 1 (single-operator posture: in-repo sibling workspace, one history), invariant 6 (deterministic, read-only at query time: Scope binary's `Cargo.lock` does not gain a single labeller-side dep).

## Deliverables

### Priority 1 (b₁) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] `gumiho-mudang-labeller/` directory exists at repo root with its own `Cargo.toml` declaring `[workspace]`. Root `Cargo.toml` lists `gumiho-mudang-labeller` in `[workspace] exclude = [...]`. `cargo build` from the repo root never compiles labeller-workspace crates and never adds labeller deps to root `Cargo.lock`.
- [ ] `scope-audit-labeller-core` crate exists under `gumiho-mudang-labeller/scope-audit-labeller-core/`. Defines:
  - The `Labeller` trait (one method that takes an iterator/stream of v2 `SampleRecord` JSONL rows and yields v2 `SampleRecord` rows with the seven labeller-fillable fields populated, plus `labeller_id`).
  - JSONL read helper: parses one line at a time, validates `schema_version == "2"`, rejects on mismatch with the same diagnostic the Scope CLI emits in `label_pass`.
  - JSONL write helper: serialises one line at a time, preserves field ordering documented in [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md) § Record schema.
  - Re-exports / type definitions that mirror the v2 record shape **without** depending on any `gumiho-mudang-scope` / `gumiho-mudang-cli` crate. The contract is the schema doc; the types are duplicated on the labeller side.
- [ ] `scope-audit-labeller-noop` reference impl crate exists under `gumiho-mudang-labeller/scope-audit-labeller-noop/`. Implements `Labeller` by passing every record through unchanged with `labeller_id: "noop:reference-v0"` stamped, every other labeller-fillable field left `null`. Integration test runs the binary end-to-end against a fixture sample file and asserts a v2-conformant output stream.
- [ ] `gumiho-mudang-labeller/README.md` documents: the v2 JSONL contract pointer, the workspace-exclusion rationale, the trait shape, the noop impl, and the convention each concrete labeller (LLM/LSP/hybrid) inherits when sprint 0010/0011/0012 ships.
- [ ] **No path or workspace dependency edge from `gumiho-mudang-labeller/**` back to any Scope crate.** The labellers consume only [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md).

### Priority 1 (b₁) implementation deliverables

- [ ] Root `Cargo.toml` gains `[workspace] exclude = ["gumiho-mudang-labeller"]` (or extends the existing list).
- [ ] `gumiho-mudang-labeller/Cargo.toml` declares its own `[workspace] members = ["scope-audit-labeller-core", "scope-audit-labeller-noop"]` with own `resolver = "2"`, own `[workspace.package]`, own `[workspace.dependencies]`.
- [ ] `gumiho-mudang-labeller/.gitignore` excludes that workspace's `target/` separately so the two build dirs do not collide.
- [ ] New CI gate **labeller-workspace-isolation** — narrow-grep gate (modelled on `gate_charter.sh`) verifying:
  - Root `Cargo.toml` excludes `gumiho-mudang-labeller`.
  - No Scope crate's `Cargo.toml` declares a `path = "../gumiho-mudang-labeller/..."` dep.
  - No `gumiho-mudang-labeller/**/Cargo.toml` declares a `path = "../gumiho-mudang-scope..."` or `path = "../gumiho-mudang-cli..."` dep.
  Wired into `just gate-refactor` and recorded as `active` in [`CI-GATES.md`](../CI-GATES.md) in the same commit as the script.
- [ ] R-entry registered in [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) for the mechanical workspace-isolation rule (likely **R14**, allocate during sprint).
- [ ] Cross-link from [`SELF-CORRECTION-CYCLE.md`](../SELF-CORRECTION-CYCLE.md) — new "Labeller workspace" subsection naming the sibling workspace, the core crate, and the contract surface.
- [ ] Glossary entries verified or added: `scope-audit-labeller-core`, `Labeller trait`, `labeller workspace`. If missing, halt and add via the glossary's own commit channel before resuming (per [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc)).

---

## Ambiguities resolved before this sprint opens

- **Physical location of the "separate workspace"** — sibling dir `gumiho-mudang-labeller/` at repo root, separate `[workspace]`, excluded from root `Cargo.toml`. Landed on `main` in the sprint 0005 prep amendment commit (see [`SELF-CORRECTION-STATE.md` § Log](../SELF-CORRECTION-STATE.md#log) for SHA).
- **Sprint scope** — sub-item (b) split into (b₁) scaffolding (this sprint) + (b₂)/(b₃)/(b₄) concrete labellers (sprints 0010/0011/0012). Landed on `main` in the same amendment.
- **Crate naming** — `scope-audit-labeller-{core,llm,lsp,hybrid}` (keeps the BACKLOG-original names) + `scope-audit-labeller-noop` (reference impl added during this sprint plan). Confirmed in the same amendment.

---

## CI gates activated in this sprint

- [ ] **labeller-workspace-isolation** (`just gate-labeller-isolation`) — `planned` → `active`. Two narrow-grep checks: root-side exclude declared; no cross-workspace path deps in either direction.

## Glossary terms touched

`scope-audit-labeller-core`, `Labeller trait`, `labeller workspace`. Confirm in [`GLOSSARY.md`](../GLOSSARY.md); add via the glossary's commit channel before resuming if missing.

## Reporting

- **Branch**: `selfcorrect/sprint-0005-labeller-workspace-scaffolding`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint). Focus must verify: (i) zero Scope-crate dependency edges added in either direction; (ii) `cargo build` at repo root does not compile labeller crates; (iii) the v2 contract types on the labeller side are a faithful duplicate of [`AUDIT-LABEL-SCHEMA.md`](../AUDIT-LABEL-SCHEMA.md), not an import of Scope's types.

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** after extension; **labeller-workspace-isolation gate active**; [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) R14 entry committed in the same commit as the gate script; [`CI-GATES.md`](../CI-GATES.md) row flipped `planned` → `active` in the same commit. Scope workspace `Cargo.lock` unchanged by this sprint (no Scope dependency gained). `cargo build` from repo root demonstrably does not touch the labeller workspace.

## Out of scope for this sprint

- `scope-audit-labeller-llm` — sprint 0010 owns (b₂).
- `scope-audit-labeller-lsp` — sprint 0011 owns (b₃).
- `scope-audit-labeller-hybrid` — sprint 0012 owns (b₄).
- Multi-labeller aggregation policy — sprint 0006 owns (i).
- Per-PR continuous re-audit in CI — sprint 0007 owns (c).
- Any change to the v2 schema — that surface is frozen at sprint 0004's close until a future bump.
- ML-driven patch suggester — sprint 0008 owns (f).
