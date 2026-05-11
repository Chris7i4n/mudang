# 0009 — Expand domain edge kinds to maximize Rails / Tokio / Axum / React coverage

- **Status:** TODO (reopens scope's `ARCHITECTURAL-REFACTOR.md` R0 whitelist before R0 ships)
- **Decision:** the 14 net-new edge kinds proposed by R0 cover the first-tier patterns of common stacks but leave production-critical patterns (middleware, validation, error handlers, websocket, client-side routing, auth guards, async task spawn naming) on `calls` generic, where scope's value-add over LSP collapses.
- **Tracking:** _<scope-side issue / PR link to be added>_

---

## Goal

Maximize scope's ability to **model the working semantics** of four stacks the user actively maintains: **Rails, Tokio, Axum, React**. The kinds list in R0 must cover the patterns that consumers (LLM agents, mudang composer cases like `mudang triggers`, `mudang api-surface`, `mudang find-tests`) actually need to reason about.

Patterns that fall into the generic `calls` edge force consumers to re-derive structure from raw `Symbol.name` text — defeats the moat. The whitelist must close the gap **before R0 ships**, because expanding the `CHECK` whitelist after the migration costs another migration.

---

## Where this lives

- **scope-side**: `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` R0 whitelist + matching migration. The decision is scope's; this TODO is mudang's recorded position arguing for the expansion.
- **mudang-side**: every composition case that depends on filtering by edge kind (Case T `triggers`, Case M `api-surface`, Case X `find-tests`, Case J `flow`, Case W `xref-monorepo`) gets richer when the kind list distinguishes middleware from handler, validation from data, auth-guard from route, etc.

This is a **pre-R0 amendment** request, not a post-R0 follow-up. The cost of fixing later is another full migration pass.

---

## Current R0 whitelist (baseline)

21 kinds total in R0:

- **Existing** (7): `calls`, `imports`, `extends`, `implements`, `instantiates`, `references`, `references_type`.
- **New universal** (1): `contains`.
- **New domain** (13): `http_route`, `queue_handler`, `orm_relation`, `green_thread_spawn`, `renders`, `hook_use`, `inherits_from`, `migration`, `cron`, `feature_flag`, `awaits_on`, `channel_send`, `channel_recv`. (R0 ships with `green_thread_spawn` rather than the original `goroutine_spawn` — see "Concurrency taxonomy" below for the 4-kind split and rationale.)

---

## Per-stack gap audit

Status legend: ✅ covered cleanly · ⚠️ partial / friction · ❌ falls to generic `calls`, semantic lost.

### Rails

| Rails pattern | Today's coverage | Status |
|---------------|-------------------|--------|
| `get '/foo' => 'controller#action'` | `http_route` | ✅ |
| `resources :users` (RESTful 7 routes) | `http_route` × 7 | ✅ |
| `namespace :admin` / `scope '/v1'` | `calls` | ❌ — route mounting lost |
| `mount Engine => '/path'` | `calls` | ❌ — engine mount lost |
| `before_action :authenticate` | `calls` | ❌ — **middleware/auth gap** |
| `after_action`, `around_action` | `calls` | ❌ — middleware gap |
| `skip_before_action` | `calls` | ❌ |
| `rescue_from Exception, with: :handle_err` | `calls` | ❌ — **error handler gap** |
| `validates :email, presence: true` | none (metadata only) | ❌ — **validation gap** |
| `validates_with CustomValidator` | `calls` | ❌ |
| `scope :active, -> {…}` | `calls` | ❌ — query scope |
| Controller `< ApplicationController` | `inherits_from` / `extends` | ✅ |
| `belongs_to :user` | `orm_relation` | ✅ |
| `has_many :posts, dependent: :destroy` | `orm_relation` | ✅ (callback in metadata) |
| `has_one`, `has_and_belongs_to_many` | `orm_relation` | ✅ |
| ActiveJob `< ApplicationJob` + `perform_later` | `queue_handler` | ✅ |
| ActionMailer + delivery | `queue_handler` (analogy) | ⚠️ — mailer ≠ queue semantically |
| ActionCable channel (`Channel` subclass) | `inherits_from` only | ❌ — **websocket gap** |
| Rails Engine mounting | `calls` | ❌ |
| Concerns (`include Trackable`) | `references` | ✅ generic |
| `after_create :do_something` callback | `calls` | ❌ — **callback gap** (related to middleware) |
| Active Storage `has_one_attached :avatar` | `orm_relation` (loose) | ⚠️ |
| `helper_method :current_user` | `references` | ⚠️ — helper registration |
| `<%= render 'shared/header' %>` | `renders` | ✅ |
| Layouts | `renders` | ✅ |
| Migration files | `migration` | ✅ |
| `add_index :users, :email` | `calls` | ❌ — DB index gap |
| `add_foreign_key` | `references` | ✅ generic OK |
| Flipper / Rollout flags | `feature_flag` | ✅ |
| `sidekiq-cron` / `whenever` | `cron` | ✅ |
| `Rails.cache.delete(key)` | `calls` | ❌ — **cache gap** |
| Sidekiq middleware | `calls` | ❌ — middleware gap |
| Rake task definition | `calls` | ❌ — **rake_task entry-point gap** |
| Background jobs (delayed_job, GoodJob) | `queue_handler` | ✅ |

**Rails-blocking gaps**: middleware (action filters), validations, error handlers, websocket (ActionCable), callbacks, route mounting/namespacing, cache bindings, rake task entry points.

### Tokio

| Tokio pattern | Today's coverage | Status |
|---------------|-------------------|--------|
| `tokio::spawn(fut)` | `goroutine_spawn` (Go-named) | ❌ — **wrong kind**: tokio task is stackless coroutine, not green thread. Resolved by `runtime_task_spawn` (Tier 2) |
| `tokio::spawn_blocking(closure)` | `goroutine_spawn` | ❌ — same fix; metadata flag `blocking=true` distinguishes |
| `tokio::task::LocalSet` | `calls` | ❌ — `runtime_task_spawn` covers |
| `std::thread::spawn(closure)` (used inside tokio app) | `calls` | ❌ — **kernel thread gap**: `os_thread_spawn` (Tier 3) |
| `tokio::select! { … }` | `awaits_on` (parcial) | ⚠️ — branching async semantic lost |
| `tokio::join!`, `tokio::try_join!` | `awaits_on` × N | ⚠️ — parallel-await semantic lost |
| `tokio::pin!` | none | OK (control flow, not edge) |
| `tokio::time::sleep` | `calls` | ✅ |
| `tokio::time::interval` | `calls` | ⚠️ — periodic semantic could feed `cron` analogue |
| `tokio::time::timeout(d, fut)` | `awaits_on` + `calls` | ⚠️ — timeout wrap lost |
| `tokio::sync::mpsc` send | `channel_send` (Go-named) | ⚠️ |
| `tokio::sync::mpsc` recv | `channel_recv` (Go-named) | ⚠️ |
| `tokio::sync::broadcast` send/recv | `channel_send`/`channel_recv` | ⚠️ — broadcast ≠ mpsc semantically |
| `tokio::sync::oneshot` send/recv | `channel_send`/`channel_recv` | ⚠️ — one-shot ≠ streaming |
| `tokio::sync::watch` send/recv | `channel_send`/`channel_recv` | ⚠️ — watch (last-value) ≠ mpsc |
| `tokio::sync::Mutex::lock()` | `calls` | ✅ |
| `tokio::sync::RwLock` | `calls` | ✅ |
| `tokio::sync::Semaphore::acquire()` | `calls` | ✅ |
| `tokio::sync::Notify` | `calls` | ❌ — **notify/wait gap** |
| `tokio::signal::ctrl_c()` | `calls` | ❌ — **signal handler gap** |
| `tokio_stream::Stream` impl | `implements` | ✅ |
| `tokio::net::TcpListener::bind` | `calls` | ⚠️ — lower-level than `http_route`, no edge for "listener" |
| `tokio::process::Command` | `calls` | ❌ — **kernel process gap**: `os_process_spawn` (Tier 3) |
| `#[tokio::main]` | none | ❌ — **async entry-point gap** |
| `#[tokio::test]` | none | ❌ — async test entry-point gap |
| `tokio_util::sync::CancellationToken` | `calls` | ❌ — **cancellation gap** |
| `tokio_util::codec` | `implements` | ✅ |

**Tokio-blocking gaps**: concurrency-primitive taxonomy (current `goroutine_spawn` mislabels tokio task as green thread — resolved by 4-kind split below), channel-type distinction (mpsc/broadcast/oneshot/watch collapsed), signal handlers, async entry points, cancellation tokens, notify/wait primitives.

### Axum

| Axum pattern | Today's coverage | Status |
|--------------|-------------------|--------|
| `Router::new()` | none (no edge) | OK |
| `.route("/foo", get(h))` | `http_route` | ✅ |
| `.route("/foo", get(g).post(p))` | `http_route` × 2 | ✅ |
| `.nest("/api", api_router)` | `calls` | ❌ — **route mount/nest gap** |
| `.merge(other_router)` | `calls` | ❌ — route merge gap |
| `.layer(TraceLayer::new_for_http())` | `calls` | ❌ — **middleware gap (huge)** |
| `.layer(AuthLayer)` | `calls` | ❌ — **auth middleware gap** |
| `.layer(CompressionLayer)` | `calls` | ❌ |
| `.layer(CorsLayer)` | `calls` | ❌ |
| Extractors `Path<T>`, `Query<T>`, `Json<T>`, `State<T>`, `Extension<T>` | `references_type` | ✅ |
| Custom extractor (`impl FromRequestParts`) | `implements` | ✅ |
| `IntoResponse` impl | `implements` | ✅ |
| Error response chain (`Result<T, E>` + `IntoResponse` for `E`) | `implements` + `?` propagation | ✅ |
| WebSocket via `WebSocketUpgrade` extractor | `http_route` (parcial) | ⚠️ — **websocket lifecycle gap** |
| SSE (Server-Sent Events) | `http_route` (parcial) | ⚠️ — **SSE streaming gap** |
| Tower service composition | `implements` | ⚠️ — tower layer semantic distinct from generic `implements` |
| Static file serving (`ServeDir`) | `calls` | ✅ |
| gRPC via `tonic` Service impl | `implements` + `calls` | ❌ — **grpc handler gap** |
| `validator` crate integration | `calls` + `references_type` | ❌ — validation gap |
| OpenAPI generation (`utoipa`) | metadata | ⚠️ — derive macros emit registration |

**Axum-blocking gaps**: middleware (Tower `.layer` — *the* core Axum pattern), route mounting/merging, websocket handlers, SSE streams, gRPC handlers, validation bindings, tower layer distinction.

### React

| React pattern | Today's coverage | Status |
|---------------|-------------------|--------|
| Functional component definition | `kind=function` + metadata | ✅ |
| `<Foo prop={x} />` invocation | `renders` | ✅ |
| Child rendering | `renders` chain | ✅ |
| `useState`, `useEffect`, `useMemo`, `useCallback`, `useRef` | `hook_use` | ✅ |
| `useContext(Context)` | `hook_use` | ✅ |
| Custom hooks `useXxx()` | `hook_use` | ✅ |
| `useReducer(reducer, initialState)` | `hook_use` + `references_type` | ✅ |
| `Context.Provider` | `renders` | ✅ |
| `createContext()` | `calls` | ✅ |
| `forwardRef`, `memo()` | metadata flag | ✅ |
| `lazy(() => import('./Foo'))` | `calls` | ❌ — **lazy/dynamic import gap** |
| `Suspense` boundary | `renders` | ✅ |
| Error Boundary | `renders` + metadata | ⚠️ — error_handler conceptual but `renders` collapses |
| HOC `withSomething(Foo)` | `calls` | ❌ — **HOC wrap gap** |
| Render-prop pattern | `renders` + `calls` | ⚠️ complex |
| `createPortal(child, container)` | `renders` | ✅ |
| Redux/Zustand `createStore` | `calls` | ❌ — **store create gap** |
| Redux `useSelector(selector)` | `hook_use` + `references` | ❌ — **state selector gap** |
| Redux `useDispatch()` | `hook_use` | ✅ |
| Zustand `useStore(s => s.foo)` | `hook_use` + `references` | ❌ — state selector gap |
| React Query `useQuery({ queryKey, queryFn })` | `hook_use` | ❌ — **query/data fetch gap** |
| SWR `useSWR(key, fetcher)` | `hook_use` | ❌ — query gap |
| `<Route path="/foo" element={<Foo/>} />` | `calls` | ❌ — **client-side route gap** |
| `<Link to="/foo">` | `references` | ⚠️ — string path, no client_route binding |
| `useNavigate()` | `hook_use` | ✅ |
| react-hook-form `useForm()` + `register` | `hook_use` + `references` | ❌ — **form/validation gap** |
| Formik / Yup binding | `calls` + `references_type` | ❌ — validation gap |
| Component prop types | `references_type` | ✅ |
| Server Components (RSC) | metadata flag | ✅ |
| Next.js `getServerSideProps` / `getStaticProps` | `calls` | ❌ — **server data fetch gap** |
| Next.js page file routing (filesystem-based) | `http_route` (server side) + client_route gap | ⚠️ |
| `useTranslation()` (react-i18next) | `hook_use` | ✅ |

**React-blocking gaps**: lazy/dynamic imports, HOC patterns, client-side routes (React Router / Next.js page routing distinct from server `http_route`), store creation, state selectors, query/data-fetch bindings (React Query / SWR), form/validation bindings, Next.js server functions.

---

## Cross-stack gap summary

Five gaps appear in **3 or more** of the four stacks and break composition queries:

| Gap | Rails | Tokio | Axum | React | Composition impact |
|-----|:-----:|:-----:|:-----:|:-----:|---------------------|
| **Middleware / filter chain** | ✅ | — | ✅ | — | `mudang triggers` shows raw handler; misses "this route runs auth + logging before handler"; `mudang impact` on a middleware sees zero downstream |
| **Validation binding** | ✅ | — | ✅ | ✅ | `mudang api-surface` cannot say "this endpoint validates with Schema X"; LLM agent answering "what shape does this endpoint accept?" cannot use scope |
| **Error handler binding** | ✅ | — | ✅ | ✅ | `mudang trace` from error → handler skipped; rescue chain invisible |
| **WebSocket / streaming handler** | ✅ | — | ✅ | — | LLM agent asking "which routes are WS?" cannot distinguish from REST |
| **Client-side / SPA route** | — | — | — | ✅ | Cross-stack flow (Next.js page → API → Django) breaks at the client boundary; flow gives up |

Three more appear in **2** stacks:

| Gap | Stacks | Composition impact |
|-----|--------|---------------------|
| **Auth guard** | Rails, Axum | `mudang api-surface` cannot answer "which routes are public?" |
| **Cache binding** | Rails, React | Invalidation impact analysis impossible |
| **Concurrency-primitive taxonomy** | Tokio, React, Rails, Go (when added) | Tokio task ≠ green thread ≠ kernel thread ≠ kernel process — current `goroutine_spawn` collapses 4 operationally-distinct primitives. Resolved by 4-kind split: `os_process_spawn` / `os_thread_spawn` / `green_thread_spawn` / `runtime_task_spawn` |

Two are language/framework-specific but still common:

| Gap | Stacks | Notes |
|-----|--------|-------|
| **HOC / decorator wrap** | React | Different from `renders`; wraps the component, doesn't render it |
| **Store create + state select** | React | Redux/Zustand/Pinia/Vuex — data plane parallel to component plane |

---

## Proposed expansion (3 tiers)

### Tier 1 — must land in R0 (5 kinds)

Each addresses a gap that fires in **3 or more** of the target stacks. Without these, scope falls back to `calls` and composition cases lose the moat.

| Kind | Replaces today | Why must land |
|------|-----------------|----------------|
| `middleware` | `calls` | Middleware is the most common pattern not covered; appears in every web framework |
| `validates_with` | none | Validation binding is the prime LLM-agent question "what shape does this endpoint accept?" |
| `error_handler` | `calls` | Error handler ↔ exception type binding; impact-graph requires it |
| `websocket_handler` | `http_route` (parcial) | WS lifecycle differs from REST; cannot collapse into `http_route` |
| `client_route` | `calls` / `references` | Server (HTTP) vs client (SPA) routing distinction; powers cross-stack flow |

### Tier 2 — strongly recommended in R0 (5 kinds)

Each addresses a gap that fires in **2** of the target stacks, or one stack where it's *core* to the framework.

| Kind | Why |
|------|-----|
| `auth_guard` | Rails before_action + Axum AuthLayer + Django @login_required + React PrivateRoute. "Public vs protected routes?" |
| `cache_binding` | Rails.cache + Redis + React Query cache. Invalidation queries impossible without it |
| `runtime_task_spawn` | Tokio `tokio::spawn`, asyncio `create_task`, JS Promise/microtask, C# `Task.Run`, Swift `Task { … }`. **Stackless coroutine on event loop** — cannot sync-block without freezing the worker. Distinct from `green_thread_spawn` (stackful) and `os_thread_spawn` (kernel) — see "Concurrency taxonomy" below |
| `route_mount` | Axum `.nest` / `.merge` + Rails `namespace` / `mount` + Express `Router.use`. Route grouping invisible today |
| `store_select` | Redux selector + Zustand selector + Vuex getter. State-plane navigation parallel to component plane |

### Tier 3 — judgement call, may defer (7 kinds)

Each is real but appears in fewer stacks or has narrower utility.

| Kind | Stack | Trade-off |
|------|-------|-----------|
| `sse_stream` | Axum, Express, Rails | Separate from `websocket_handler` but similar lifecycle; could collapse |
| `signal_handler` | Tokio, Node, Django signals, Phoenix PubSub | Cross-paradigm naming is awkward |
| `cancel_token` | Tokio, AbortController (JS) | Niche but Tokio-canonical |
| `lazy_load` | React, JS dynamic import, Vue defineAsyncComponent | Could fit `calls` with metadata |
| `query_binding` | React Query, SWR, Apollo GraphQL | **Scope restricted** to `useQuery → queryFn` reference only. The plugin does **not** follow `queryFn` body to extract the fetched URL — that would be B1 flow analysis. The composer layer composes the edge with `http_route` separately if the queryFn calls `fetch('/api/x')` and an `http_route` exists at `/api/x`. Valuable but specific |
| `os_process_spawn` | Tokio `tokio::process::Command`, Node `child_process`, Python `subprocess` / `multiprocessing.Process`, Ruby `system()` / `Process.spawn`, Java `ProcessBuilder` | **Kernel process** — separate address space, IPC required. Distinct from every in-process primitive. `os_` prefix makes the kernel-vs-runtime boundary unmistakable in agent output |
| `os_thread_spawn` | `std::thread::spawn`, `pthread_create`, `java.lang.Thread`, Python `threading.Thread`, Ruby `Thread.new` (MRI), JS `new Worker(...)` | **Kernel thread (1:1)** — shared address space, preemptively scheduled by OS. Distinct from `green_thread_spawn` (user-space M:N) and `runtime_task_spawn` (stackless). Critical for "can sync-block here?" / "memory cost per spawn?" queries |

### Tier 4 — out of scope for this expansion

Genuinely too narrow or too speculative to bake in:

- `trace_span` (distributed tracing)
- `circuit_breaker`
- `i18n_key` (translation key references)
- `rake_task` (rake-only)
- `db_index` (could fit migration metadata)
- `mailer_handler` (mailers fit `queue_handler` with metadata)

These remain as `calls` + metadata for now. Re-evaluate after a year of usage.

---

## Concurrency taxonomy — 4-kind split

The original R0 whitelist names `goroutine_spawn` as the single in-process
concurrent-unit edge kind. That single kind collapses four operationally
distinct primitives that the agent must distinguish. This section
documents the split and the per-runtime classification rules.

### Why four kinds, not one

Concurrent units differ along three orthogonal axes:

| Axis | OS process | OS thread | Green thread | Async task |
|------|:----------:|:---------:|:------------:|:----------:|
| Own stack? | ✅ | ✅ | ✅ stackful | ❌ state machine |
| Scheduler | kernel | kernel (1:1) | runtime user-space (M:N) | runtime event loop |
| Address space | isolated | shared | shared | shared |
| Sync-block safe? | ✅ | ✅ | ✅ (runtime parks) | ❌ blocks worker |
| Spawn cost | ms | 10–100 µs | µs | ns–µs |
| Examples | `Command::new`, `fork`, `subprocess` | `std::thread::spawn`, `pthread`, JVM `Thread`, Web Worker | `go func()`, JVM virtual thread (Loom), Erlang `spawn`, Akka actor | `tokio::spawn`, `asyncio.create_task`, JS Promise, C# `Task`, Swift `Task` |

Three of three axes diverge across the four mechanisms. Collapsing them into
one kind hides distinctions an LLM agent operationally needs (e.g., "is this
`std::fs::read` call safe inside this spawn site?" — answer differs per
kind).

### The four kinds

| Kind | Tier | Meaning |
|------|:----:|---------|
| `os_process_spawn` | 3 | Kernel creates new process. Separate address space; IPC for communication |
| `os_thread_spawn` | 3 | Kernel creates thread (1:1). Shared heap; preemptively scheduled by OS |
| `green_thread_spawn` | R0 (rename from `goroutine_spawn`) | User-space runtime creates stackful concurrent unit, M:N over kernel threads. Cooperatively scheduled (Go, Erlang) or preempted by runtime checkpoints (Go 1.14+) |
| `runtime_task_spawn` | 2 | User-space runtime creates **stackless** state machine driven by event loop / executor. Cooperative yield via `.await` / `yield`; cannot sync-block worker |

### Per-runtime classification

These rules belong in the per-language and per-framework docs once each
plugin ships. Captured here to lock the taxonomy across the four kinds.

| Runtime / API | Kind | Notes |
|---------------|------|-------|
| `tokio::spawn(fut)` | `runtime_task_spawn` | Stackless future on tokio worker pool |
| `tokio::task::spawn_blocking(closure)` | `runtime_task_spawn` | Still tokio-runtime-managed; metadata `blocking=true` flags the blocking pool variant |
| `tokio::task::LocalSet` | `runtime_task_spawn` | Single-thread variant |
| `tokio::task::JoinSet` | `runtime_task_spawn` × N | One edge per spawned task |
| `std::thread::spawn(closure)` | `os_thread_spawn` | Kernel thread |
| `tokio::process::Command::spawn` | `os_process_spawn` | Forks child process |
| `std::process::Command::spawn` | `os_process_spawn` | Same |
| `async-std::task::spawn` | `runtime_task_spawn` | Same model as tokio |
| `smol::spawn` | `runtime_task_spawn` | Same |
| `rayon::spawn` / `rayon::scope` | `os_thread_spawn` | Work-stealing on a thread pool; underlying primitive is kernel threads |
| `go func() { … }` | `green_thread_spawn` | Goroutine; M:N stackful with Go runtime scheduler |
| `runtime.LockOSThread()` (Go) | none | Annotates an existing goroutine; not a spawn |
| `os/exec.Command{}.Start()` (Go) | `os_process_spawn` | Forks kernel process |
| `asyncio.create_task(coro)` | `runtime_task_spawn` | Stackless coroutine on event loop |
| `asyncio.ensure_future(...)` | `runtime_task_spawn` | Same |
| `threading.Thread(target=…).start()` (Python) | `os_thread_spawn` | Kernel thread (GIL-bound but real thread) |
| `multiprocessing.Process(target=…).start()` (Python) | `os_process_spawn` | Forks kernel process |
| `subprocess.Popen(...)` (Python) | `os_process_spawn` | Same |
| `concurrent.futures.ThreadPoolExecutor.submit` (Python) | `os_thread_spawn` | Wrapped kernel thread |
| `concurrent.futures.ProcessPoolExecutor.submit` (Python) | `os_process_spawn` | Wrapped kernel process |
| `new Worker('worker.js')` (JS) | `os_thread_spawn` | V8 isolate on separate kernel thread; isolated heap (no shared memory unless `SharedArrayBuffer`) |
| `new SharedWorker(...)` (JS) | `os_thread_spawn` | Same |
| `child_process.spawn` / `fork` (Node) | `os_process_spawn` | Forks kernel process |
| `Promise.resolve().then(...)` (JS) | `runtime_task_spawn` | Microtask on event loop. Edge emitted only when the predicate is confident the promise represents a meaningful concurrent unit (i.e., a top-level `new Promise((res) => ...)` or explicit `queueMicrotask`); plain `.then` chains on existing promises do not emit |
| `queueMicrotask(fn)` (JS) | `runtime_task_spawn` | Same |
| `setImmediate(fn)` (Node) | `runtime_task_spawn` | Event loop next-tick |
| `setTimeout(fn, 0)` (JS) | `runtime_task_spawn` | Same (next macro-task) |
| `Task.Run(action)` (C#) | `runtime_task_spawn` | TPL task on thread pool, stackless |
| `Task { … }` (Swift) | `runtime_task_spawn` | Structured async task |
| `Thread.start { … }` (JVM platform thread) | `os_thread_spawn` | Kernel thread |
| `Thread.ofVirtual().start { … }` (JVM virtual thread, Loom) | `green_thread_spawn` | M:N stackful on Loom carrier threads |
| `CompletableFuture.runAsync(...)` (Java) | `runtime_task_spawn` | TPL-equivalent; stackless on ForkJoinPool |
| `ProcessBuilder().start()` (Java) | `os_process_spawn` | Kernel process |
| `Thread.new { … }` (Ruby MRI) | `os_thread_spawn` | Kernel thread; GVL-bound but real |
| `Fiber.new { … }` (Ruby) | `green_thread_spawn` | Stackful, cooperative — closer to green thread than to async task |
| `Ractor.new { … }` (Ruby 3+) | `green_thread_spawn` | Isolated actor on Ruby runtime threads |
| `system("cmd")`, `Process.spawn("cmd")`, backticks (Ruby) | `os_process_spawn` | Kernel process |
| Async gem `Async { … }` (Ruby) | `runtime_task_spawn` | Fibers-driven scheduler; stackless from agent perspective |
| `Akka` actor `system.actorOf(...)` (Scala) | `green_thread_spawn` | Akka dispatcher on JVM threads |
| Erlang/Elixir `spawn/1`, `spawn_link/1`, `Task.async/1` | `green_thread_spawn` | BEAM process; stackful M:N |

### Edge cases

- **`tokio::task::spawn_blocking`** — debated: runs on a blocking thread
  pool of real kernel threads, but is initiated as a tokio task and the
  caller awaits it as a future. Classified `runtime_task_spawn` with
  `metadata.blocking=true`; the agent reading the metadata knows the
  variant uses a kernel thread underneath.
- **Goroutine on Go 1.14+** — preemptive at function-call checkpoints but
  still M:N stackful. Stays `green_thread_spawn`; preemption is a
  scheduler property, not a structural one.
- **Web Worker without `SharedArrayBuffer`** — separate V8 isolate,
  fully isolated heap. Closer to `os_process_spawn` semantically but
  still a kernel thread inside the same OS process; classified
  `os_thread_spawn` for honesty about the kernel primitive.
- **Erlang process** — Erlang calls them "processes" but they are
  user-space scheduled on BEAM scheduler threads; classified
  `green_thread_spawn` because the operational properties (stackful,
  M:N, preempted by BEAM) match the green-thread model. Calling them
  `os_process_spawn` would mislead.
- **Promise constructor chains** — only the top-level new-promise creation
  (or explicit microtask scheduling via `queueMicrotask`) emits an edge.
  Pure `.then` continuations on an existing promise are not new concurrent
  units; the predicate must not emit `runtime_task_spawn` for those.

### R8 audit impact

Splitting one kind into four lets `scope audit confidence` measure
precision per primitive separately. Drift in green-thread detection
(e.g., a goroutine pattern silently matching tokio tasks) surfaces
immediately as a precision drop on `green_thread_spawn` rather than
hiding inside a noisy aggregate. Combined with the resolver-side
metadata flag for `spawn_blocking`, the audit can also surface
"`runtime_task_spawn` edges where `blocking=true`" as a separate slice
without growing the kind list further.

### Channel naming (out of scope here)

The original whitelist also includes `channel_send` and `channel_recv`.
Those names are runtime-agnostic and already serve Tokio (mpsc /
broadcast / oneshot / watch) and Go (unbuffered / buffered) cleanly.
Channel-flavor distinction (mpsc vs broadcast vs oneshot) is a
framework-plugin concern — emitted via metadata on the existing kinds
rather than new edge kinds. No rename or split is required for channels
under this TODO.

---

## R0 schema addition required: `edges.args_text`

Tier 1+2 kinds in §"Proposed expansion" depend on capturing **call-site
arguments**. Patterns where the meaningful binding lives in the args:

- Rails `before_action :authenticate, only: [:create, :update]`
- Rails `rescue_from ExceptionClass, with: :handler_method`
- Rails `validates :email, presence: true`
- Axum `.layer(AuthLayer)`, `.nest("/api", api_router)`
- Express `app.use(middleware)`, `app.use('/api', router)`
- Redux `useSelector(s => s.foo)`

R0 today does **not** capture call-site args. Without args_text, these
patterns collapse to generic `calls` edges where the framework plugin
cannot distinguish them from any other call.

### Decision

Add a single nullable column to the `edges` table:

```sql
ALTER TABLE edges ADD COLUMN args_text TEXT NULL;
```

No new reserved metadata keys on `symbols`. The existing three
(`decorators`, `annotations`, `template_calls`) cover AST-decorator-driven
patterns; the new column covers call-site-driven patterns.

### Why this option over reserved metadata keys

Alternative considered and rejected: add per-symbol reserved metadata
keys such as `callbacks`, `error_rescues`, `route_decls`, `layer_calls`.
Rejected because:

1. **E2 violation risk** — naming a key `callbacks` requires the
   language plugin to decide which calls "are callbacks", which is
   name-based interpretation forbidden by `LANGUAGE-PLAYBOOK.md`
   Step 4 rule E2. Defining the key as "every call at class-body
   top level" is structural but loses the value of the key (everything
   collapses there).
2. **Sprawl** — each new framework would tempt a new reserved key;
   the schema becomes informal over time and the reserved-key list
   grows uncontrollably.
3. **Worse performance** — SQLite `json_extract` per row is 5–10×
   slower than a column read; no native index on JSON paths.
4. **Doesn't fit the call** — args belong to the call site, which is
   the edge in scope's model. Symbol metadata is for declarations,
   not for calls.

### Why args_text on edges is R5-compliant

`ARCHITECTURAL-REFACTOR.md` R5 explicitly permits framework plugins
to regex over `edges WHERE kind='calls'`:

> "Framework plugins that need hook-style matching apply the regex
> themselves over `Symbol.name` and `edges WHERE kind='calls'` rows;
> that is allowed at the framework layer."

Adding `args_text` extends what the regex sees from just symbol
names to also the call's argument text. The framework layer's
authority to interpret is unchanged. The language plugin's E2 line
is not crossed: the language plugin only captures the **raw text
slice**, never interprets it.

### Per-pattern coverage

Which mechanism covers each Tier 1+2 kind:

| Kind | Mechanism | Source pattern |
|------|-----------|----------------|
| `middleware` | `args_text` on `calls` edge | Rails `before_action :x`, Axum `.layer(X)`, Express `app.use(X)` |
| `validates_with` | `decorators` / `annotations` (existing) **or** `args_text` for call-form (Rails `validates :foo, with: V`) | Mixed source — declarator vs call |
| `error_handler` | `args_text` on `calls` edge | Rails `rescue_from E, with: :h` |
| `websocket_handler` | `template_calls` (existing) + handler type | Similar to `http_route` |
| `client_route` | `template_calls` (existing — React `<Route>`) + filesystem layout via R4 (Next.js) | Mixed — declarator vs filesystem |
| `auth_guard` | `args_text` on `calls` edge | Rails `before_action :authenticate`, Axum `.layer(AuthLayer)`, Django `@login_required` (via `decorators` for the decorator form) |
| `cache_binding` | `args_text` on `calls` edge | `Rails.cache.delete(key)`, `redis.set(key, …)` |
| `runtime_task_spawn` | predicate over `calls` edges matching the per-runtime classification table in "Concurrency taxonomy" | `tokio::spawn`, `asyncio.create_task`, `queueMicrotask`, `Task.Run` |
| `green_thread_spawn` (rename of `goroutine_spawn` in R0) | same predicate path | `go func()`, JVM virtual thread, Erlang `spawn`, Akka actor |
| `os_thread_spawn` (Tier 3) | same predicate path | `std::thread::spawn`, `pthread_create`, `new Worker(...)` |
| `os_process_spawn` (Tier 3) | same predicate path | `Command::new`, `subprocess.Popen`, `child_process.spawn`, `ProcessBuilder` |
| `route_mount` | `args_text` on `calls` edge | Axum `.nest("/api", router)`, Rails `namespace :admin do`, Express `app.use('/api', router)` |
| `store_select` | `args_text` on `calls` edge (selector closure body captured as text) | Redux `useSelector(s => s.foo)`, Zustand `useStore(s => s.foo)` |

The reserved metadata keys (`decorators`, `annotations`,
`template_calls`) stay **unchanged**. Patterns that already had clean
support continue to flow through metadata. The new `args_text` covers
the call-site-driven gap.

### Performance impact (raw)

Per-phase estimate vs current scope:

| Phase | Δ |
|-------|---|
| Full index time | +2–5 % (byte-slice extraction per call, INSERT col bind) |
| Incremental reindex per file | <1 % (typical 100–500 calls/file, ~5–25 ms extra) |
| Existing queries (`find_refs`, `find_impact`, `find_deps`, `trace`, `flow`) | **0 %** — they do not read `args_text` |
| `sketch`, `summary` (1 extra col deserialize) | <1 % |
| New args-filter queries (framework predicate) | sub-ms — operates on `kind='calls'` subset already filtered by the R0 covering index |
| Storage | +1–3 % on DB size (NULL ≈ 1 byte; ~30 bytes/call average) |
| Row read overhead | <1 % |

Combined with R0's other additions: 5–12 % regression vs pre-refactor
baseline. `ARCHITECTURAL-REFACTOR.md` accepts < 10 %. Args_text
contribution alone is well under the gate; the **risk is the sum**.
Mitigations below bring args_text contribution to < 3 %.

### Mitigations (apply both — non-negotiable)

#### Mitigation 1 — Skip extraction when target is fully-qualified

When the call's target is resolvable without inspecting args (e.g.,
`pkg::module::function(…)`, or `obj.method(…)` where the receiver
type resolves unambiguously inside `LanguageWorkspaceContext`), the
language plugin sets `args_text = NULL`. Args are captured only when
they carry **disambiguating semantic information** a downstream
consumer (framework plugin, composition layer) will use.

Honest scope:

- **fully-qualified call** = the symbol resolver (R3) can land the
  edge on a `Resolved` status without needing args content. The
  decision is made at the **resolver layer**, not the language plugin
  — the language plugin always extracts the slice; the resolver
  decides whether to keep it.
- **wait — that places extraction cost on every call anyway**:
  acceptable. Extraction is ~5–50 µs (byte copy). The optimisation
  is on **storage**, not extraction. The resolver sets
  `args_text = NULL` on the `InsertableEdge` for fully-qualified
  targets, which avoids the SQLite INSERT bind + storage row growth.

Charter compliance:

- Does **not** violate C2 — the "fully-qualified" check is a
  structural property of the call AST (presence of explicit path,
  unambiguous receiver type inside the workspace), not a
  language-version branch.
- Does **not** violate E2 — the decision is made by the resolver
  using `LanguageWorkspaceContext`, not by name-based interpretation.

Effect: roughly 50–70 % of calls in typical code have fully-qualified
targets (library calls, std functions, typed method dispatch). Their
`args_text` stays NULL → storage and INSERT cost saved.

#### Mitigation 2 — Cap `args_text` at 2 KB

Cap individual `args_text` values at **2048 bytes**. When the literal
source slice exceeds the cap, store the prefix plus a `[truncated]`
marker:

```
"… first 2032 bytes of source slice … [truncated]"
```

(The 16-byte tail `... [truncated]` reserves room within the 2 KB
ceiling.)

Effect:

- pathological cases (10 KB JSON literal arg, multi-page block
  argument, large config literal) bloat eliminated;
- framework predicates that care about long args (rare) can re-read
  source via line numbers already in the edge row's `file` + line
  attribute — at the framework's cost, not scope's storage.

### Final cost estimate after both mitigations

| Phase | Δ |
|-------|---|
| Full index time | +1–2 % (extraction still happens; bind + storage skipped on fully-qualified) |
| Incremental reindex per file | <1 % |
| Existing queries | **0 %** |
| New args-filter queries (framework predicate) | sub-ms |
| Storage | +0.5–1.5 % |
| Row read overhead | <1 % |
| **Combined with R0 total** | comfortably inside the 10 % gate |

### Acceptance criteria

- `ALTER TABLE edges ADD COLUMN args_text TEXT NULL` lands in the
  R0 migration (no separate migration).
- Language plugins extract the raw source slice for every `calls`
  edge they emit; the resolver (R3) decides whether to keep it
  (mitigation 1).
- Args longer than 2 KB are truncated at extraction with an
  explicit suffix marker (mitigation 2).
- Framework plugins use `edges WHERE kind='calls' AND args_text LIKE
  '%pattern%'` (or in-memory regex over the pre-filtered subset)
  to emit Tier 1 + Tier 2 derived edges.
- No new reserved metadata keys are added to `symbols`.
- Benchmark suite stays inside the 10 % regression gate combined
  with the other R0 additions.

### Why this stays a single change to R0

Splitting `args_text` into a follow-up migration after R0 means:

- a second `ALTER TABLE` and a second schema-version bump;
- a window where Tier 1+2 patterns cannot land because their
  precondition (args capture) is missing;
- inconsistency between newly-indexed and re-indexed edges (some
  carry `args_text`, others don't).

Folding the column into R0 is one migration, one schema-version
bump (0 → 1), and the whole Tier 1+2 work can begin at the same
post-R0 moment.

---

## Naming friction: `goroutine_spawn` → 4-kind taxonomy

Original R0 whitelist names `goroutine_spawn` as the single in-process
concurrent-unit edge. The earlier draft of this TODO offered three
options (rename to `task_spawn` / add tier-2 `task_spawn` / status
quo). All three were rejected during review because they collapsed
operationally-distinct primitives into one kind. The settled answer is
the 4-kind split documented under "Concurrency taxonomy" above:

| R0 baseline | Final whitelist |
|-------------|-----------------|
| `goroutine_spawn` | renamed to `green_thread_spawn` (stackful M:N: goroutine, JVM virtual thread, Erlang process, Akka actor) |
| — | `runtime_task_spawn` added at **Tier 2** (stackless coroutine on event loop: tokio task, asyncio Task, JS Promise/microtask, C# Task, Swift Task) |
| — | `os_thread_spawn` added at **Tier 3** (kernel thread 1:1: `std::thread::spawn`, Web Worker, JVM platform Thread, Python `threading.Thread`) |
| — | `os_process_spawn` added at **Tier 3** (kernel process: `Command::new`, `subprocess`, `child_process`, `ProcessBuilder`) |

R0 ships with `green_thread_spawn` (renamed) only. Tier 2 +
Tier 3 add the remaining three kinds inside the same R0 transaction so
the whitelist is final before the migration lands; renaming or adding
later requires another migration.

### Channel naming (unchanged)

`channel_send` and `channel_recv` keep their R0 names. Both terms are
runtime-agnostic (Tokio, Go, Erlang, async-std all use the word
"channel"); no rename is required. Channel flavor (mpsc / broadcast /
oneshot / watch) is tagged via `metadata.flavor` on each row rather
than new kinds — agent queries that care about flavor filter on
metadata, agent queries that care about direction filter on the kind.

---

## Composition cases that improve materially

With Tier 1 + Tier 2 kinds in R0, the following composition cases gain real power:

| Case | Today (with R0 baseline) | With Tier 1+2 |
|------|---------------------------|----------------|
| Case T `triggers` | Shows entry points (route / cron / queue) | Plus middleware chain, auth guards, error handlers — full reachability map |
| Case M `api-surface` | Public symbols crossing module boundary | Plus the validation schema each endpoint accepts and the error responses it returns |
| Case X `find-tests` | Tests calling the symbol | Plus tests calling the middleware / auth chain that wraps the symbol |
| Case J `flow` cross-language | React component → ... → Django view | Plus React Router → API client → server route resolves cleanly via `client_route` ↔ `http_route`. Algorithm in `docs/CROSS-LANG-STITCHING.md` §4–§5; `args_text` + `framework` + `symbols.metadata` from this TODO are the inputs it consumes. |
| Case W `xref-monorepo` | Cross-project refs | Plus middleware reuse across projects; auth guard sharing visible |
| Case N `dead-code` | Symbols with zero inbound | Plus middleware never `.layer`ed; validators never bound |
| Case P `health` | Diagnostics aggregate | Plus "this route has no auth_guard and no middleware" architectural lint |
| New: `mudang routes` | (not previously possible at quality) | List every route with its middleware chain, validation schema, auth guard, error handler — single query |

---

## Recommendation

1. **Open scope-side issue** against `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` R0 requesting:
   - Rename `goroutine_spawn` → `green_thread_spawn` in the R0 baseline (no count change).
   - Tier 1 (5 kinds) merged into the whitelist before R0 ships.
   - Tier 2 (5 kinds, including `runtime_task_spawn`) merged in the same R0 transaction.
   - Tier 3 (7 kinds, including `os_thread_spawn` and `os_process_spawn`) merged in the same R0 transaction so the spawn taxonomy is final at migration time.
   - `edges.args_text TEXT NULL` column added to the R0 migration (per "R0 schema addition required").

2. **Cross-link** this TODO from `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` R0 section once the scope-side discussion opens, so the audit trail is complete.

3. **Lock R0 migration only after** the whitelist debate settles. Phase A of mudang's roadmap waits on R0 acceptance; expanding now is cheaper than re-migrating later.

4. **Document the chosen tier list** in scope's CHARTER §6 soft-expansion zone update or in the per-framework docs under `gumiho-mudang-scope/docs/frameworks/`.

---

## Why this is mudang-side first

Scope's charter §6 lists domain edges as the moat against LSP. Mudang composes them. **Mudang is the consumer that knows which gaps hurt composition.** Scope didn't model these gaps because scope's audit is plugin-internal (R0–R12 closures); the consumer-side cost of an under-tier kind list is visible from mudang's composition cases.

Scope-side R0 owners may choose to push back on Tier 2 (legitimate trade-off: schema migration cost, plugin authoring complexity). The Tier 1 set is harder to defer because each one fires in **most** of the four stacks the user actively maintains.

---

## Non-goals

- This TODO does **not** redesign scope's plugin architecture. R5 FrameworkPlugin model (graph-only via metadata) holds — these new kinds are emitted via the same predicate path.
- This TODO does **not** add `.scm` queries per framework (R5 explicitly rejects that path).
- This TODO does **not** propose new symbol kinds; the existing 13 are enough.
- This TODO does **not** redesign the channel taxonomy (`channel_send` / `channel_recv` keep their R0 names; flavor goes in metadata).

---

## Cross-refs

- `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` R0 — the whitelist this TODO requests be expanded.
- `gumiho-mudang-scope/docs/CHARTER.md` §4 q4 + §6 — the moat rationale for domain edges.
- `gumiho-mudang-scope/docs/FRAMEWORK-PLAYBOOK.md` — the per-framework adoption flow; new kinds need framework predicates emitting them.
- `docs/SCOPE-LSP-COMPOSITION.md` §14 — composition cases that depend on rich edge-kind filtering.
- `docs/ROADMAP.md` phase A — gated on R0 acceptance; expansion happens within phase A.
- `docs/todos/0006-split-scope-crate.md` — orthogonal to this; both touch scope but don't conflict.
