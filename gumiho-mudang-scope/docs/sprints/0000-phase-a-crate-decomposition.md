# Sprint 0000 — Phase A (internal): Crate decomposition

> **Source of truth**: [`docs/ARCHITECTURE.md` §2.2](../../../docs/ARCHITECTURE.md#22-scope-decomposition-phase-a-internal) and [`docs/todos/0006-split-scope-crate.md`](../../../docs/todos/0006-split-scope-crate.md).
> **Phase**: A — but **not** owned by `ARCHITECTURAL-REFACTOR.md`. This sprint is a structural prerequisite governed by mudang's umbrella docs (ROADMAP §A line 54–59). It lands inside ROADMAP Phase A so that every subsequent R-move sprint lands code in the final sub-crate, not the legacy monolith.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Decompose the current monolithic `gumiho-mudang-scope` crate into the
five sub-crates declared by `docs/ARCHITECTURE.md` §2.2, with the
legacy crate name becoming a thin façade. **No behaviour changes.**
File moves only.

After this sprint:

- The five sub-crates compile and pass the existing test suite.
- Every R-move sprint (0001 → 0007) lands its code in the correct
  sub-crate per `docs/todos/0006-split-scope-crate.md` § "Ordering with
  the R-moves".
- External consumers (currently: the in-tree CLI; eventually: the
  composer) continue to import through `gumiho-mudang-scope` —
  the façade re-exports.

## What this sprint is and is not

- **Is**: a structural decomposition (file moves + crate manifests +
  re-exports + workspace `Cargo.toml`).
- **Is not**: an R-move. No `ARCHITECTURAL-REFACTOR.md` row flips
  here. No schema migration. No API surface change for downstream
  consumers.

## Why this sprint exists (and is not absorbed into sprint 0001)

Three reasons recorded so future readers do not collapse them:

1. **Distinct ownership.** `ARCHITECTURAL-REFACTOR.md` owns R0–R12.
   The crate decomposition is owned by `docs/ARCHITECTURE.md` §2.2 and
   `docs/todos/0006-split-scope-crate.md`. Mixing the two ownership
   surfaces inside one sprint blurs which document is the
   source of truth when a question arises.
2. **R-moves must land in final sub-crates.** Per
   [`docs/todos/0006-split-scope-crate.md` § Ordering with the R-moves](../../../docs/todos/0006-split-scope-crate.md):
   - R0 → `scope-graph`
   - R1, R2, R3 → primarily `scope-core` and `scope-index`
   - R4 → primarily `scope-workspace`
   - R5 → trait in `scope-core`, dispatch in `scope-index`
   - R10 → façade crate (`gumiho-mudang-scope`)
   - R12 → applies to every sub-crate
   Running an R-move against the monolith and then moving the code
   afterwards doubles the diff and breaks the "land in final shape"
   invariant.
3. **Linear sprint discipline.** The sprint plan (`sprints/README.md`)
   commits to one R-move sprint at a time. Combining R0/R1 with a
   five-way crate split would violate the "one bounded unit per
   sprint" rule and make review intractable.

## Prerequisites

None. This is the first sprint of the refactor.

## Charter alignment

- **`gumiho-mudang-scope/docs/CHARTER.md`** invariants — every one
  of §3's seven invariants holds before and after this sprint;
  decomposition is structural only. The §5 hard limits are unchanged
  and apply to every sub-crate equally
  ([`docs/todos/0006` § Non-goals](../../../docs/todos/0006-split-scope-crate.md#non-goals)).
- **`docs/ARCHITECTURE.md`** §2.1 — "scope vs lsp", "lsp basic-only",
  "composer separate from CLI", "edit as its own crate" — the
  decomposition is the precondition for these clean splits to land
  later in mudang's Phases C and E.

## Deliverables

Mirrored from
[`docs/todos/0006-split-scope-crate.md` § Affected code](../../../docs/todos/0006-split-scope-crate.md#affected-code)
and § Acceptance.

### File moves

- [ ] `gumiho-mudang-scope/src/core/parser.rs` → `scope-core/src/parser.rs`.
- [ ] `gumiho-mudang-scope/src/languages/*` → `scope-core/src/languages/*`.
- [ ] `gumiho-mudang-scope/src/core/indexer.rs` → `scope-index/src/indexer.rs`.
- [ ] `gumiho-mudang-scope/src/core/embedder.rs` (text builder) →
      `scope-index/src/embedder.rs`. Runtime / store implementations
      stay deferred to Phase D (LanceDB) per ROADMAP.
- [ ] `gumiho-mudang-scope/src/core/graph.rs` → `scope-graph/src/graph.rs`.
- [ ] `gumiho-mudang-scope/src/sql/schema.sql` → `scope-graph/src/sql/schema.sql`.
- [ ] `gumiho-mudang-scope/src/core/searcher.rs` → `scope-search/src/searcher.rs`.
- [ ] `gumiho-mudang-scope/src/core/workspace_graph.rs` →
      `scope-workspace/src/workspace_graph.rs`.
- [ ] `gumiho-mudang-scope/src/lib.rs` → façade re-export of the five
      sub-crates' public types.

### Crate manifests

- [ ] `scope-core/Cargo.toml`, `scope-index/Cargo.toml`,
      `scope-graph/Cargo.toml`, `scope-search/Cargo.toml`,
      `scope-workspace/Cargo.toml` exist with minimal dependency lists
      (each depends only on what its files require).
- [ ] `gumiho-mudang-scope/Cargo.toml` depends on the five sub-crates
      and re-exports their public types via `lib.rs`.
- [ ] Workspace `Cargo.toml` lists the five sub-crates as members.

### Acceptance ([source](../../../docs/todos/0006-split-scope-crate.md#acceptance))

- [ ] Monorepo builds with five sub-crates + façade.
- [ ] No new crate dependency cycle (`cargo check -p <crate>` per
      sub-crate succeeds without bringing in unrelated grammars).
- [ ] `gumiho-mudang-composer` (when it lands in Phase C) will be
      able to depend on the façade or directly on individual sub-crates
      — verified by ensuring the public types of each sub-crate are
      re-exported and visible.
- [ ] `gumiho-mudang-edit` (Phase E) will be able to depend on
      `scope-core` alone — verified by inspecting `scope-core/Cargo.toml`
      and confirming it does not transitively pull `scope-graph` or
      `scope-index`.
- [ ] Existing R-acceptance tests (and the current test suite as a
      whole) still pass — same test binaries, same assertions, after
      the move.
- [ ] Per-crate `cargo doc` builds; façade crate's docs link through
      to the sub-crates.

### Non-goals (recorded explicitly)

Per [`docs/todos/0006-split-scope-crate.md` § Non-goals](../../../docs/todos/0006-split-scope-crate.md#non-goals):

- This sprint does **not** rename the workspace manifest
  (`scope-workspace.toml` → `mudang-workspace.toml`, TODO 0002).
- This sprint does **not** rename the index directory
  (`.scope/` → `.mudang/`, TODO 0001).
- This sprint does **not** amend `gumiho-mudang-scope/docs/CHARTER.md`.
- This sprint does **not** introduce new public types — it relocates
  existing ones.

---

## Ambiguities to clarify before code lands

Per [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human),
each ambiguity below is resolved by an amendment commit to the relevant
source-of-truth doc (`docs/ARCHITECTURE.md`, `docs/todos/0006`) **before**
this sprint opens.

1. **Crate naming**: `scope-core` / `scope-index` / `scope-graph` /
   `scope-search` / `scope-workspace` — confirm these names are final.
   No `gumiho-` prefix per current docs. Question: are these
   workspace-member crates or top-level published crate names?
2. **Façade depth**: `lib.rs` re-exports every public type, or only
   the curated subset currently public on `gumiho-mudang-scope`?
   `docs/ARCHITECTURE.md` §2.2 implies re-export of the public types
   only — confirm.
3. **Compiled-out features**: does `scope-search` ship with FTS5
   enabled by default while LanceDB is feature-gated until Phase D?
   `docs/todos/0004-onnx-and-lancedb-distinction.md` is the source-of-truth
   to align with.
4. **`scope-workspace` content**: today `workspace_graph.rs` is the only
   file. Will the R4 split (LanguageWorkspaceContext /
   FrameworkWorkspaceContext, sprint 0002) live here or in `scope-core`?
   `docs/todos/0006` says "primarily inside `scope-workspace`" for R4 —
   confirm and lock so sprint 0002 lands code in the right crate.

---

## CI gates activated in this sprint

None. The R-move CI gates listed in
[`CI-GATES.md`](../CI-GATES.md#gate-inventory) ship in sprints 0001+.

This sprint does add per-crate `cargo check` to CI so each sub-crate
builds in isolation (a cycle-breaking guarantee for downstream
consumers); that wiring is part of the deliverables, not a CI-GATES
inventory row.

## Glossary terms touched

None. No new term. If the decomposition surfaces a term that
`GLOSSARY.md` should carry (unlikely — these are file moves), halt and
add via the glossary's own commit channel before the move ships.

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0000-crate-decomposition`, cut from `main`.
- **Open**: this sprint does not flip any row in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) because no R-move is
  involved. Instead, the sprint open is logged via the
  [`docs/todos/0006-split-scope-crate.md`](../../../docs/todos/0006-split-scope-crate.md)
  tracking line ("Tracking" field) — link to the branch / draft PR.
- **Codex review**: before close, run the canonical command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base main`
  - `--title "sprint 0000 — crate decomposition"`
  - Prompt focus: `docs/ARCHITECTURE.md §2.2`,
    `docs/todos/0006-split-scope-crate.md § Acceptance`, charter
    invariants (§3, applied to every sub-crate equally).
  Attach report to PR body; address blockers before closing.
- **Close**: update `docs/todos/0006` `Status: TODO` →
  `Status: shipped` with commit SHA and date. The mudang umbrella
  `docs/README.md` and `docs/todos/README.md` index reflect the new
  status.
- **Merge**: squash-merge or rebase-merge to `main`. Sprint 0001 is
  cut from the post-merge `main`.

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. Four ambiguities above are resolved by source-doc amendments before
   code lands.
3. `docs/todos/0006-split-scope-crate.md` status is `shipped` with the
   merge commit recorded.
4. The five sub-crates exist; the façade re-exports them; existing
   tests pass; no dependency cycle; `cargo doc` builds per crate.
5. `REFACTOR-STATUS.md` is untouched (no R-move advanced).
6. Sprint 0001 can now open knowing the target crate for each R-move's
   code per `docs/todos/0006-split-scope-crate.md § Ordering with the R-moves`.

## Out of scope for this sprint

- Any R-move (R0–R12). Sprint 0001 onwards.
- TODO 0001 (`.scope/` → `.mudang/` rename) — separate umbrella decision.
- TODO 0002 (`scope-workspace.toml` → `mudang-workspace.toml` rename)
  — separate umbrella decision.
- TODO 0003 (GitHub URL updates) — separate umbrella decision.
- Composer crate creation (TODO 0007) — Phase C of mudang ROADMAP.
- LSP basic-RPC contract (TODO 0008) — Phase B of mudang ROADMAP.
- Watcher deletion (TODO 0005) — Phase C of mudang ROADMAP.
