# Cross-language stitching

How the composer joins edges produced by different language plugins
into end-to-end cross-language relationships (React `fetch` ↔ Rails
route, Celery enqueue ↔ Python worker, Kafka publish ↔ Kafka
subscribe, etc).

Companion to:

- `ARCHITECTURE.md` §3 — composer's public surface.
- `SCOPE-LSP-COMPOSITION.md` §4 Case J — "Cross-language: React
  component → Django view" — establishes the moat; this doc specifies
  the mechanism.
- `gumiho-mudang-scope/docs/CHARTER.md` §3.4 + §8 — polyglot single
  graph as moat (LSP cannot do this).
- `gumiho-mudang-scope/docs/FRAMEWORK-PLAYBOOK.md` §219 / R5 —
  `applies_to_languages` opt-in, framework predicate writes
  normalization metadata.
- `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` R0 — edge-kind
  whitelist (38 kinds) + `edges.args_text` column that this layer
  consumes. Shipped 2026-05-12.
  The original recommendation lives in
  [`docs/todos/0009-expand-domain-edge-kinds.md`](./todos/0009-expand-domain-edge-kinds.md)
  (status: absorbed by R0).
- `docs/todos/0007-composer-crate.md` — the crate that owns this code.

Phase: composer (phase C). Implementation depends on scope R0 landing
first (sprint 0001 close on `main`).

---

## 1. Purpose

Scope emits per-language edges. Each plugin runs inside its language
boundary (LANGUAGE-PLAYBOOK rule E2 forbids cross-language
interpretation). The graph is polyglot at storage but not at meaning —
a `http_call` row from a TypeScript file and a `route_handler` row
from a Ruby file are unrelated until something stitches them.

That "something" is the **composer's cross-lang stitcher**. It joins
edges by **anchor strings** (URLs, queue names, topic names, env var
names) carried in `edges.args_text` and resolves the equivalent
backend ↔ frontend pair.

This document specifies:

- which kinds of anchors are recognised;
- which edge-kind pairs participate per anchor;
- how `args_text` is normalised so the JOIN matches;
- how `metadata` written by framework plugins (`base_url`,
  `mount_prefix`, `version_prefix`) enters the algorithm;
- the confidence policy for tagging stitched edges;
- the failure modes when an anchor cannot be resolved.

It does **not** specify:

- type-level shape matching (DTO ↔ struct compatibility — that is LSP
  territory and is excluded from mudang's reach by design);
- runtime call tracing (out of scope, not a static analysis concern);
- cross-repository stitching (deferred behind `scope link`; see
  `POST-REFACTOR-PLAN.md`).

---

## 2. Where this lives

| Layer | Owns |
|---|---|
| Language plugin | Raw edge with `args_text` literal from AST. **Does not interpret.** |
| Framework plugin | `metadata.base_url` / `metadata.mount_prefix` / `metadata.method` / `metadata.version_prefix` on the relevant symbol or edge. Predicate stage only; still no cross-lang reach. |
| Scope (graph layer) | Storage of edges, args_text, metadata. No stitching. |
| **Composer** | **All stitching logic.** Reads scope rows, normalises, joins, emits synthetic cross-lang edges into its own derived view. |
| LSP | Never involved. Cross-lang stitching does not call any LSP method. |

The boundary is hard: scope refuses to write a `frontend_calls_backend`
row, because that would force scope to interpret cross-language
semantics, which violates the charter. The composer holds those rows
in its own table or computes them on demand per query.

---

## 3. Anchor types

A complete list of anchor strings the stitcher recognises. Each anchor
has a producer side, a consumer side, and a normaliser.

| Anchor | Producer edge kind | Consumer edge kind | Normaliser |
|---|---|---|---|
| HTTP endpoint | `http_call` (Tier 0 baseline) | `http_route` / `route_handler` (Tier 0) | URL + method (§4.1) |
| Client route declaration | `client_route` (Tier 1) | `http_route` (Tier 0) — when SPA mounts API client at compile time | URL + method |
| WebSocket channel | `websocket_handler` (Tier 1, client) | `websocket_handler` (Tier 1, server) | channel name (§4.2) |
| Background job | `runtime_task_spawn` / `green_thread_spawn` / `bg_job_enqueue` | `bg_job_handler` | queue + job class name (§4.3) |
| Pubsub topic | `event_publish` | `event_subscribe` | topic name (§4.4) |
| Env var | `env_read` | `env_read` (other side) | var name (literal) |
| DB table | `query_binding` | migration column declaration | table name + column name |
| GraphQL operation | `query_binding` (client) | `route_handler` of the GraphQL resolver | operation name |
| Feature flag | `flag_read` (future) | `flag_read` (future) | flag key (literal) |
| gRPC method | `http_call` (gRPC over HTTP/2) | `route_handler` (server stub) | service + method |

Anchor types not yet covered: file-based RPC (named pipes), shared
memory, hardware bus. Out of scope; revisit only on a real trigger
recorded in `FRAMEWORK-TRIGGERS.md`.

---

## 4. Normalisers per anchor

The normaliser is a pure function `(args_text, metadata) → canonical
form`. Both sides of the JOIN run the same normaliser. Equality of the
canonical form is the JOIN predicate.

### 4.1 URL + method

Inputs:

- `args_text` — captured literal from the AST (e.g. `"GET /api/users/:id"`).
- `metadata.method` — written by framework predicate when the method
  lives outside the URL literal (Rails `get`/`post`/`put` DSL, Axios
  `client.get(path)`).
- `metadata.base_url` — Axios `create({ baseURL })`, Django
  `path('api/', ...)` mount, etc.
- `metadata.mount_prefix` — Rails `scope '/api' do ... end`,
  React Router `<Route path="/v1">`.
- `metadata.version_prefix` — explicit version segment when the
  framework plugin treats it as identity (Rails Versionist, FastAPI
  versioning).

Canonical form:

```
<METHOD> <ABS_PATH_WITH_PARAMS_NORMALISED>
```

Algorithm:

1. Verb: `metadata.method` (preferred) → uppercase. If absent and
   `args_text` starts with a verb token, lift it. If still absent,
   set to `*` and flag confidence=low.
2. Absolute path: prepend `metadata.base_url` then
   `metadata.mount_prefix` then `metadata.version_prefix`. Each is
   optional; concatenate left-to-right and collapse repeated `/`.
3. Strip query string and fragment.
4. Strip trailing `/` unless the path is `/`.
5. Canonicalise template parameters: any segment matching `:name`,
   `{name}`, `${name}`, `<type:name>`, `:name?`, `*name` collapses to
   the literal token `:param`. The parameter name itself is **not**
   part of the canonical form, only its position.
6. Lowercase scheme + host if present (`http://api.example.com/x` →
   `http://api.example.com/x`); path stays case-sensitive (per RFC
   3986 §6.2.2.1).

Examples:

| Input (client) | Input (server) | Canonical form |
|---|---|---|
| `fetch("/api/users/${id}")` + axios baseURL `/v1` | Rails `get '/users/:id'` mounted under `scope '/api'` + Versionist `:v1` | `GET /api/v1/users/:param` (matches) |
| `client.post("/users", body)` | FastAPI `@app.post("/users")` | `POST /users` (matches) |
| ``fetch(`/users?id=${id}`)`` (query string) | Rails `get '/users'` | `GET /users` (matches; query stripped) |
| `fetch("/users/")` (trailing slash) | Rails `get '/users'` | `GET /users` (matches; slash stripped) |
| `fetch(routeFor(user))` (no literal) | Rails `get '/users'` | client side unresolvable → no stitch |

### 4.2 WebSocket channel

Canonical form: `<scheme>://<host>/<channel>` after the same
mount-prefix concatenation. Method is irrelevant (always `WS` /
`WSS`).

### 4.3 Background job

Canonical form:

```
<queue>:<job_class_name>
```

Inputs:

- `args_text` — enqueue call literal (`Sidekiq.perform_async`,
  `Celery.delay(...)`, `BullMQ queue.add(name, ...)`).
- `metadata.queue` — explicit queue name when the framework writes it
  separately (Sidekiq `sidekiq_options queue: :urgent`).
- Job class name from the symbol resolution on the consumer side.

Normalisation:

1. Lower-case queue name.
2. Class name kept as written (case-sensitive — class identity).
3. When queue is unspecified on either side, use `default`.

### 4.4 Pubsub topic

Canonical form: `<topic_namespace>:<topic_name>`. Namespace is the
broker prefix when the framework plugin distinguishes brokers (Kafka
cluster name, Redis db number, NATS subject root). Topic name stays
case-sensitive.

### 4.5 Env var, DB table, GraphQL operation, feature flag

Literal-equality normalisation only. Trim whitespace; case sensitivity
follows the underlying system (env vars are case-sensitive on POSIX;
SQL identifiers depend on quoting). Document the per-system rule in
the per-framework doc.

---

## 5. Algorithm

Single-pass JOIN per anchor type, executed against the scope SQLite
graph from the composer.

```rust
fn stitch_anchor(graph: &Graph, anchor: AnchorKind) -> Vec<StitchedEdge> {
    let producers = graph.edges(anchor.producer_kinds, anchor.producer_frameworks);
    let consumers = graph.edges(anchor.consumer_kinds, anchor.consumer_frameworks);

    let mut by_form: HashMap<CanonicalForm, Vec<&Edge>> = HashMap::new();
    for e in &consumers {
        if let Some(form) = anchor.normalise(e) {
            by_form.entry(form).or_default().push(e);
        }
    }

    let mut out = Vec::new();
    for p in &producers {
        let Some(form) = anchor.normalise(p) else { continue };
        for c in by_form.get(&form).into_iter().flatten() {
            out.push(StitchedEdge {
                producer: p.id,
                consumer: c.id,
                anchor: anchor.name,
                canonical: form.clone(),
                confidence: classify_confidence(p, c, &form),
                provenance: "composer:stitch".into(),
            });
        }
    }
    out
}
```

Composition-level shape: `Composer::flow` (and related cross-lang
commands listed in `ARCHITECTURE.md` §3.1) call `stitch_anchor` for
the relevant anchor types, then traverse the resulting edges as if
they were normal graph rows.

### 5.1 Caching

Stitch results are cached in `.mudang/composer-cache/stitch.sqlite`
keyed by:

- the set of source-file hashes that produced each side;
- the framework plugin version that wrote `metadata.base_url` /
  `mount_prefix` (incrementing these invalidates entries).

Cache invalidation is event-driven via the notify cascade
(`NOTIFY-API.md` §6): any file change that re-indexes an edge with a
relevant kind also drops the matching stitch entries.

---

## 6. Confidence policy

Every stitched edge carries a confidence tier. Callers (CLI / agent /
IDE) decide how to surface them.

| Tier | Condition |
|---|---|
| `high` | Both sides have **literal** `args_text` (no template variables in raw form), method matches, metadata for `base_url` / `mount_prefix` is present on either side that needs it. |
| `medium` | One side carries template parameters that canonicalise away (`:id` ↔ `${id}`); method matches; prefixes resolved. |
| `low` | Method missing on one side (canonicalised to `*`); or `base_url` resolved heuristically (e.g. axios client without an explicit `create`, default-relative URL); or one side's `args_text` was reconstructed by the framework predicate from non-literal AST (string concatenation collapsed). |
| `drop` | Either side's `args_text` is purely dynamic and the framework predicate could not resolve it. No stitched edge emitted. |

The composer **always** records the confidence; downstream output
(CLI table, JSON, agent payload) decides whether to filter. Default
output shows `high` and `medium` and hides `low` behind `--strict` /
`--all` flags.

---

## 7. Failure modes

| Mode | Behaviour |
|---|---|
| Dynamic URL with no resolution | Producer edge stays in the scope graph; composer emits no stitched edge; `mudang flow` reports "unresolved client call: `fetch(${BASE}/x)` at `web/api.ts:42`" so the user can decide whether to refactor for analysability. |
| Multiple matching servers (microservices) | All matches emitted; each carries the same `canonical` form and its own consumer reference. Caller distinguishes by `consumer.file` / `consumer.framework`. |
| Wildcard route on server (`/api/*`) | Treated as prefix anchor when the server-side framework predicate writes `metadata.wildcard = true`. Stitched edges from any matching client get confidence=`medium` (cannot be tighter without method+exact-path agreement). |
| Version mismatch (`/v1/users` vs `/v2/users`) | Different canonical forms after `version_prefix` concatenation → no match → no stitched edge. Surfaces as "unresolved" if user runs `--all`. |
| Trailing-slash mismatch | Normaliser strips trailing `/`; matches regardless. |
| Conflicting `metadata.method` (axios `client.get` body says POST) | Producer-side framework predicate is wrong; composer logs a warning, emits the stitch with the literal verb from `args_text`, confidence=`low`. |
| Two language plugins both claim the producer side (e.g. both TS and JS plugins fire on a `.tsx` file) | Indexer pre-filter (`applies_to_languages` per FRAMEWORK-PLAYBOOK §219) prevents duplicate emission; if duplicates occur, dedupe by `(file_hash, line, col)`. |

---

## 8. What scope must provide (post-R0)

The stitcher consumes only fields that exist after R0 ships
(sprint 0001 close on `main`; final whitelist + `args_text` per
[`docs/todos/0009-expand-domain-edge-kinds.md`](./todos/0009-expand-domain-edge-kinds.md),
absorbed into R0). Required:

- `edges.kind` — whitelist of 38 kinds includes every producer /
  consumer kind named in §3.
- `edges.args_text` — TEXT NULL, capped at 2 KB, with Mitigation 1
  (resolver-skip on fully-qualified targets) and Mitigation 2
  (`[truncated]` marker) per `ARCHITECTURAL-REFACTOR.md` R0.
- `edges.framework` — per-edge framework tag (`react`, `axios`,
  `rails`, `fastapi`, …) so the stitcher can scope to specific
  producer / consumer framework sets when an anchor demands it.
- `edges.producer` + `edges.pattern_id` — provenance, surfaced in
  stitched edges' debug output.
- `edges.confidence` — input to §6 policy.
- `symbols.metadata` JSON — `base_url`, `mount_prefix`,
  `version_prefix`, `method`, `queue`, `wildcard`. Optional fields;
  the stitcher copes with absence.

No new column is required for stitching itself. The composer's own
table (or on-demand projection) carries the synthesised cross-lang
rows.

---

## 9. Public API surface (composer)

Sketched here; final shape lives in `ARCHITECTURE.md` §3.1.

```rust
impl Composer {
    pub fn flow(&self, from: &str, to: &str, opts: FlowOpts) -> Result<FlowResult>;

    pub fn stitched_edges(
        &self,
        anchor: AnchorKind,
        opts: StitchOpts,
    ) -> Result<Vec<StitchedEdge>>;

    pub fn unresolved_anchors(
        &self,
        anchor: AnchorKind,
    ) -> Result<Vec<UnresolvedAnchor>>;
}

pub enum AnchorKind {
    Http,
    WebSocket,
    BgJob,
    Pubsub,
    EnvVar,
    DbTable,
    Graphql,
    FeatureFlag,
    Grpc,
}

pub struct StitchedEdge {
    pub producer: EdgeId,
    pub consumer: EdgeId,
    pub anchor: &'static str,
    pub canonical: String,
    pub confidence: Confidence,
    pub provenance: String,
}
```

`flow` consumes stitched edges transparently. Power users (audits,
inventories, "who calls this endpoint") use `stitched_edges` directly.
`unresolved_anchors` surfaces producer rows whose `args_text` could
not be normalised — material for refactor prompts and visibility into
the dynamic-URL gap.

---

## 10. Coverage estimates

Rough expectations once the full pipeline lands. Not a contract;
calibrate against real fixtures as scope-side plugins ship.

| Anchor | Estimated coverage |
|---|---|
| HTTP endpoint (literal URL + method) | ~95% |
| HTTP endpoint with template params | ~85% |
| HTTP endpoint with dynamic baseURL resolved by framework predicate | ~60% |
| Pubsub topic (literal) | ~95% |
| Background job (literal class + queue) | ~90% |
| Env var | ~100% |
| GraphQL operation name | ~80% (Apollo + graphql-ruby / -python predicates) |
| Shape / type compatibility | 0% (not in scope; LSP territory) |

The 60% baseline on dynamic baseURL is the headline gap. Improving it
is a framework-plugin task (better predicate inference), not a
stitcher task.

---

## 11. Non-goals

- Type-level cross-lang matching. Body shape ↔ struct shape comparison
  is outside mudang's reach and outside this layer's contract.
- Runtime call confirmation. Static stitching only; no instrumentation,
  no traces.
- Cross-repository / cross-`.mudang/` stitching. Deferred until
  `scope link` ships per `POST-REFACTOR-PLAN.md`.
- Schema-level diff (this is `mudang health` / `mudang verify`
  territory).
- Symbol rename across the stitched boundary. Phase E edit layer +
  LSP is responsible; this layer only exposes the join, not the
  semantics for safe rename.

---

## 12. Cross-references

- `ARCHITECTURE.md` §3 — composer surface (includes `flow`).
- `SCOPE-LSP-COMPOSITION.md` §4 Case J — original case statement.
- `SCOPE-LSP-COMPOSITION.md` §12.1 + §16.6 — cross-lang edges tagged
  `provenance: scope`; no LSP confirmation path. Stitched edges
  inherit `provenance: composer:stitch` and remain unconfirmed by LSP
  by design.
- `SUBSTRATE-PRIMARY.md` §4.5 — `mudang flow` daily-workflow combo.
- `NOTIFY-API.md` §6 — cascade invalidation for stitch cache.
- `gumiho-mudang-scope/docs/CHARTER.md` §3.4 + §8 — polyglot moat.
- `gumiho-mudang-scope/docs/FRAMEWORK-PLAYBOOK.md` §219 — predicate
  metadata pipeline + `applies_to_languages` pre-filter.
- `gumiho-mudang-scope/docs/LANGUAGE-PLAYBOOK.md` rule E2 — language
  plugins do not interpret cross-language semantics.
- `docs/todos/0007-composer-crate.md` — owning crate.
- `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` R0 — edge-kind
  whitelist (38 kinds) + `edges.args_text` column this layer consumes.
- `docs/todos/0009-expand-domain-edge-kinds.md` — historical
  recommendation (status: absorbed by R0).
