# Sprint 0001 — Phase A: Schema and storage closures

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R0](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures) and [§ R1](../ARCHITECTURAL-REFACTOR.md#r1--typed-edge-insertion-api).
> **Phase**: A. Atomic — both R-moves ship together or neither ships.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Land the new edge schema (R0) and the typed insertion API (R1) as a
single atomic change. Everything downstream — typed plugin output (R2),
resolution typestate (R3), framework infrastructure (R5), audit
subcommand (R8) — assumes both are in place.

## R-moves shipped this sprint

- **R0 — Schema closures** ([§ R0](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures))
- **R1 — Typed edge insertion API** ([§ R1](../ARCHITECTURAL-REFACTOR.md#r1--typed-edge-insertion-api))

## Prerequisites

- Sprint 0000 (crate decomposition) shipped. The five sub-crates
  (`scope-core`, `scope-index`, `scope-graph`, `scope-search`,
  `scope-workspace`) and the `gumiho-mudang-scope` façade must exist
  on `main` so this sprint's R-move code lands in its final sub-crate
  per [`docs/todos/0006-split-scope-crate.md` § Ordering with the R-moves](../../../docs/todos/0006-split-scope-crate.md#ordering-with-the-r-moves):
  R0 → `scope-graph` (schema), R1 → `scope-core` / `scope-graph`
  (sealed `Edge` + `EdgeBuilder` typestate, splitting across the trait
  surface and the storage layer per `docs/todos/0006`).

## Charter alignment

- **Hard limits** ([`CHARTER.md` §5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)):
  R10 (Phase D) lands the typed-output enforcement of "no type / borrow /
  lint diagnostics"; R0 prepares the schema for it by adding
  `confidence` + `status` so honest ambiguity ([D2](../LANGUAGE-PLAYBOOK.md#category-d--resolution-discipline))
  is representable.
- **Soft expansion zone** ([`CHARTER.md` §6](../CHARTER.md#6-soft-expansion-zone--scope-expands-freely-here)):
  R0 lands the 30 domain edge kinds, surrogate `edge_id` PK, confidence
  and provenance metadata, decorator/annotation argument capture
  (`args_text` plus the three reserved `Symbol.metadata` keys), and
  partial-index recording (`skipped_ranges`).
- **Per-language IN scope** ([`CHARTER.md` §7](../CHARTER.md#7-per-language-scope-and-non-scope)):
  Go `green_thread_spawn` rename (was `goroutine_spawn`) lands here.
- **Invariants** ([`CHARTER.md` §3](../CHARTER.md#3-core-invariants--must-never-break)):
  invariant 4 (single polyglot graph) constrains the metadata keys to be
  template-system-agnostic — `template_calls`, not `jsx_renders`.

## Deliverables

Mirrored from each R-move's **Acceptance** section. Every checkbox is a
pointer; the bullet's *content* lives in the linked source.

### R0 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures))

- [ ] Every insert path goes through R1's typed builder; struct-literal
      `Edge { … }` outside `core::graph` is a compile error.
- [ ] Multi-row inserts of the same `(from_id, to_id, kind)` succeed,
      demonstrating the surrogate `edge_id` PK no longer collapses
      domain identity.
- [ ] Queries that filter `confidence='high' AND status='resolved'` run
      against the re-indexed corpus. Precision validation is **not** a
      gate here — it lands with R8 in Phase D ([§ R0 Acceptance](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures)).

### R0 schema deliverables ([source target state](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures))

The schema diff is described in the R0 "Target state" subsection. Sprint
deliverables track the diff:

- [ ] `edges.edge_id INTEGER PRIMARY KEY AUTOINCREMENT`; composite PK
      dropped; covering index `(from_id, to_id, kind)` added.
- [ ] `edges.confidence` (`high|medium|low`, NOT NULL, CHECK constraint).
- [ ] `edges.status` (`resolved|ambiguous|dangling`, NOT NULL, CHECK
      constraint).
- [ ] `edges.producer` TEXT NOT NULL.
- [ ] `edges.pattern_id` TEXT NOT NULL.
- [ ] `edges.capture_id` TEXT NULL.
- [ ] `edges.framework` TEXT NULL.
- [ ] `edges.args_text` TEXT NULL with Mitigation 1 (resolver skips
      fully-qualified targets) and Mitigation 2 (`[truncated]` marker at
      2 KB cap). See [`GLOSSARY.md` · `edges.args_text`](../GLOSSARY.md#schema).
- [ ] `edges.kind` whitelist = 38 entries
      (8 universal + 30 domain across R0 baseline 13 + Tier 1 5 + Tier 2 5 + Tier 3 7).
      The exhaustive list is in
      [`ARCHITECTURAL-REFACTOR.md` § R0 → edge kind whitelist](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures)
      and mirrored in
      [`CHARTER.md` §6 — domain edges row](../CHARTER.md#6-soft-expansion-zone--scope-expands-freely-here).
- [ ] Rename `goroutine_spawn` → `green_thread_spawn`
      ([`CHARTER.md` §7 Go IN scope](../CHARTER.md#go)).
- [ ] 4-kind concurrency split is representable: `os_process_spawn`,
      `os_thread_spawn`, `green_thread_spawn`, `runtime_task_spawn`
      ([`GLOSSARY.md` · 4-kind concurrency split](../GLOSSARY.md#plugin-shapes)).
- [ ] `symbols.kind` whitelist = 13 entries (10 legacy + `macro` +
      `module` + `trait`).
- [ ] `file_hashes.skipped_ranges TEXT NOT NULL DEFAULT '[]'`. Populated
      later in R2 (plugin-driven) and R6 (tree-sitter-error-driven); the
      schema lands here.
- [ ] **No in-place migration**. Pre-1.0 single-user wipe policy:
      `rm -rf .scope/ && scope index` rebuilds from source. See R0
      "Migration" subsection.

### R0 metadata-shape documentation ([source target state](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures))

The `symbols.metadata` column is unchanged at the schema level (TEXT JSON);
R0 documents the **JSON shape** language plugins must populate. R2
enforces the population mechanically; in this sprint the shape is
documented and committed for reference:

- [ ] `decorators: [{name, args_text?}]` — Python `@`, TS `@`, every
      grammar with a dedicated decorator node.
- [ ] `annotations: [{name, args_text?}]` — Java/C# annotations, Rust
      attribute_item nodes.
- [ ] `template_calls: [{name, args_text?}]` — JSX, ERB, Jinja, HEEx,
      Razor, Slim/Haml. **Template-system-agnostic** name; the
      polyglot single-graph invariant ([`CHARTER.md` §3 invariant 4](../CHARTER.md#3-core-invariants--must-never-break))
      forbids JSX-specific naming.
- [ ] **No reserved `hooks` key.** Hook-style matching is framework-layer
      regex over `Symbol.name`, never a language-plugin reserved key
      ([§ R0 → metadata schema](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures)).

### R1 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r1--typed-edge-insertion-api))

- [ ] `Edge` struct-literal construction outside `core::graph` is a
      compile error (fields demoted to `pub(crate)`).
- [ ] `Graph::insert_*` accepts only `InsertableEdge`. `RawEdge` does
      **not** implement `Insertable`.
- [ ] Removing any required `.confidence()` / `.producer()` /
      `.pattern_id()` call on `EdgeBuilder` is a compile error
      (typestate pattern).
- [ ] `EdgeBuilder` exposes **no** `.status(...)` method
      (compile-time trait inspection). See
      [`GLOSSARY.md` · `EdgeBuilder`](../GLOSSARY.md#refactor-types)
      and the R1 "Target state" subsection.
- [ ] No plugin or storage code constructs an insertable edge without
      going through the builder → resolution flow.

---

## Ambiguity to clarify before code lands

This sprint surfaces one cross-phase coupling that the source document
does not fully resolve:

> R1's acceptance bullet "`Graph::insert_*` accepts only `InsertableEdge`
> (output of resolution)" requires the **type** `InsertableEdge` to exist
> when Phase A ships. The **behavior** of the resolver that converts
> `RawEdge → InsertableEdge` ships in R3 (Phase B,
> [§ R3](../ARCHITECTURAL-REFACTOR.md#r3--pipeline-ordering-via-type-state)).
> Phase A therefore needs a placeholder resolver of some kind to satisfy
> compilation while still routing all production paths through it.

**Question for the human before sprint 0001 starts**:

- Does Phase A ship a *trivial* resolver stub
  (e.g., every `RawEdge` is mapped to `InsertableEdge { status: Dangling, … }`
  or similar) that R3 then replaces wholesale? — OR —
- Does Phase A ship the type signatures only, with `unimplemented!()`
  on every resolution path, leaving the binary unable to index until R3
  ships?

The acceptance bullet ("queries that filter `confidence='high' AND
status='resolved'` are runnable against the re-indexed corpus") implies
the binary must at least produce *some* `Resolved` rows after Phase A,
which favors option 1 with a stub that assigns `status=Resolved` when
the lookup is trivially unique and `status=Dangling` otherwise.

**Do not proceed past this sprint definition** until the human selects.
Per [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human),
the resolution is committed to `ARCHITECTURAL-REFACTOR.md` (the
source-of-truth document for R-moves) on `main` **before** this sprint's
branch opens. The rule lives in the source doc, not in the sprint
branch.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **Edge sealed** (`just ci-edge-sealed`) — `planned` → `active`.
- [ ] **Builder requires fields** (`just test-builder`) — `planned` →
      `active`.
- [ ] **Builder forbids status** (`just test-builder`) — `planned` →
      `active`.

The other gates listed in `CI-GATES.md` ship in later sprints; this
sprint does not touch their rows.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`Edge`, `EdgeBuilder`, `RawEdge`, `InsertableEdge`, `Producer`,
  `pattern_id`, `capture_id`, `Symbol`](../GLOSSARY.md#refactor-types)
- [`Confidence`, `status`, orthogonality, cleanest-signal filter](../GLOSSARY.md#confidence-and-status-orthogonal)
- [`StatusData`, `file_hashes.skipped_ranges`, Surrogate PK,
  `edges.args_text`, Schema bumps](../GLOSSARY.md#schema)
- [`EdgeKind` (38), `kind (symbols)` (13), reserved metadata keys,
  4-kind concurrency split](../GLOSSARY.md#plugin-shapes)
- [Polyglot single graph](../GLOSSARY.md#architecture)

A sprint never edits the glossary. If a new term emerges, halt and add
it via the glossary's own commit channel before resuming.

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0001-schema-storage`, cut from `main`
  (post-merge of sprint 0000).
- **Base**: `main` directly — Phase A's R-move set is a single sprint,
  so no phase integration branch is needed.
- **Open**: flip R0 and R1 rows in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append two log entries (one per move) noting
  `branch refactor/sprint-0001-schema-storage` and
  `notes: sprint 0001 opened`.
- **Codex review**: before the `REFACTOR-STATUS.md` transition commit,
  run the canonical command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base main`
  - `--title "sprint 0001 — R0+R1"`
  - Prompt focus: R0 and R1 acceptance bullets, charter §3 invariant 4
    (polyglot single graph), §5 hard limits, CI gates this sprint
    activates (Edge sealed, Builder requires fields, Builder forbids
    status).
  Attach report to PR body; address blockers.
- **Close**: flip both rows to `shipped` with commit SHAs and dates.
  Append log entries listing the acceptance bullets demonstrated.
  In the same commit, flip the **Phase A** row in the phase snapshot
  table to `shipped` (Phase A's only sprint is this one).
- **Merge**: squash-merge or rebase-merge to `main`. Sprint 0002's
  branch is cut from the post-merge `main`, not from this branch.

## Definition of done

All of the following hold simultaneously:

1. Every checkbox in the **Deliverables** section above is checked.
2. The cross-phase ambiguity above is resolved by an
   `ARCHITECTURAL-REFACTOR.md` amendment on `main`, merged before this
   sprint's branch opens.
3. The three CI gates listed above are `active` in `CI-GATES.md` and
   in CI itself.
4. `REFACTOR-STATUS.md` shows R0, R1, and Phase A all `shipped`.
5. No `NEEDS REVIEW` entry exists in any `docs/languages/<name>.md`
   that this sprint's schema changes touch (plugins are still pre-R2
   shape; the compliance log entries for R0/R1 land here even though
   the per-language plugin's R2 work is in sprint 0003).
6. Re-indexing the maintainer's reference corpus produces a `.scope/`
   that satisfies the R0 schema diff above and is queryable.

## Out of scope for this sprint

- R2/R3/R4/R5/R6/R7/R8/R9/R10/R11/R12 — every other R-move is later.
- Per-language depth feature work — paused until Phase E acceptance
  (see [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md)).
- Cross-language stitching at the composer layer — that is mudang-level
  work governed by [`docs/CROSS-LANG-STITCHING.md`](../../../docs/CROSS-LANG-STITCHING.md)
  in the umbrella repo; scope-side, this sprint only **enables** it by
  populating the necessary fields (`args_text`, domain edge kinds).
- `.scope/` → `.mudang/` rename
  ([`docs/todos/0001`](../../../docs/todos/0001-rename-scope-dir.md))
  — separate orchestration-layer decision, not part of the refactor.
- Sub-crate split of `gumiho-mudang-scope` — **already shipped in
  sprint 0000**; this sprint lands R0/R1 code in the sub-crates that
  decomposition created.
