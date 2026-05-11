# 0007 — Create `gumiho-mudang-composer` crate

- **Status:** TODO (phase C, sub-track C.1)
- **Decision:** introduce a new library crate that is the canonical public API of mudang. The CLI becomes a thin wrapper over it; other consumers (MCP servers, IDE plugins, internal tools) depend on the same crate.
- **Tracking:** _<issue / PR link to be added>_

## Decision

Create `gumiho-mudang-composer` as the canonical public API of mudang.
It orchestrates `gumiho-mudang-scope`, `gumiho-mudang-lsp`, and (in
phase E) `gumiho-mudang-edit`. The CLI binary becomes a clap-only
adapter on top.

Architectural shape: `docs/ARCHITECTURE.md` §3. Public surface
example: `docs/ARCHITECTURE.md` §3.1.

## What moves into the composer

- composition logic from `SCOPE-LSP-COMPOSITION.md` (modes 1–5,
  levels 0–3, §5.4 merge algorithms, §17 decision tree, §18 budget);
- cross-language stitching logic from `CROSS-LANG-STITCHING.md`
  (anchor normalisers, JOIN algorithm, `Composer::flow` /
  `stitched_edges` / `unresolved_anchors`, stitch cache);
- the LSP cache under `.mudang/lsp-cache/` (composition doc §6);
- the convenient LSP-method wrappers (today nonexistent; built on
  top of `gumiho-mudang-lsp`'s basic-RPC layer);
- the event bus (file-change fan-out to scope + LSP + future edit +
  subscribers — see TODO 0005);
- the notify API (CLI + IPC + Rust) implementation;
- the daemon-mode runtime (long-lived process holding warm LSP, AST
  cache, notify queue);
- the diagnostics aggregator (subscribes to LSP `publishDiagnostics`
  push + `workspace/diagnostic` pull);
- the AST cache in phase E (`docs/ARCHITECTURE.md` §3.2; see
  `SCOPE-LSP-COMPOSITION.md` §1.2 mode 4 cross-references).

## What stays out

- parsing source code — stays in `scope-core`;
- LSP wire protocol — stays in `gumiho-mudang-lsp`;
- file mutation — stays in `gumiho-mudang-edit` (phase E);
- relational schema — stays in `scope-graph`;
- raw filesystem, shell, git, network — never absorbed (see
  `docs/ARCHITECTURE.md` §8 boundary contract).

## Affected code

- new crate `gumiho-mudang-composer/`.
- `gumiho-mudang/src/main.rs` and command handlers — refactored so
  every subcommand is an 8–15 line clap → composer call → output
  formatter pipeline.
- `gumiho-mudang-cli` (current crate) merged into the new CLI crate;
  alternatively kept as `gumiho-mudang-cli` but emptied of command
  logic.
- existing CLI subcommands map 1:1 to composer methods; output
  formatters move into `gumiho-mudang/src/output/`.

## Public API contract

- the composer exposes commands, not endpoints. Each command is a
  function that takes typed options and returns a typed result.
- every result type includes `provenance` per
  `SCOPE-LSP-COMPOSITION.md` §8.
- errors are typed; the CLI maps them to exit codes; library consumers
  match on them.
- daemon mode is opaque to library consumers — they call the same
  functions whether running in-process or against a daemon.

## Acceptance

- every CLI subcommand calls into the composer;
- composer crate is callable from a third-party Rust consumer (proven
  by an `examples/external_consumer.rs` smoke test);
- composer can run as a daemon over a Unix-socket protocol;
- composer hosts the LSP cache and the event bus;
- composer hosts the notify API (TODO 0005 acceptance overlaps here);
- composer exposes diagnostics aggregation;
- no LSP method, scope query, or edit op is implemented in the CLI
  crate.

## Dependencies

- TODO 0005 (delete scope watcher) — composer's event bus replaces
  the deleted watcher;
- TODO 0006 (split scope crate) — composer depends on the sub-crates
  or the façade;
- TODO 0008 (LSP basic-RPC scope) — composer wraps that surface;
- phase A and phase B of `docs/ROADMAP.md` complete.

## Non-goals

- this TODO does not define the daemon's Unix-socket wire protocol in
  full — that lives in `docs/NOTIFY-API.md` (phase C deliverable);
- this TODO does not specify which CLI flags / subcommands change —
  the existing surface is preserved verbatim, only the implementation
  moves;
- this TODO does not introduce new commands beyond those already in
  `SCOPE-LSP-COMPOSITION.md` §4 + §14.
