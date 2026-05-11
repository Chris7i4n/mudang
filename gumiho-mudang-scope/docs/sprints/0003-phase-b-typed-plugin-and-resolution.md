# Sprint 0003 — Phase B: Typed plugin output and resolution pipeline

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R2](../ARCHITECTURAL-REFACTOR.md#r2--languageplugin-output-type-closure) and [§ R3](../ARCHITECTURAL-REFACTOR.md#r3--pipeline-ordering-via-type-state).
> **Phase**: B (second sprint of three).
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Restructure the language-plugin layer so that plugins return typed
captures (R2) and the indexer pipeline forces `extract → resolve → write`
order via Rust's type system (R3). After this sprint, language plugins
cannot directly emit edges; they cannot assign `status`; they cannot
skip the resolution stage. The mechanical safeguards for D2, E2, F1,
and the plugin-skip channel feeding R6 all land here.

## R-moves owned by this sprint

- **R2 — LanguagePlugin output type closure** ([§ R2](../ARCHITECTURAL-REFACTOR.md#r2--languageplugin-output-type-closure))
- **R3 — Pipeline ordering via type-state** ([§ R3](../ARCHITECTURAL-REFACTOR.md#r3--pipeline-ordering-via-type-state))

## Prerequisites

- Sprint 0001 shipped: R0 (`producer`, `pattern_id`, `args_text`,
  `skipped_ranges` schema, the 38-kind whitelist) and R1 (`EdgeBuilder`,
  sealed `Edge`) are required at the type level.
- Sprint 0002 merged into `refactor/phase-b`: R4
  (`LanguageWorkspaceContext`) is available to the resolver in R3; R7
  (dispatch) ensures the indexer is the sole driver of plugin
  invocation. These rows remain `in-progress` until Phase B merges to
  `main`.

## Charter alignment

- **Invariants** ([`CHARTER.md` §3](../CHARTER.md#3-core-invariants--must-never-break)):
  invariant 5 (tree-sitter resilience) — R2's `RawCaptures.skipped_ranges`
  is the plugin-side mechanism that feeds R6's malformed-source harness.
- **Universal language boundaries** ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  - **D2** (no best-guess fallback) — mechanical after R3. Status is
    the resolver's output; confidence is preserved verbatim through
    resolution
    ([`LANGUAGE-PLAYBOOK.md` Category D, D2](../LANGUAGE-PLAYBOOK.md#category-d--resolution-discipline)).
  - **D3** (no symbol-id collision resolution by guessing) — mechanical
    after R0 surrogate PK + R3 status assignment.
  - **E2** (no metadata interpretation in language plugin) — mechanical
    after R2: plugin returns capture results and the three reserved
    metadata keys
    ([`LANGUAGE-PLAYBOOK.md` Step 5 → Metadata schema for framework primitives](../LANGUAGE-PLAYBOOK.md#metadata-schema-for-framework-primitives))
    but not edges.
  - **F1** (no multi-pass semantic analysis in plugin) — mechanical
    after R3 typestate pipeline.

## Deliverables

### R2 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r2--languageplugin-output-type-closure))

- [ ] Trait inspection of `LanguagePlugin` shows no method whose
      signature implies forbidden behavior (inference, expansion,
      resolution, evaluation, narrowing, overload resolution).
      The negative trait shape is documented as a comment in the trait
      module. Trait-shape audit ships in sprint 0004 (R12).
- [ ] Plugin output type is `RawCaptures { captures, metadata,
      skipped_ranges }` per the R2 target-state code block. No plugin
      returns edges directly.
- [ ] A separate `Extractor` layer converts `RawCaptures` to
      `EdgeBuilder` calls. Per-kind logic moves into the `Extractor`;
      the plugin's role is purely capture + metadata + skip recording.
- [ ] Every existing language plugin (`rust`, `python`, `go`,
      `typescript`, `java`, `csharp`, `ruby`) is rewritten to return
      `RawCaptures`. Existing fixture suite produces **identical edges**
      before and after refactor, modulo `confidence` and `status` which
      are now explicit ([§ R2 Acceptance](../ARCHITECTURAL-REFACTOR.md#r2--languageplugin-output-type-closure)).
- [ ] A fixture where a plugin emits a `skipped_ranges` entry produces
      a `file_hashes` row containing that entry verbatim alongside any
      tree-sitter-error skips (R6 lights this gate up in sprint 0007;
      the **wiring** lands here).

### R2 metadata population ([source](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures) and [Language playbook Step 5](../LANGUAGE-PLAYBOOK.md#metadata-schema-for-framework-primitives))

- [ ] Every language plugin populates the three reserved
      `Symbol.metadata` keys where the AST exposes them:
      - `decorators` — Python `@`, TS `@`, anywhere a dedicated
        decorator node exists.
      - `annotations` — Java / C# annotations, Rust `attribute_item`.
      - `template_calls` — JSX in TS/TSX; ERB partials in Ruby; Jinja
        `{% include %}` / `{% extends %}` in Python; HEEx in Elixir
        (if/when Elixir lands); Razor in C#; Slim / Haml in Ruby.
- [ ] Languages that have no concept matching a key **omit** the key
      from the JSON (not an empty array) — the distinction
      "language did not implement this surface" vs "AST has no instances"
      must be preserved.
- [ ] No `hooks` key. Hook-style matching is framework-plugin work
      (R5, sprint 0005), not language-plugin work
      ([§ R0 metadata schema](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures)).

### R3 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r3--pipeline-ordering-via-type-state))

- [ ] Attempting to insert a `RawEdge` does not compile (storage
      signature accepts only `InsertableEdge`).
- [ ] Constructing an `InsertableEdge` outside the resolver module
      does not compile.
- [ ] The resolution pass produces edges with explicit
      `Resolved | Ambiguous | Dangling` status for every edge, with
      one row per candidate target on `Ambiguous` (multiplicity is
      representable per R0's surrogate `edge_id`).
- [ ] The extractor's `confidence` is preserved **verbatim** through
      resolution. Resolution does not downgrade a high-precision
      pattern to medium just because the lookup found multiple
      candidates
      ([`LANGUAGE-PLAYBOOK.md` D2 — orthogonality](../LANGUAGE-PLAYBOOK.md#category-d--resolution-discipline)).
- [ ] The `symbol_name_from_id` text-fallback path is deleted. Status is
      no longer implicit.
- [ ] The trivial / stub resolver from sprint 0001 (whichever variant
      the human selected — see sprint 0001 § Ambiguity to clarify) is
      replaced by the real resolver that consults
      `LanguageWorkspaceContext`.

---

## Ambiguities to clarify before code lands

Each ambiguity below is resolved by an amendment to the cited
source-of-truth document on `main` **before** this sprint's branch
opens. The rule lives in the source doc, not in the sprint branch.

1. **Extractor location.** R2 introduces an `Extractor` layer between
   plugin output and edge insertion. The location in the (post-sprint-0000)
   sub-crate tree is not specified — candidates include
   `scope-core/src/extract/`, `scope-index/src/extract/`, or a sub-module
   of `scope-core::plugin`. It affects R12's process-spawn denylist
   path filter ([`CI-GATES.md` § Process-spawn denylist](../CI-GATES.md#gate-inventory)).
   Resolution amends `ARCHITECTURAL-REFACTOR.md` R2 "Target state" with
   the chosen path; `docs/todos/0006-split-scope-crate.md § Ordering
   with the R-moves` may need a clarifying line if the choice deviates
   from "primarily inside scope-core and scope-index".

2. **Resolver location.** R3 introduces the resolver as a distinct
   stage. Location candidates: `scope-graph/src/resolve/`,
   `scope-index/src/resolve/`. Same impact on `CI-GATES.md` path
   filter. Resolution amends `ARCHITECTURAL-REFACTOR.md` R3 "Target
   state".

3. **Ambiguous edges and downstream queries.** R3 says ambiguous
   resolution emits one row per candidate target. The
   [`SCOPE-LSP-COMPOSITION.md`](../../../docs/SCOPE-LSP-COMPOSITION.md)
   composer layer at the mudang side reads these rows. The contract
   between scope's `Ambiguous` status and the composer's cleanest-signal
   filter is not specified here; if the composer assumes a single row
   per `(from_id, to_id, kind)` it will break. Resolution amends the
   relevant `docs/SCOPE-LSP-COMPOSITION.md` section to lock the contract,
   or amends `ARCHITECTURAL-REFACTOR.md` R3 "Target state" to declare
   the producer's commitment.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **Insertable typestate** (`just test-typestate`) — `planned` →
      `active`. Compile-fail test asserting `Graph::insert(RawEdge)` does
      not compile and the `InsertableEdge` constructor is unreachable
      outside the resolver module.

No other CI gate flips this sprint; the trait-shape and spawn-denylist
audits ship in sprint 0004.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`RawCaptures`, `Capture`, `MetadataField`, `SkippedRange`](../GLOSSARY.md#refactor-types)
- [`LanguagePlugin`, `Extractor`, reserved metadata keys](../GLOSSARY.md#plugin-shapes)
- [`Confidence`, `status`, orthogonality, cleanest-signal filter](../GLOSSARY.md#confidence-and-status-orthogonal)
- [Resolution pass, Typestate pipeline](../GLOSSARY.md#architecture)

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0003-typed-plugin-resolution`, cut from
  `refactor/phase-b` after sprint 0002 merged into it.
- **Base**: `refactor/phase-b`, **not** `main`.
- **Open**: flip R2 and R3 rows in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append log entries noting branch name.
- **Codex review**: before the sprint-close commit, run the canonical
  command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base refactor/phase-b`
  - `--title "sprint 0003 — R2+R3"`
  - Prompt focus: R2 and R3 acceptance bullets, D2 / D3 / E2 / F1
    mechanical enforcement, the three reserved metadata keys
    population rule (no `hooks` key), Insertable typestate CI gate.
  Attach report to PR body; address blockers.
- **Close**: demonstrate R2/R3 acceptance on the sprint branch and
  rebase-merge it into `refactor/phase-b`. R2 and R3 remain
  `in-progress` in `REFACTOR-STATUS.md` until the Phase B integration
  branch merges to `main`; `shipped` is reserved for main.
- **Merge**: rebase-merge sprint branch into `refactor/phase-b`.

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. Three ambiguities above are resolved before code lands.
3. The Insertable typestate CI gate is `active`.
4. `REFACTOR-STATUS.md` shows R2 and R3 `in-progress` on
   `refactor/phase-b`; R4 and R7 are also still `in-progress` there
   until the Phase B phase-close commit merges to `main`.
5. Each `docs/languages/<name>.md` compliance log
   ([`LANGUAGE-PLAYBOOK.md` Step 6](../LANGUAGE-PLAYBOOK.md#step-6--per-language-doc-template))
   records that D2, D3, E2, F1 are now mechanically enforced — no
   `NEEDS REVIEW` left for these rules on any active language plugin.
6. Existing fixture suite produces edges identical to pre-refactor
   modulo `confidence` and `status`. Diff is reviewed and approved.

## Out of scope for this sprint

- Immutable source guarantee (`&mut` audit) — sprint 0004 (R9).
- Macro definition-only trait shape — sprint 0004 (R11).
- Trait-shape audit + process-spawn denylist — sprint 0004 (R12).
- Framework plugin trait body and predicates — sprint 0005 (R5).
- Confidence audit subcommand and typed output schema — sprint 0006
  (R8, R10).
- Malformed-source fixtures and integration test — sprint 0007 (R6).
