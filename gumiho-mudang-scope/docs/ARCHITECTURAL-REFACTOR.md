# Architectural Refactor

Companion to `CHARTER.md`, `LANGUAGE-PLAYBOOK.md`, `FRAMEWORK-PLAYBOOK.md`. Replaces the previous expansion plan.

The charter defines what Scope is. The playbooks define the rules every language and framework plugin must respect: the architectural hard limits in charter section 5, the 18 universal language-plugin boundaries in `LANGUAGE-PLAYBOOK.md` Step 4, and the framework gotcha checklist in `FRAMEWORK-PLAYBOOK.md` Step 4.

Today, only some of those rules are enforced by the architecture; the rest depend on plugin-author discipline, code review, and compliance logs. That arrangement is fragile: a plugin that quietly violates a rule can land without anyone noticing for months, and the violation compounds across the index.

This document describes the **architectural refactor** that closes that gap. After the refactor, the architecture itself prevents — or automatically detects — every rule violation that can be mechanically prevented or detected. Rules that genuinely cannot be enforced by code remain on a small, explicit "discipline-only" list, not silently delegated everywhere.

The refactor is the primary work item until it ships. Feature work (per-language depth, framework rollout) is paused until the architecture is closed. Building features on an unclosed architecture compounds the very problem this refactor exists to solve.

Live status of each move and phase is tracked in `REFACTOR-STATUS.md`. Work items queued for after the refactor closes are listed in `POST-REFACTOR-PLAN.md`; no item there starts before Phase E acceptance.

---

## Why this comes before features

Building per-language or per-framework depth on the current architecture means:

- Each new plugin is a new opportunity to violate one of the 18 rules.
- Compliance is verified by manual log-walking and code review on every PR.
- Violations that slip through erode confidence in the index — and once an index is known to lie, the lie has to be unwound from every consumer (queries, agents, downstream tools).
- Recovery from a polluted index is far more expensive than the up-front refactor cost.

Closing the architecture inverts the cost curve. The refactor is one-time, painful but bounded. After it ships, every future plugin inherits compliance from the types, traits, and tests, with no per-plugin review burden for the mechanically enforceable rules.

---

## Three classes of constraint

Every rule from charter, language playbook, and framework playbook lands in exactly one of three classes:

1. **Mechanically enforceable** — the architecture makes the violation impossible. The offending code does not compile, or the offending output cannot be produced through the public API.
2. **Mechanically detectable** — the architecture allows the violation to compile and run, but a test or audit catches it before merge or before release.
3. **Discipline only** — the architecture cannot prevent or detect the violation; only review and judgment catch it. Items in this class are explicitly listed and bounded.

The refactor's goal: move every rule out of class 3 wherever possible, into class 1 first and class 2 second. Class 3 is for rules that genuinely require human judgment (e.g., "is this plugin doing flow analysis?" — undecidable in general). The class-3 list is short and explicit.

---

## Inventory of constraints and current enforcement

This table is the audit baseline. Each row is one rule plus its current enforcement class. The refactor moves rows from "discipline" to "mechanical".

### Charter hard limits (charter section 5)

| Rule | Current class | Refactor target class |
|---|---|---|
| No compiler/interpreter invocation | discipline (code does not invoke toolchains today, but `Command::new` is reachable from any module — see `src/commands/setup.rs:39` for legitimate self-invocation; nothing prevents a future `Command::new("tsc")` from compiling) | detectable (R12 extended with a process-spawn denylist that fails CI on `Command::new("rustc"\|"tsc"\|"go"\|"python"\|...)` inside plugin/extractor paths) |
| No live type inference | discipline | detectable (R12 trait-shape lint + R8 audit) |
| No macro/template/preprocessor expansion | discipline | detectable (R11 trait-shape lint + R8 audit) |
| No editor-buffer state | mechanical (no LSP-style mutable buffer; `scope index --watch` polls the filesystem and re-reads from disk on each event — no in-process buffer protocol exists) | unchanged |
| No network at query time | mechanical-by-absence (no HTTP/network client is currently linked into any plugin/extractor/query path) | detectable (R12 spawn-denylist extension fails CI on `std::net::*` / `reqwest` / `hyper` / `tokio::net` / `ureq` introduction inside plugin/extractor/query paths; current target is the dependency tree, current scope is grep+cargo-deny) |
| No generic instantiation tracking | discipline | detectable (R12 + R8) |
| No trait-bound checking | discipline | detectable (R12 + R8) |
| No lifetime / borrow analysis | discipline | detectable (R12 + R8) |
| No reflection / dynamic dispatch resolution | discipline | detectable (R12 + R8) |
| No conditional-type evaluation | discipline | detectable (R12 + R8) |
| No metaclass / monkey-patching resolution | discipline | detectable (R12 + R8) |
| No semantic rename refactor | mechanical (no write path in plugin) | unchanged |
| No type errors / borrow errors / lint diagnostics | discipline | mechanical (R10 — typed output schema has no diagnostic fields) |

### 18 language-plugin boundaries (LANGUAGE-PLAYBOOK Step 4)

| Rule | Current class | Refactor target class |
|---|---|---|
| A1 no type inference | discipline | detectable (R12 lint + R8 audit) |
| A2 no constraint solving | discipline | detectable (R12 lint + R8 audit) |
| A3 no type-system name resolution | discipline | detectable (R12 lint + R8 audit) |
| B1 no flow analysis | discipline | discipline (judgment-bound) |
| B2 no runtime / dynamic resolution | discipline | detectable (R12 lint + R8 audit) |
| B3 no assumption of valid syntax | discipline | detectable (R6 — malformed-source harness + skipped_ranges schema) |
| C1 no macro/template expansion | discipline | detectable (R11 trait-shape lint) |
| C2 no version-specific quirks | discipline | discipline (judgment-bound) |
| D1 no cross-file resolution beyond config | partial | mechanical (R4 — WorkspaceContext is the only path) |
| D2 no best-guess fallback | partial | mechanical (R0 + R1 + R3 — confidence/status required at insertion, resolution mandatory) |
| D3 no collision resolution by guessing | discipline | mechanical (R0 surrogate PK + status='ambiguous' is representable; R1 forces explicit confidence) |
| E1 no semantic correctness assertions | discipline | mechanical (R10 — output schema has no diagnostic field) |
| E2 no metadata interpretation in language plugin | partial | mechanical (R5 — graph-only via metadata; FrameworkPlugin trait shape forbids AST) |
| E3 no heuristic hot-path optimization | discipline | discipline (judgment-bound) |
| F1 no multi-pass semantic analysis in plugin | partial | mechanical (R3 — pipeline order via type-state) |
| F2 no write-back to source | partial | mechanical (R9 — immutable references everywhere) |
| F3 no file-format parsing beyond tree-sitter | discipline | mechanical (R4 — config readers are the only path; plugin trait does not expose file IO) |
| F4 no content sniffing | mechanical (indexer dispatch by extension+shebang) | unchanged (R7 — formalize as the only path) |

### Why detectable, not mechanical, for trait-shape rules

A name-based audit (R12 trait-shape) catches a method called `infer_type_at`, but does not catch a helper method named `compute_X_for_Y` that performs the same work. The process-spawn denylist (R12 second gate) catches `Command::new("rustc")` literally but not `Command::new(env::var("CC")?)` — the binary name resolves at runtime. True mechanical enforcement would require module isolation (separate crate with no `tree_sitter` dep on the inference path), an explicit dependency denylist, or a sandboxed plugin runtime — all feasible follow-ups but heavier than the refactor's scope.

For now the closure is the **combination** of three layers: trait-shape audit, process-spawn denylist, and R8 confidence audit. The trait-shape and spawn audits catch sloppy implementations at PR time; the R8 audit catches the symptom (overconfident edges) when a clean-but-forbidden implementation evades both gates by being correctly named and dynamically invoked. **Honest framing**: a determined plugin author can still write a correctly-named helper and a runtime-resolved compiler call that R8 cannot easily reach if its precision is high — that residual surface is why the discipline-only universal list (B1, C2, E3) is short but not empty, and why the `detectable` label below is best-effort rather than exhaustive. Detection is the gate, not prevention; rules listed as `detectable` are enforced by the combined audits **for typical violations**, with a small unobserved area for the determined-evasion case that falls back to code review. Module isolation (a separate crate with no `tree_sitter` dep on the plugin path) plus an explicit dependency denylist would close this residual surface mechanically — both are feasible follow-ups beyond the refactor's scope, parked behind their own triggers.

### Framework playbook gotcha categories (FRAMEWORK-PLAYBOOK Step 4)

The 15 gotcha categories are **per-instance decisions**, not universal rules. They are not in any class of the inventory above. Instead, they are recorded in `docs/frameworks/<name>.md` via the template walkthrough table; a framework plugin is not shippable until every category has an explicit decision. The refactor's contribution is indirect: by enforcing E2 mechanically (R5 graph-only via metadata), framework plugins receive parsed `Symbol.metadata` and `Edge` rows, never raw AST, which prevents whole categories of accidental cross-layer leakage and makes per-instance walkthrough tractable.

---

## Refactor moves

Each move has: **ID, rules it enforces, current state, target state, migration steps, acceptance**.

### R0 — Schema closures

- **Enforces**: D2, D3, plus identity, provenance, and partial-index recording.
- **Current state**:
  - `edges` table has no `confidence`, no `status`, no `producer`, no `pattern_id`, no `capture_id` columns.
  - `edges` PK is `(from_id, to_id, kind)`, which collapses multiple call sites between the same pair into one row, hides per-line provenance, and prevents two HTTP routes pointing at the same handler.
  - `edges.kind` whitelist (`calls`, `imports`, `extends`, `implements`, `instantiates`, `references`, `references_type`) lacks `contains` even though the language playbook treats `contains` as a universal edge.
  - `file_hashes` has no field for ranges that were parsed but not indexed (e.g., a tree-sitter ERROR node region).
- **Target state**:
  - **Identity**: `edges.edge_id INTEGER PRIMARY KEY AUTOINCREMENT`. Drop the composite PK. Add a non-unique covering index on `(from_id, to_id, kind)` for the existing query patterns. Multiple edges of the same kind between the same pair are allowed — each row carries its own line and provenance.
  - **Confidence and status**:
    - `edges.confidence TEXT NOT NULL CHECK (confidence IN ('high','medium','low'))` — no default, every insert must specify (enforced via `EdgeBuilder` in R1).
    - `edges.status TEXT NOT NULL CHECK (status IN ('resolved','ambiguous','dangling'))` — no default.
  - **Provenance**:
    - `edges.producer TEXT NOT NULL` — identifier of the producing plugin or layer (`rust_lang`, `python`, `framework:flask`, `resolution`, …).
    - `edges.pattern_id TEXT NOT NULL` — short slug naming the pattern that produced the edge (`calls.method`, `imports.use`, `http_route.decorator_literal`, …). Used by the R8 audit to localize tier drift to a specific pattern.
    - `edges.capture_id TEXT NULL` — the tree-sitter capture name (`@call`, `@http_route`, …) when applicable, for cross-reference with `.scm` queries.
    - `edges.framework TEXT NULL` — populated only for framework-derived edges.
    - `edges.args_text TEXT NULL` — call-site or declaration-site argument literal as written in source, capped at 2 KB. Populated for edge kinds whose anchor information lives in the arguments (HTTP routes, queue enqueues, env reads, GraphQL operations, pubsub topics, etc.). Two mitigations bound the cost:
      - **Mitigation 1**: the resolver does **not** write `args_text` when the edge's target is a fully-qualified import — the symbol identity already carries the information, and duplicating the literal inflates the index without adding signal. The column stays `NULL` for those rows. This preserves rule E2 — language plugins capture the raw literal; interpretation lives in framework plugins (R5) and downstream consumers.
      - **Mitigation 2**: when a literal exceeds the 2 KB cap, the column stores the first 2 KB followed by the marker `[truncated]`. Audits (R8) treat truncated rows as candidates for confidence downgrade.
  - **Edge kind whitelist additions** (38 total after this move; 8 universal + 30 domain):
    - **Universal (8 total)**: `calls`, `imports`, `extends`, `implements`, `instantiates`, `references`, `references_type` (existing 7) + `contains` (new — universal lexical containment, used by every language plugin).
    - **R0 baseline domain (13)**: `http_route`, `queue_handler`, `orm_relation`, `green_thread_spawn` (renamed from an earlier `goroutine_spawn` draft; the new name is operationally accurate across Go, Erlang/Elixir processes, JVM virtual threads, and any stackful M:N runtime), `renders`, `hook_use`, `inherits_from`, `migration`, `cron`, `feature_flag`, `awaits_on`, `channel_send`, `channel_recv`.
    - **Tier 1 domain (5 must-land for production-stack coverage)**: `middleware`, `validates_with`, `error_handler`, `websocket_handler`, `client_route`.
    - **Tier 2 domain (5 strongly-recommended)**: `auth_guard`, `cache_binding`, `runtime_task_spawn` (stackless coroutine — Tokio task, Python asyncio task, JS Promise constructor; operationally distinct from `green_thread_spawn` because a sync block on the worker thread does **not** park the runtime), `route_mount`, `store_select`.
    - **Tier 3 domain (7 judgement-call)**: `sse_stream`, `signal_handler`, `cancel_token`, `lazy_load`, `query_binding`, `os_process_spawn` (kernel process — `fork`, `tokio::process::Command`, Python `multiprocessing.Process`, Node `child_process.spawn`), `os_thread_spawn` (kernel thread — `std::thread::spawn`, Python `threading.Thread`, JS Web Worker).
    **No generic primitive edges** (`decorator_call`, `annotation_call`, `template_render`, `hook_call`) are added — primitives live in `symbols.metadata` per R5; only domain edges are top-level rows.
    The 4-kind concurrency split (`os_process_spawn` / `os_thread_spawn` / `green_thread_spawn` / `runtime_task_spawn`) records operational differences in stack ownership, scheduler, address space, and sync-block safety. A producer-side plugin picks one based on what the runtime actually does, not on the surface API spelling — `tokio::spawn` emits `runtime_task_spawn` (stackless), `std::thread::spawn` emits `os_thread_spawn`, `tokio::task::spawn_blocking` still emits `runtime_task_spawn` with `metadata.blocking=true`.
  - **Symbol kind whitelist additions**: `macro`, `module`, `trait`. Today (`src/sql/schema.sql`) Rust traits and Ruby modules are coerced into `interface`, collapsing semantics. Final whitelist: `function`, `class`, `method`, `interface`, `struct`, `enum`, `const`, `type`, `property`, `variant`, `macro`, `module`, `trait` (13). Optional semantic renames (`const` → `constant`, `type` → `type_alias`) are deferred to a follow-up migration — they do not block R0.
  - **Symbols metadata structured fields** (no schema change; documents the JSON shape `symbols.metadata` must carry, populated by language plugin):
    - `decorators`: `[{name: string, args_text: string?}]` — AST `decorator` nodes (Python, TS), `@decorator` attributes
    - `annotations`: `[{name: string, args_text: string?}]` — Java/C# annotations, Rust `attribute_item` nodes
    - `template_calls`: `[{name: string, args_text: string?}]` — AST template/component-call nodes. Populated by every language whose grammar exposes a dedicated template/component invocation: JSX components in TS/TSX (`<Foo prop={x} />`), ERB partial calls in Ruby (`render :user`), Jinja `{% include %}` / `{% extends %}` in Python, HEEx function components in Elixir (`<.user_card user={@user} />`), Razor in C#, Slim/Haml in Ruby, etc. The key is **template-system-agnostic** by design — naming it after any one syntax (e.g., `jsx_renders`) would violate the polyglot single-graph invariant (CHARTER §3 invariant 4). The `name` is the called template/component name as written; `args_text` is the raw argument/props/locals/context text without interpretation.
    - Other free-form keys may exist; the three above are reserved for framework consumption (R5). **Hooks-style detection** (e.g., React `^use[A-Z]` on call expressions) is intentionally **not** a reserved metadata key — applying a regex to a function name to decide "this is a hook" is interpretation of a naming convention, which violates E2 (`LANGUAGE-PLAYBOOK.md` Step 4) at the language-plugin layer. Framework plugins that need hook-style matching apply the regex themselves over `Symbol.name` and `edges WHERE kind='calls'` rows; that is allowed at the framework layer (per R5).
  - **Partial-index recording**: `file_hashes.skipped_ranges TEXT NOT NULL DEFAULT '[]'` — JSON array `[{start_line, end_line, reason}]`. Populated when tree-sitter recovery skipped a region or when a sub-tree was deliberately not indexed.
- **Migration**:
  - **No in-place migration.** Scope is pre-1.0, single-user (maintainer-only). Old `.scope/` indexes are discarded; the user runs `rm -rf .scope/ && scope index` to rebuild from source. Adding `scope migrate`, `legacy_backfill` conservative defaults, and `PRAGMA user_version` refusal flow protects users who do not yet exist; revisit only when the first external user files a trigger.
  - The schema lands as-is: new tables created with the final shape (`edges.edge_id AUTOINCREMENT`, NOT NULL columns with no defaults, new whitelists enforced by CHECK constraints).
- **Acceptance** (mechanical only — precision validation lives in R8 and is not a Phase A gate):
  - Every insert path goes through R1's typed builder; struct-literal `Edge { … }` outside `core::graph` is a compile error.
  - Multi-row inserts of the same `(from_id, to_id, kind)` succeed, demonstrating the PK no longer collapses domain identity.
  - Queries that filter `confidence='high' AND status='resolved'` are runnable against the re-indexed corpus; whether the filtered subset is in fact higher-precision is an R8 measurement and lands when R8 ships (Phase D), not as a gate on Phase A. R0's contribution is the schema and the constraint; R8's contribution is the evidence that the constraint pays off.

### R1 — Typed edge insertion API

- **Enforces**: D2, D3, E1, E2.
- **Current state**:
  - `Edge` (`src/core/graph.rs:45`) has fully `pub` fields. Anyone can construct an edge by struct-literal and bypass any future builder.
  - Edge inserts go through ad-hoc SQL or a thin helper that allows partial fields.
- **Target state**:
  - **`Edge` is sealed**. All fields move to `pub(crate)` (or `pub(super)`) inside the `core::graph` module. Callers outside the module cannot construct `Edge` directly. The only remaining public API on `Edge` is the read-only field accessors used by query consumers.
  - **`EdgeBuilder` is the sole producer of `RawEdge`**. The builder requires `from`, `to`, `kind`, `confidence`, `producer`, `pattern_id` to be set before `.build()`. Missing any required field is a compile-time error (typestate pattern). `capture_id` and `framework` are optional. **The builder does not accept `status`** — `status` is the resolution layer's output (R3), never the extractor's. This eliminates the prior R1↔R3 conflict where extraction could write a terminal status and resolution would short-circuit.
  - **`Graph` storage API accepts only `InsertableEdge`** — there is no `insert_edge(Edge)` or `insert_edge(RawEdge)` overload. `RawEdge` is not insertable; only the resolution layer (R3) converts `RawEdge` → `InsertableEdge` and assigns `status` based on lookup outcome. Insertion of a struct-literal `Edge` or a `RawEdge` is a compile error because the storage signature does not accept either.
  ```rust
  // extraction stage — builder yields RawEdge, no status:
  let raw = Edge::builder()
      .from(symbol_id)
      .to(target_ref)
      .kind(EdgeKind::Calls)
      .confidence(Confidence::High)        // extraction-tier precision
      .producer(Producer::Lang("rust"))
      .pattern_id("calls.method")
      .capture_id("@call")                 // optional
      .build();                            // RawEdge — status is absent

  // resolution stage — only producer of InsertableEdge:
  let insertable = resolver.resolve(raw, &ctx)?;   // status assigned here
  insertable.insert(&db)?;
  ```
  - **No short-circuit at extraction**: even when the extractor knows the target is unambiguous (e.g., a fully-qualified reference), the resolution stage still runs and assigns `status=Resolved`. Resolution is the only path that touches `status`, and `status` is the only field that distinguishes lookup outcomes (`Resolved` / `Ambiguous` / `Dangling`). The gate is unconditional; the previous "set Resolved at builder time, resolution short-circuits" path is removed.
- **Migration**: replace every direct insert with the builder → resolve flow; remove `Edge { … }` literal usages; demote `Edge` field visibility; CI grep gate for `Edge {` outside `core::graph` fails the build; remove the legacy `insert_edge(Edge)` helper from public API; remove any `.status(...)` setter from `EdgeBuilder`.
- **Phase A resolver stub** (resolves the cross-phase coupling between R1's `InsertableEdge` type and R3's resolver behaviour): Phase A ships a **trivial resolver** that maps every `RawEdge` to `InsertableEdge` with `status` assigned by a workspace-symbol-table lookup against `to_id`'s text — single match → `Resolved`, multiple → `Ambiguous`, zero → `Dangling`. The stub does **not** consult `LanguageWorkspaceContext` (which lands in R4) and does **not** apply the language-aware visibility / scope rules that the real resolver will apply in R3. The stub exists so the binary can index between Phase A merge and R3 merge; it is **not** a contribution to D2/D3 quality and its output is not a baseline for R8's audit. R3 replaces the stub **wholesale**; no Phase B sprint may extend or patch the stub in place. The stub MUST be registered in [`REFACTOR-STATUS.md` § Stubs outstanding](./REFACTOR-STATUS.md#stubs-outstanding) when sprint 0001 opens, and removed from that table when sprint 0003 closes. Phase B does not close while the stub row remains. The refactor as a whole does not close (Phase E acceptance) while any row in the Stubs outstanding table remains.
- **Acceptance**:
  - `Edge` struct-literal construction outside `core::graph` is a compile error.
  - `Graph::insert_*` accepts only `InsertableEdge` (output of resolution); `RawEdge` does not implement the `Insertable` trait.
  - Removing any required `.confidence()`/`.producer()`/`.pattern_id()` call is a compile error.
  - `EdgeBuilder` exposes no `.status(...)` method (compile-time check via trait inspection); attempting to set status at extraction does not compile.
  - No plugin or storage code constructs an insertable edge without going through the builder → resolution flow.

### R2 — LanguagePlugin output type closure

- **Enforces**: A1, A2, A3, B2, C1, E1, E2, F1, plus the plugin-skip channel for B3 (R6).
- **Current state**: `LanguagePlugin` returns edges directly; plugin chooses what to emit. A plugin could emit an edge that implies type inference, macro expansion, or any other forbidden behavior, and nothing in the trait shape catches it. There is no channel for a plugin to record "I deliberately skipped this region", which leaves R6's `plugin_skip` reason unreachable from the new model.
- **Target state**: plugin returns `RawCaptures` — a typed bag of `.scm` capture results, declared metadata (decorator args, type annotations as text, etc.), **and an explicit `skipped_ranges` field** for regions the plugin chose to skip (e.g., a macro body it cannot interpret). The indexer concatenates the plugin-reported skips with its own tree-sitter-error skips before writing `file_hashes.skipped_ranges` (R6). A separate `Extractor` layer converts `RawCaptures` to `Edge::builder()` calls. The conversion layer applies confidence rules consistently and is the only place that knows about `EdgeKind`. Plugin authors cannot directly emit edges; they cannot directly write confidence values. The extraction interface has no method whose name implies inference, expansion, resolution, or evaluation.
  ```rust
  pub struct RawCaptures {
      pub captures: Vec<Capture>,             // raw .scm capture results
      pub metadata: Vec<MetadataField>,       // decorator args, annotation text, jsx renders
      pub skipped_ranges: Vec<SkippedRange>,  // plugin-driven skips; merged by indexer with tree-sitter-error skips (R6)
  }
  pub struct SkippedRange {
      pub start_line: u32,
      pub end_line: u32,
      pub reason: String,                     // e.g., "plugin_skip:rust:unparseable_macro_body"
  }
  ```
- **Migration**: refactor `LanguagePlugin` trait; rewrite each existing plugin (Rust, Python, Go, TypeScript, Java, C#, Ruby) to return `RawCaptures`; move per-kind logic into the `Extractor`; thread `RawCaptures.skipped_ranges` through the indexer into `file_hashes.skipped_ranges`.
- **Acceptance**: trait inspection shows no method whose signature implies forbidden behavior; existing fixture suite produces identical edges before and after refactor (modulo confidence and status, which are now explicit); a fixture where a plugin emits a `skipped_ranges` entry produces a `file_hashes` row that contains that entry verbatim alongside any tree-sitter-error skips.

### R3 — Pipeline ordering via type-state

- **Enforces**: F1, D2.
- **Current state**: pipeline is `extract → write`. Resolution does not exist as a distinct stage; ambiguous edges quietly become resolved-looking via `symbol_name_from_id` text fallback. Status is implicit and unrecorded; the extractor and the writer both arguably "decide" status, so neither is auditable.
- **Target state**: pipeline is `extract → resolve → write`, encoded in the type system.
  - `RawEdge` (R1 output) carries no `status`. `RawCaptures` (R2 output) carries no per-edge status. The extractor's contract is: produce `RawEdge` with confidence/producer/pattern_id set; do not assign status.
  - The resolution stage takes `RawEdge` + `LanguageWorkspaceContext` (R4), looks up `to_id` against the workspace's symbol table, and emits `InsertableEdge` with `status` set: `Resolved` (single match), `Ambiguous` (multiple matches), or `Dangling` (no match). Resolution **does not** override the extractor's confidence; tier reflects **pattern precision** and is independent of **lookup outcome**. The two columns capture orthogonal information — `confidence` answers "how often does this pattern emit a correct edge?" and `status` answers "did we identify the unique target for this specific edge?" — and are queried independently. Consumers that demand the cleanest signal filter `confidence='high' AND status='resolved'`; consumers that want all evidence accept `status='ambiguous'` rows alongside.
  - **Why confidence is preserved through ambiguity** (resolves the prior `LANGUAGE-PLAYBOOK.md` D2 wording that conflated the two): an `extends` edge from a clean `class Foo extends Bar` is a high-precision pattern (high confidence) regardless of whether the workspace has zero, one, or three `Bar` symbols visible at lookup time. Collapsing high-pattern + ambiguous-target into "confidence=medium" hides the pattern's precision from the audit (R8) and prevents the consumer from distinguishing "noisy pattern" from "clean pattern, ambiguous workspace". Multiplicity (one row per candidate target on `Ambiguous`) is preserved by the R0 surrogate `edge_id` PK; collision-free representation is what makes the distinction recoverable.
  - `InsertableEdge` is the only type that implements the `Insertable` trait. `RawEdge` and `RawCaptures` do not.
  - **Resolution is the sole producer of `status`.** The extractor cannot construct `InsertableEdge` directly; the storage layer cannot accept `RawEdge`. Skipping resolution is a compile error because the storage call site requires `InsertableEdge`. There is no path that lets the extractor decide status.
- **Migration**: introduce typestate types `Captured` (extractor output) and `Resolved` (resolver output); route the pipeline through them; delete the `symbol_name_from_id` text-fallback path; remove any builder `.status(...)` setter from R1.
- **Acceptance**: attempting to insert a `RawEdge` does not compile; constructing an `InsertableEdge` outside the resolver module does not compile; resolution pass produces edges with explicit `Resolved | Ambiguous | Dangling` status for every edge; the extractor's confidence is preserved verbatim through resolution.

### R4 — WorkspaceContext typed access (split per layer)

- **Enforces**: D1, F3, C2 (no language-plugin access to version-coupled config).
- **Current state**: plugins access workspace via `&Path` and ad-hoc filesystem reads. A plugin could read any file. There is no separation between what a language plugin may see and what a framework plugin may see, so a single `WorkspaceContext::config()` accessor would expose `edition` / `target` / `python_requires` / `framework_versions` to language plugins indiscriminately, weakening C2.
- **Target state**: plugins receive a typed context, never raw paths. **The context is split into two traits** so that a language plugin cannot accidentally read fields that would tempt a C2 violation, while a framework plugin can read framework version because branching on it is its job (R5):
  ```rust
  // visible to language plugins (R2 LanguagePlugin):
  trait LanguageWorkspaceContext {
      fn package_for_file(&self, file: FileId) -> Option<&Package>;
      fn dependencies(&self, package: &Package) -> &[Dependency];
      fn is_workspace_internal(&self, import: &str, from: FileId) -> bool;
      fn module_layout(&self, package: &Package) -> &ModuleLayout;
      // No accessor for: edition (Cargo.toml), target (tsconfig.json),
      // python_requires (pyproject.toml), the `go` directive in go.mod,
      // .ruby-version, framework_versions. These would tempt C2 violations.
  }

  // visible to framework plugins (R5 FrameworkPlugin):
  trait FrameworkWorkspaceContext: LanguageWorkspaceContext {
      fn framework_versions(&self) -> &FrameworkVersions;  // typed map: framework name → DetectedVersion
      fn lockfile(&self) -> Option<&Lockfile>;             // parsed Gemfile.lock, package-lock.json, Cargo.lock, etc.
      // Still no raw filesystem; still no language-version fields.
      // Frameworks branch on framework version, never on language version.
  }
  ```
  No file-system handle is reachable from either trait. Config readers (Cargo.toml, package.json, pyproject.toml, Gemfile.lock, etc.) populate the typed structs; plugins consume them only. **Adding a method to `LanguageWorkspaceContext` that exposes `edition` / `target` / `python_requires` / `framework_versions` is a charter-amendment-grade change**, not a routine PR — the trait surface is the mechanical safeguard for C2.
- **Migration**: replace every `&Path` parameter in plugin trait methods with the appropriate context trait; remove direct filesystem access from plugin code; thread `FrameworkWorkspaceContext` only into `FrameworkPlugin::detect` and `match_edges`; thread `LanguageWorkspaceContext` into the resolver.
- **Acceptance**: plugin code contains no `std::fs::*` calls; language-plugin trait inspection shows it accepts `&dyn LanguageWorkspaceContext` (or a generic bound) and never `FrameworkWorkspaceContext`; framework-plugin trait inspection shows it accepts `&dyn FrameworkWorkspaceContext`; `LanguageWorkspaceContext` exposes no method whose name suggests version-coupled fields (`edition`, `target`, `python_requires`, `go_directive`, `tsconfig_target`, `framework_versions`); a CI grep gate enforces the negative trait shape.

### R5 — FrameworkPlugin operates on Symbols and Edges, not AST (graph-only via metadata)

- **Enforces**: E2, F1.
- **Current state**: framework awareness does not exist yet. The temptation, when it does, is to give framework plugins access to the raw AST so they can match patterns directly, or to ship one `.scm` query per framework per language. Both temptations must be rejected by trait shape.
- **Target state — model B (eager metadata)**:
  - `FrameworkPlugin` consumes `&[Symbol]` (with parsed `metadata` JSON) and `&[Edge]`. It does not see tree-sitter nodes, source text, or filesystem paths.
  - **Language plugins populate `symbols.metadata` with structured primitives** (the three reserved keys: `decorators`, `annotations`, `template_calls` — schema in R0). They do not emit generic primitive edges (`decorator_call`, `annotation_call`, `template_render`, etc.) — those would pollute the graph for projects that use no framework. Metadata sits on the symbol; if no framework matches, no derived edge is created. The resolved domain edges (`renders` for templates, `http_route` for routes, etc.) are emitted only when a framework predicate matches. Naming-convention surfaces (React hooks, Vue composables) are not pre-computed metadata keys; framework predicates apply name regexes to `Symbol.name` directly.
  - **FrameworkPlugin is a predicate** (SQL or Rust matcher) over the graph that emits domain edges (`http_route`, `queue_handler`, …) when its predicate matches. It is not a tree-sitter query and has no `.scm` of its own.
  - **No `queries/<lang>/frameworks/<name>.scm` files exist.** All framework knowledge lives in the framework plugin's predicate code; all AST extraction lives in the language plugin. Cross-language frameworks (e.g., a framework that exists in TS and JS variants) are matched by a single predicate that operates on graph rows produced by either language plugin.
  - **Trait shape**:
    ```rust
    trait FrameworkPlugin {
        fn name(&self) -> &str;
        fn detect(&self, ctx: &dyn FrameworkWorkspaceContext) -> Detection;   // R4 split — frameworks see framework-version + lockfile, never language version
        fn unknown_version_policy(&self) -> UnknownVersionPolicy;
        fn match_edges(
            &self,
            symbols: &[Symbol],   // pre-filtered by Detection.applies_to_languages
            edges: &[Edge],       // pre-filtered idem
            version: ResolvedVersion,
        ) -> Vec<EdgeBuilder>;    // returns builders, not edges; resolution layer (R3) finishes
    }

    struct Detection {
        detected: bool,
        version: DetectedVersion,
        applies_to_languages: Vec<SupportedLanguage>, // variants of `src/core/parser.rs::SupportedLanguage`; Rails → [Ruby]; React → [TypeScript]
    }

    /// Outcome of reading the workspace's framework version. Replaces the
    /// earlier `Option<semver::Version>` because `None` was overloaded across
    /// three distinct cases that need different policy responses.
    enum DetectedVersion {
        /// Lockfile (or equivalent pinned source) resolved to a single semver.
        /// The `Version` may have been coerced from a non-strict-semver string
        /// (e.g., Rails `7.0.4.3` → `7.0.4`; Python `3.11.0a1` → `3.11.0`)
        /// via the version-coercion layer; the per-framework doc records the rule.
        Resolved(semver::Version),
        /// Manifest declares a range but no lockfile resolved it (fresh repo,
        /// vendored fork, beta tag without parseable version, range-only
        /// `package.json` / `pyproject.toml` entry, etc.). Routed to
        /// `unknown_version_policy()`. The reader never invents a concrete
        /// version inside the range.
        Indeterminate,
        /// Framework genuinely has no versioned releases (rare). Documented in
        /// the per-framework doc with rationale; predicates ignore version,
        /// every `Pattern.available_in` is treated as `VersionReq::STAR`.
        NoVersionConcept,
    }

    enum UnknownVersionPolicy {
        Skip,                          // option A — emit zero edges (recommended default)
        StableOnlyLowConfidence,       // option B — fallback patterns with confidence=low
        AssumeLatest(semver::Version), // option C — pretend latest declared version is active
    }

    enum ResolvedVersion {
        Detected(semver::Version),     // DetectedVersion::Resolved
        Fallback,                      // DetectedVersion::Indeterminate + policy == StableOnlyLowConfidence
        Assumed(semver::Version),      // DetectedVersion::Indeterminate + policy == AssumeLatest
        Versionless,                   // DetectedVersion::NoVersionConcept
        // DetectedVersion::Indeterminate + policy == Skip never reaches
        // match_edges; the indexer short-circuits before invoking it.
    }
    ```
    No `tree_sitter::*` types appear anywhere in the trait. No `&Path`, no `&str` source, no AST nodes.
  - **Framework version is first-class**: `detect()` reads workspace config via `FrameworkWorkspaceContext` (R4 split) — `Gemfile.lock`, `package.json` + `package-lock.json`, `pyproject.toml` + `poetry.lock`, `Cargo.toml` + `Cargo.lock`, `go.mod` + `go.sum` — and returns a `DetectedVersion`. The framework predicate inspects the resolved version and branches on supported versions. Predicates that ignore version produce false positives across version boundaries and are caught by the R8 audit. This is the **deliberate asymmetry** with the language layer: rule C2 (`LANGUAGE-PLAYBOOK.md` Step 4) forbids language plugins from branching on **language** version, because language semantics are the compiler's territory; framework plugins **must** branch on **framework** version, because framework patterns are the maintainer's working surface and they diverge between releases.
  - **Version source semantics** (`DetectedVersion` variants):
    - `Resolved(v)` — a lockfile-equivalent (or pinned manifest) produced a comparable `semver::Version`. Most common case in mature workspaces. The version-coercion layer maps non-strict-semver strings (Rails `7.0.4.3` → `7.0.4`; Python `3.11.0a1` → `3.11.0`; build-metadata-coupled tags → stripped) to a single `semver::Version`. The per-framework doc records the rule used and the precision lost (e.g., dropping the 4th component erases security-patch granularity within a `7.0.4.x` line).
    - `Indeterminate` — the manifest declared a range (`^7.0`, `~3.2`, git dep with SHA, beta tag) and no lockfile resolved it; or the manifest is unparseable. The reader does **not** synthesize a version inside the range — that would silently lock in an answer the workspace has not committed to. Routed to `unknown_version_policy()`.
    - `NoVersionConcept` — framework genuinely lacks versioned releases (rare). `ResolvedVersion::Versionless` is passed straight to `match_edges`; every pattern's `available_in` is treated as `VersionReq::STAR`.
  - **Unknown-version policy** (consulted when `Detection.version == DetectedVersion::Indeterminate`): every framework plugin declares one of three policies via `unknown_version_policy()`. The indexer enforces:
    - `Skip` (recommended default) — `match_edges` is **not called**; zero domain edges emitted. The framework's per-doc records the choice with rationale.
    - `StableOnlyLowConfidence` — `match_edges` is called with `version: Fallback`. The predicate is **responsible for honoring the version tag**: it iterates only its declared fallback subset (patterns marked `available_in: VersionReq::STAR` or equivalent) and emits builders with `confidence=Confidence::Low` and `producer=Producer::Framework("<name>:fallback")` set directly. The indexer **does not mutate builder output** — that would clash with R1's typestate (every required builder field is set at `.build()` and is immutable afterwards). Risk: framework may have removed historically-stable patterns (e.g., Rails removed `before_filter` in 5.1 despite it existing since 1.0); this policy accepts that risk.
    - `AssumeLatest(version)` — `match_edges` is called with `version: Assumed(v)` where `v` is the plugin's declared latest. The predicate emits builders with `producer=Producer::Framework("<name>:assumed_<v>")` set directly (same builder-immutability constraint as above). Risk: silent false positive if the actual project is on an older major.
  - **Version granularity**: predicates use full semver via `semver::VersionReq` (e.g., `">=5.0.0, <5.1.0"`, `">=7.0.0"`). Patch-level gating is supported because real frameworks ship breaking changes in patches occasionally. The `semver` crate handles the comparison once the coercion layer maps non-strict-semver strings to a comparable `Version`. Frameworks whose versioning genuinely cannot be coerced (custom date-based, build-metadata-coupled, etc.) declare `NoVersionConcept` and rely on `available_in: VersionReq::STAR` for every pattern.
  - **Pattern catalog organization**: each framework plugin lives at `src/frameworks/<name>/` with a fixed layout:
    ```
    src/frameworks/rails/
    ├── mod.rs              # FrameworkPlugin impl: name, detect, unknown_version_policy, match_edges
    ├── patterns.rs         # ALL_PATTERNS: &[Pattern]; each Pattern carries available_in: VersionReq
    ├── predicates.rs       # the matching fns referenced by Pattern.predicate
    └── fixtures/
        ├── v5_0_x/
        ├── v6_1_x/
        └── v7_x/           # one fixture set per supported version branch
    ```
    `Pattern` struct shape:
    ```rust
    pub struct Pattern {
        pub id: &'static str,                 // e.g., "rails.belongs_to" — used in producer.pattern_id (R0)
        pub edge_kind: EdgeKind,
        pub available_in: semver::VersionReq, // single source of truth for version applicability
        pub predicate: fn(&[Symbol], &[Edge]) -> Vec<EdgeBuilder>,
    }
    ```
    `match_edges` filters: `ALL_PATTERNS.iter().filter(|p| p.available_in.matches(&version))`. The fallback subset is `ALL_PATTERNS.iter().filter(|p| p.available_in == VersionReq::STAR)`.
  - **Workspace-level detection**: `detect()` runs once per workspace, not per file. The assumption holds cleanly for the maintainer's primary stack: Cargo workspaces share dependencies via workspace inheritance; Bundler resolves a single `Gemfile.lock` per repo; root-rooted pyproject monorepos pin once. **It does not hold universally**: npm workspaces with per-package `package.json`, pip-only Python monorepos with per-package `pyproject.toml`, and any layout where each app pins independently can have different framework versions per sub-root. In those layouts the indexer reads the root manifest and assumes uniformity; edges from a wrong-version sub-app are emitted from the root-version pattern set with degraded precision. **What R8 sees and what it doesn't**: the R8 confidence audit measures **precision** by sampling emitted edges and marking them correct/incorrect; it surfaces the wrong-version drift only when the wrong-version sub-app still emits edges that lookup misclassifies as `Resolved` to a misleading target — i.e., precision drops on the sample. R8 does **not** measure **recall**: a sub-app that should produce 50 edges but produces zero (because no pattern in the root-version set matches its actual-version source) is invisible to R8 because R8 has nothing to sample. Recall regressions are caught by integration test fixtures pinned to specific versions with expected edge counts (snapshot via insta) and by the per-framework doc's "Patterns deliberately not matched" walkthrough. The known-limitation entry goes in the per-framework doc; per-sub-root detection is a future enhancement governed by trigger frequency (`docs/FRAMEWORK-TRIGGERS.md`); no workaround inside the current `detect()` shape can fix it.
  - **Cross-app queries inside one workspace**: this is exactly the scenario `tokio` queue producer in crate A → consumer in crate B is meant to model. Both ends of the queue are symbols in the same `.scope/` graph; the framework plugin's predicate matches `tx.send` callsites and `rx.recv` callsites and emits `queue_handler` edges across crate boundaries. No special configuration is required because the polyglot single graph (charter §3 invariant 4) already spans workspace members. The framework plugin author writes the predicate as if all symbols are in one pool, which they are.
  - **Cross-language match prevention** (mechanical): the indexer is the sole caller of `match_edges`. Before invoking it, the indexer applies `symbols.iter().filter(|s| detection.applies_to_languages.contains(&s.language))` (and the same for edges, joined through their endpoints). A framework plugin that declared `applies_to_languages = [Ruby]` cannot see Python symbols, even if a Python decorator happens to share a name with a Ruby callback. This closes the "Rails predicate matches a Python `@before_action` decorator and emits a false `http_route`" failure mode. The polyglot graph remains polyglot at the storage layer; cross-language matching is opt-in per framework.
- **Why not eager edges (variant A)**: emitting `decorator_call` for every `@something` in every Python file would inflate the edge graph by 10–50% in projects that use no framework. Variant B (metadata) keeps the graph clean — primitives sit in `symbols.metadata` only, and domain edges are emitted only when a framework predicate matches.
- **Why not `.scm` per framework (variant C)**: violates E2 (framework would parse AST), forces O(framework × language) `.scm` files, duplicates B3 tolerance per framework, and makes cross-cutting queries ("all HTTP routes regardless of framework") infeasible. The `.scm` model is rejected.
- **Migration**: when framework infrastructure is built (after R0–R4 ship), implement the predicate-shaped trait with `Detection.applies_to_languages`; wire the indexer-level language pre-filter; do not introduce a `.scm` per framework loader; ensure language plugins emit the three reserved metadata keys (`decorators`, `annotations`, `template_calls`) where their AST exposes them. Hooks-style detection (React `^use[A-Z]` calls, Vue composition API conventions) is implemented in the framework plugin via name-regex matchers over `Symbol.name` and `edges.kind='calls'` rows — never as a reserved metadata key, since matching by naming convention violates E2 at the language-plugin layer.
- **Acceptance**:
  - `FrameworkPlugin` trait inspection shows no `tree_sitter::*` types and no `&Path` / `&str` source parameters.
  - `Detection.applies_to_languages` is a required field; `Vec<SupportedLanguage>` cannot be empty for a `detected: true` plugin (CI gate).
  - `unknown_version_policy()` is a required method; the per-framework doc records the choice with rationale.
  - No `queries/<lang>/frameworks/` directory exists.
  - A framework plugin successfully emits `http_route` edges from a corpus where the language plugin populated `metadata.decorators` correctly, with no AST access on the framework side.
  - Removing language plugin metadata population produces zero domain edges from the framework plugin (graph-only contract).
  - Integration test: a fixture with a Python file containing a decorator name that matches a Ruby framework's predicate produces zero edges from that framework plugin (cross-language pre-filter is honored).
  - Integration test: a fixture pinned to a framework version outside the plugin's declared pattern set (no `Pattern.available_in` matches) produces zero edges from that framework plugin's predicate path.
  - Integration test: a fixture with no resolvable framework version (no lockfile present) produces edges consistent with the plugin's declared `unknown_version_policy()` — zero edges if `Skip`, only fallback-tagged edges if `StableOnlyLowConfidence`, full latest-version edges if `AssumeLatest` (each tagged in `producer`).
  - Integration test: a Cargo workspace with two member crates where one defines a `Sender<T>` and the other defines a matching `Receiver<T>` produces a `queue_handler` cross-crate edge from the framework plugin (validates cross-app graph queries).
  - Pattern audit: every Pattern in `ALL_PATTERNS` has a non-empty `id`, a `VersionReq`, and a referenced predicate fn; CI grep gate enforces.

### R6 — Malformed-source test harness

- **Enforces**: B3.
- **Current state**: no systematic tests for malformed input. Plugins may panic on edge cases. The schema has no field for partially-indexed regions, so even a plugin that does the right thing has nowhere to record what it skipped.
- **Target state**:
  - **Schema** (already covered by R0): `file_hashes.skipped_ranges TEXT NOT NULL DEFAULT '[]'` carries the JSON array `[{start_line, end_line, reason}]`.
  - **Indexer behavior**: when tree-sitter recovery encounters an `ERROR` node region, the parser records the region's line range and reason (`tree_sitter_error`, `unrecoverable_node`, `plugin_skip`, …) into `skipped_ranges` for the file. A plugin that deliberately skips a sub-tree (e.g., a macro body it cannot interpret) appends its own range with reason `plugin_skip:<plugin_name>:<rationale>`.
  - **Fixture set**: deliberately broken sources per language — truncated mid-function, unbalanced braces, EOF inside a string, mixed indentation collapse, missing closing tag in JSX, unterminated raw string, etc. Minimum 5 fixtures per supported language. **Out of scope for R6**: invalid UTF-8 at the byte level. The indexer reads source via `std::fs::read_to_string` (`src/core/indexer.rs:195` for incremental, `src/core/indexer.rs:486` for full); on a non-UTF-8 file the read returns an error, which the full-index path logs via `tracing::warn!` and drops from the parsed set (`src/core/indexer.rs:107`), and the incremental-index path propagates as a fatal `with_context` error (`src/core/indexer.rs:195`). **Today there is no `file_hashes` row for the dropped file** — the file is silently absent from the index in full mode and aborts the run in incremental mode. R6 does not change this. Recording such files honestly (a `file_hashes` row with empty symbols, a whole-file skipped range, and reason `read_error:invalid_utf8`; or switching to byte-level reading with `String::from_utf8_lossy`) is a separate refactor that needs its own trigger; the R6 scope is constrained to malformed-but-readable sources where tree-sitter recovery has something to work with.
  - **Integration test** (CI gate) runs every plugin against every malformed fixture and asserts:
    - No panic on any fixture.
    - The parseable prefix produces at least one symbol.
    - `file_hashes.skipped_ranges` for the fixture is non-empty when the file is partially malformed (i.e., the indexer recorded the skipped region rather than silently dropping it).
    - The skipped range covers the lines that the human-readable expectation says should be skipped (snapshot test via insta).
- **Migration**: write the fixture set; thread `skipped_ranges` recording into the indexer; add the integration test; fix any plugin that panics or fails to record.
- **Acceptance**:
  - CI fails when any plugin panics on any fixture in the malformed set.
  - CI fails when a partially-malformed fixture produces an empty `skipped_ranges` (silent skip is no longer acceptable).
  - Snapshot tests pin the recorded reason and range per fixture so future regressions surface as snapshot diffs.

### R7 — Indexer-level dispatch enforcement

- **Enforces**: F4.
- **Current state**: file-to-language dispatch is mostly extension-based but not formalized as the only path. A plugin could theoretically inspect a file's content to opt in.
- **Target state**: dispatch is entirely controlled by the indexer using `(extension, shebang)`. Each plugin declares `accepts_extension(&str) -> bool` and `accepts_shebang(&str) -> bool`. Plugins have no other way to opt in.
- **Migration**: audit every plugin entry point; remove any content-sniffing code; ensure indexer is the sole dispatcher.
- **Acceptance**: dispatch table is single-sourced in the indexer; plugins cannot self-activate.

### R8 — Confidence audit subcommand

- **Enforces**: detection layer for D2, E1 — **precision side only**.
- **Current state**: no audit. Tier drift is invisible until consumers complain.
- **Target state**: `scope audit confidence` samples N edges per (kind, confidence) combination from emitted edges and produces a precision report:
  - For each `confidence='high'` edge in the sample, the user (or an LLM agent) marks correct/incorrect.
  - Aggregated: precision per tier per kind per `producer` per `pattern_id`.
  - If any tier is below its target precision (high ≥ 95%, medium ≥ 70%, low no minimum), the report flags the offenders and points to the producing plugin and pattern.
- **What R8 measures and what it does not**: R8 measures **precision** — of the edges emitted at a given tier, what fraction is correct. R8 does **not** measure **recall** — there is no signal in `scope audit confidence` for "the framework should have produced 50 edges in this fixture but produced zero". A predicate that simply stops matching (because the workspace is on a version outside its declared `available_in`, because the language plugin stopped populating a reserved metadata key, because a pattern has a typo and silently never fires) emits zero rows, and zero rows are not sampleable. Recall regressions are caught elsewhere:
  - **Integration test fixtures with expected edge counts** (snapshot via insta), version-pinned per framework. A fixture that was producing 12 edges and now produces 11 fails the snapshot diff.
  - **Per-framework doc walkthroughs** ("Patterns deliberately not matched", "Patterns matched") — re-walked at every framework version bump and at every grammar bump; gaps surface as walkthrough deltas.
  - **`scope audit coverage`** (out of R8's scope) is a separate future subcommand that would walk every declared pattern and report the count of edges emitted per pattern per fixture; a sustained zero is the recall signal. This is not built in R8 because it requires a separate ground-truth corpus per pattern; trigger-deferred.
- The subcommand runs against the maintainer's reference fixture corpus before each release.
- **Migration**: implement the subcommand; build the reference fixture corpus; document explicitly in the help text that the report is precision-only.
- **Acceptance**: subcommand produces a parseable report; offenders are identifiable to specific plugins and patterns; help text and report header both state "precision report; recall is measured by integration-test snapshots, not this subcommand."

### R9 — Immutable source guarantee

- **Enforces**: F2, F1.
- **Current state**: plugin trait signatures pass `&str` for source content (already immutable). But helper functions and intermediate state may pass `&mut`. A future change could introduce a write path inadvertently.
- **Target state**: every plugin trait method takes `&str`, `&Tree`, and the appropriate context trait (`&dyn LanguageWorkspaceContext` for language plugins, `&dyn FrameworkWorkspaceContext` for framework plugins, per the R4 split) — all immutable references only. No `&mut` reaches the plugin layer for source-related types. Static lint or grep-based CI gate enforces.
- **Migration**: audit signatures; convert any `&mut` to `&` where source is involved; add CI check.
- **Acceptance**: trait audit shows no mutable source access; CI catches any introduction of one.

### R10 — Typed output schema

- **Enforces**: E1.
- **Current state**: output formatters (sketch, summary, compact, json) do raw string concatenation. A future change could leak diagnostic-shaped strings into output.
- **Target state**: output schemas are typed structs (`SymbolSketch`, `EdgeSummary`, `CompactView`). No field exists for diagnostic, error, or correctness-assertion content. Formatters serialize structs; they do not concatenate strings.
- **Migration**: define structs; convert formatters; remove direct string-building paths.
- **Acceptance**: output schema audit shows no fields named `error`, `warning`, `diagnostic`, `is_valid`, etc.

### R11 — Macro definition-only by trait shape

- **Enforces**: C1.
- **Current state**: macro handling is per-plugin discretion. A plugin could implement an `expand_macro` helper inadvertently.
- **Target state**: `LanguagePlugin` trait has no method named `expand_*`, `evaluate_*`, or anything implying expansion. The only legal output for a macro is a `Symbol` with `kind=macro` (definition only). Invocation sites are recorded as `references` edges from the call site to the macro symbol — never expanded.
- **Migration**: trait audit; remove or rename any method that implies expansion.
- **Acceptance**: trait inspection shows no expansion-shaped method; macro definitions and invocations are indexed as plain symbols and edges.

### R12 — Type-system-free trait audit + process-spawn denylist

- **Enforces**: A1, A2, A3, B2, "no compiler/interpreter invocation" charter row, and charter type-system limits.
- **Current state**: trait method names are mostly innocent, but no formal audit prevents future drift; `Command::new` is reachable from every module, including plugin paths (a future `Command::new("tsc")` would compile silently).
- **Target state** (two complementary audits):
  - **Trait-shape audit**: the `LanguagePlugin` trait and the `Extractor` (R2) audit cleanly — no method name suggests inference, evaluation, narrowing, or resolution beyond what `LanguageWorkspaceContext` (R4) exposes. Extracting a method named `infer_type_at`, `solve_constraint`, `evaluate_conditional`, or `resolve_overload` is grounds for refactor reversal. The negative trait shape is documented as a comment in the trait module.
  - **Process-spawn denylist**: a CI grep gate fails the build if any file under `src/languages/`, `src/frameworks/`, `src/core/parser.rs`, `src/core/extract*.rs`, or `src/core/resolve*.rs` contains `Command::new(`, `process::Command`, or `std::process::Command`. Self-invocation of `scope` (the only legitimate current case, in `src/commands/setup.rs:39`) is excluded by path. Adding a new allowed call requires an allowlist comment with rationale and is reviewable as a charter-amendment-grade change.
- **Honest limit**: the trait-shape audit catches obvious names but not a helper named `compute_X_for_Y` that performs the same forbidden work. The process-spawn denylist catches literal `Command::new(` introductions but not a runtime-resolved `Command::new(env::var("CC")?)`. This is why A1–A3 + B2 + "no compiler/interpreter invocation" remain `detectable`, not `mechanical`. The R8 confidence audit is the symptom-side safety net when a clean implementation evades both gates.
- **Migration**: write down the negative trait shape (the methods that must not exist) as a comment in the trait module; add `scripts/audit_trait_shape.sh` and `scripts/audit_no_spawn.sh` to CI; both must pass for merge.
- **Acceptance**: trait inspection shows no method whose name implies forbidden behavior; the spawn-denylist gate fails on `Command::new(` introduction in plugin paths; both gates run on every PR; introducing a new allowlist entry requires explicit rationale.

---

## Phase order

The refactor lands in five phases. Each phase ships atomically: all moves in the phase land together or none land. Partial closure is dangerous because it creates new combinations where some violations are prevented and others are still possible in unexpected ways.

### Phase A — Schema and storage closures

**Moves**: R0, R1.

Lands first. Everything else depends on the new edge schema and the typed insertion API.

### Phase B — Plugin layer closures

**Moves**: R2, R3, R4, R7, R9, R11, R12.

Restructures `LanguagePlugin` so that plugins are forced into the bounded shape. Touches every plugin's source.

### Phase C — Framework layer closure

**Moves**: R5.

Lands when framework infrastructure is first introduced. Until then, R5 is a design constraint, not a code change.

### Phase D — Output and audit closures

**Moves**: R8, R10.

Closes the output side and adds the detection layer.

### Phase E — Test harness closures

**Moves**: R6.

Adds the malformed-source gate. Lands last so all plugins are already in their final shape when the gate is activated.

---

## What remains discipline-only after the refactor

These three universal rules cannot be enforced by code; they require human judgment.

- **B1 (no flow analysis)** — flow analysis can take many syntactic forms; static detection of "is this plugin doing flow analysis" is itself an unbounded analysis problem. Safety net: code review against the explicit rule, plus the R8 confidence audit catches the symptoms (overconfident edges) when the root cause is missed.
- **C2 (no version-specific compiler-quirk modelling)** — distinguishing "syntactic capture" from "semantic interpretation tied to a language version" requires knowledge of the specific language. Safety net: code review against the explicit rule.
- **E3 (no heuristic hot-path optimization)** — distinguishing "honest fast path" from "approximate fast path" requires reading the implementation's intent. Safety net: code review against the explicit rule.

These three are explicit, listed, and bounded. Reviewers know exactly what to check. No other universal rule is delegated to discipline.

### Per-instance decisions (not universal rules)

The 15 framework-gotcha categories from FRAMEWORK-PLAYBOOK Step 4 and the rule-temptation entries in language plugin docs are **per-instance decisions logged in the per-plugin templates** (`docs/frameworks/_TEMPLATE.md`, `docs/languages/_TEMPLATE.md`). They are not universal rules and they are not in any class of the inventory; the contract is that every adopted plugin has a complete walkthrough table with explicit decisions. The refactor's mechanical contributions (R5 trait shape, R0 metadata schema) make those walkthroughs tractable but do not encode the decisions themselves — the decisions are framework-specific and live with the framework's doc.

---

## Acceptance for the refactor as a whole

The refactor is complete when all of the following hold:

- Every universal rule in the inventory tables (charter section 5 and LANGUAGE-PLAYBOOK Step 4) is in class 1 (mechanical), class 2 (detectable), or the explicit class-3 universal list (B1, C2, E3). FRAMEWORK-PLAYBOOK Step 4 categories are per-instance decisions tracked via templates, not a fourth class.
- The universal class-3 list contains exactly three rules: B1, C2, E3. **Detectable is best-effort**: rules in class 2 retain a small residual surface (correctly-named helpers, runtime-resolved spawns) that the combined audits do not catch; that residual is acknowledged in "Why detectable, not mechanical" above and is a known gap, not a closure failure.
- Every active language plugin's compliance log (`docs/languages/<name>.md`) has zero `NEEDS REVIEW` entries; every rule's compliance status is explicit.
- Every active framework plugin's gotcha walkthrough table (`docs/frameworks/<name>.md`) has an explicit decision in every row of the 15-category checklist.
- The full benchmark suite shows < 10% regression from pre-refactor baseline. The refactor cost is acceptable; uncontrolled performance loss is not.
- `scope audit confidence` exists, runs against the reference fixture corpus, and produces a parseable precision report per (kind, tier, producer, pattern_id).
- The CI pipeline includes the malformed-source gate (R6), the typed-trait audit (R12), and the immutable-source check (R9).

After the refactor ships, future feature work resumes against the closed architecture. A new feature plan (per-language depth items, framework rollout order, etc.) can be drafted then, and each item will be testable for compliance directly against the new types and traits — no compliance log walking required for the mechanical rules.

---

## What this document does not contain

- **Specific feature additions.** Per-language depth, per-framework plugins, vector embeddings, time-travel queries — none of these belong here. They are queued in `POST-REFACTOR-PLAN.md` and start after Phase E acceptance.
- **Sprint dates.** Phases must ship in order; the calendar is flexible. The work is bounded but not time-boxed.
- **Migration shortcuts.** Every move is atomic with its phase. Half-applied refactors create the very instability the closure is meant to eliminate.

When in doubt about whether a change belongs in the refactor or in future feature work, the test is: **does it move a rule out of class 3?** If yes, it is part of the refactor. If no, it is feature work and waits.
