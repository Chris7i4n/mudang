# Sprint 0006 — Phase D: Typed output schema

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R10](../ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema).
> **Phase**: D — first of two sprints. Phase D atomic close lands on `refactor/phase-d` integration branch after sprint 0007 ships R8.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Lock output schemas as typed structs with no diagnostic fields, mechanically enforcing **E1** (no semantic correctness assertions) at the output boundary. R8 (confidence audit) ships in the **next** sprint (0007) on the same `refactor/phase-d` integration branch; Phase D rows in `REFACTOR-STATUS.md` flip to `shipped` only when sprint 0007 closes and `refactor/phase-d` merges to `main`.

## Phase D split (mid-Phase scope decision — 2026-05-12)

Phase D was originally scoped as a single atomic sprint shipping R10 + R8. Mid-sprint discovery, recorded in [`REFACTOR-STATUS.md` log → sprint 0006 mid-sprint re-scope](../REFACTOR-STATUS.md), revealed:

- R10 under the strict reading (see "R10 scope decision" below) touches **all 14 CLI commands** plus a 1477-LOC `formatter.rs`. Realistic estimate: ~3-5 days work; 28 `insta` snapshot tests re-anchor.
- R8 ships a subcommand, the reference fixture corpus skeleton, sampling + manual-label pipeline, JSON/TSV report writer, and a new CI gate (`scope audit confidence`).

Combined scope did not fit one sprint without forcing partial deliverables. Per [`README.md` § 1 — atomic phase shipment is preserved at the `main`-branch level via phase-integration branches](./README.md#1-linear-order-no-parallel-sprints--atomic-phase-shipment-to-main), Phase D follows the Phase B precedent: two sprints (0006 + 0007) onto `refactor/phase-d`, then a single phase-close merge to `main`.

## R-moves shipped this sprint

- **R10 — Typed output schema** ([§ R10](../ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema))

R8 lives in [sprint 0007](./0007-phase-d-confidence-audit.md).

## Prerequisites

- Phase C `shipped`: not strictly required for R10 alone, but Phase D opens after Phase C closes per linear-order rule (§1).
- Phase A `shipped`: R10's typed-struct conversion replaces the legacy string-concatenation formatters; R0's schema columns are the input.

## Charter alignment

- **Hard limits** ([`CHARTER.md` §5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)): "No type / borrow / lint diagnostics" — R10 is the mechanical closure via output-struct shape ("no field named `error`, `warning`, `diagnostic`, `is_valid`, etc.").
- **Universal language boundaries** ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  - **E1** (no semantic correctness assertions) — mechanical after R10.

## Deliverables

### R10 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema))

- [x] Output schemas are typed structs (`SymbolSketch`, `EdgeSummary`, `CompactView`, and any others currently driven by raw string concatenation). Formatters serialize structs; they do not concatenate strings. **Shipped chunks 1, a-e (typed `--json` paths) + g.A-g.E (typed plain-text via `impl Display`).** `EdgeSummary` dropped chunk c as YAGNI — re-introduce when a downstream consumer needs edge-uniform handling; the three concrete edge types already `derive(Serialize)` and pass through `JsonOutput` directly.
- [x] Output-schema audit (`scripts/audit_output_schema.sh`) catches fields named `error`, `warning`, `diagnostic`, `is_valid`, `lint`, `correctness`. **Shipped pre-implementation in commit `990ea1a` so the gate guarded every subsequent struct conversion; `ci-output-schema` is the 13th `gate-refactor` gate.**
- [x] Existing output formats (`sketch`, `summary`, `compact`, `json`) preserve their token budgets — the typed shape does not balloon output. **Verified: zero `.snap.new` files after the full chunk-g Display conversion (13 `.snap` files match byte-for-byte; the pre-implementation estimate of 28 snapshots was high).**

#### R10 scope decision (locked 2026-05-12)

The R10 target-state phrase *"Formatters serialize structs; they do not concatenate strings"* is read **strictly** in sprint 0006:

- Every output renderer in `gumiho-mudang-cli` — `sketch`, `summary`, `compact`, `json` envelope, plus the per-command surfaces (`refs`, `deps`, `impact`, `trace`, `flow`, `entrypoints`, `status`, `workspace`, `map`, `find`, `similar`, `source`, `diff`) — converts to a `#[derive(Serialize)]` struct or enum at the output boundary.
- Procedural `println!` / `format!` collapses into `impl Display` on the typed output struct.
- `serde_json::json!()` ad-hoc tree construction is removed from the codebase. Every JSON-emitting path serializes a concrete typed value.

Rationale: maximises the "make illegal states unrepresentable" Rust idiom (sum types over render variants prevent constructing a `ClassSketch`-shaped output for a method-shaped symbol); enables compile-time-optimized marshaling (`#[derive(Serialize)]` streams directly to the writer with no `serde_json::Value` intermediate); opens the door to schema-export for LSP composition (recorded in [`gumiho-mudang-lsp/docs/SCOPE_OUTPUT_INTEROP.md`](../../../gumiho-mudang-lsp/docs/SCOPE_OUTPUT_INTEROP.md) and `SCOPE-LSP-COMPOSITION.md` § 5.4).

Costs accepted: ~3-5 days work; all 28 `insta` snapshot tests re-anchor; ~50 KB binary size from serde-derive monomorphization.

The strict reading is locked in [`ARCHITECTURAL-REFACTOR.md` § R10 → Sprint 0006 scope decision](../ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema) for codex review and downstream sprint references.

---

## Ambiguities to clarify before code lands

None. All four pre-Phase-D ambiguities were specific to R8 and live in [sprint 0007](./0007-phase-d-confidence-audit.md). R10's scope decision is the only Phase-D-side ambiguity that affects this sprint, and it is already locked above + in `ARCHITECTURAL-REFACTOR.md § R10`.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [x] **Output schema audit** (`just ci-output-schema`) — `planned` → `active` (landed 2026-05-12 in this sprint's first chunk, before any struct conversion, so the gate guards every subsequent code change in the sprint).

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [Gate, Gate status](../GLOSSARY.md#ci-gates)

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and [`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0006-typed-output-schema`, cut from `main` after Phase C merged. Merges to `refactor/phase-d` integration branch.
- **Integration branch**: `refactor/phase-d` — cut from `main` post-Phase-C-merge; carries sprints 0006 + 0007; the **completed phase** merges to `main`.
- **Open**: flip R10 row in [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to `in-progress`. Append log entry noting branch name. (R8 stays `unstarted` — moves to sprint 0007.)
- **Codex review**: before the sprint-close commit, run the canonical command from [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint) with:
  - `--base main`
  - `--title "sprint 0006 — R10"`
  - Prompt focus: R10 acceptance bullets, E1 mechanical enforcement (no diagnostic-shaped output fields), strict-reading scope decision (all 14 commands converted), `#[derive(Serialize)]` displacement of `serde_json::json!()` ad-hoc trees, snapshot byte-budget parity.

  Attach report to PR body; address blockers.
- **Close**: flip R10 to `shipped`. Merge to `refactor/phase-d` (not `main`). The **Phase D** row in the phase snapshot table flips to `shipped` in the sprint-0007 close commit when `refactor/phase-d` merges to `main`.

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. The Output schema audit CI gate is `active` in `CI-GATES.md` and CI.
3. `REFACTOR-STATUS.md` shows R10 `shipped`; Phase D row stays `in-progress` until sprint 0007 closes.
4. `cargo test --workspace` is green; all `insta` snapshots are re-anchored and reviewed.
5. `serde_json::json!()` macro usages in `gumiho-mudang-cli/src/commands/` and `gumiho-mudang-cli/src/output/` are removed; every JSON-emitting code path serializes a typed value.

## Implementation log

Chunk-by-chunk shipping log for R10 strict reading. Full per-chunk notes (including the `EdgeSummary` YAGNI rationale and the renderer-list explosion) live in [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) under the `R10 chunk-by-chunk delivery` log row.

| Chunk | Commit | Scope |
|---|---|---|
| 1 — schema scaffold | `689694c` | `output/schema/{compact_symbol,symbol_sketch,edge_summary}.rs` skeletons (`EdgeSummary` later dropped in chunk c) |
| a — sketch.rs | `46a9159` | 19 `json!()` sites → `SymbolSketch<'a>` sum-type (6 variants) |
| b — summary.rs | `38247ca` | 2 `json!()` sites → `Summary<'a>` sum-type |
| c — refs.rs | `9812109` | 2 `json!()` sites → `RefsGrouped<'a>` + `EdgeSummary` YAGNI'd; `deps.rs` / `impact.rs` already typed |
| d — index.rs | `ab0b667` | 7 `json!()` sites → `IndexFullResult` / `IndexIncrementalResult` / `IndexIncrementalUpToDate` + `IndexEvent` sum-type |
| e — source/init/setup | `a73c7f7` | Last 3 `json!()` sites → `SourceView` / `InitResult` / `SetupResult` (zero `json!()` in CLI after this) |
| g.A — sketch Display | `aef3a3c` | 6 sketch `print_*` fns → `XSketchView` + `impl fmt::Display` |
| g.B — refs/deps Display | `a3f372e` | 6 refs/deps/workspace-refs `print_*` fns → `XView` + Display |
| g.C — impact/trace/flow Display | `13b5fad` | 3 transitive-analysis `print_*` fns → `XView` + Display |
| g.D — entrypoints/map Display | `f5d086a` | 2 multi-section `print_*` fns → `XView` + Display |
| g.E — status/incremental/find/workspace Display | `2f434d7` | Last 6 `print_*` fns → `XView` + Display; `formatter.rs` reaches zero `println!` / `eprintln!` |

## Out of scope for this sprint

- R8 confidence audit subcommand — moves to [sprint 0007](./0007-phase-d-confidence-audit.md).
- Cross-crate output-struct extraction (a future shared `gumiho-mudang-scope-output` crate) — captured in [`gumiho-mudang-lsp/docs/SCOPE_OUTPUT_INTEROP.md`](../../../gumiho-mudang-lsp/docs/SCOPE_OUTPUT_INTEROP.md) as forward-looking, not in this sprint.
- `ts-rs` / `typeshare` binding generation — forward-looking, not in this sprint.
- Per-language depth feature work — post-refactor.
- Performance regression measurement — handled by the refactor-as-a-whole acceptance criterion enforced at Phase E close.
