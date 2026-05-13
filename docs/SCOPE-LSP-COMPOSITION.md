# Scope + LSP Composition

How `mudang` combines the syntactic engine (`gumiho-mudang-scope`) with
Language Server Protocol clients (`gumiho-mudang-lsp`) to deliver
answers neither layer can produce alone.

This document is the design contract for the composition layer. It does
not amend `gumiho-mudang-scope/docs/CHARTER.md` — Scope's hard limits
remain. Mudang is the **orchestrator** that delegates the work Scope is
forbidden from doing to an LSP server when one is reachable.

---

## 1. Operating principle

Each layer is fixed to what it is good at. Mudang routes queries to the
right layer, then merges results with explicit provenance.

| Layer | Strengths | Weaknesses |
|-------|-----------|------------|
| **Scope** | Polyglot single graph, recursive traversal (impact / trace / flow), cross-language queries, batch over millions of symbols, no toolchain required, tolerant of broken source, persistent and committable, token-budgeted output | Syntactic only — no type inference, fuzzy `to_id` text matching, method overloads collapse, generics not instantiated, macro bodies opaque, dynamic dispatch invisible |
| **LSP** | Semantic resolution, method dispatch, generic instantiation, trait bound checking, lifetime / borrow (rust-analyzer), inferred types, macro expansion, diagnostics, semantic rename, re-export chain following | Stateful daemon, slow cold start (rust-analyzer 5–30 s), requires the full toolchain, large RAM footprint, per-file granularity (no global recursive traversal), stalls on broken code, mono-language per server |

**The fixed rule:** Scope never grows into LSP territory; LSP never tries
to be the polyglot graph. Mudang is the place where both meet.

### 1.2 The five operating modes

Mudang operates in one of five distinct modes, depending on which layer
contributes data to a query. The four-level routing model in §2 covers
modes 1, 2, 3 explicitly. Modes 4 and 5 are described here so the full
picture is captured in one place.

| # | Mode | Engines involved | When |
|---|------|------------------|------|
| 1 | **Scope-only** | Scope alone | structural / batch / polyglot / broken-source / offline / no toolchain |
| 2 | **LSP-only** | LSP alone (Scope not consulted) | type-at, rename, diagnostics, hover, signatureHelp |
| 3 | **Composed online** | Scope + LSP at query time | refs / impact / sketch / implementers — each row tagged with provenance |
| 4 | **Enrich offline** | LSP enriches Scope's own data | Tier 2 embeddings, schema augmentation (§14.5) |
| 5 | **Scope as LSP load reducer** | Scope provides candidate set, LSP confirms | impact --strict, references at scale (§14.6) |

Modes 1, 2, 3 are **online** — per-query routing decided by the §2 level
table. Mode 4 is **offline** — index-time augmentation; no query latency
impact. Mode 5 is a structural pattern within mode 3 worth naming
separately because it inverts the typical "LSP enriches Scope" framing:
here Scope makes LSP tractable at scale by handing it a bounded
candidate set.

All five modes are implemented inside the **composer crate**
(`gumiho-mudang-composer`). Scope and LSP are pure libraries that the
composer orchestrates; neither knows about the other directly. See
`docs/ARCHITECTURE.md` §3 for the composer's role and
`docs/ROADMAP.md` for when each piece lands.

Mode 4 specifically (LSP enriches Scope offline) is wired into the
notify pipeline — every `file_changed` event triggers tier 2
re-enrichment for affected symbols. The complete spec lives in
`docs/NOTIFY-API.md` §9 alongside cascade levels and queue policy.

### 1.3 Internal Scope query surfaces

Scope itself is not monolithic. It exposes three internal query
surfaces, each optimised for a different kind of question.

| Surface | Storage | Query type | Used by |
|---------|---------|------------|---------|
| α — **Graph** | SQLite (`symbols`, `edges`) | exact lookup, recursive CTE | refs, impact, deps, trace, flow, sketch, summary |
| β — **FTS** | SQLite FTS5 (`symbols_fts`) | keyword BM25 | `find` (current) |
| γ — **Vector** | LanceDB (post-refactor) | ANN cosine | `find --semantic` (post-refactor) |

The composition layer treats Scope as one engine, but routing decisions
within Scope choose among α/β/γ based on query shape:

- "Who calls X?" → α
- "Find symbols whose name matches `auth*`" → β
- "Find code that does authentication" → γ
- "Both kinds of match for `auth`" → β ∪ γ (fused ranking)

LSP enrichment (mode 4) targets γ specifically — it augments the text
that becomes a vector. It does not touch α or β.

---

## 2. The four-level routing model

Mudang routes each query through one of four levels. The level depends
on what the query needs, what infrastructure is available, and what the
caller asks for.

```
┌──────────────────────────────────────────────────────────────┐
│  Level 0 — Scope only (no LSP touch)                         │
│  Structural, batch, cross-language, recall-heavy.            │
├──────────────────────────────────────────────────────────────┤
│  Level 1 — Scope first, LSP only for ambiguous edges         │
│  Default for interactive use when an LSP server is reachable.│
├──────────────────────────────────────────────────────────────┤
│  Level 2 — Scope + LSP joined, both contribute               │
│  Symmetric merge with provenance tags on every row.          │
├──────────────────────────────────────────────────────────────┤
│  Level 3 — LSP only (pass-through)                           │
│  Capabilities Scope's charter §5 forbids: rename, diagnostics│
│  hover types, semantic completions.                          │
└──────────────────────────────────────────────────────────────┘
```

### Auto-level selection

The default is **auto**. Mudang picks a level per query based on three
signals:

1. **Capability of the query** — does it semantically *require* LSP?
   (e.g. `mudang type-at` is always Level 3; `mudang map` is always
   Level 0).
2. **Reachability of an LSP server** — is the matching server running
   or launchable for this workspace's languages?
3. **Caller override** — `--scope-only`, `--require-lsp`,
   `--prefer-lsp`, `--prefer-scope` flags pin the level.

Decision table:

| Query needs … | LSP available? | Caller override? | Level |
|----------------|----------------|------------------|-------|
| Structural / graph only | n/a | n/a | 0 |
| Disambiguation of `status=ambiguous` edges | yes | none | 1 |
| Disambiguation | no | none | 0 (degraded — flag in output) |
| Semantic confirmation of refs / callers / impact | yes | none | 2 |
| Semantic confirmation | no | none | 0 (recall only, low-confidence flag) |
| Type info at position | yes | n/a | 3 |
| Type info at position | no | n/a | error: "LSP required for this query" |
| Anything | yes | `--scope-only` | 0 |
| Anything | yes/no | `--require-lsp` | error if LSP unavailable |

### Auto-level transitions in one session

Auto level is per-query, not per-session. A single session can run
`mudang map` (Level 0) then `mudang refs Foo --strict` (Level 2)
without re-deciding global state.

---

## 3. Capability map — Scope's gaps mapped to LSP methods

Every row below is a Scope limit (charter §5, plus the issues surfaced
in the architectural audit). Each row names the LSP method that closes
it and the resulting composition level.

| Scope gap | LSP method | Composition level | Notes |
|-----------|-----------|-------------------|-------|
| `obj.method()` collapses every `method` regardless of receiver type | `textDocument/typeDefinition` on receiver + `textDocument/definition` on call | 1 (resolve ambiguous) or 2 (enrich refs) | Receiver type is the missing piece — Scope captures the syntax, LSP names the target |
| Method overload pick (same name, different signatures) | `textDocument/definition` at call site | 1 | Scope edge `to_id="charge"` becomes ID-resolved |
| Generic instantiation context (`Vec<i32>` vs `Vec<String>`) | `textDocument/hover` | 3 (point query) or 2 (enrich sketch) | Scope cannot model; LSP knows from inference |
| Trait bound satisfaction (`T: Send + Sync` chain) | `textDocument/implementation` + hover | 2 (enrich rdeps) | LSP traverses constraint chain |
| Inferred return type when annotation missing (`fn foo() { ... -> i32 }` with no `-> i32`) | `textDocument/inlayHint` | 2 (enrich `sketch`) | Annotated with `(inferred)` provenance |
| Macro body content (after expansion) | rust-analyzer `experimental/expandMacro` | 3 | Outside charter; pass-through only |
| Re-export chain (`pub use crate::a::b::C`) crossing files | `textDocument/definition` follows the chain | 1 + 2 | Scope post-refactor handles trivial cases; LSP nails the chain |
| Cross-file dynamic dispatch (`Box<dyn Trait>.method()` → concrete impl reachable at call site) | `textDocument/implementation` from the trait method | 2 | Scope cannot; LSP enumerates impls |
| Blanket impls (`impl<T: Foo> Bar for T`) | `textDocument/implementation` | 2 | Scope reads only literal `impl Bar for Type` |
| Conditional types TS (`T extends U ? A : B`) | `hover` | 3 | tsserver evaluates |
| Mapped types TS (`{[K in keyof T]: ...}`) | `hover` | 3 | tsserver evaluates |
| Diagnostics (does this file compile?) | `textDocument/publishDiagnostics` | 3 | Charter §5 forbids inside Scope; mudang exposes |
| Semantic rename (atomic, exact reference set) | `textDocument/rename` + `workspace/willRenameFiles` | 3 | Out of Scope; mudang as thin shim |
| Inferred function param type (Python without annotations, TS with widening) | `inlayHint` | 2 (enrich `sketch` / `summary`) | LSP fills the blank |
| Decorator factory return type tracking | `hover` on the decorator name | 2 | Useful for Python framework edges |
| Lifetime / borrow violations | rust-analyzer diagnostics | 3 | Pure compiler territory |

Conversely — capabilities Scope owns and LSP cannot answer:

| Scope-only capability | Why LSP cannot do it |
|------------------------|----------------------|
| Cross-language query (`flow ReactComponent DjangoView`) | LSP servers are per-language; no shared graph |
| Recursive impact / trace / flow with bounded depth | LSP `callHierarchy` is one step at a time; no path materialization |
| Federated workspace (multiple `.scope/` graphs joined query-time) | LSP is mono-project |
| `mudang map` (architectural overview) | LSP has no entry-points-by-fanout primitive |
| Domain edges (`http_route`, `queue_handler`, `orm_relation`) | LSP returns semantic types, not framework patterns |
| Token-budgeted output for LLM agents | LSP returns IDE-shaped JSON-RPC |
| Index over broken / mid-refactor source | LSP stalls |
| Offline / sandboxed runners with no toolchain | LSP requires the compiler installed |
| Persistent committable index across machines | LSP has no on-disk index portable to other developers |
| `mudang diff --ref main` (git-aware structural diff) | LSP is atemporal |

---

## 4. Real query cases

This section walks through concrete searches an LLM agent or developer
actually performs, showing the per-level behaviour, the merge logic,
and the provenance of every row.

### Case A — "Where is `processPayment` called?"

User goal: list every call site of `processPayment` to assess blast
radius before changing its signature.

**Level 0 — Scope only**

```text
$ mudang refs processPayment
processPayment (function in src/payments/service.ts:42)
  calls
    src/orders/checkout.ts:118  OrderController.checkout
    src/orders/refund.ts:55     processRefund
    src/jobs/retry_payment.ts:34  retryJob              [status=ambiguous]
    src/webhooks/stripe.ts:201   handleStripe          [status=ambiguous]
  total: 4 (limit 100)
```

Scope finds all four via `to_id LIKE '%::processPayment' OR
'%.processPayment'`. Two are marked `ambiguous` because two functions
in the workspace share the name `processPayment` (one in `service.ts`,
one in `legacy/payments.js`) and Scope cannot tell which the call site
targets.

**Level 1 — Scope first, LSP for the ambiguous rows**

Same query with `--require-lsp` or auto when LSP is reachable:

```text
$ mudang refs processPayment
processPayment (function in src/payments/service.ts:42)
  calls
    src/orders/checkout.ts:118  OrderController.checkout   [scope+lsp]
    src/orders/refund.ts:55     processRefund              [scope+lsp]
    src/jobs/retry_payment.ts:34  retryJob                 [scope+lsp]
    src/webhooks/stripe.ts:201   handleStripe              [scope only — lsp says legacy/payments.js::processPayment]
  total: 3 (calls into src/payments/service.ts::processPayment)
        1 (excluded — different target)
```

Mudang's flow for each `ambiguous` row:

1. Scope provided `(file, line)` of the call site.
2. Mudang sends `textDocument/definition` to tsserver at that
   position.
3. tsserver returns the declaration site of the actual `processPayment`
   referenced.
4. If the returned declaration matches the queried symbol's location →
   promote `status: Resolved`, `producer: lsp_resolution`. Cache.
5. If it does not match → exclude from the result, but list separately
   under "excluded" so the user sees what Scope's recall picked up.

**Level 2 — Symmetric merge**

```text
$ mudang refs processPayment --semantic
processPayment (function in src/payments/service.ts:42)
  calls
    src/orders/checkout.ts:118  OrderController.checkout    [scope+lsp]
    src/orders/refund.ts:55     processRefund               [scope+lsp]
    src/jobs/retry_payment.ts:34  retryJob                  [scope+lsp]
    src/admin/dynamic_dispatch.ts:67  processViaRegistry    [lsp only]   ← reflective call Scope missed
  total: 4
```

The LSP-only row appears because the call is dynamic
(`registry[name](args)`) — Scope's edge query never captured it, but
LSP traced the registry table and found a definite reach.

### Case B — "What breaks if I change `PaymentConfig`?"

User goal: blast-radius analysis.

**Level 0**

Scope walks the reverse-dependency CTE.

```text
$ mudang impact PaymentConfig --depth 3
PaymentConfig (class in src/config/payment.ts:12)
  depth 1: 14 direct dependents
  depth 2: 38 transitive
  depth 3: 67 transitive
  test files: 22 (separated)
```

The 38 / 67 numbers include some over-counting because Scope's fuzzy
match treats every `to_id="PaymentConfig"` as a hit, including a
locally-shadowed identical name in a test fixture.

**Level 2 — LSP-confirmed impact**

```text
$ mudang impact PaymentConfig --depth 3 --strict
PaymentConfig (class in src/config/payment.ts:12)
  depth 1: 14 direct dependents              (14 confirmed, 0 false)
  depth 2: 36 transitive                     (36 confirmed, 2 dropped — name collision in tests/fixtures/)
  depth 3: 64 transitive                     (64 confirmed, 3 dropped)
  test files: 22
```

The flow:

1. Scope produces the candidate set via reverse-dependency CTE.
2. For each candidate edge, mudang asks LSP for the actual referenced
   symbol at that call site.
3. Mismatches are dropped from the count but listed under
   `--show-dropped` for inspection.
4. The `--strict` flag means only LSP-confirmed entries count toward
   the total.

When LSP is unavailable, `--strict` is a hard error rather than a
silent degradation, because the caller has explicitly asked for the
semantic guarantee.

### Case C — "Sketch this class with real return types"

User goal: get a class outline an LLM can read in ~180 tokens, with
inferred return types filled in.

**Level 0**

```text
$ mudang sketch PaymentService
class PaymentService (src/payments/service.ts:18)
  constructor(client: StripeClient, db: Db)
  processPayment(amount: Decimal, userId: string)
  validateCard(card)
  refund(txId, reason)
  audit(userId)
```

`processPayment`, `validateCard`, `refund`, `audit` have no return
type annotation — Scope cannot infer.

**Level 2 — LSP-enriched sketch**

```text
$ mudang sketch PaymentService
class PaymentService (src/payments/service.ts:18)
  constructor(client: StripeClient, db: Db)
  processPayment(amount: Decimal, userId: string): PaymentResult       [inferred]
  validateCard(card: CardInput): boolean                                [inferred]
  refund(txId: string, reason: RefundReason): Promise<RefundResult>     [inferred]
  audit(userId: string): void                                           [inferred]
```

The `[inferred]` tag tells the LLM "this type came from tsserver, not
from the source's annotation." If tsserver is unavailable, the column
falls back to blank with a one-line footer:
`note: inferred return types unavailable (no LSP)`.

The flow:

1. Scope produces the structural sketch.
2. For each method without an explicit `return_type` in metadata,
   mudang asks LSP `textDocument/inlayHint` at the function header.
3. Hints are spliced into the sketch fields, tagged.
4. Cached in `.scope/lsp-cache/inlay/{file_hash}/{position}.json`.

### Case D — "Which implementations does `Repository` have?"

User goal: enumerate trait / interface implementers, including blanket
impls and derived ones.

**Level 0**

```text
$ mudang refs Repository --kind implements
Repository (interface in src/db/repo.ts:8)
  implements
    src/db/sql_repository.ts:14  SqlRepository
    src/db/memory_repository.ts:9  MemoryRepository
```

Scope only sees literal `class X implements Repository` syntax.

**Level 2**

```text
$ mudang refs Repository --kind implements --semantic
Repository (interface in src/db/repo.ts:8)
  implements
    src/db/sql_repository.ts:14   SqlRepository                 [scope+lsp]
    src/db/memory_repository.ts:9  MemoryRepository             [scope+lsp]
    src/db/cached.ts:22            CachedRepository<T>          [lsp only — blanket impl: impl<T> Repository for Cached<T>]
    src/test/mock_repo.ts:5        MockRepository               [lsp only — derived via Mocked<T> macro]
```

The LSP-only rows are the strongest reason a developer needs the
semantic layer for refactor decisions. A signature change on the
interface will affect rows Scope alone cannot see.

### Case E — "Type at this position"

User goal: pure semantic query. No structural component.

**Level 3 — LSP pass-through**

```text
$ mudang type-at src/payments/service.ts:42:18
let result = this.client.charge(amount)
                              ^
type: Promise<ChargeResult>
defined: node_modules/stripe/index.d.ts:1234
```

Mudang adds nothing beyond what LSP returns — it just routes. The
command does not exist in Scope's surface; calling it without LSP is
a hard error.

### Case F — "Find code that handles authentication errors"

User goal: semantic intent search.

**Level 0 — Scope only (current)**

FTS5 BM25 over `name | signature | docstring | path | callers …`.
Hits things named `handle*Auth*Error*` or whose docstring contains
"authentication" — keyword-driven.

**Level 0 + post-refactor vector embeddings (still no LSP)**

ONNX-produced vectors stored in LanceDB. Cosine similarity to the
query embedding. Catches `validateToken`, `verifySession`,
`onUnauthorized` even when no word matches literally.

**Level 2 — Vector embeddings + LSP enrichment**

Same vector search, plus for the top N hits mudang asks LSP for the
hover signature to display real types in the result list. Useful when
the embedding text was compact and the user wants to see the actual
parameter / return shape before opening the file.

LSP does not improve the *ranking* — it improves the *display* of the
top hits.

### Case G — "Rename `legacyProcess` to `processLegacyOrder` everywhere"

User goal: atomic semantic rename.

**Level 3 — LSP only**

Scope cannot guarantee the exact reference set (charter §5 hard
limit). Mudang routes the entire request to LSP `textDocument/rename`,
applies the workspace edit, and reports the file count.

When LSP is unavailable, mudang refuses with: `rename requires LSP;
run with --scope-only --unsafe to perform a best-effort text replace
(NOT recommended).` The unsafe path explicitly does *not* claim
semantic correctness.

### Case H — "Trace from any API entry point to `chargeCustomer`"

User goal: understand which routes can reach this function.

**Level 0 — Scope only**

```text
$ mudang trace chargeCustomer
chargeCustomer
  ← PaymentService.processPayment
    ← OrderController.checkout
      ← POST /api/orders/checkout (http_route)
  ← BatchProcessor.run
    ← cron: hourly-charge (cron)
```

This is Scope's home turf — domain edges (`http_route`, `cron`) +
recursive backward traversal — and LSP cannot answer it natively.

**Level 1 — Scope with LSP cleanup**

Same trace, but every intermediate edge that was `status=ambiguous` in
Scope is resolved against LSP before path materialization. Result is
identical in shape, but with `[verified]` tags on each step and zero
spurious alternative paths through name collisions.

### Case I — "What does this macro expand to?"

User goal: see the body of a macro invocation.

**Level 3 — LSP pass-through (rust-analyzer only)**

Scope indexes macro definitions as `kind=macro` (post-R0) and macro
invocations as `references` edges. It never expands. For expansion,
mudang routes to rust-analyzer's `experimental/expandMacro` and
returns the expanded source verbatim.

### Case J — "Cross-language: which React components are rendered by a route that hits a Django endpoint?"

User goal: end-to-end flow across two language stacks.

**Level 0 — Scope only**

Polyglot graph with domain edges (`http_route` Django side, `renders`
React side, `imports` linking the API client module). Recursive flow
traversal returns the path.

**LSP cannot do this** — each LSP server is mono-language; no joined
graph exists. This case is the strongest argument for Scope and one
of the four moats from CHARTER §8.

---

## 5. The composition flow (sequence diagrams in text)

### 5.1 Default auto-flow for `mudang refs <symbol>`

```
caller
  │
  ▼
mudang::refs ─────────────┐
  │                       │
  │  1. resolve symbol    │
  ▼                       │
scope::find_symbol        │
  │                       │
  │  2. structural refs   │
  ▼                       │
scope::find_refs ─────────┤
  │                       │
  │  result set R         │
  │  - some status=Resolved
  │  - some status=Ambiguous
  │  - some status=Dangling
  ▼                       │
  branch: R contains      │
          ambiguous rows? │
   ┌──────┴──────┐        │
   no            yes      │
   │             │        │
   │             ▼        │
   │     lsp::available?  │
   │      ┌──────┴──┐     │
   │      no        yes   │
   │      │         │     │
   │      │         ▼     │
   │      │   for each amb. row:
   │      │     lsp::definition(file,line)
   │      │     match?
   │      │      ┌───┴───┐
   │      │      yes     no
   │      │      │       │
   │      │   promote  exclude
   │      │   status=  list under
   │      │   Resolved "dropped"
   │      │      │       │
   │      │      └───┬───┘
   │      │          ▼
   │      │   update graph (cache)
   ▼      ▼          ▼
emit result with provenance tags
```

### 5.2 Auto-level for `mudang impact <symbol> [--strict]`

```
caller
  │
  ▼
mudang::impact
  │
  │  scope::find_impact (recursive CTE, depth N)
  ▼
candidate set C
  │
  │  branch: --strict flag
  ┌──┴───┐
  no     yes
  │      │
  │      ▼
  │  lsp::available?
  │  ┌────┴────┐
  │  no        yes
  │  │         │
  │  │     for each candidate:
  │  │       lsp::definition at edge's (file,line)
  │  │       does it really point at the queried symbol?
  │  │         yes → keep with [scope+lsp]
  │  │         no  → drop, record under "dropped"
  │  │     loop
  │  │         │
  │  ▼         ▼
  │  error: "strict mode requires LSP"
  │
  ▼  ▼
emit grouped-by-depth, with provenance tags and dropped-list footer
```

### 5.3 Auto-level for `mudang sketch <symbol>`

```
caller
  │
  ▼
mudang::sketch
  │
  │  scope::sketch (struct overview, signatures from metadata)
  ▼
sketch S
  │
  │  for each method in S with no return_type in metadata:
  │    cache hit?
  │       yes → splice cached inlayHint
  │       no  → if lsp::available:
  │               inlayHint(file, line)
  │               splice + cache
  │             else:
  │               leave blank, add footer note
  ▼
emit sketch with [inferred] tags where LSP filled gaps
```

### 5.4 Merge algorithm

When mode 3 produces a Level 2 result, mudang executes one of two merge
algorithms depending on the query shape. The §1.2 verb sub-variants
(compose-merge / compose-backfill) map directly onto these algorithms.

#### 5.4.0 Scope's producer contract — multi-row Ambiguous

Composer correctness depends on Scope's row contract. Scope commits the
following producer-side guarantees (locked by
`ENFORCEMENT-MAP.md` § R3 → "Multi-row Ambiguous — scope's
producer commitment"):

- **One row per candidate target on `status='ambiguous'`.** When the
  resolver finds N candidate targets for a single source edge, it
  emits N rows — same `(from_id, kind, source_position)`, different
  `to_id` per candidate. The candidate set is preserved on disk as
  evidence; Scope never picks a tiebreak.
- **`confidence` is orthogonal to `status`.** A row's `confidence`
  reflects pattern precision (extractor output, immutable through
  resolution). A row's `status` reflects lookup outcome
  (`resolved` / `ambiguous` / `dangling`). Composer code must not
  treat low-confidence-ambiguous and high-confidence-ambiguous as
  the same signal.
- **Surrogate `edge_id` PK (R0).** Multi-row Ambiguous coexists with
  uniqueness because the PK is a surrogate, not the natural
  `(from_id, to_id, kind)` tuple. Composer reads MUST use `edge_id`
  for stable row identity; queries that join on
  `(from_id, to_id, kind)` may return multiple rows by design.
- **`dangling` rows are evidence too.** When Scope sees a call site
  but cannot find any candidate in the workspace, it emits one row
  with `status='dangling'`. The composer's LSP-enrichment pass
  promotes such rows when LSP resolves them (`compose-backfill`,
  §5.4.1) — Scope never drops the row at production time.

**Cleanest-signal filter shape.** Mudang consumers that want the
highest-confidence, unambiguous signal apply:

```sql
WHERE confidence = 'high'
  AND status = 'resolved'
```

Consumers that want the full candidate set (recall-heavy, audit, or
semantic-enrichment input) accept `status='ambiguous'` rows alongside,
and disambiguate via the merge algorithms below.

**Semantic enrichment lives in a separate table — `edge_enrichments`
(planned).** Background enrichment (LSP today; potentially type-checker
batch output, rust-analyzer JSON, AI annotation, or runtime trace in
the future) **does not mutate Scope's `edges` rows**. Instead, each
enrichment writes a row to a sibling `edge_enrichments` table that
references the underlying scope edge via FK:

```sql
edge_enrichments
  enrichment_id PK
  edge_id            FK → edges     -- which Scope row this enrichment is about
  evidence_kind      TEXT           -- 'semantic' (extensible: 'runtime-trace', 'ai-annotation', ...)
  evidence_source    TEXT           -- 'lsp:rust-analyzer', 'lsp:typescript-language-server',
                                    --   'type-checker:tsc', etc. — the TOOL is data, not column name
  outcome            TEXT           -- 'confirmed' | 'superseded' | 'rejected' | 'resolved-from-dangling'
  supersedes_with_edge_id  INTEGER  -- FK nullable → edges (when enrichment picks a different candidate
                                    --   row from the multi-row Ambiguous set produced by Scope)
  resolved_to_id     INTEGER        -- nullable; populated when enrichment resolves a dangling
                                    --   to a target that Scope could not see syntactically
  confidence         TEXT           -- enrichment's own confidence tier (orthogonal to edge.confidence)
  produced_at        INTEGER        -- unix-ts; cache invalidation per row, mirrors §6 LSP cache rules
```

**Why a separate table, not `lsp_*` columns on `edges`:**

- **Scope's row is pristine.** `edges` reflects only Scope's
  syntactic+structural verdict. R8 confidence audit measures
  scope-pattern precision without enrichment overlay polluting the
  math; R3 typestate (`InsertableEdge` is the sole `Insertable` and
  is constructed only inside `scope-graph::resolve`) stays airtight
  because enrichment is a second writer into a different table, not
  a second path into `edges`.
- **The semantic dimension is a category, not a tool.** Column
  naming carries semantics, not implementation. `lsp_` as a column
  prefix bakes today's tool name into the schema; tomorrow a
  rust-analyzer batch dumper, a type-checker JSON pipeline, or any
  other type-aware engine would either misuse the column or force a
  schema migration. With `evidence_kind` + `evidence_source` as data
  columns, new engines add rows, not schema.
- **Cache hygiene matches LSP cache (§6) one-to-one.** Each enrichment
  row carries `produced_at`; invalidation rules from §6 map directly
  onto deleting `edge_enrichments` rows by `(edge_id, evidence_source)`.
- **Charter §5 mechanical reinforcement.** Scope cannot grow into LSP
  territory because Scope's resolver has no write path into
  `edge_enrichments`. The two layers' outputs live in different tables
  by construction.

**Composer query shapes:**

```sql
-- cleanest scope-only signal
SELECT * FROM edges
 WHERE confidence = 'high' AND status = 'resolved';

-- scope-resolved OR semantically-confirmed
SELECT e.* FROM edges e
  LEFT JOIN edge_enrichments en
    ON en.edge_id = e.edge_id
   AND en.evidence_kind = 'semantic'
 WHERE e.confidence = 'high'
   AND (
        e.status = 'resolved'
     OR en.outcome IN ('confirmed', 'resolved-from-dangling')
   );

-- semantic supersession: enrichment picked a specific candidate from
-- Scope's Ambiguous set
SELECT e.* FROM edges e
  JOIN edge_enrichments en
    ON en.supersedes_with_edge_id = e.edge_id
 WHERE en.outcome = 'superseded';
```

The exact column types and DDL ship in the enrichment sprint (out of
scope for sprint 0003). This section locks the **shape contract** —
separate table, tool name as data, scope's `edges` rows never mutated
by enrichment.

**What Mudang must not do:**

- Collapse multi-row Ambiguous to a single row at read time without
  applying the merge algorithms below (the row choice carries
  provenance the caller may need).
- Demand a "scope picked one for me" API. Scope's commitment is
  candidate-set fidelity; tiebreaks are mudang's job, governed by §5.4.
- UPDATE `edges` rows from the enrichment pipeline. Enrichment writes
  to `edge_enrichments` only; `edges` is append-only relative to the
  scope indexer's own re-indexes.

#### 5.4.1 compose-backfill (Scope leads, LSP fills)

Used by: `refs --strict`, `impact --strict`, `sketch --semantic`,
`summary --semantic`, `explain`.

```
def compose_backfill(query):
    rows = scope.execute(query)             # full candidate set
    for row in rows:
        if row.status == "ambiguous" or query.wants_semantic_detail:
            lsp_data = lsp.query_at(row.file, row.line, query.lsp_method)
            if lsp_data:
                row.merge(lsp_data)
                row.provenance = "scope+lsp"
            else:
                row.provenance = "scope"     # degraded
        else:
            row.provenance = "scope"
    return rows
```

Properties:
- output cardinality is Scope's; LSP cannot add or remove rows;
- LSP can promote `status: ambiguous` → `status: resolved`;
- LSP can drop a row (moved to a separate `dropped` list) when it
  resolves to a different target than Scope claimed.

#### 5.4.2 compose-merge (symmetric union)

Used by: `refs --semantic`, `implementers`, `call-graph --strict`,
`hierarchy`.

```
def compose_merge(query):
    scope_rows = scope.execute(query)
    lsp_rows   = lsp.execute_equivalent(query)

    joined = {}
    for r in scope_rows:
        key = dedupe_key(r)
        joined[key] = {row: r, providers: {"scope"}}

    for r in lsp_rows:
        key = dedupe_key(r)
        if key in joined:
            joined[key].providers.add("lsp")
            joined[key].row = merge_fields(joined[key].row, r)
        else:
            joined[key] = {row: r, providers: {"lsp"}}

    return [tag_provenance(v) for v in joined.values()]
```

#### 5.4.3 Dedupe key

Primary tuple: `(file, line, column, target_id_or_name)`.

When `column` is absent in Scope's edge metadata (most current edges
pre-R0), the key falls back to `(file, line, target_name)`. Post-R0
schema upgrade adds `column` where tree-sitter captured it, sharpening
dedupe and reducing false merges.

#### 5.4.4 Conflict resolution

When Scope and LSP report different resolved targets for the same join
key:

| Situation | Policy |
|-----------|--------|
| Scope says target T₁ (fuzzy text match), LSP says T₂ (semantic) | LSP wins. Row tagged `[lsp-resolution]`. Scope's T₁ listed under `dropped`. |
| Both report the same logical target but at different declaration sites (re-export chain) | Keep LSP's canonical declaration. Record Scope's location as `redirect` field. |
| LSP returns a trait method, Scope a concrete impl (or vice versa) | Emit both rows, tagged `[trait-method]` and `[concrete-impl]`. Caller chooses which to follow. |
| LSP returns multiple candidates (overload set), Scope a single | Emit all. Tag the selected one with `[lsp-selected]` when `signatureHelp` confirms. |
| LSP says "no target found", Scope says T | Keep Scope's row tagged `[scope-only — lsp dangling]`. Probable cause: dynamic dispatch or LSP indexing lag. |

#### 5.4.5 Cardinality guards

`compose-merge` can return more rows than either layer alone. The total
is bounded by `scope_rows ∪ lsp_rows`. Mudang surfaces a warning when
the LSP-only contribution exceeds 30 % of the merged set — it usually
means Scope missed a pattern (dynamic dispatch, macro derivation,
reflective lookup) that deserves a §16 entry.

---

## 6. Cache model

LSP responses are expensive. Mudang caches them under
`.scope/lsp-cache/` (or `.mudang/lsp-cache/` after TODO 0001 lands).

```
.scope/lsp-cache/
├── definition/
│   └── {file_hash}/
│       └── {line}_{col}.json       # { uri, range, server, timestamp }
├── implementation/
│   └── {file_hash}/
│       └── {line}_{col}.json       # list of impl locations
├── inlay/
│   └── {file_hash}/
│       └── header_{line}.json      # inlay hints for one function header
├── hover/
│   └── {file_hash}/
│       └── {line}_{col}.json       # hover content
└── version
```

### Invalidation rules

1. **File hash key** — every cache entry is keyed by the SHA-256 of
   the source file (the same hash Scope uses in `file_hashes`). When
   Scope's incremental indexer detects a change, all cache entries
   under `{old_hash}` are evicted in the same transaction.
2. **Server version key** — `cache_root/version` records `{server,
   server_version}`. Any change invalidates everything.
3. **TTL fallback** — entries older than 30 days are evicted on next
   read regardless of hash.
4. **Cross-file invalidation** — when a file's exports change (Scope
   detects via diff in `symbols` rows of that file), entries that
   pointed *to* that file are evicted too.

### What is not cached

- `textDocument/diagnostics` — always live; the whole point is current
  state.
- `textDocument/rename` — one-shot side-effecting; no caching ever.
- `workspace/symbol` queries — too cheap to cache, too wide to key.

---

## 7. LSP server lifecycle

The **composer crate** (`gumiho-mudang-composer`) manages one LSP
server per language present in the workspace, via the basic-RPC pool
exposed by `gumiho-mudang-lsp`. The LSP crate itself owns the
transport (spawn / initialize / send / receive / shutdown); the
composer owns the lifecycle policy below. See `docs/ARCHITECTURE.md`
§3.4 and §5 for the crate-level split.

The scope-internal file watcher
(`gumiho-mudang-scope/src/core/watcher.rs`) is **deleted** as part of
this layering; file-change events flow through the composer's event
bus to both scope and LSP (see `docs/ARCHITECTURE.md` §4 and
`docs/todos/0005-delete-scope-watcher.md`).

```
workspace languages    →    LSP server selection
─────────────────         ────────────────────────
rust                   →    rust-analyzer
typescript / tsx       →    tsserver (typescript-language-server)
python                 →    pyright (or pylsp if configured)
go                     →    gopls
java                   →    jdtls
ruby                   →    ruby-lsp (or solargraph)
csharp                 →    omnisharp / csharp-ls
```

### Discovery

Each server's binary is looked up via:

1. Explicit path in `.scope/config.toml` `[lsp.<lang>]` section
   (`binary = "/usr/local/bin/rust-analyzer"`).
2. `PATH` lookup for the conventional binary name.
3. If neither found → server registered as unavailable; queries
   needing it degrade per Section 2.

### Spawn policy

- **Lazy** — server is spawned on the first query that needs it,
   not at startup.
- **Per-workspace** — one process per language per workspace root.
- **Idle teardown** — server is killed after 5 minutes idle to
   reclaim RAM (rust-analyzer can hold 4 GB on a large workspace).
- **`mudang lsp warm <lang>`** — explicit subcommand to spawn ahead
   of time, useful in CI before a batch of `--strict` queries.

### Capability negotiation

`initialize` exchanges capabilities. Mudang records what the server
supports, and queries that need a missing capability degrade. Example:
some servers do not implement `callHierarchy/incomingCalls` — for
those, mudang falls back to Scope-only impact analysis with a
note in the result footer.

### Failure modes

| Failure | Behaviour |
|---------|-----------|
| Server binary not found | Auto-level avoids it; `--require-lsp` errors with install instructions |
| Server crashes mid-query | Mudang retries once, then degrades to Scope-only for the query, marks `lsp_status: degraded` |
| Server timeout (default 30 s per request, configurable) | Same as crash |
| Server returns malformed JSON-RPC | Same as crash, plus a `tracing::warn!` line |
| Server claims the file is not indexed yet | Mudang waits up to 10 s, then proceeds without |

---

## 8. Provenance tags

Every row mudang emits carries one of these tags:

| Tag | Meaning |
|-----|---------|
| `scope` | Produced by Scope's graph; LSP not consulted (Level 0) |
| `scope+lsp` | Produced by Scope and confirmed by LSP (Level 1 or 2) |
| `lsp` | Produced by LSP, not present in Scope's graph (Level 2 or 3) |
| `lsp-resolution` | Originally `scope` with `status=ambiguous`; promoted to `Resolved` after LSP lookup |
| `dropped` | Was in Scope's result, LSP confirmed it targets a different symbol; excluded but listed |
| `degraded` | LSP was expected (Level 1 / 2) but unavailable; result is Scope-only |

JSON output:

```json
{
  "command": "refs",
  "symbol": "processPayment",
  "data": [
    {
      "file": "src/orders/checkout.ts",
      "line": 118,
      "context": "OrderController.checkout",
      "provenance": "scope+lsp",
      "lsp_resolved_to": "src/payments/service.ts::processPayment::function::42"
    },
    {
      "file": "src/admin/dynamic_dispatch.ts",
      "line": 67,
      "context": "processViaRegistry",
      "provenance": "lsp"
    }
  ],
  "dropped": [
    {
      "file": "src/webhooks/stripe.ts",
      "line": 201,
      "reason": "lsp resolved to legacy/payments.js::processPayment, not src/payments/service.ts::processPayment"
    }
  ],
  "lsp_status": "available",
  "level": 2
}
```

---

## 9. Configuration

`.scope/config.toml` (or `.mudang/config.toml` post TODO 0001) gains
a `[lsp]` section:

```toml
[lsp]
default_level = "auto"          # auto | scope-only | strict
idle_timeout_seconds = 300
request_timeout_seconds = 30

[lsp.rust]
binary = "rust-analyzer"        # optional override
init_options = { cargo = { allFeatures = true } }

[lsp.typescript]
binary = "typescript-language-server"
init_options = {}

[lsp.python]
binary = "pyright-langserver"
init_options = {}

[lsp.go]
# omit a section to disable a language entirely
```

CLI flags override config:

- `--lsp-level <0|1|2|3>` — pin the level.
- `--scope-only` — equivalent to `--lsp-level 0`.
- `--strict` — equivalent to `--lsp-level 2` with hard-error if LSP unavailable.
- `--require-lsp` — fail fast if no LSP server can answer.
- `--prefer-lsp` — prefer LSP-produced rows on conflicts (default for `--strict`).
- `--prefer-scope` — keep Scope's recall-heavy view.
- `--no-cache` — bypass LSP cache.

---

## 10. CI and offline behaviour

CI runners and offline sandboxes are first-class consumers. Mudang
behaves consistently in both:

- **No LSP installed** → auto-level collapses to 0 for every query.
  `lsp_status: unavailable` in JSON output. Exit status remains 0 for
  Level 0/1 queries; non-zero for `--require-lsp`.
- **Toolchain present but slow** → mudang exposes `mudang lsp warm
  <lang>` for CI scripts to pay the cold-start cost once before
  running queries.
- **Reproducible CI runs** → `--scope-only` is the recommended default
  in CI for queries that do not need semantic guarantees, because
  Scope is deterministic and LSP servers across versions are not.

---

## 11. What this document is not

- **Not an amendment to Scope's charter.** Scope's hard limits
  (`gumiho-mudang-scope/docs/CHARTER.md` §5) are untouched. Mudang
  delegates the forbidden-to-Scope work to LSP; it does not relax
  Scope's invariants.
- **Not a feature plan.** Implementation order is governed by
  Scope's `ENFORCEMENT-MAP.md` (Phase E must close first) and
  by `gumiho-mudang-lsp`'s own milestones.
- **Not a guarantee that every query exists today.** Many of the
  example commands in Section 4 (`mudang type-at`, `mudang refs
  --semantic`, `mudang rename`) are post-refactor work, queued behind
  Scope's Phase E acceptance and behind a separate
  `gumiho-mudang-lsp` rollout plan.

The intent is to fix the **division of labour** and the **decision
table for auto-routing**, so that the implementations land against a
shared mental model rather than ad-hoc per-subcommand judgement.

---

## 12. Open questions

These are deliberate gaps to revisit when implementation begins.

1. **Multi-root workspaces** — when a workspace has Rust + TS + Python
   members, mudang spawns three servers. Does `mudang impact` across
   the polyglot graph fan out to all three? Probably yes for nodes
   in each language, but cross-language edges have no semantic
   confirmation path. Resolution: tag cross-language edges
   `provenance: scope` regardless of LSP availability; document the
   limit.
2. **Server bootstrapping in agentic loops** — an LLM agent firing
   one query per second hits cold start on every server. Pre-warming
   policy under agent workloads needs measurement.
3. **rust-analyzer's `experimental/*` endpoints** — useful but
   unstable. Lock to a known set; refuse to pass through arbitrary
   experimental methods.
4. **LSP-side caching of mudang queries** — some servers cache
   workspace/symbol results; mudang's cache may shadow staler data.
   Resolution: rely on hash-based invalidation, not server
   timestamps.
5. **Provenance in workspace mode** — when scope-workspace.toml
   federates N projects, LSP cache must be per-project. Path
   collisions across projects need explicit namespacing.

When an open question is answered, move the resolution out of this
section and into the relevant numbered section.

---

## 13. Full LSP capability matrix

Comprehensive routing table for every LSP method mudang considers. Each
row records what mudang asks the method for, how the answer is merged
with Scope (compose / resolve / enrich / passthrough / skip), the
composition level, and the cache strategy.

Composition verbs:
- **compose** — both layers contribute (see sub-variants below).
- **resolve** — Scope provides a candidate, LSP disambiguates.
- **enrich** — Scope provides structure, LSP adds semantic detail.
- **passthrough** — LSP-only; mudang routes and tags.
- **skip** — not exposed via mudang (UX-only or out of scope).

#### Compose sub-variants

The `compose` verb covers two distinct merge patterns. They share
provenance handling but differ in cardinality and join logic. Section
5.4 specifies the algorithms.

- **compose-merge** (symmetric) — both layers contribute distinct rows
  and the output is the union. Example: `textDocument/references`
  returns LSP-only dynamic-dispatch rows; Scope returns its own
  structural rows; mudang emits the union with per-row provenance.
- **compose-backfill** (asymmetric) — Scope leads with the full
  candidate set; LSP fills in details (return types, inferred params,
  resolved receivers) per row. Example: `inlayHint` enriching `sketch`.
  Output cardinality is Scope's; LSP cannot add or remove rows.

In the routing column below the verb appears as `compose` without
distinguishing — the distinction matters at implementation time, not
when picking which method to call.

### 13.1 Navigation methods

| Method | Mudang uses it for | Composition | Level | Cache key |
|--------|---------------------|-------------|-------|-----------|
| `textDocument/declaration` | Forward declaration (C/C++ headers, TS ambient declarations) | resolve | 1 | `(file_hash, line, col)` |
| `textDocument/definition` | Resolve `to_id` text → exact symbol ID | resolve | 1 | `(file_hash, line, col)` |
| `textDocument/typeDefinition` | Receiver type for `obj.method()` disambiguation | resolve | 1 | `(file_hash, line, col)` |
| `textDocument/implementation` | Enumerate impls of trait / interface (blanket impls, derived impls) | enrich | 2 | `(file_hash, line, col)` |
| `textDocument/references` | Symmetric merge with `scope::find_refs`; LSP-only rows = dynamic dispatch, reflective lookups, derive-generated calls | compose | 2 | `(file_hash, line, col, includeDeclaration)` |
| `textDocument/documentHighlight` | Within-file usages; mudang prefers `find_refs` scoped to file but falls back here for languages without a plugin | enrich | 2 | `(file_hash, line, col)` |
| `workspace/symbol` | Fuzzy global symbol search; mudang prefers `scope find` (FTS5 + vector). LSP fallback only when both fail or for languages without a plugin | enrich | 2 | not cached (cheap) |

### 13.2 Hierarchy methods

| Method | Mudang uses it for | Composition | Level | Cache key |
|--------|---------------------|-------------|-------|-----------|
| `callHierarchy/prepare` | Anchor item for the in/out queries | passthrough | 3 | `(file_hash, line, col)` |
| `callHierarchy/incomingCalls` | One-step backward; mudang prefers `find_impact --depth 1` for recall, LSP confirms or backfills dynamic | compose | 2 | `(symbol_uri, version)` |
| `callHierarchy/outgoingCalls` | One-step forward; mudang prefers `find_deps --depth 1`, same compose pattern | compose | 2 | `(symbol_uri, version)` |
| `typeHierarchy/prepare` | Anchor for super / sub queries | passthrough | 3 | `(file_hash, line, col)` |
| `typeHierarchy/supertypes` | Chain to parent classes / traits; Scope has `is_a` edges but no constraint propagation — LSP closes the loop | enrich | 2 | `(symbol_uri, version)` |
| `typeHierarchy/subtypes` | All subclasses transitive; Scope has direct edges, LSP gives the full lattice including blanket / derived | enrich | 2 | `(symbol_uri, version)` |
| `textDocument/moniker` | Cross-package identity (LSIF / monorepo); mudang prefers monikers for cross-project queries when available | resolve | 1 | `(file_hash, line, col)` |

### 13.3 Semantic detail methods

| Method | Mudang uses it for | Composition | Level | Cache key |
|--------|---------------------|-------------|-------|-----------|
| `textDocument/hover` | Inferred types, doc comments, deprecation tags | enrich | 2 or 3 | `(file_hash, line, col)` |
| `textDocument/signatureHelp` | Overload resolution at a call site; powers `explain-overload` | resolve | 1 | not cached (interactive only) |
| `textDocument/inlayHint` | Inferred return types, param types, generic instantiations | enrich | 2 | `(file_hash, range)` |
| `textDocument/inlineValue` | Debugger-context values; mudang skips (not static-analysis) | skip | — | — |
| `textDocument/semanticTokens/full` | Token classification (parameter / variable / type); used to refine `summary` and detect type-vs-value usages in ambiguous edges | enrich | 2 | `(file_hash, full)` |
| `textDocument/semanticTokens/full/delta` | Incremental tokens; mudang fetches full on stale, delta on warm | enrich | 2 | `(file_hash, previousResultId)` |
| `textDocument/foldingRange` | Block boundaries; mudang uses to scope `source <symbol>` slices to a tight range | enrich | 2 | `(file_hash)` |
| `textDocument/selectionRange` | Smart selection expansion; mudang skips (UX-only) | skip | — | — |
| `textDocument/documentSymbol` | Per-file symbol outline; mudang prefers Scope's outline (tree-sitter-driven, language-plugin-aware). LSP fallback only for languages without a plugin | resolve | 1 | not cached (cheap) |
| `textDocument/documentLink` | URL detection in comments / strings; mudang skips | skip | — | — |
| `textDocument/colorPresentation` / `documentColor` | UI-only; mudang skips | skip | — | — |

### 13.4 Diagnostics

| Method | Mudang uses it for | Composition | Level | Cache key |
|--------|---------------------|-------------|-------|-----------|
| `textDocument/publishDiagnostics` (push) | Subscribe and aggregate for `mudang health` | passthrough | 3 | never |
| `textDocument/diagnostic` (pull) | On-demand single-file diagnostics | passthrough | 3 | never |
| `workspace/diagnostic` | Workspace-wide pull; mudang batches for `health --workspace` | passthrough | 3 | never |

### 13.5 Editing methods

| Method | Mudang uses it for | Composition | Level | Cache key |
|--------|---------------------|-------------|-------|-----------|
| `textDocument/rename` | Atomic semantic rename | passthrough | 3 | never |
| `textDocument/prepareRename` | Validate the cursor is on a renamable symbol before `rename` | passthrough | 3 | never |
| `textDocument/linkedEditingRange` | Linked rename ranges (JSX tag pair); mudang skips (UX-only) | skip | — | — |
| `textDocument/codeAction` | Quick fixes, refactors, source actions — exposed via `mudang codeaction` for agentic use | passthrough | 3 | never |
| `textDocument/codeLens` | Inline runnable indicators (run test, debug); exposed via `mudang runnables` | passthrough | 3 | `(file_hash)` short TTL |
| `workspace/applyEdit` | Server → client edit application; mudang accepts and applies atomically | passthrough | 3 | never |
| `workspace/willRenameFiles` / `didRenameFiles` | Coordinate file moves with refactors | passthrough | 3 | never |
| `workspace/willCreateFiles` / `willDeleteFiles` | Same family; surfaced via `mudang move-file`, `mudang delete-file` | passthrough | 3 | never |
| `workspace/executeCommand` | Server-defined commands (allowlisted per server — see §15) | passthrough | 3 | never |
| `textDocument/formatting` / `rangeFormatting` / `onTypeFormatting` | Mudang skips (not analysis) | skip | — | — |
| `textDocument/completion` / `completionItem/resolve` | Mudang skips (not analysis) | skip | — | — |

### 13.6 Command-to-method crosswalk

Quick reverse map: which user-facing `mudang` command uses which LSP
method as its primary call.

| Mudang command | Primary LSP methods |
|----------------|---------------------|
| `mudang refs --strict` | `references`, `definition` |
| `mudang impact --strict` | `references`, `definition` |
| `mudang sketch --semantic` | `inlayHint`, `hover` |
| `mudang summary --semantic` | `hover` |
| `mudang type-at` | `hover`, `typeDefinition` |
| `mudang implementers` | `implementation`, `typeHierarchy/subtypes` |
| `mudang supertypes` | `typeHierarchy/supertypes` |
| `mudang call-graph --strict` | `callHierarchy/{incoming,outgoing}Calls` |
| `mudang rename` | `prepareRename`, `rename`, `applyEdit` |
| `mudang codeaction` | `codeAction` |
| `mudang runnables` | `codeLens`, server commands |
| `mudang health` | `workspace/diagnostic`, `publishDiagnostics` |
| `mudang explain` | `hover`, `inlayHint`, `signatureHelp`, `documentSymbol` |
| `mudang explain-overload` | `signatureHelp`, `definition` |
| `mudang verify` | `definition` (mass), `references` (sample) |
| `mudang find-tests` | `references` + path-priority filter |
| `mudang dead-code` | `references` (confirm zero) |
| `mudang deprecation` | `hover` + Scope metadata |

---

## 14. Extended composition catalog (cases K–Z)

These cases extend Section 4. Together with §4 they form the complete
catalog of compositions mudang implements. Order is rough dependency
order — earlier cases supply building blocks for later ones.

### Case K — `mudang verify`: audit Scope's graph against LSP truth

Goal: sample-audit Scope's edges, report drift, and gate the build on
graph health.

**Level 2 — required**

```text
$ mudang verify --sample 500 --kind calls
sampled 500 calls edges
  confirmed   468 (93.6 %)
  drift        12 ( 2.4 %)   ← Scope's to_id resolved to the wrong target
  dropped      20 ( 4.0 %)   ← LSP says the call points elsewhere
  no-lsp-info   0

drift:
  src/payments/service.ts:42  "charge"
    scope:  src/payments/service.ts::charge::function::18
    lsp:    src/payments/stripe.ts::charge::function::104
  ...
```

Flow:
1. Sample N edges weighted by confidence tier (post-R0/R8).
2. For each, `textDocument/definition` at the edge's call site.
3. Compare LSP's location to Scope's resolved `to_id`.
4. Emit confirmed / drift / dropped histogram.

Run in CI on a sampled subset weekly. Failure threshold is configurable.

### Case L — `mudang test-impact`: tests affected by a change

Goal: enumerate tests whose execution graph reaches a symbol.

**Level 2 — required**

```text
$ mudang test-impact PaymentService --strict
PaymentService (class in src/payments/service.ts:18)
  direct test files (callers in tests/ or *_test.* or test_*.*):  6
  transitive test files (depth 3):                                14
  unique test functions:                                          47
  run command:
    pytest tests/payments/test_service.py tests/integration/...
```

Flow:
1. `find_impact PaymentService --depth N` candidate set.
2. Filter by path priority: `test/`, `tests/`, `*_test.py`, `*.test.ts`,
   `*Test.java`, `*_spec.rb`.
3. LSP `references` confirms the chain for `--strict`.
4. Framework-aware command emitter (pytest, jest, cargo, go test).

### Case M — `mudang api-surface`: module's public boundary

Goal: enumerate what crosses a module / crate's public boundary.

**Level 1 — required**

```text
$ mudang api-surface src/payments
public symbols (visible outside the boundary)
  function   processPayment     src/payments/service.ts:42
  function   refund              src/payments/service.ts:88
  class      PaymentService     src/payments/service.ts:18
  type       PaymentResult      src/payments/types.ts:5
  enum       PaymentError       src/payments/errors.ts:12
total: 5
unused outside boundary: 1   ← PaymentError — candidate for `pub(crate)` / internal
```

Flow:
1. Scope filters `symbols` by `visibility = "public"` + module-path prefix.
2. For each, `find_refs` with path filter "outside the module."
3. Zero-external-ref symbols flagged as narrowing candidates.
4. `--strict` adds LSP confirmation.

### Case N — `mudang dead-code`: unreferenced symbols

Goal: list symbols with no inbound `references` and no path to a known
entry-point family.

**Level 2 — required for honesty**

```text
$ mudang dead-code src/payments --strict
candidates:
  function   _legacyHash       src/payments/legacy.ts:14    [0 refs, no entry-point]
  function   debugDump          src/payments/util.ts:88      [0 refs, no entry-point]
warnings:
  function   onWebhook           src/payments/webhooks.ts:5  [0 refs, but entry=http_route — keep]
```

Flow:
1. Scope: symbols with no inbound edges.
2. Exclude known entry-point edge kinds: `http_route`, `cli_entry`,
   `cron`, `queue_handler`, `test`, `__module__`.
3. LSP `references` confirms zero usage (catches dynamic / reflective
   calls Scope missed).
4. Output `candidates` (zero refs after LSP) vs `warnings`
   (zero in graph but framework entry).

The command never deletes. It only lists.

### Case O — `mudang deprecation`: uses of `@deprecated` symbols

Goal: enumerate call sites of deprecated symbols, grouped by deprecated
target.

**Level 2 — required**

```text
$ mudang deprecation src/legacy
deprecated symbols in scope: 8
  function   oldCharge        src/legacy/payments.ts:14   [deprecated since v3.0]
    used at:
      src/orders/legacy_path.ts:55    OrderController.legacy_checkout
      src/admin/scripts/migrate.ts:120  migrateOrders
  ...
total deprecated usages: 23
```

Flow:
1. Scope identifies deprecated symbols via metadata (`#[deprecated]`,
   `@deprecated` JSDoc, `__deprecated__` Python, etc.) captured at index
   time post-R0.
2. For each, `find_refs` + LSP confirm.
3. Group output by deprecated symbol with version annotation from
   `hover` doc string when available.

### Case P — `mudang health`: workspace diagnostics summary

Goal: structured overview of compile errors / warnings across the
workspace.

**Level 3 — LSP only**

```text
$ mudang health
diagnostics summary:
  errors:     12  across   4 files
  warnings:   89  across  31 files
  hints:     142  across  47 files

top files by error count:
  src/payments/stripe.ts             5
  src/orders/refund.ts               4

top diagnostic codes:
  TS2345 (Argument type mismatch)    8
  TS6133 (Unused variable)          12
```

Flow:
1. `workspace/diagnostic` pull (or subscribe to `publishDiagnostics`
   for servers that only push).
2. Aggregate per file / per severity / per code.
3. Optional `--since main` filters to files changed since a git ref.
4. Never cached.

### Case Q — `mudang hierarchy`: type hierarchy (super + sub)

Goal: walk inheritance / trait-impl chain in both directions in one call.

**Level 2 — required**

```text
$ mudang hierarchy Repository
Repository (interface in src/db/repo.ts:8)
  supertypes:
    Pingable                src/db/contracts.ts:5            [scope+lsp]
  subtypes:
    SqlRepository           src/db/sql_repository.ts:14      [scope+lsp]
    MemoryRepository        src/db/memory_repository.ts:9    [scope+lsp]
    CachedRepository<T>     src/db/cached.ts:22              [lsp only — blanket impl]
    MockRepository          tests/mocks/repo_mock.ts:5       [lsp only — derived]
```

Flow:
1. Scope `is_a` edges in both directions to depth N.
2. `typeHierarchy/supertypes` and `typeHierarchy/subtypes` to surface
   LSP-only rows (blanket, derived).
3. Symmetric merge with provenance.

### Case R — `mudang call-graph`: full reachable graph

Goal: enumerate every reachable callee (or caller) up to depth N as a
flat list or tree.

**Level 2 — required for completeness**

```text
$ mudang call-graph processPayment --direction downstream --depth 5
processPayment
  ├─ this.client.charge       Stripe.charge                [lsp resolved]
  │  └─ this.client.api.post  StripeApi.post               [lsp resolved]
  ├─ this.db.insert            Db.insert                   [scope only]
  ├─ this.audit                PaymentService.audit        [scope only]
  │  └─ this.logger.info        Logger.info                [scope only]
  ...
total reachable: 47 nodes, 64 edges, max depth 5
```

Flow:
1. Scope `find_deps --depth N` BFS for downstream (or `find_impact` for
   upstream).
2. For each call-edge in the BFS, `callHierarchy/outgoingCalls` (or
   `incomingCalls`) adds dynamic-dispatch reachables.
3. Per-edge provenance tag.

Heavier than `impact` / `trace` — materializes every step, not just
paths to roots.

### Case S — `mudang since <ref>`: semantic diff vs git ref

Goal: structural / semantic delta between current state and a git ref.
Not a text diff.

**Level 2 — required for accuracy on renames / moves**

```text
$ mudang since main
symbols added:     7
symbols removed:   2
symbols renamed:   1
  processOrder → processOrderCheckout   src/orders/service.ts:42
signatures changed: 5
  PaymentService.processPayment
    before: (amount: Decimal, userId: string)
    after:  (amount: Decimal, userId: string, context: ChargeContext)
files changed: 14
```

Flow:
1. Scope indexes both `HEAD` and the ref (worktree or git-cat-file
   streaming).
2. Compare `symbols` rows by `(file, name, kind)`.
3. Suspicious add/remove pairs disambiguated by LSP `references` →
   rename detection.
4. Signature diff via `hover` in both states.

Costly; the comparator-side index is cached by commit SHA.

### Case T — `mudang triggers`: framework entry points to a symbol

Goal: enumerate external triggers that can reach a symbol.

**Level 0 — Scope home turf** (post-R5 framework-plugin work)

```text
$ mudang triggers chargeCustomer
chargeCustomer reachable from:
  http_route        POST   /api/orders/checkout       (Express)
  http_route        POST   /api/admin/recharge         (Express)
  cron              hourly-charge                      (node-cron)
  queue_handler     stripe-webhook                     (BullMQ)
  cli_entry         scripts/manual_charge.ts           (bin entry)
```

Flow:
1. `find_impact` to root.
2. Filter roots by edge kind in {`http_route`, `cron`, `queue_handler`,
   `cli_entry`, `test`}.
3. Group + emit per trigger family.

LSP is irrelevant — framework patterns are Scope's domain.

### Case U — `mudang generic-usage`: concrete types filling a generic

Goal: enumerate every concrete substitution observed in the workspace.

**Level 3 — LSP required**

```text
$ mudang generic-usage Repository
Repository<T> instantiations
  T = User           src/users/service.ts:42
  T = Order          src/orders/service.ts:88
  T = PaymentRecord  src/payments/service.ts:120
  T = (test) MockEntity   tests/mocks/entity.ts:14
total: 4 distinct T
```

Flow:
1. Scope identifies the generic definition.
2. For each `usage` edge to the generic, LSP `hover` at the call site
   extracts the substituted type.
3. Aggregate distinct substitutions.

Scope cannot do this — no type inference. LSP is the only path.

### Case V — `mudang explain <symbol>`: full context dump

Goal: one-shot agent-friendly context for a symbol.

**Level 2 — required for completeness**

```text
$ mudang explain processPayment
processPayment (function in src/payments/service.ts:42)

signature:
  async processPayment(amount: Decimal, userId: string,
                       context?: ChargeContext): Promise<PaymentResult>
                                                  ^^^^^^^^^^^^^^^^^^^ inferred

doc:
  Charge a payment via the configured client.
  Idempotent on (userId, amount, idempotencyKey).

callers (4):
  OrderController.checkout     src/orders/checkout.ts:118
  processRefund                src/orders/refund.ts:55
  retryJob                     src/jobs/retry_payment.ts:34
  processViaRegistry            src/admin/dynamic_dispatch.ts:67  [lsp only]

callees (8): client.charge, db.insert, audit, logger.info, …

impact (depth 3): 67 transitive, 22 in test files

deprecation: none

related tests: 14   (run with: mudang find-tests processPayment)

triggers (entry points):
  http_route   POST /api/orders/checkout
  http_route   POST /api/admin/recharge
  cron         hourly-charge
```

Composes Cases A, C, D, L, O, Q, R, T into one call. Single-shot
agent context; typical budget ~600 tokens.

### Case W — `mudang xref-monorepo`: cross-project references

Goal: in a federated workspace, find refs that cross project boundaries.

**Level 1 — Scope owns this, LSP optional**

```text
$ mudang xref-monorepo SharedTypes.Order
referenced in 3 projects:
  api/        12 refs                  [scope]
  worker/      5 refs                  [scope]
  admin-ui/    3 refs                  [scope]
total: 20 cross-project refs
```

Flow:
1. Workspace federation provides the joined graph.
2. `find_refs` returns rows tagged with their owning project.
3. Group by project.
4. LSP `moniker` (when servers support LSIF-style monikers) confirms
   cross-package identity for the supported languages.

LSP-only path is structurally impossible — each LSP server is
per-project.

### Case X — `mudang find-tests`: tests covering a symbol

Goal: per-test enumeration (not per-test-file as in Case L) suitable for
TDD loops.

**Level 2 — required**

```text
$ mudang find-tests processPayment
processPayment is covered by:
  direct (test functions calling it):
    test_process_payment_happy        tests/payments/test_service.py:42
    test_process_payment_idempotent   tests/payments/test_service.py:88
  indirect (test → caller → processPayment):
    test_checkout_flow                 tests/integration/checkout_spec.ts:14
    test_retry_on_failure              tests/jobs/retry_spec.ts:5
total: 4 tests
```

Flow:
1. `find_impact` filtered to test files (Case L).
2. Categorize: direct callers vs indirect.
3. LSP `references` confirms the chain for `--strict`.

### Case Y — `mudang explain-overload <pos>`: disambiguate at call site

Goal: at a specific call site, show which overload resolves and which
candidates lost.

**Level 3 — LSP required**

```text
$ mudang explain-overload src/payments/service.ts:42:18
this.client.charge(amount)
                   ^
selected overload:
  charge(amount: Decimal): Promise<ChargeResult>
candidates not chosen:
  charge(amount: Decimal, currency: string): Promise<ChargeResult>
  charge(req: ChargeRequest): Promise<ChargeResult>
```

Flow:
1. LSP `signatureHelp` at the position.
2. Format selected vs candidate overloads.
3. `--show-rules` includes the server's disambiguation reasoning when
   available.

Pure pass-through.

### Case Z — `mudang symbols-since <ref>`: public-surface diff

Goal: API-level diff scoped to public surface, between current state
and a git ref.

**Level 0 — Scope only**

```text
$ mudang symbols-since main --public-only
added:
  src/payments/refund_v2.ts::refundV2::function       [public]
  src/payments/types.ts::RefundReason::enum            [public]
removed:
  src/payments/legacy.ts::oldCharge::function          [was public]
moved:
  src/orders/service.ts::processOrder::function →
    src/orders/checkout.ts::processOrder
```

Flow:
1. Scope reindexes the ref's tree.
2. Set difference on `symbols` filtered to visibility = public.
3. Move detection by fuzzy match on signature.

LSP not consulted — pure structural diff.

### Case AA — `mudang index --enrich-embeddings`: LSP-augmented embeddings

Goal: produce embedding vectors that capture semantics Scope's syntactic
text alone cannot reach. This is the canonical implementation of
**mode 4** from §1.2.

**Mode 4 — offline; no query-time level.**

Scope's default embedding text (`embedder::build_embedding_text`) is
syntactic only:

```text
function process | callers: ... | callees: authenticate, save_user
| signature: fn process(req: Request) | path: src/api/users.rs
```

LSP enrichment adds semantic facts the source does not contain literally:

| Source | LSP method | What enters the embedding text |
|--------|-----------|---------------------------------|
| Function without `-> T` annotation | `inlayHint` | Inferred return type |
| `obj.method(x)` | `hover` on receiver | Canonical receiver type |
| Generic call site | `hover` | Concrete substitution (`Vec<UserId>`) |
| `impl Trait for T` | `implementation` | Blanket + derived impls list |
| Trait bound `T: Send + Sync` | `hover` + `typeDefinition` | Constraint chain expanded |
| Local doc only | `hover` (full) | Upstream interface doc joined in |
| Symbol kind ambiguous (tree-sitter guess) | LSP authoritative `kind` | `method` vs `function` vs `constructor` |
| Deprecation flag | `hover` tags | `[deprecated v3.0]` token |
| Param / variable / type ambiguity | `semanticTokens` | Per-token classification |
| Local type `Decimal` | `typeDefinition` | Canonical FQN (`bignumber.js::BigDecimal`) |

Enriched text example for the same `process`:

```text
function process
| callees: authenticate (-> Result<User, AuthError>),
           save_user (-> Result<(), DbError>)
| sig: fn process(req: Request) -> Result<(), ApiError>   [inferred]
| param.req.type: HttpRequest<UserPayload>
| trait_bounds: req: Request + Send
| doc: "validates request and persists user" (from interface)
| path: src/api/users.rs
```

#### Pipeline

```
Symbol
  → scope::build_embedding_text_v1  (syntactic)        → vector_v1
                  ↓
  lsp::enrich_symbol(hover, inlayHint, implementation, semanticTokens)
                  ↓
  → scope::build_embedding_text_v2  (semantic-augmented) → vector_v2
                  ↓
  LanceDB store (two tables: vectors_v1_syntactic / vectors_v2_enriched)
```

#### Dual-tier storage strategy

Two LanceDB tables coexist:

| Tier | Population trigger | Latency budget | Fallback |
|------|---------------------|----------------|----------|
| v1 (syntactic) | every index / reindex | ms per symbol | always present |
| v2 (enriched) | post-index background batch | 50–500 ms per symbol (LSP-bound) | falls back to v1 when LSP unavailable |

Workspace stays queryable on v1 while v2 builds in background.

#### Query flow

```
mudang find "<intent>"
  → ANN on v2 (if present)        → results_v2
  → ANN on v1                      → results_v1
  → rank-fusion (RRF), de-dup by symbol_id
  → return top-N tagged embedding_tier: v1 | v2 | fused
```

Each result row carries `embedding_tier` so downstream consumers know
how the rank was produced.

#### Cache key

`(source_hash, model, dim, tier, lsp_server_version?)`. Tier v1 ignores
the LSP version; tier v2 must include it. A server upgrade triggers
re-embed of tier v2 only.

#### When to skip enrichment

- symbol importance tier `low` (zero callers, no public visibility) —
  skip; v1 is sufficient;
- symbol whose body did not change but a transitively referenced type
  did — re-enrich (caller-graph-driven invalidation);
- LSP cold start in flight — defer enrichment to next idle window;
- LSP returned an error or unknown — keep v2 absent for this symbol,
  caller-side falls back to v1.

#### Cost honesty

| Scale | Tier v1 indexing | Tier v2 indexing |
|-------|------------------|------------------|
| 1 k symbols | 1–3 s | 30–60 s |
| 10 k | 10–20 s | 5–15 min |
| 100 k | 100–200 s | 1–3 h (background) |

Tier v2 is a daemon, never a blocking index step.

#### Non-goals

- Not a replacement for ANN on v1. Both tiers coexist.
- Not a graph augmentation. Scope's `symbols` and `edges` tables remain
  unchanged. Only the embedding text is enriched.
- Not a re-implementation of `workspace/symbol`. It produces dense
  vectors at index time, not on-demand keyword matches.

### Case BB — Scope as LSP load reducer (mode 5 formalised)

Goal: make Level 2 queries tractable at workspace scale by letting
Scope generate the candidate set and constraining LSP to confirmation
only.

**Mode 5 — implementation pattern within Level 2.**

#### The problem

`mudang impact PaymentService --depth 3 --strict` without Scope as
candidate provider would require LSP to:

1. find references to `PaymentService` (1 LSP call);
2. for each reference, find its caller's references (N₁ LSP calls);
3. recurse to depth 3 (N₂ × N₁ LSP calls);
4. result: potentially thousands of LSP roundtrips, several minutes.

With Scope as candidate-set provider:

1. Scope CTE returns 67 transitive candidates in ms;
2. LSP confirms each of the 67 (67 calls, ~30 s with cold cache);
3. total: bounded, predictable.

#### Pattern signature

```python
result = lsp.confirm_set(
    candidates  = scope.find_recursive(query),
    lsp_method  = "definition" | "references",
    strict      = True,
)
```

#### Commands using this pattern

| Command | Scope provides | LSP confirms via |
|---------|----------------|-------------------|
| `impact --strict` | recursive `find_impact` candidate set | `references` per node |
| `refs --strict` | `find_refs` raw set | `definition` per edge call-site |
| `dead-code --strict` | symbols with zero inbound edges | `references` confirming zero |
| `verify` | sampled edges from graph | `definition` per call-site |
| `test-impact --strict` | impact candidates filtered to test paths | `references` per node |
| `call-graph --strict` | `find_deps` / `find_impact` BFS | `callHierarchy/{out,in}goingCalls` per node |
| `xref-monorepo` (mono-lang variant) | cross-project candidates | `moniker` or `references` |

#### Why this matters

LSP alone:
- has no recursive primitive (`callHierarchy` is one-step at a time),
- has no batch API (every confirmation is a separate JSON-RPC roundtrip),
- has no way to express "give me everything reachable in 3 hops."

Scope alone:
- has the recursion (CTE),
- has the batch traversal,
- has zero semantic confirmation.

Mode 5 makes the two complementary. Without it, Level 2 queries either
return inflated counts (Scope-only fuzz) or take impractically long
(LSP-only recursion).

#### Latency budget rule

Mudang refuses to execute mode 5 with candidate counts above a budget
threshold. See §18 for the full budget model.

```toml
[lsp]
strict_max_candidates         = 1000      # default
strict_latency_budget_seconds = 120
```

When the candidate set exceeds the budget, mudang either emits a
non-strict Scope-only result with a footer noting the budget was
exceeded, or (with `--allow-large`) proceeds at user cost.

---

## 15. Server-specific compositions

Endpoints outside the LSP spec that mudang exposes per server. These
power several Section 14 commands.

### 15.1 rust-analyzer experimental

| Endpoint | Mudang command | Purpose |
|----------|----------------|---------|
| `experimental/expandMacro` | `mudang expand-macro <pos>` | Full macro body expansion at a call site (Case I) |
| `experimental/parentModule` | `mudang parent-module <pos>` | Jump from a file to its `mod` declaration |
| `experimental/relatedTests` | `mudang find-tests` (rust path) | rust-analyzer's notion of related `#[test]` items |
| `experimental/runnables` | `mudang runnables` (rust path) | Enumerate `cargo test` invocations for a file / module |
| `experimental/viewHir` | `mudang view-hir <pos>` | Show rust-analyzer's HIR — diagnostic / advanced |
| `experimental/viewMir` | `mudang view-mir <pos>` | Show rust-analyzer's MIR |
| `experimental/syntaxTree` | `mudang syntax-tree <pos>` | rust-analyzer's parse tree (useful when Scope's tree-sitter disagrees) |
| `experimental/ssr` (structural search & replace) | `mudang ssr <pattern>` | rust-analyzer's structural rewrite — gated behind `--unsafe` |
| `experimental/moveItem` | not exposed | UX-oriented refactor |
| `experimental/recursiveMemoryLayout` | not exposed | Niche layout dump |

Allowlist policy: only the experimental endpoints listed above are
exposed. New ones require an entry in this table. Resolves Open
Question §12.3.

### 15.2 gopls workspace commands

| Command | Mudang command | Purpose |
|---------|----------------|---------|
| `gopls.tidy` | `mudang go tidy` | `go mod tidy` |
| `gopls.vendor` | `mudang go vendor` | Vendor deps |
| `gopls.run_tests` | `mudang runnables` (go path) | Run tests |
| `gopls.list_known_packages` | not exposed | Internal lookup |
| `gopls.regenerate_cgo` | not exposed | Niche |
| `gopls.gc_details` | not exposed | Build diagnostic |

Invoked via `workspace/executeCommand`. Allowlisted; arbitrary command
execution against an LSP server is refused.

### 15.3 typescript-language-server / tsserver

| Endpoint | Mudang command | Purpose |
|----------|----------------|---------|
| `_typescript.goToSourceDefinition` | `mudang go-to-source <pos>` | Skip type-only `.d.ts` and jump to implementation |
| `_typescript.findAllFileReferences` | `mudang refs --file-scope` | Within-file refs without LSP roundtrip per call |
| `_typescript.organizeImports` | `mudang ts organize-imports <file>` | Sort + remove unused |
| `_typescript.fixAll` | `mudang ts fix-all <file>` | Apply all available quick fixes |
| `_typescript.removeUnusedImports` | `mudang ts remove-unused-imports` | Subset of fixAll |

`goToSourceDefinition` is especially valuable for refactor work where
the regular `definition` lands on a `.d.ts` stub.

### 15.4 pyright commands

| Command | Mudang command | Purpose |
|---------|----------------|---------|
| `pyright.createtypestub` | not exposed | Generate stubs for untyped deps |
| `pyright.organizeimports` | `mudang py organize-imports` | Sort + group |
| `pyright.restartserver` | `mudang lsp restart python` | Recover from stale server state |

### 15.5 jdtls (Java)

| Command | Mudang command | Purpose |
|---------|----------------|---------|
| `java.organizeImports` | `mudang java organize-imports` | Standard |
| `java.applyRefactoringCommand` | not exposed | Too broad; use specific `codeAction` items |
| `java.project.refreshDiagnostics` | `mudang health --refresh` (java path) | Force a diagnostics pull |

### 15.6 Discovery and version pinning

Each server has a documented protocol-version baseline. Mudang refuses
to enable a server-specific endpoint if the live server's version is
below the pinned minimum.

```toml
[lsp.rust]
binary = "rust-analyzer"
min_version = "2024-01-01"          # date string for rust-analyzer

[lsp.typescript]
min_version = "4.5.0"

[lsp.go]
min_version = "0.15.0"
```

Capability negotiation already happens via `initialize`; the version
floor catches cases where the server claims a capability it does not
actually support reliably.

### 15.7 Per-language capability matrix

Where Level 2 actually delivers, per language. "yes" means
production-ready in the auto path; "partial" means the method exists
but with known reliability or latency issues; "no" means mudang falls
back to Scope-only for that capability.

| Capability | rust-analyzer | tsserver | pyright | gopls | jdtls | ruby-lsp | solargraph |
|------------|:-------------:|:--------:|:-------:|:-----:|:-----:|:--------:|:----------:|
| `definition` | yes | yes | yes | yes | yes | yes | partial |
| `typeDefinition` | yes | yes | yes | yes | yes | partial | partial |
| `implementation` | yes | partial | yes | yes | yes | partial | no |
| `references` | yes | yes | yes | yes | yes | partial | partial |
| `hover` (inferred types) | yes | yes | yes | yes | yes | partial | partial |
| `inlayHint` | yes | yes | yes | yes | partial | no | no |
| `callHierarchy/incoming` | yes | yes | partial | yes | yes | no | no |
| `callHierarchy/outgoing` | yes | yes | partial | yes | yes | no | no |
| `typeHierarchy/supertypes` | yes | yes | yes | partial | yes | no | no |
| `typeHierarchy/subtypes` | yes | yes | partial | partial | yes | no | no |
| `semanticTokens` | yes | yes | yes | yes | yes | yes | partial |
| `rename` | yes | yes | yes | yes | yes | partial | no |
| `prepareRename` | yes | yes | yes | yes | yes | partial | no |
| `codeAction` | yes | yes | yes | yes | yes | yes | partial |
| `workspace/diagnostic` (pull) | yes | partial | yes | yes | partial | yes | no |
| Cold start (s, large workspace) | 10–30 | 5–15 | 3–8 | 1–3 | 30–60 | 2–5 | 5–10 |
| RAM peak (GB) | 3–5 | 1–2 | 1–2 | 0.5–1 | 3–4 | 0.5 | 0.5 |

#### Language-specific caveats

- **rust-analyzer** — best Level 2 coverage of any server. Macro-derived
  impls fully expanded. `experimental/expandMacro` is the only path to
  see macro bodies. Cold start dominates first-use latency.
- **tsserver** — `implementation` recall is partial: TS uses structural
  compatibility, so "everything assignable" is technically infinite; the
  server returns the practically-implementing set. `goToSourceDefinition`
  (§15.3) is needed for refactor work to skip `.d.ts` stubs.
- **pyright** — `callHierarchy` partial because Python's dynamic
  dispatch and duck typing make incoming-call enumeration approximate.
  `typeHierarchy/subtypes` partial for the same reason.
- **gopls** — fast, low RAM. `typeHierarchy` partial because Go's
  structural-interface model means "all types implementing I" is a
  workspace search, slower than nominal-inheritance languages.
- **jdtls** — high RAM, slow cold start. Once warm, full Level 2.
  `workspace/diagnostic` partial — push-based in some versions.
- **ruby-lsp / solargraph** — weakest Level 2 coverage. Ruby's
  metaprogramming makes static resolution fundamentally partial.
  Mudang falls back to Scope-only for impact / refs in these languages
  more often. `--strict` mode warns explicitly when the active server
  is in the weak-coverage tier.

#### Auto-level fallback policy per language

When LSP is technically available but returns `partial` for a capability
the query needs, mudang:

1. emits the LSP-derived rows;
2. tags them `[lsp partial-reliability]`;
3. supplements with Scope's recall set tagged `[scope only]`;
4. records the per-server reliability tier in JSON output's
   `lsp_reliability` field.

This is distinct from §16's "sub-measured limits" — sub-measured covers
edge cases that affect all servers; §15.7 covers per-server reliability
tiers.

---

## 16. Sub-measured limits

Even with LSP available, several semantic categories are genuinely
fuzzy. Mudang does not treat LSP responses as ground truth. This section
catalogs known fuzz so the composition layer can degrade gracefully.

### 16.1 Dynamic dispatch resolution

`Box<dyn Trait>.method()`: rust-analyzer enumerates *all* impls of the
trait, not the runtime-resolvable one. The LSP-only rows in Case D
(`MockRepository`, `CachedRepository<T>`) are reachable in *some*
runtime configuration — not necessarily this one.

Mudang policy: tag LSP-only impl rows with
`[lsp only — runtime-conditional]` when the trait has > 3 impls,
signalling the row may not apply to every execution path.

### 16.2 TypeScript narrowing depth

`tsserver` flow analysis has bounded depth. Conditional types, deep
nested unions, and recursive mapped types may return `any` or `unknown`
from `hover` even when a human can prove the type. The fallback is the
literal annotation if any.

Mudang policy: when `hover` returns a type containing `any` and the
source has an explicit annotation, mudang emits both:
`hover: any | (annotated: number)`.

### 16.3 Inferred-field lag

In long-running tsserver sessions, field inference can lag behind file
changes. Mudang invalidates the LSP cache by file hash, but a *live*
query may hit a server that has not yet reprocessed the file. Symptoms:
`hover` returns the old field set after an edit.

Mudang policy: cache key uses the on-disk SHA. If the file is dirty in
the editor (mudang cannot detect this from CLI), both the cache and the
server may be stale. Output footer notes
`lsp_freshness: filesystem-hash` so the caller knows the basis.

### 16.4 Const evaluation

`rust-analyzer` partially evaluates const expressions. `tsserver`
similarly evaluates simple literal types. Neither does full const-eval;
complex `const fn` bodies are not concretized.

Mudang policy: `hover` results that contain `const fn` placeholder
syntax are emitted as-is with `[const-eval-incomplete]` tag.

### 16.5 Effect / capability inference

Not in the LSP spec. Effect systems (Roc, Koka) and capability tracking
(Scala 3 caps) live in server-specific endpoints when at all. Mudang
does not attempt to expose these — generic LSP methods do not surface
them.

### 16.6 Cross-server federation

LSP has no protocol for "ask Python LSP about a Rust symbol it imports
via FFI." This is a genuinely mudang-side concern. Cross-language edges
in Scope's graph carry no LSP confirmation; tagged `provenance: scope`
regardless of LSP availability. Resolves Open Question §12.1.

No future LSP-based confirmation path is planned for cross-language
edges — Scope is the only layer that sees the polyglot graph.

### 16.7 Stale server state

A server running for hours can fall out of sync after large file moves
or branch switches. Symptoms: `definition` returns the old location;
`references` misses recently added call sites.

Mudang policy:
- `mudang lsp restart <lang>` bounces explicitly.
- `mudang lsp status` reports last-`initialize` time per server.
- Per-request health check: if `definition` returns a location outside
  any indexed file, mudang treats the server as stale and retries once
  after a refresh.

### 16.8 Macro-generated symbols outside `expandMacro`

`#[derive(Serialize)]` generates `impl Serialize for Foo`.
rust-analyzer often resolves calls to derived methods via the macro
expansion path, but its `references` may or may not include callers of
the generated method.

Mudang policy: when `references` on a symbol whose parent is a
macro-derived impl returns suspiciously low counts, mudang adds a footer:

```
note: callers may be incomplete — symbol is in a macro-derived impl;
re-run with --expand-macros for rust-analyzer's full view
```

### 16.9 Servers disagreeing

Two servers serving overlapping files (tsserver + ESLint LSP,
rust-analyzer + clippy LSP) can return conflicting diagnostics and
definitions. Mudang surfaces one server per language at a time;
linter-class servers are out of scope.

### 16.10 Workspace symbol search recall

`workspace/symbol` is fuzzy and per-server: some servers (jdtls, gopls)
return excellent recall; others (tsserver) return narrow keyword
matches. Mudang does not rely on it for primary search — Scope's FTS5
plus post-refactor vector search is the primary path. `workspace/symbol`
is the fallback when both fail.

---

## 17. Decision tree (consolidated)

A single flow from input query to output mode + level + verb. Read
top-down; the first matching condition wins. This consolidates §1.2
(modes), §2 (levels), §3 (capability map), and §13 (per-method verbs).

```
INPUT: query Q
─────────────────────────────────────────────────────────────────────
1.  Does Q require semantic-only capability?
    (rename, type-at, diagnostics, hover, expandMacro, codeAction,
     signatureHelp, explain-overload)
    YES  → mode 2 (LSP-only), level 3.
           LSP unavailable: ERROR (or scope-only --unsafe variant for
           rename).
    NO   → continue.

2.  Is Q a pure structural / batch / polyglot query?
    (map, trace cross-language, flow cross-language, triggers,
     symbols-since, xref-monorepo, deps, structural diff)
    YES  → mode 1 (Scope-only), level 0.
           LSP not consulted regardless of availability.
    NO   → continue.

3.  Does Q request the public-surface or API-shape view?
    (api-surface, symbols-since --public-only, dead-code)
    YES  → mode 1 (Scope-only), level 0 default.
           With `--strict`: mode 5 (Scope candidates + LSP confirm),
                            level 2, verb compose-backfill.
    NO   → continue.

4.  Does Q have a recursive depth > 1 over a candidate set?
    (impact, deps, call-graph, test-impact)
    YES  → if `--strict`: mode 5 (Scope candidate set + LSP confirm),
                          level 2, verb compose-backfill.
                          Apply latency budget rule (§18).
           else:           mode 1, level 0 — recall-only.
    NO   → continue.

5.  Does Q ask for refs / implementers / callers at depth 1?
    (refs, implementers, hierarchy)
    YES  → if LSP available and (`--strict` or `--semantic` or auto):
           mode 3, level 2, verb compose-merge (§5.4.2).
           else: mode 1, level 0.
    NO   → continue.

6.  Does Q ask for structural detail with semantic enrichment?
    (sketch --semantic, summary --semantic, explain)
    YES  → mode 3, level 2, verb compose-backfill (§5.4.1).
           degraded: mode 1, level 0 with `[lsp unavailable]` tag.
    NO   → continue.

7.  Does Q ask for intent / vector search?
    (find, find --semantic)
    YES  → mode 1, level 0; query surface β + γ (§1.3).
           Optional LSP enrich-display for top-N (mode 3, level 2,
           compose-backfill on display fields only — ranking still
           Scope-driven).
           Cross-reference Case AA (§14.5) for tier 2 embeddings
           which DO affect ranking offline.
    NO   → continue.

8.  Is Q an index-time enrichment trigger?
    (`mudang index --enrich-embeddings`, scheduled tier-2 daemon tick)
    YES  → mode 4 (offline). No query-time level. §14.5.
    NO   → fall through.

9.  Default: mode 1, level 0.
```

### 17.1 Decision inputs

The tree consults four signals on each branch:

1. **Capability** — does Q semantically *require* LSP?
2. **Recursion** — is depth > 1?
3. **Reachability** — is the required LSP server up?
4. **Caller override** — `--strict`, `--scope-only`, `--require-lsp`,
   `--prefer-lsp`, `--prefer-scope`, `--allow-large`.

`--scope-only` at any branch forces mode 1 / level 0. `--require-lsp`
forces an error if LSP is not available for the chosen branch.
`--allow-large` bypasses the latency budget in §18.

### 17.2 Verb selection within mode 3

Once the tree lands on mode 3, the verb is determined by query shape:

| Query shape | Verb | §5.4 algorithm |
|-------------|------|-----------------|
| Scope provides the candidate set, LSP confirms each | compose-backfill | 5.4.1 |
| Both layers enumerate independently and the union is the answer | compose-merge | 5.4.2 |
| One side disambiguates one ambiguous field on each row | resolve | n/a (per-row LSP call inside backfill) |
| One side adds metadata fields without affecting cardinality | enrich | n/a (per-row LSP call inside backfill) |

---

## 18. Cost and latency budget

Mudang refuses to execute requests that would consume disproportionate
time. Budgets are configurable and surfaced in JSON output when applied.

### 18.1 Per-query budgets

| Phase | Default | Configurable as |
|-------|---------|-----------------|
| Scope CTE traversal | 5 s | `scope.cte_timeout_seconds` |
| LSP request (single) | 30 s | `lsp.request_timeout_seconds` |
| LSP batch (mode 5) | 120 s total | `lsp.strict_latency_budget_seconds` |
| LSP candidate ceiling (mode 5) | 1 000 candidates | `lsp.strict_max_candidates` |
| Tier 2 embedding (per symbol) | 500 ms | `embeddings.enrich_timeout_ms` |
| Tier 2 batch (background daemon) | 1 h per workspace | `embeddings.enrich_batch_budget_seconds` |

### 18.2 Budget-exceeded actions

When any budget is exceeded, mudang:

1. logs at `tracing::warn` level;
2. attaches a `budget_exceeded` footer to the result;
3. degrades to the next-lower mode where the answer is still meaningful;
4. exits non-zero only if `--strict` was requested.

### 18.3 Cost-aware auto-level

Auto-level (§2) consults the budget. When the candidate count exceeds
`strict_max_candidates`, mudang prefers mode 1 over mode 5 even with LSP
available, and surfaces:

```text
mode-downgrade: 5 → 1
reason: 4 213 candidates exceeds strict_max_candidates=1000
re-run with --allow-large to override (will take ~3.5 min at current LSP latency)
```

### 18.4 Cold-start handling

`mudang lsp warm <lang>` is the recommended path for batched workloads.
When auto-level encounters a cold server with > 10 s estimated cold
start and the caller did not request `--strict`, mudang downgrades to
mode 1 for that query and emits:

```text
note: lsp.rust cold; estimated 22 s. running scope-only.
re-run after `mudang lsp warm rust` for full Level 2.
```

### 18.5 Tier 2 embedding budget

Tier 2 enrichment (Case AA, mode 4) runs as a background daemon. It
respects:

- per-symbol timeout (`embeddings.enrich_timeout_ms`) — drop and tag
  symbol as `tier2_unavailable` on timeout;
- total batch budget (`embeddings.enrich_batch_budget_seconds`) — pause
  and resume on next idle window;
- LSP availability — pause when LSP becomes unavailable mid-batch;
  resume on next health check.

Tier 2 never blocks tier 1 queries.

### 18.6 Cross-reference

Budget actions cross-reference §5.4.5 (cardinality guards), §7
(failure modes), and §16.10 (workspace/symbol fallback) to keep
degradation behaviour consistent across the doc.
