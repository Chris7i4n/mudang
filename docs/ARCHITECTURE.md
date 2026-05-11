# Mudang architecture

How the crates fit together. This document explains **why** the
monorepo splits the way it does, what each crate may and may not do,
and what the composer layer enforces on top.

Read this before `SCOPE-LSP-COMPOSITION.md` (the composition
contract), `ROADMAP.md` (the build order), or `SUBSTRATE-PRIMARY.md`
(the workflow).

---

## 1. Lib-first principle

Mudang is a **library** that exposes its full surface through a public
Rust API. The CLI is **one consumer** of that library — not the source
of truth.

Any other crate (a future MCP server, an IDE plugin, an internal tool,
a test harness) gets the same surface by depending on the same
composer crate. There is no CLI-only behaviour.

Implications:

- the CLI binary is a thin wrapper that maps clap subcommands to
  composer function calls;
- every command emits structured data; the CLI renders it to TTY or
  JSON;
- the composer's API is the **only** place where commands are
  implemented;
- changing a command's behaviour is one place to touch.

---

## 2. Crate map

| Crate | Kind | Role | What it may **not** do |
|-------|------|------|------------------------|
| `gumiho-mudang-scope` | lib (façade) | Read-only graph engine: parser, indexer, graph, FTS, vector store | edit anything; talk to LSP; talk to filesystem outside index roots |
| `gumiho-mudang-lsp` | lib | Basic LSP RPC client: spawn, initialize, send request, receive response, push notifications | composition logic; caching policy; convenience wrappers; aggregation |
| `gumiho-mudang-edit` (phase E) | lib | AST edit primitives (insert / replace / remove), file create / delete / move on graph-tracked entities | type inference; semantic refactors; arbitrary fs / shell / git |
| `gumiho-mudang-composer` (phase C) | lib | Orchestrate scope + LSP + edit; expose command-level public API; event bus; notify API; cache | parse code itself; speak LSP wire protocol; mutate files outside the edit crate |
| `gumiho-mudang` | bin (CLI) | clap subcommands → composer calls; output formatting | implement any command; cache anything; talk to LSP or scope directly |

### 2.1 Why each split

- **scope vs lsp**: lifetimes are different. Scope is fast, persistent,
  polyglot. LSP is stateful, slow to start, mono-language. Mixing them
  in one crate destroys testability of either.
- **lsp basic-only**: a convenience method like
  `find_refs_with_lsp_confirmation()` is composition logic. Composition
  belongs in the composer. The LSP crate stays a transport.
- **composer separate from CLI**: lib-first principle. Other consumers
  must reach the same surface without invoking a subprocess.
- **edit as its own crate**: scope's charter forbids editing. Edit
  needs its own invariants doc. Keeping it separate keeps scope's
  read-only guarantee credible.

### 2.2 Scope decomposition (phase A internal)

To make AST edit (phase E) compose cleanly, the current
`gumiho-mudang-scope` crate is decomposed during phase A:

| Sub-crate | Owns |
|-----------|------|
| `scope-core` | tree-sitter `Parser`, `Symbol`, `Edge`, language plugins |
| `scope-index` | `Indexer`, file-hash table, incremental SHA-256 pipeline, embedding text builder |
| `scope-graph` | SQLite schema, graph queries (`find_refs`, `find_impact`, …) |
| `scope-search` | FTS5 backend + LanceDB backend, `Searcher` trait |
| `scope-workspace` | federated workspace facade |

`gumiho-mudang-scope` becomes a façade crate re-exporting the
sub-crates' public types. Existing API consumers (mainly the composer
and the CLI) keep working through the façade.

**Layout**: the five sub-crates live **nested inside** the façade
crate's directory (`gumiho-mudang-scope/scope-core/`,
`gumiho-mudang-scope/scope-index/`, etc.), not as siblings at the
workspace root. They are workspace members regardless. See
`docs/todos/0006-split-scope-crate.md § Sprint 0000 ambiguity
resolutions` for the locked naming, layout, façade depth, backend
scope, and R4 destination.

The edit crate (`gumiho-mudang-edit`, phase E) reuses
`scope-core`'s parser and language plugins **without** pulling in the
SQLite graph layer. The decomposition makes that possible.

Captured in `docs/todos/0006-split-scope-crate.md`.

---

## 3. The composer crate

`gumiho-mudang-composer` is the public library API of mudang. It is
where every layer meets.

### 3.1 Public surface (shape, not exhaustive)

```rust
pub struct Composer {
    scope: Arc<ScopeFacade>,
    lsp:   Arc<LspPool>,
    edit:  Option<Arc<EditEngine>>,      // populated in phase E
    cache: Arc<CompositionCache>,
    bus:   Arc<EventBus>,
}

impl Composer {
    pub fn open(workspace: &Path) -> Result<Self>;

    // Navigation (modes 1–5 / levels 0–3)
    pub fn find(&self, query: &str, opts: FindOpts) -> Result<FindResult>;
    pub fn refs(&self, symbol: &str, opts: RefsOpts) -> Result<RefsResult>;
    pub fn impact(&self, symbol: &str, opts: ImpactOpts) -> Result<ImpactResult>;
    pub fn sketch(&self, symbol: &str, opts: SketchOpts) -> Result<SketchResult>;
    pub fn explain(&self, symbol: &str) -> Result<ExplainResult>;
    pub fn map(&self) -> Result<MapResult>;
    // … rest of §4 and §14 commands from SCOPE-LSP-COMPOSITION.md

    // Pass-through LSP (mode 2)
    pub fn type_at(&self, pos: Position) -> Result<HoverResult>;
    pub fn rename(&self, pos: Position, new_name: &str) -> Result<WorkspaceEdit>;
    pub fn diagnostics(&self) -> Result<DiagnosticsSummary>;

    // Edit (phase E)
    pub fn edit(&self, op: EditOp, opts: EditOpts) -> Result<EditAck>;

    // Notifications
    pub fn notify(&self, paths: &[PathBuf], opts: NotifyOpts) -> Result<NotifyAck>;
    pub fn subscribe(&self, events: &[EventKind]) -> EventStream;
}
```

### 3.2 What the composer owns

- the **composition logic** in `SCOPE-LSP-COMPOSITION.md` (modes 1–5,
  levels 0–3, the §17 decision tree, the §5.4 merge algorithms);
- the **cross-language stitching layer** in
  `CROSS-LANG-STITCHING.md` (anchor normalisers, JOIN, stitch cache,
  `Composer::flow` and friends);
- the **LSP cache** under `.mudang/lsp-cache/` (composition doc §6);
- the **AST cache** when phase E lands;
- the **event bus** for file-change and diagnostics events;
- the **convenient LSP method wrappers** ("find references for
  symbol X", "rename across workspace") on top of the LSP crate's
  basic-RPC primitives;
- the **notify API** (`Composer::notify`, daemon IPC, CLI binding).

### 3.3 What the composer does **not** own

- parsing source code (scope owns it);
- speaking LSP wire protocol (lsp crate owns it);
- mutating files (edit crate owns it);
- the relational graph schema (scope-graph owns it).

### 3.4 Daemon mode

The composer can run as a daemon when long-lived state is wanted (warm
LSP servers, AST cache, notify queue). The daemon exposes a
Unix-socket JSON protocol; the CLI auto-detects a running daemon and
forwards calls.

The daemon **replaces** the current
`gumiho-mudang-scope/src/core/watcher.rs`. Watcher responsibilities
migrate into the composer's event bus during phase C (see
`docs/todos/0005-delete-scope-watcher.md`).

---

## 4. The unified `file_changed` event

A single event source — external `notify` call, daemon-side watcher,
editor plugin, git hook — fans out through the composer's bus to
**both** scope and LSP, and to any subscriber:

```
                       ┌───────────────────┐
                       │ external notifier │
                       │ (CLI / daemon /   │
                       │  editor / hook)   │
                       └─────────┬─────────┘
                                 │
                                 ▼
                       ┌───────────────────┐
                       │ Composer EventBus │
                       └───┬───────────┬───┘
                           │           │
              ┌────────────┘           └─────────────┐
              ▼                                       ▼
   ┌─────────────────────┐                ┌──────────────────────┐
   │  scope::reindex(p)  │                │  lsp::didChange(p)   │
   │                     │                │                       │
   │  • tree-sitter      │                │  • forward to server  │
   │    re-parse         │                │  • subscribe to       │
   │  • graph delta      │                │    diagnostics push   │
   │  • dangling edges   │                │                       │
   │  • cache evict      │                │                       │
   │  • embeddings dirty │                │                       │
   └─────────────────────┘                └──────────────────────┘
              │                                       │
              └────────────┬──────────────────────────┘
                           ▼
              composer.emit(
                reindex.completed,
                cache.invalidated,
                graph.invalidated,
                diagnostics.updated)
```

Both consumers see the **same** event. Neither knows about the other
directly. The composer is the only place where their fan-out is
coupled.

This is the architectural reason `core/watcher.rs` dies: the watcher
was scope-internal and single-consumer. The composer's bus is
multi-consumer and externally driveable (CLI / IPC / Rust).

---

## 5. The LSP crate's deliberately small surface

`gumiho-mudang-lsp` is a transport-layer crate by design. Its public
surface is approximately:

```rust
pub struct LspClient { /* one per language server */ }

impl LspClient {
    pub fn spawn(language: Language, config: &LspConfig) -> Result<Self>;
    pub fn initialize(&mut self) -> Result<ServerCapabilities>;
    pub fn shutdown(self) -> Result<()>;

    pub fn request<T: LspRequest>(&self, params: T::Params) -> Result<T::Result>;
    pub fn notify<T: LspNotification>(&self, params: T::Params) -> Result<()>;

    pub fn subscribe_diagnostics(&self) -> impl Stream<Item = PublishDiagnosticsParams>;
    pub fn subscribe_any(&self, method: &str) -> impl Stream<Item = JsonValue>;

    pub fn capabilities(&self) -> &ServerCapabilities;
}

pub struct LspPool { /* per-language, per-workspace */ }

impl LspPool {
    pub fn for_language(&self, lang: Language) -> Option<Arc<LspClient>>;
    pub fn warm_all(&self) -> Result<()>;
    pub fn restart(&self, lang: Language) -> Result<()>;
    pub fn status(&self) -> PoolStatus;
}
```

That's it. No `find_implementations_with_cache`. No
`rename_across_workspace`. No composition with scope. No caching. No
retry policy beyond a single attempt.

The composer wraps these primitives into the convenient API in §3.1.

Why so small: the LSP wire protocol is unstable per server. The right
place to absorb that instability is the composer's wrappers, not the
transport crate.

Captured in `docs/todos/0008-lsp-basic-rpc-scope.md`.

---

## 6. The CLI's deliberately small surface

`gumiho-mudang` is a clap-based binary. For each subcommand:

```rust
fn cmd_refs(args: RefsArgs) -> Result<()> {
    let composer = Composer::open(args.workspace())?;
    let result   = composer.refs(&args.symbol, args.into_opts())?;
    args.output_format.render(result, io::stdout())
}
```

Eight to fifteen lines per command. No command-specific logic.

The CLI's *only* unique responsibilities are:

- argument parsing (clap);
- workspace discovery (resolve cwd → workspace root);
- output formatting (TTY pretty vs JSON);
- exit-code mapping (composer error → process exit code).

Anything else moves to the composer.

---

## 7. How an external crate uses mudang

A third-party crate (an MCP server, an IDE plugin, a language-server
proxy, an internal tool) depends on the composer directly:

```toml
[dependencies]
gumiho-mudang-composer = { path = "../gumiho-mudang/gumiho-mudang-composer" }
```

```rust
use mudang_composer::Composer;

let c = Composer::open(workspace)?;
let result = c.refs("ProcessPayment", Default::default())?;
for row in result.rows {
    println!("{} {}", row.file.display(), row.line);
}
```

Same surface the CLI uses. No subprocess hop. No JSON parsing
round-trip.

---

## 8. Boundary contract

See `SUBSTRATE-PRIMARY.md` §5 for the workflow rule. The architectural
shape that backs it:

| Operation | Owned by | Notes |
|-----------|----------|-------|
| Code navigation queries | composer (scope-backed) | refs, impact, sketch, summary, find, trace, flow |
| Code structural reads | composer (scope-backed) | source, sketch |
| Code structural edits | composer (edit crate, phase E) | insert / replace / remove, file create / delete / move |
| Semantic edits | composer (LSP-backed) | rename, codeAction, organizeImports |
| Diagnostics | composer (LSP-backed) | health |
| Raw file read (non-code) | **not mudang** | Read tool / cat |
| Raw file write (non-code) | **not mudang** | Write tool |
| Filesystem listing / glob | **not mudang** | ls / fd |
| Shell execution | **not mudang** | bash |
| Git operations | **not mudang** | git CLI |
| Network | **not mudang** | not in mission |

Mudang owns **code-aware** operations on graph-tracked entities.
Everything else stays in orthogonal tools.

---

## 9. Relation to other docs

- `ROADMAP.md` — when each piece is built.
- `SCOPE-LSP-COMPOSITION.md` — how the composer composes (modes 1–5,
  levels 0–3, decision tree, merge algorithms).
- `CROSS-LANG-STITCHING.md` — composer's anchor-string JOIN that
  turns the polyglot graph into end-to-end cross-language flows.
- `SUBSTRATE-PRIMARY.md` — daily workflow built on this architecture.
- `gumiho-mudang-scope/docs/CHARTER.md` — what scope refuses to do.
- `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` — scope's
  internal R-moves.
- `NOTIFY-API.md` — the notify protocol (written ahead of phase C as
  the design contract; covers lib / CLI-daemon / CLI-one-off usage
  modes, the 8-step cascade flow, and the tier 2 enrichment hook).
- `EDIT-LAYER.md` (written in phase E) — the edit crate's invariants.
- `docs/todos/0005-delete-scope-watcher.md` — watcher deletion.
- `docs/todos/0006-split-scope-crate.md` — scope decomposition.
- `docs/todos/0007-composer-crate.md` — composer creation.
- `docs/todos/0008-lsp-basic-rpc-scope.md` — LSP minimality contract.

---

## 10. Non-goals

- This document is not a Rust-API reference. Generated docs live
  elsewhere.
- This document is not a coding standard.
- This document is not a release plan. Build order is in `ROADMAP.md`.
