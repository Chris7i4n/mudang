# 0006 — Split `gumiho-mudang-scope` into sub-crates

- **Status:** TODO (phase A internal)
- **Decision:** decompose the current monolithic `gumiho-mudang-scope` crate into focused sub-crates so the AST edit crate (phase E) and the composer (phase C) can depend only on what they need.
- **Tracking:** _<issue / PR link to be added>_

## Decision

Decompose `gumiho-mudang-scope` into:

| Sub-crate | Owns |
|-----------|------|
| `scope-core` | tree-sitter `Parser`, `Symbol`, `Edge`, language plugins |
| `scope-index` | `Indexer`, file-hash table, incremental SHA-256 pipeline, embedding text builder |
| `scope-graph` | SQLite schema, graph queries (`find_refs`, `find_impact`, `find_deps`, `find_call_paths`, `find_flow_paths`) |
| `scope-search` | FTS5 backend + LanceDB backend, `Searcher` trait |
| `scope-workspace` | federated workspace facade |

`gumiho-mudang-scope` remains as a thin façade crate that re-exports
the sub-crates' public types so existing API consumers (mainly the
composer and the CLI) keep working through the same import path.

## Motivation

- the AST edit crate (`gumiho-mudang-edit`, phase E) needs
  `scope-core`'s parser and language plugins **without** pulling in
  the SQLite graph layer;
- tests for `scope-graph` should not require the full parser plus
  every language grammar to compile;
- internal API boundaries today are not enforced — `parser.rs` and
  `graph.rs` know each other's internals freely;
- splitting compile units reduces incremental build time on the
  monorepo;
- the R-moves in `ARCHITECTURAL-REFACTOR.md` (R3 typestate pipeline,
  R4 WorkspaceContext split) become easier when each phase owns one
  sub-crate;
- a third-party consumer that wants only "tree-sitter parse + symbol
  extraction" gets a small dependency surface instead of the whole
  scope crate.

## Affected code

- `gumiho-mudang-scope/src/core/parser.rs` → moves to `scope-core`.
- `gumiho-mudang-scope/src/languages/*` → move to `scope-core`.
- `gumiho-mudang-scope/src/core/indexer.rs` → moves to `scope-index`.
- `gumiho-mudang-scope/src/core/embedder.rs` (text builder) → moves
  to **`scope-search`** (corrected from the earlier `scope-index`
  target). `searcher.rs` is the sole consumer of
  `build_embedding_text`, `split_camel_case`, and `split_snake_case`;
  `indexer.rs` does not call the embedder directly (it goes through
  the `Searcher` trait). Placing the embedder in `scope-index` would
  create a cycle (`scope-search` → `scope-index` for embedder,
  `scope-index` → `scope-search` for `Searcher`); placing it next to
  its sole caller in `scope-search` keeps the dependency graph acyclic
  (`scope-search` → `scope-core` only). Runtime / store
  implementations (when phase D lands) also live in `scope-search`.
- `gumiho-mudang-scope/src/core/graph.rs` → **split** between
  `scope-core` and `scope-graph`:
  - `Symbol` and `Edge` struct definitions (with their `serde` derives)
    move to `scope-core/src/types.rs`. They are the type backbone of
    parser output and language-plugin signatures; per the § Decision
    table, `scope-core` owns them.
  - `impl Symbol { fn from_row }` becomes a private free function
    `symbol_from_row(row: &rusqlite::Row) -> Symbol` inside
    `scope-graph/src/graph.rs`. The inherent method's only caller is
    inside `graph.rs`; the public surface is preserved by the façade
    re-exporting `Symbol` / `Edge` from `scope-core`.
  - Everything else (`Graph`, `ChangedFiles`, `ClassRelationships`,
    `CallerInfo`, `Reference`, `ImpactNode`, `ImpactResult`,
    `Dependency`, `CallPathStep`, `CallPath`, `TraceResult`, the SQL
    helpers, and `impl Graph`) moves intact to
    `scope-graph/src/graph.rs`, which adds `use scope_core::{Symbol,
    Edge}`.
  This split is the **only** non-trivial code change inside sprint
  0000; it is required by the § Decision table (scope-core owns
  Symbol/Edge) **and** the § Acceptance bullet "gumiho-mudang-edit
  (phase E) depends on `scope-core` only — never on `scope-graph` or
  `scope-index`". If Symbol stayed in scope-graph, edit would inherit
  SQLite transitively through scope-core, breaking the bullet.
- `gumiho-mudang-scope/src/sql/schema.sql` → moves to `scope-graph`.
- `gumiho-mudang-scope/src/core/searcher.rs` → moves to `scope-search`.
- `gumiho-mudang-scope/src/core/workspace_graph.rs` → moves to
  `scope-workspace`.
- `gumiho-mudang-scope/src/core/watcher.rs` → moves to `scope-index`
  (tightly coupled to the indexer; reindex is the watcher's only
  consumer). TODO 0005 deletes it during mudang phase C; until then,
  it lives next to its sole client.
- `gumiho-mudang-scope/src/core/mod.rs` → **deleted**. The `core`
  namespace dissolves; each sub-crate is its own root.
- `gumiho-mudang-scope/src/config/` (entire directory: `mod.rs`,
  `project.rs`, `workspace.rs`) → moves to `scope-core`. Reason:
  `indexer` (scope-index) and the CLI both consume the config types,
  so they live in the dependency root (scope-core). The façade
  re-exports `scope_core::config::*` so existing CLI imports continue
  to resolve through `gumiho_mudang_scope::config::*`.
- `gumiho-mudang-scope/src/queries/<lang>/*.scm` → moves to
  `scope-core/src/queries/<lang>/`. The language plugins (also in
  scope-core) load these via `include_str!`; they belong with their
  sole consumer.
- `gumiho-mudang-scope/src/lib.rs` → becomes a façade re-export.

## Ordering with the R-moves

The split happens **during** phase A so each R-move lands in its final
sub-crate, not in the legacy monolith.

- R0 (schema closures) — completes inside `scope-graph` once split.
- R1 (typed EdgeBuilder), R2 (RawCaptures), R3 (typestate pipeline) —
  primarily inside `scope-core` and `scope-index`.
- R4 (WorkspaceContext split) — primarily inside `scope-workspace`.
- R5 (FrameworkPlugin) — `scope-core` for the trait, `scope-index`
  for the dispatch.
- R10 (typed output) — façade crate (`gumiho-mudang-scope`).
- R12 (trait-shape audit) — applies to every sub-crate.

## Acceptance

- monorepo builds with the five sub-crates plus the façade.
- `gumiho-mudang-composer` depends on the façade or directly on
  individual sub-crates as appropriate (composer source documents
  which).
- `gumiho-mudang-edit` (when phase E lands) depends on `scope-core`
  only — never on `scope-graph` or `scope-index`.
- existing R-acceptance tests still pass.
- no new crate dependency cycle.
- per-crate `cargo doc` builds, and the façade crate's docs link
  through to the sub-crates.

## Non-goals

- this TODO does not rename the workspace manifest (TODO 0002) or the
  index directory (TODO 0001);
- this TODO does not amend scope's charter — the
  `gumiho-mudang-scope/docs/CHARTER.md` §5 invariants stay unchanged
  and apply to every sub-crate equally;
- this TODO does not introduce new public types beyond what already
  exists — it relocates them.

## Sprint 0000 ambiguity resolutions

Locked before sprint 0000 opens, per
`gumiho-mudang-scope/docs/sprints/README.md` §3 ambiguity protocol.

### 1. Crate naming and workspace layout

- **Names**: `scope-core`, `scope-index`, `scope-graph`, `scope-search`,
  `scope-workspace`. **No `gumiho-` prefix.** The façade crate keeps
  its current name `gumiho-mudang-scope`. The workspace root remains
  `publish = false`; sub-crate names need not be globally unique.
- **Layout**: **nested** under the façade crate's directory.

  ```
  gumiho-mudang-scope/
    Cargo.toml              # façade crate manifest
    src/lib.rs              # re-exports
    scope-core/
      Cargo.toml
      src/...
    scope-index/
      Cargo.toml
      src/...
    scope-graph/
      Cargo.toml
      src/...
    scope-search/
      Cargo.toml
      src/...
    scope-workspace/
      Cargo.toml
      src/...
  ```

  Rationale: the five sub-crates are an internal implementation detail
  of the scope family; nesting keeps every refactor diff inside one
  top-level directory and preserves the "scope = one family of crates"
  mental model. The workspace root's `members` list gains the five
  nested paths.

### 2. Façade re-export depth

The façade crate's `lib.rs` re-exports **every public item** currently
exposed by `gumiho-mudang-scope::*` before the split. The split is a
relocation, not a surface change. Curating the public surface is a
separate, post-refactor decision; sprint 0000 ships a 1:1 re-export so
that `gumiho-mudang-cli`, future composer code, and any external
consumer compile without source changes.

### 3. `scope-search` backends and feature gates

Sprint 0000 moves `core/searcher.rs` → `scope-search/src/searcher.rs`
**as-is**. Today that means FTS5 only. **No LanceDB code, no feature
gates, no `Searcher` trait split lands in sprint 0000.** LanceDB
adoption is mudang Phase D scope, governed by TODO 0004
(`0004-onnx-and-lancedb-distinction.md`); the `Searcher` trait split
and the `LanceSearcher` backend land then. Sprint 0000 is file moves
only.

### 4. `scope-workspace` content and R4 destination

`workspace_graph.rs` moves to `scope-workspace`. **R4's
`LanguageWorkspaceContext` / `FrameworkWorkspaceContext` split (sprint
0002) lands inside `scope-workspace`**, not `scope-core`. This locks
the destination for sprint 0002 so the ambiguity does not resurface
when R4 implementation begins.

This list is the canonical resolution. Any contradiction between
sprint 0000's deliverables and this list is resolved in favour of this
list; the sprint document is amended.
