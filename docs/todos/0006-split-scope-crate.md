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
  to `scope-index`. Runtime / store implementations (when phase D
  lands) live in `scope-search`.
- `gumiho-mudang-scope/src/core/graph.rs` → moves to `scope-graph`.
- `gumiho-mudang-scope/src/sql/schema.sql` → moves to `scope-graph`.
- `gumiho-mudang-scope/src/core/searcher.rs` → moves to `scope-search`.
- `gumiho-mudang-scope/src/core/workspace_graph.rs` → moves to
  `scope-workspace`.
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
