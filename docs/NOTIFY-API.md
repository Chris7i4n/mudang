# Notify API

How file-change events enter mudang and how they propagate through
scope, LSP, embedding tiers (v1 and v2), and the AST cache.

This document is the protocol contract for the composer's event bus
and the notify entry points around it. It is the canonical spec for
phase C, sub-track C.2 of `docs/ROADMAP.md`.

The scope-internal watcher
(`gumiho-mudang-scope/src/core/watcher.rs`) is **deleted** as part of
this work. Responsibilities move to the composer
(`docs/ARCHITECTURE.md` §4, `docs/todos/0005-delete-scope-watcher.md`).

---

## 1. Motivation

When code is edited outside mudang — by another agent, by a non-LSP
editor, by a git checkout, by a generated-code step — the mudang
index drifts:

- `scope` graph references symbols that no longer exist;
- LSP-cache rows are keyed against stale file hashes;
- embedding vectors (tier 1 and tier 2) are out of date;
- AST cache (phase E) holds an old tree.

Once any one of those is wrong, every subsequent query is silently
wrong.

The notify API is the **single, uniform entry point** that tells
mudang "this file changed; sync everything related." It fans out to
every consumer atomically, so the substrate-primary thesis
(`docs/SUBSTRATE-PRIMARY.md` §1) keeps holding.

---

## 2. Usage modes

Mudang is **library-first** (`docs/ARCHITECTURE.md` §1). The CLI is
one of three usage shapes; the notify API works in all three.

### 2.1 Library consumer (no daemon, no socket)

A long-running Rust process holds a `Composer` instance. Scope graph,
LSP pool, caches all live inside that process's RAM. The consumer
calls `composer.notify(paths)` directly.

```rust
use mudang_composer::{Composer, NotifyOpts, CascadeLevel};

let c = Composer::open(workspace)?;
// composer holds scope graph + LSP pool + LSP cache + (phase E) AST
// cache in THIS process's address space
loop {
    let evt = my_external_source.next_event();
    c.notify(
        &evt.paths,
        NotifyOpts { cascade: CascadeLevel::Full, ..Default::default() },
    )?;
}
```

Suitable for: MCP servers, IDE plugins, long-running AI agents,
internal tools, language-server proxies, test harnesses.

No daemon process. No IPC. The consumer owns the lifecycle.

### 2.2 CLI with daemon (recommended for interactive dev)

```bash
mudang daemon start            # spawns; caches warm; listens on socket
mudang daemon status           # health, pid, uptime, queue depth
mudang daemon stop             # graceful shutdown
mudang daemon restart          # bounce, preserves queue if --preserve-queue
```

A background process holds a `Composer` instance with the same caches
as in §2.1. Subsequent `mudang <cmd>` calls auto-detect the daemon
(via `.mudang/daemon.pid` and `.mudang/daemon.sock`) and route through
IPC, hitting warm caches.

`mudang daemon start` is the explicit CLI equivalent of "lib consumer
but on the command line." The user opt-in keeps the process alive.

Suitable for: a single dev iterating in one workspace across many
commands and editor saves.

### 2.3 CLI one-off (no daemon)

```bash
mudang notify foo.rs --cascade full   # open composer, notify, exit
```

Each command opens a fresh `Composer`, runs the op, exits. No warm
caches survive across invocations. Slower (LSP cold start every time;
no AST cache; no LSP cache) but valid for scripts, CI, git hooks
where lifecycle is per-call by design.

Suitable for: CI, scheduled jobs, git hooks, throwaway scripts.

### 2.4 Decision matrix

| Situation | Mode |
|-----------|------|
| Long-running agent / MCP server / IDE plugin | §2.1 lib |
| Interactive dev with many commands per session | §2.2 daemon |
| CI / git hooks / one-off scripts | §2.3 no daemon |
| Driving from another Rust crate without subprocess hop | §2.1 lib |
| Embedded inside a larger application | §2.1 lib |

All three modes call the same `Composer::notify(...)` method. The
daemon is **not** a separate code path; it's a long-lived process
hosting the same Composer the lib consumer would hold.

---

## 3. CLI surface

```bash
# Single notification
mudang notify <path> [<path> ...]
mudang notify --stdin                  # paths via stdin, newline-separated
mudang notify --dir <path>             # recursive within a directory
mudang notify --git-changed            # working tree (uncommitted)
mudang notify --git-staged             # only staged
mudang notify --since <git-ref>        # diff vs ref
mudang notify --all                    # equivalent to `mudang reindex --full`

# Daemon control
mudang daemon start [--watch-files] [--preload-lsp <langs>]
mudang daemon stop [--graceful-timeout-seconds N]
mudang daemon status [--json]
mudang daemon restart [--preserve-queue]

# Event subscription (for scripts / agents)
mudang events --follow [--events <kind>,<kind>,...]
mudang events --since <iso-timestamp>
```

### 3.1 Notify flags

| Flag | Meaning | Default |
|------|---------|---------|
| `--cascade <none\|graph\|full>` | Cascade level (§7) | `full` |
| `--async` | Return immediately; daemon queues | `false` (sync) |
| `--ack` | Block until done; equivalent to default | `true` |
| `--dry-run` | Report what would be done; touch nothing | `false` |
| `--source <name>` | Origin tag for audit log | `"cli"` |
| `--json` | Emit ack as JSON instead of text | `false` |
| `--no-daemon` | Force in-process even if daemon is running | `false` |
| `--require-daemon` | Hard-error if no daemon | `false` |
| `--watch-files` (daemon start) | Enable filesystem watcher inside the daemon | `false` |

### 3.2 Daemon-start flags

| Flag | Meaning | Default |
|------|---------|---------|
| `--watch-files` | Spawn an inotify/FSEvents watcher inside the daemon | `false` |
| `--preload-lsp <langs>` | Warm these LSP servers at startup | from config |
| `--preload-ast <paths>` | Pre-parse these paths into AST cache (phase E) | from config |
| `--socket <path>` | Override socket path | `.mudang/daemon.sock` |
| `--pid-file <path>` | Override pid file | `.mudang/daemon.pid` |
| `--foreground` | Don't detach | `false` |

---

## 4. IPC protocol (daemon mode)

Transport: Unix socket at `.mudang/daemon.sock` (location overridable).
Wire format: line-delimited JSON. Each request and each event is a
single JSON object terminated by `\n`.

### 4.1 Requests

```jsonl
→ {"op":"notify","id":"01J...","paths":["src/foo.rs"],"cascade":"full","ack":true,"source":"nvim-lsp"}
← {"op":"ack","id":"01J...","stats":{"files_reindexed":1,"symbols_added":3,"symbols_removed":1,"edges_invalidated":7,"lsp_cache_evicted":4,"embeddings_v1_dirty":3,"embeddings_v2_queued":3,"duration_ms":47}}
```

```jsonl
→ {"op":"status"}
← {"op":"status","daemon_pid":12345,"workspace":"/Users/.../proj","index_root":".mudang","watcher_active":true,"stale_files":0,"queue_depth":0,"uptime_s":3600,"lsp_pool":{"rust":{"state":"warm","pid":12346},"typescript":{"state":"idle","pid":12347}}}
```

```jsonl
→ {"op":"subscribe","id":"01K...","events":["reindex.completed","cache.invalidated","tier2.completed"]}
← {"event":"reindex.completed","id":"01J...","paths":["src/foo.rs"],"stats":{...},"ts":"2026-05-10T12:34:56Z"}
← {"event":"tier2.completed","id":"01J...","symbol_ids":["src/foo.rs::Foo::class::12"],"ts":"2026-05-10T12:34:58Z"}
... (stream continues until client disconnects or `unsubscribe`)
```

```jsonl
→ {"op":"flush"}                      # drain queue synchronously
← {"op":"ack","queue_flushed":12}

→ {"op":"reindex","scope":"all"}      # nuclear reindex
← {"op":"ack","files_reindexed":523,"duration_ms":4720}

→ {"op":"shutdown","graceful":true}
← {"op":"ack","shutting_down":true}
```

### 4.2 Server-pushed events (without subscription)

The daemon does **not** push unsolicited events. Subscribers opt in
explicitly via `op: "subscribe"`. This keeps the protocol predictable
under low-power clients.

### 4.3 Error responses

```jsonl
← {"op":"error","id":"01J...","code":"BUDGET_EXCEEDED","message":"4213 paths exceeds notify.max_batch=1000","hint":"split the call or set --allow-large"}
```

Error codes are defined in §10.

### 4.4 Protocol versioning

Each connection begins with a handshake:

```jsonl
→ {"op":"hello","client":"mudang-cli/0.5.0","protocol":1}
← {"op":"hello","daemon":"mudang-composer/0.5.0","protocol":1,"workspace":"..."}
```

Protocol version is a single integer. Bump on breaking changes. The
daemon rejects connections with mismatched major version.

---

## 5. Rust API (library mode)

```rust
pub struct Notifier { /* lives inside Composer */ }

impl Composer {
    pub fn notifier(&self) -> &Notifier;
}

impl Notifier {
    pub fn notify(&self, paths: &[PathBuf], opts: NotifyOpts) -> Result<NotifyAck>;
    pub fn notify_async(&self, paths: &[PathBuf], opts: NotifyOpts) -> Result<NotifyHandle>;
    pub fn subscribe(&self, events: &[EventKind]) -> EventStream;
    pub fn status(&self) -> NotifierStatus;
    pub fn flush(&self) -> Result<FlushAck>;
}

pub struct NotifyOpts {
    pub cascade: CascadeLevel,
    pub source:  Option<String>,
    pub timeout: Option<Duration>,
    pub dry_run: bool,
}

pub enum CascadeLevel {
    None,    // SHA check only; no work performed
    Graph,   // scope graph + tier 1 embeddings
    Full,    // graph + LSP cache + tier 1/2 embeddings + AST cache
}

pub struct NotifyAck {
    pub id:                String,
    pub paths:             Vec<PathBuf>,
    pub files_reindexed:   usize,
    pub symbols_added:     usize,
    pub symbols_removed:   usize,
    pub edges_invalidated: usize,
    pub lsp_cache_evicted: usize,
    pub embeddings_v1_dirty: usize,
    pub embeddings_v2_queued: usize,
    pub ast_cache_dropped:   usize,
    pub duration_ms:       u64,
}

pub enum EventKind {
    ReindexStarted,
    ReindexCompleted,
    ReindexFailed,
    GraphInvalidated,
    CacheInvalidated,
    DiagnosticsUpdated,
    Tier1Reembedded,
    Tier2Queued,
    Tier2Completed,
    Tier2Failed,
    AstCacheInvalidated,
}
```

The Rust API is the canonical surface. CLI and IPC are thin wrappers
that translate to and from `Notifier::notify(...)`.

---

## 6. Cascade flow (the 8-step pipeline)

When `notify(path, cascade=full)` is called — by any of the three
usage modes — the composer runs this pipeline:

```
notify(path, cascade=full)
   │
   ▼
1. SHA-256(file_content) — if equal to cached hash, NO-OP. EARLY EXIT.
   │
   ▼
2. tree-sitter re-parse → symbols_new, edges_new
   │  (via scope-core; tolerates broken source per CHARTER §5)
   │
   ▼
3. SQLite transaction BEGIN
     a. DELETE FROM symbols WHERE file = path
     b. DELETE FROM edges   WHERE file = path
     c. INSERT new rows
     d. UPDATE file_hashes SET hash = sha
   COMMIT
   │  (atomic; readers see before-or-after, never partial)
   │
   ▼
4. Edges previously pointing TO symbols that are now removed:
   - mark status = dangling
   - schedule resolution pass (composer event)
   │
   ▼
5. Cache invalidation cascade (composer-owned):
   - LSP cache: evict entries keyed by old_hash
   - AST cache (phase E): drop tree for path
   - embeddings v1 (tier 1): mark dirty; re-embed background
   - embeddings v2 (tier 2): queue tier-2 enrichment when LSP idle
                              (see §9 — LSP-enriches-Scope path)
   │
   ▼
6. LSP server notification (if at least one LSP is running):
   - send `workspace/didChangeWatchedFiles` to each affected server
   - server reprocesses on its own clock
   │
   ▼
7. Publish events to subscribers:
   - reindex.completed
   - graph.invalidated (with symbol_ids added / removed)
   - cache.invalidated (with cache keys)
   - tier2.queued (per affected symbol_id)
   │
   ▼
8. Return NotifyAck with stats
```

Steps 1–4 are synchronous and atomic. Step 5 is partially async
(tier 2 embeddings + LSP cache eviction run on background workers).
Step 6 is fire-and-forget. Step 7 may complete out of order across
subscribers.

---

## 7. Cascade levels

| Level | Steps run | Use when |
|-------|-----------|----------|
| `none` | 1 only (SHA check; report dirty) | health check; "is this file synchronized?"; no work wanted |
| `graph` | 1, 2, 3, 4, 5 (only tier 1 portion), 8 | internal edit; LSP / tier 2 irrelevant or expensive |
| `full` | 1 through 8 | **default**; external agent or editor made an edit |

Selection rules:

- if the caller is **not sure**, use `full`. The composer makes `full`
  cheap when there is nothing to do (step 1 short-circuits).
- batch notifies of N paths still execute steps 5–7 once per affected
  symbol, not once per file — the composer dedupes.
- `none` is the right call when only "is the index stale?" matters,
  e.g. `mudang status --strict` in CI.

---

## 8. Event taxonomy

```
reindex.started        { id, paths, ts }
reindex.completed      { id, paths, stats, duration_ms, ts }
reindex.failed         { id, paths, error, ts }
graph.invalidated      { symbol_ids_added, symbol_ids_removed, edges_invalidated, ts }
cache.invalidated      { cache_keys, ts }
diagnostics.updated    { file, severity_counts, ts }
tier1.reembedded       { symbol_ids, duration_ms, ts }
tier2.queued           { symbol_ids, ts }
tier2.completed        { symbol_ids, duration_ms, ts }
tier2.failed           { symbol_ids, error, ts }
ast_cache.invalidated  { files, ts }   # phase E
```

Each event carries the `id` of the originating `notify` call when
applicable, and an ISO-8601 `ts`. Subscribers filter by `EventKind`
when calling `subscribe(...)`.

---

## 9. Tier 2 integration (LSP enriches Scope)

**Critical**: this is where mode 4 of `SCOPE-LSP-COMPOSITION.md` §1.2
(LSP-enriches-Scope-offline) connects to the notify pipeline. The
tier 2 embedding daemon is one of the consumers of notify events —
not a separate trigger source.

Cross-references:
- `SCOPE-LSP-COMPOSITION.md` §14.5 — Case AA (full tier 2 spec);
- `docs/todos/0004-onnx-and-lancedb-distinction.md` — Tier 2 follow-up;
- `docs/ARCHITECTURE.md` §3.2 — composer ownership.

### 9.1 Sequence for a single file change

```
notify(src/foo.rs, cascade=full)
   │
   ▼
... steps 1–4 from §6 ...
   │
   ▼
step 5: embeddings v2 queue
   │   per affected symbol_id:
   │     add to .mudang/tier2-queue.jsonl
   │     emit tier2.queued event
   ▼
[ later, idle window or scheduled tick ]
   │
   ▼
tier 2 daemon (composer-side worker):
   1. drain queue items in batch
   2. for each symbol_id:
        a. fetch scope's syntactic embedding text (build_embedding_text_v1)
        b. LSP queries: hover, inlayHint, implementation, semanticTokens
        c. join into build_embedding_text_v2 (enriched)
        d. ONNX embed → vector_v2
        e. write to LanceDB table vectors_v2_enriched
   3. emit tier2.completed event with symbol_ids + duration
```

### 9.2 LSP availability handling

The tier 2 worker:

- **pauses** when the required LSP server is unavailable (cold,
  crashed, or never spawned);
- **resumes** when the server returns to `warm` state;
- never blocks tier 1 queries — tier 1 always works without LSP;
- per-symbol failure: drop and tag `tier2_unavailable` in the cache
  key; queue can retry later.

### 9.3 Backpressure

```toml
[embeddings.tier2]
queue_max_depth          = 10000          # default
batch_size               = 32             # symbols per batch
enrich_timeout_ms        = 500            # per symbol
batch_budget_seconds     = 60             # total per drain cycle
idle_only                = true           # only run when LSP idle
pause_on_lsp_busy        = true
```

If the queue exceeds `queue_max_depth`, the composer:

1. emits a `tier2.failed` event with reason `queue_overflow`;
2. degrades fan-out: subsequent notifies skip tier 2 queueing until
   the queue drains below 50 % of the cap;
3. logs `tracing::warn`.

Tier 1 queueing is unaffected — that path is fast and always runs.

### 9.4 Cache key tied to LSP version

Tier 2 entries are keyed by
`(source_hash, model, dim, tier = v2, lsp_server_version)`. An LSP
server upgrade invalidates **all** tier 2 entries for that language
and re-enqueues them at next idle window. Tier 1 is untouched.

This is why phase D ships tier 1 first (lower risk) and tier 2 as a
follow-up (`docs/ROADMAP.md` phase D acceptance).

### 9.5 Failure tagged, never silent

When tier 2 fails for a symbol, the absence is visible:

- the vector simply does not exist in `vectors_v2_enriched`;
- a row is recorded in `.mudang/tier2-failures.log` with
  `(symbol_id, reason, ts)`;
- `mudang find --semantic` still returns the symbol from tier 1
  results; the rank-fusion step (composition doc §14.5 query flow)
  detects v2 absence and falls back to v1 alone for that symbol;
- the absence does **not** degrade other queries.

---

## 10. Workflows

### 10.1 External agent edited files

```bash
# Agent wrote 3 files using its own toolchain
mudang notify src/auth.rs src/users.rs src/db.rs --cascade full --source claude-agent
# Returns ack with stats; everything synchronized
```

### 10.2 Git checkout / rebase

```bash
git checkout feature/new-auth
git diff --name-only main HEAD | mudang notify --stdin --cascade full
```

Hook `.git/hooks/post-checkout`:

```bash
#!/usr/bin/env bash
prev="$1"; new="$2"; branch_change="$3"
if [ "$branch_change" = "1" ]; then
  git diff --name-only "$prev" "$new" | mudang notify --stdin --async
fi
```

### 10.3 Editor without LSP-via-mudang

Plugin sends on file save:

```bash
mudang notify "$file" --async --source nvim-autocmd
```

### 10.4 Long-running agent (lib mode)

```rust
let c = Composer::open(workspace)?;
let events = c.notifier().subscribe(&[EventKind::Tier2Completed]);

tokio::spawn(async move {
    while let Some(e) = events.next().await {
        println!("tier 2 ready for {:?}", e.symbol_ids);
    }
});

loop {
    let request = next_agent_request().await;
    request.edit_files()?;
    c.notifier().notify(&request.paths, Default::default())?;
    // agent now queries through composer; tier 2 enrichment runs in
    // background; agent gets richer find() results once tier2.completed
    // fires
}
```

### 10.5 Bulk reconciliation after long offline session

```bash
mudang notify --git-changed --async
mudang events --follow --events reindex.completed
# watch in real time until daemon drains queue
```

### 10.6 Health check before strict CI run

```bash
mudang notify --all --cascade none --json | jq '.stale_files'
# 0 → safe; nonzero → reindex needed before --strict queries
```

### 10.7 Recovery from drift

```bash
mudang status                          # list stale files
mudang notify --all --cascade full     # nuclear reindex
mudang verify --sample 500 --strict    # confirm graph health
```

---

## 11. Guarantees

| Property | Guarantee |
|----------|-----------|
| Atomicity per file | SQLite transaction; readers see state before or after, never partial |
| Idempotency | SHA-256 check at step 1; duplicate notify = no-op |
| Ordering | FIFO per consumer; sync calls complete in arrival order |
| Backpressure | Queue ceiling configurable (`notify.queue_max_depth = 10000`); excess returns `BUDGET_EXCEEDED` |
| Crash safety | Queue persisted to `.mudang/notify-queue.jsonl`; replayed on daemon restart |
| Concurrent writers | SQLite WAL; readers never block; writers serialized |
| Multi-consumer fan-out | All consumers receive the same event; failure in one does not block others |
| Tier 2 independence | Tier 2 failures do not affect tier 1 or scope graph |

---

## 12. Failure modes

| Situation | Code | Behaviour |
|-----------|------|-----------|
| Path does not exist anymore | `path_missing` (informational) | DELETE rows for that path; emit `reindex.completed` with `files_reindexed: 1, symbols_removed: N`; succeed |
| Parse error in source | `parse_partial` | Emit partial symbols; flag `parse_errors=true` on file row; succeed |
| Path outside indexed roots | `out_of_root` | Warn; skip; non-error exit |
| Daemon dead | n/a | Fall back to in-process; warn that async unavailable |
| LSP server crash during cascade | `lsp_degraded` | Reindex completes; LSP step marked degraded; ack tagged |
| Embedding runtime unavailable | `embedder_unavailable` | Reindex completes; v1/v2 queued for next idle window |
| Queue depth exceeded | `BUDGET_EXCEEDED` | Reject; suggest `--allow-large` |
| Workspace lock contention | `lock_held` | Retry up to 3 times; then error |
| Disk full | `io_disk_full` | Hard fail; ack with error |
| Daemon shutting down | `shutting_down` | Reject new requests; complete in-flight |

---

## 13. Daemon lifecycle

### 13.1 Discovery

- pid file: `.mudang/daemon.pid` (PID of running daemon process);
- socket: `.mudang/daemon.sock` (Unix-domain socket);
- CLI checks pid file → verifies process alive → uses socket;
- if pid file exists but process is dead, CLI cleans up and falls back
  to in-process.

### 13.2 Startup

```
mudang daemon start
   │
   ▼
1. acquire workspace lock (.mudang/daemon.lock); fail fast if held
2. write pid file
3. open Unix socket; chmod 0600
4. open Composer (loads scope graph, opens LSP pool)
5. load persisted queue from .mudang/notify-queue.jsonl
6. start filesystem watcher if --watch-files
7. announce on socket; accept first client
```

### 13.3 Shutdown

```
SIGTERM | mudang daemon stop
   │
   ▼
1. stop accepting new connections
2. drain in-flight requests (within --graceful-timeout-seconds)
3. flush queue to disk
4. close LSP pool (sends shutdown to each server)
5. close socket; remove pid file
6. release workspace lock
7. exit 0
```

### 13.4 Queue persistence

Notify requests with `--async` are appended to
`.mudang/notify-queue.jsonl` before ack is returned. On daemon restart
the queue is replayed in arrival order.

```jsonl
{"id":"01J...","paths":["src/foo.rs"],"cascade":"full","source":"nvim","ts":"2026-05-10T12:34:56Z","status":"pending"}
{"id":"01K...","paths":["src/bar.rs"],"cascade":"graph","source":"git-hook","ts":"2026-05-10T12:35:01Z","status":"completed"}
```

Completed entries are compacted out periodically (`notify.compact_interval_seconds`).

### 13.5 Embedded vs detached daemon

- `mudang daemon start --foreground` — runs in foreground; useful for
  systemd, supervisord, Docker;
- `mudang daemon start` — detaches and writes pid file;
- in lib mode (§2.1), the daemon concept does not apply — the
  consumer process **is** the daemon for its own purposes.

---

## 14. Configuration

`.mudang/config.toml`:

```toml
[notify]
default_cascade           = "full"          # none | graph | full
queue_max_depth           = 10000
max_batch                 = 1000            # per single notify call
sync_default              = true
compact_interval_seconds  = 600
audit_log                 = ".mudang/notify-audit.log"

[notify.daemon]
socket_path     = ".mudang/daemon.sock"
pid_file        = ".mudang/daemon.pid"
watch_files     = false                     # opt-in filesystem watcher
graceful_timeout_seconds = 30

[notify.daemon.watcher]                     # only used when watch_files=true
debounce_ms     = 200
ignore_patterns = [".git/**", "target/**", "node_modules/**", ".mudang/**"]

[notify.cascade]
emit_events_for = ["reindex.completed", "graph.invalidated",
                   "cache.invalidated", "tier2.completed"]
                                            # subset of EventKind to emit
```

---

## 15. Security

### 15.1 Socket permissions

- Unix socket created with mode `0600`;
- only the user that started the daemon can connect;
- multi-user machines should not share a daemon across users (one per
  user-workspace pair).

### 15.2 Source tagging

Every notify call records `source` (defaults to `"cli"` /
`"library"` / `"daemon-watcher"`). The source string is opaque text;
the daemon uses it only for audit-log entries.

```jsonl
{"id":"01J...","source":"claude-agent","paths":["src/foo.rs"],"ts":"..."}
```

The audit log lives at `.mudang/notify-audit.log` and is append-only.

### 15.3 No remote network surface

The notify API is local-only by design. No TCP socket. No HTTP. If a
remote driver is needed, the user is expected to forward the Unix
socket explicitly (e.g. SSH socket forwarding, container volume
mount). The protocol does not authenticate; it relies on filesystem
permissions and process-level isolation.

### 15.4 Path sanitization

Paths submitted via notify are canonicalised. Symlinks resolve to
their targets. Paths outside the workspace root are rejected with
`out_of_root` rather than silently ignored.

---

## 16. What this API is not

- **Not a general-purpose pub/sub.** Only file-change-derived events
  flow through it. No arbitrary message passing.
- **Not a remote API.** Local sockets only (§15.3).
- **Not a replacement for LSP file notifications.** The composer
  **forwards** to LSP via `workspace/didChangeWatchedFiles` (step 6).
  LSP clients still need to be aware of changes through their own
  protocol.
- **Not a transaction log.** Audit log records what was notified, not
  how the graph evolved. Graph history belongs to the database (or
  git, for source files).
- **Not a place for cascading user-defined hooks.** Subscribers
  receive events; they do not get to mutate them or veto the cascade.

---

## 17. Relation to other docs

- **`docs/ROADMAP.md`** — when this API ships (phase C, sub-track C.2).
- **`docs/ARCHITECTURE.md`** — §4 the unified `file_changed` event;
  §3 composer surface; §5 LSP basic-RPC; §3.4 daemon mode.
- **`docs/SCOPE-LSP-COMPOSITION.md`** — §1.2 modes (esp. mode 4
  enrich-offline); §6 cache model; §14.5 Case AA (tier 2 spec).
- **`docs/SUBSTRATE-PRIMARY.md`** — §3.4 warmup (uses daemon);
  §7.2 stale index mitigation.
- **`docs/todos/0004-onnx-and-lancedb-distinction.md`** — tier 2
  embedding pipeline.
- **`docs/todos/0005-delete-scope-watcher.md`** — the deletion this
  API enables.
- **`docs/todos/0007-composer-crate.md`** — the crate that hosts
  this API.
