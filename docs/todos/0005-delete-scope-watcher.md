# 0005 — Delete `gumiho-mudang-scope/src/core/watcher.rs`

- **Status:** TODO (phase C, sub-track C.2)
- **Decision:** the file watcher inside the scope crate is deleted. File-change events are externally driven and dispatched by the composer's event bus.
- **Tracking:** _<issue / PR link to be added>_

## Decision

Scope no longer owns a file watcher. The current
`gumiho-mudang-scope/src/core/watcher.rs` and its uses in the indexer
and the CLI's `watch` command move to the composer crate
(`gumiho-mudang-composer`) during phase C of `docs/ROADMAP.md`.

## Reasons

- file-change events have **multiple consumers** (scope graph, LSP
  server, AST cache, embedding daemon, third-party subscribers); the
  right home is the composer's event bus, not a scope-internal watcher;
- the watcher inside scope conflates "detect change" with "reindex";
  the new design separates them — notify API → event bus → scope
  consumer + LSP consumer + …;
- external drivers (`mudang notify`, daemon IPC, editor plugins, git
  hooks) need a uniform entry point that does not exist while the
  watcher lives inside scope;
- the watcher's existence inside scope conflicts with the lib-first
  principle (`docs/ARCHITECTURE.md` §1) — a watcher is a side-effect,
  not a library primitive.

## Affected code

- `gumiho-mudang-scope/src/core/watcher.rs` — deleted.
- `gumiho-mudang-scope/src/core/mod.rs` — drop the `watcher` module
  declaration.
- `gumiho-mudang-scope/src/core/indexer.rs` — remove direct watcher
  coupling. The indexer exposes `reindex(paths)` callable from outside
  and does not own any monitoring.
- `gumiho-mudang-cli` (or its successor inside the new CLI crate) —
  the `watch` subcommand routes through the composer daemon, not
  through scope.
- new: `gumiho-mudang-composer/src/event_bus.rs` — the replacement.
- new: `gumiho-mudang-composer/src/file_watcher.rs` — the composer-side
  filesystem watcher (kept here because **it has to talk to multiple
  consumers**).

## Migration notes

- the daemon binary (composer in daemon mode) carries the file
  watcher when one is wanted (`watch_files = true` in
  `.mudang/config.toml`);
- the CLI's old `mudang index --watch` keeps working but is
  implemented via the composer daemon, not via the scope-internal
  watcher;
- external editors / agents that want to drive reindexing call
  `mudang notify <paths…>` instead of relying on a watcher;
- this TODO does **not** change the incremental indexing pipeline
  inside scope — only the trigger side. SHA-256 dedupe, tree-sitter
  re-parse, and graph delta logic stay in `scope-index`.

## Acceptance

- `gumiho-mudang-scope/src/core/watcher.rs` removed from the tree.
- the `gumiho-mudang-scope` façade and all sub-crates compile without
  any `notify` / `notify-debouncer` / watcher-related dependency.
- `mudang index --watch` still works through the composer daemon.
- `mudang notify <paths…>` triggers reindex without any scope-internal
  watcher running.
- the composer's event bus emits `reindex.completed` and
  `cache.invalidated` for every notified change.

## Dependencies

- composer crate exists (`docs/todos/0007-composer-crate.md`).
- `gumiho-mudang-lsp` basic-RPC layer exists (phase B), so the bus has
  an LSP consumer that can forward `didChangeWatchedFiles`.

## Non-goals

- this TODO does not write the `NOTIFY-API.md` protocol spec — that is
  produced alongside, inside sub-track C.2;
- this TODO does not change the on-disk index directory name (TODO
  0001) or workspace manifest name (TODO 0002).
