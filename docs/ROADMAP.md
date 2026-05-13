# Mudang implementation roadmap

This document is the **immutable** order of work for the
`gumiho-mudang` monorepo. Phases are sequential. No phase begins
before the prior phase is at "Acceptance" state.

> **Hard rule.** Nothing in this roadmap may be reordered, skipped, or
> silently absorbed into another phase. Changes require an explicit
> roadmap amendment (this file edited in PR, reviewed, dated, with
> the requesting party recorded in the changelog at the bottom).

---

## Phase ordering (immutable)

```
A — Scope refactor                       (shipped; R-entries R0…R12)
        │
        ▼
B — LSP basic-RPC completion             (next; transport-layer only, no wrappers)
        │
        ▼
C — Composer + Notify API + Diagnostics  (parallel sub-tracks within C)
        ║   C.1 composer crate
        ║   C.2 notify API (external + internal events; deletes scope watcher)
        ║   C.3 LSP diagnostics aggregation
        │
        ▼
D — LanceDB + GPU embedder               (tier 1, then tier 2)
        │
        ▼
E — CodeStruct AST edit layer            (gumiho-mudang-edit crate)
```

No cross-cutting shortcuts.

---

## Phase A — Scope refactor (shipped)

Enforcement map: `gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md`.

R0–R12 are part of the current architecture. Scope's own internal
phase letters used during the refactor are unrelated to this roadmap's
phases A–E and must not be conflated.

### Demonstrated

- Every R-entry landed.
- Schema in its R0 shape.
- Confidence audit (R8) green.
- Trait-shape audit (R12) green.
- Charter §5 invariants unchanged.
- **Crate decomposition** per `docs/ARCHITECTURE.md` §2.2 and
  `docs/todos/0006-split-scope-crate.md`. Scope is split into
  `scope-core`, `scope-index`, `scope-graph`, `scope-search`,
  `scope-workspace`; `gumiho-mudang-scope` is a façade re-export.

### Acceptance

The scope crate (façade + 5 sub-crates) compiles, indexes a polyglot
fixture workspace, passes the R-entry acceptance test suite, and exposes
clean public types ready for the composer to consume.

---

## Phase B — LSP basic-RPC completion

Goal: `gumiho-mudang-lsp` exposes **only** primitive JSON-RPC
operations against a running language server. Nothing higher level.

### Surface (intentionally minimal)

- spawn / initialize / shutdown per language server;
- send any LSP request, receive any LSP response;
- subscribe to push notifications (publishDiagnostics, server custom);
- capability negotiation (mirror what `initialize` returned);
- minimal pool management: one client per language per workspace.

### What this phase explicitly does **not** add

- per-method convenience wrappers ("find references for symbol X");
- per-method caches;
- composition with scope;
- diagnostics aggregation;
- file-change handling;
- automatic retry beyond a single attempt.

All higher-level orchestration lives in the composer (phase C). The
rationale is in `docs/ARCHITECTURE.md` §5 and
`docs/todos/0008-lsp-basic-rpc-scope.md`.

### Dependencies

Phase A complete.

### Acceptance

- one LSP server per language reachable through the library;
- every method listed in `SCOPE-LSP-COMPOSITION.md` §13 is callable
  via the generic `request(...)` / `notify(...)` pair;
- capability negotiation surfaces unsupported methods cleanly;
- cold start, idle teardown, crash recovery from
  `SCOPE-LSP-COMPOSITION.md` §7 implemented and tested;
- no dependency on `gumiho-mudang-scope`;
- no caching layer.

---

## Phase C — Composer + Notify API + Diagnostics

The composer (`gumiho-mudang-composer`) is the canonical public
library API of mudang. The CLI becomes a thin wrapper over it. Other
consumers (MCP servers, IDE plugins, internal tools) depend on the
same crate.

Three sub-tracks. C.1 is the foundation; C.2 and C.3 may proceed in
parallel once C.1's skeleton is in place.

### C.1 — Composer crate

Create `gumiho-mudang-composer`. Move into it:

- composition logic from `SCOPE-LSP-COMPOSITION.md` (modes 1–5,
  levels 0–3, §5.4 merge algorithms, §17 decision tree);
- the LSP cache under `.mudang/lsp-cache/`;
- the convenient LSP-method wrappers (today nonexistent; built on top
  of `gumiho-mudang-lsp`'s basic-RPC layer);
- the daemon-mode runtime (long-lived process holding warm LSP and
  later the AST cache).

The CLI (`gumiho-mudang`) refactors so every subcommand is an 8–15
line clap → composer call → output formatter pipeline. The
composer's API is the only place commands are implemented.

Captured in `docs/todos/0007-composer-crate.md`.

### C.2 — Notify API + watcher deletion

A single `file_changed` event source fans out through the composer's
event bus to **both** scope and LSP (and any subscribers). The
scope-internal file watcher (`gumiho-mudang-scope/src/core/watcher.rs`)
is **deleted**; its responsibilities move into the composer's event
bus.

Three entry points to the event bus:

- **CLI**: `mudang notify <paths…>` and friends.
- **IPC**: Unix-socket protocol against a running daemon.
- **Rust**: `mudang_composer::Notifier` programmatic API.

Cascade levels (`none` / `graph` / `full`) control how far the
invalidation propagates (scope graph, LSP cache, AST cache,
embeddings).

Captured in `docs/todos/0005-delete-scope-watcher.md`. The full
protocol lives in `docs/NOTIFY-API.md` (written ahead of phase C as
the design contract; implementation lands during sub-track C.2).

### C.3 — LSP diagnostics aggregation

Composer subscribes to `publishDiagnostics` push and
`workspace/diagnostic` pull across every active LSP server. Aggregates
per-file, per-severity, per-code. Powers `mudang health`
(`SCOPE-LSP-COMPOSITION.md` §14 Case P).

Not cached — diagnostics are point-in-time state.

Surfaces as a stream consumable from outside the composer (subscribers
can listen for diagnostic events alongside file-change events).

### Dependencies

Phases A and B complete.

### Acceptance

- composer library callable from rust with the full mudang command
  surface;
- CLI ports every existing command to composer-backed implementations;
- notify API live across CLI + IPC + Rust;
- `scope/src/core/watcher.rs` removed from the tree;
- `file_changed` event fans out to scope and LSP atomically;
- diagnostics aggregation surfaces `mudang health`;
- a third-party Rust consumer (smoke test
  `examples/external_consumer.rs`) drives the composer end-to-end.

---

## Phase D — LanceDB + GPU embedder

Sources of truth:

- `docs/todos/0004-onnx-and-lancedb-distinction.md`;
- `SCOPE-LSP-COMPOSITION.md` §14.5 (Case AA, tier 2 enrichment);
- `SUBSTRATE-PRIMARY.md` §3.2 (embedding stack), §3.3 (GPU profile).

Two sub-stages.

### D.1 — Tier 1 (syntactic embeddings)

- `Embedder` trait with ONNX runtime impl;
- `Searcher` trait with `LanceSearcher`;
- replace FTS5 as the primary backend for `mudang find`;
- GPU backend (`cuda` / `metal`);
- cache key `(provider, model, dim, tier = v1)`.

Shippable without D.2.

### D.2 — Tier 2 (LSP-enriched embeddings)

- composer-side enrichment pipeline (mode 4 from §1.2);
- second LanceDB table `vectors_v2_enriched`;
- background daemon (idle-time enrichment, respects budget §18);
- rank fusion in `mudang find`;
- cache key extended with `lsp_server_version`.

### Dependencies

Phase C complete (composer hosts the enrichment pipeline).

### Acceptance

- `mudang find --semantic` returns vector-based results from tier 1;
- tier 2 daemon runs without blocking tier 1 queries;
- GPU backend active when configured; CPU fallback verified;
- cache invalidation cascades through the composer's event bus.

---

## Phase E — CodeStruct AST edit layer

Source of truth: `docs/EDIT-LAYER.md` (written when this phase opens).

Crate: `gumiho-mudang-edit`. Composer orchestrates.

### Invariants (captured here so they are not lost between phases)

- scope remains **read-only**; edit lives **outside** scope's charter;
- five safety gates: dry-run default, tree-sitter pre-parse check,
  pre/post LSP diagnostic diff, post-edit scope reindex, atomic apply
  (tempfile + rename);
- AST cache resident (RAM-rich and reference profiles), per
  `SUBSTRATE-PRIMARY.md` §3 update during this phase;
- no port of CodeStruct source — CC-BY-NC-4.0 license blocks direct
  reuse. Reimplement from the paper (arXiv 2604.05407) using
  tree-sitter primitives already in `scope-core`.

### Routing

- semantic edits (rename, extract, organize imports) → LSP via
  composer;
- structural edits (insert / replace / remove, file create / delete /
  move) → mudang-edit via composer.

### Dependencies

Phase D complete.

### Acceptance

- `mudang edit` CLI commands live, backed by composer;
- five gates enforced; dry-run default verified;
- per-language structural-edit primitives for at least: rust, ts,
  python;
- AST cache integrated into composer alongside scope graph and LSP
  cache;
- `EDIT-LAYER.md` written with the same depth as
  `SCOPE-LSP-COMPOSITION.md`.

---

## Dependency graph (condensed)

```
A (scope refactor)
        │
        ▼
B (LSP basic-RPC)
        │
        ▼
C (composer + notify + diagnostics)   ───── deletes scope watcher
        │
        ▼
D (LanceDB + GPU embedder)            ───── tier 1, then tier 2
        │
        ▼
E (CodeStruct AST edit)               ───── new edit crate
```

---

## What this roadmap is not

- **Not a release plan.** Each phase may take weeks; specific
  milestones belong in the issue tracker.
- **Not a feature list.** Specific commands and capabilities live in
  `SCOPE-LSP-COMPOSITION.md` §4 and §14.
- **Not a place for "if time allows".** Anything not listed here is
  either already inside a phase's acceptance criteria or out of scope.

## Amendments

To amend this roadmap:

1. Open a PR editing this file.
2. Record the change in the changelog below, naming the requester.
3. The PR must reference explicit approval from the project owner.

(Amendment dates live in git history.)

## Changelog

- Initial roadmap captured from design discussion. Five phases (A–E)
  locked. Watcher deletion folded into phase C. Scope decomposition
  folded into phase A.
- Phase A (scope refactor) shipped. R0–R12 part of the current
  architecture — see `gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md`
  (rule→implementation map) and
  `gumiho-mudang-scope/docs/POST-REFACTOR-PLAN.md` for queued work.
