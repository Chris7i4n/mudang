# Scope Charter

Single source of truth for what Scope is, where it expands, and where it deliberately stops. When a feature, sprint, or refactor proposal arises, consult this document first. If the proposal violates a hard limit, it is rejected without further debate. If it sits inside the soft expansion zone, it is eligible for sprint planning.

This charter is stable. Revisions require an explicit charter-amendment commit.

### Companion documents

The charter defines what Scope is. The following companions define how it is built and maintained, in the order to consult them:

- **`ENFORCEMENT-MAP.md`** — rule→implementation map. Maps each charter / playbook rule to the R-entry and audit that mechanically enforces or detects it. Durable reference for "where is rule X enforced?"
- **`LANGUAGE-PLAYBOOK.md`** — procedure for adopting a new language plugin and the 18 universal boundaries every language plugin must respect.
- **`FRAMEWORK-PLAYBOOK.md`** — procedure for adopting a new framework plugin, the 15 gotcha categories, and version strategy.
- **`docs/languages/<name>.md`** and **`docs/frameworks/<name>.md`** — per-instance gotcha logs and compliance records, one per adopted plugin. Templates live next to them as `_TEMPLATE.md`.

When a question recurs, check the charter first, then the playbook of the relevant layer, then the per-instance doc. The enforcement map documents how the architecture mechanically enforces — or detects — the charter's hard limits and the playbooks' rules.

---

## 1. Mission

Scope provides **persistent, polyglot, framework-aware code intelligence shaped for LLM agents** — not for human IDE interaction.

One sentence boundary: *if a senior developer can answer the question by grepping, reading import statements, and consulting config files, Scope can model it; if they need to invoke the compiler or run the type checker, Scope must not.*

---

## 2. Who Scope serves

Scope serves **LLM coding agents** running in shells, sandboxes, and CI runners. It does not serve human IDE users. This is a deliberate choice with consequences:

- Output is optimized for token efficiency, not for visual rendering.
- Queries are batch-precomputed, not interactive.
- The interface is a shell command and an MCP server, not a JSON-RPC editor protocol.
- The index is read-only, deterministic, and side-effect-free.

LSP servers exist and serve human IDE users well. Scope does not compete with LSP on its home turf.

### Single-operator posture

Scope today serves exactly one user. The repository owner operates every deployment. There are no external installs, no third-party `.scope/` indexes in the wild, no upgrade-from-version-N migration paths to honour.

This collapses an entire class of compatibility work the codebase otherwise has to carry:

- **No backward-compatibility shims.** When a schema, JSON shape, or on-disk format changes, the canonical path is wipe + reindex. The operator deletes `.scope/` (or `.mudang/`) and runs `index --full`.
- **No dual-read / dual-write paths during migrations.** A column or JSON key has exactly one canonical shape per binary version. Code that tolerates "shape A or shape B" is anti-pattern: it normalises ambiguity into the type system and rots through subsequent shape changes.
- **No version detectors that branch behaviour by stored format.** `Graph::open` may refuse a schema it does not own and surface the wipe instruction; it does not transparently upgrade.
- **No `_deprecated` columns, no `// legacy` reader arms, no `version: u32` fields wedged into shapes to coordinate readers across binary releases.**

This is a deliberate budget choice. Backward-compat is one of the highest-leverage sources of rot in long-lived codebases; deciding explicitly that we will not pay for it lets every refactor commit to the cleanest possible shape and delete the old one in the same landing.

If Scope ever ships to additional operators (released crate, distributed binary, hosted index), this section is the first thing to revisit — and the trade-off becomes a real one. Until then, the premise stands and reviewers should flag any "compat shim" as a violation in the same severity tier as breaking a § 3 invariant.

---

## 3. Core invariants — must never break

Any change that breaks one of these is rejected.

1. **Shell-callable, no daemon required.** `scope <cmd>` returns within a single process invocation. No long-running language servers.
2. **No language toolchain dependency.** Scope reads source files. It does not invoke `rustc`, `tsc`, `go build`, `python -c`, or any compiler/interpreter.
3. **Persistent on-disk index.** SQLite under `.scope/`. Portable, commit-able, survives across sessions and machines.
4. **Single polyglot graph.** All languages share one `symbols`/`edges` schema. Cross-language queries are first-class.
5. **Tree-sitter resilience.** The index updates correctly even when source code does not compile. Mid-refactor, broken branches, generated code with gaps — all must produce a useful (if incomplete) index.
6. **Deterministic, read-only at query time.** No network calls. No mutable buffer state. Two queries against the same `.scope/` return the same answer.
7. **LLM-shaped output.** Token-budgeted views (`summary` ~30 tok, `sketch` ~180 tok, `compact` JSON). Output formats may change; the budget orientation may not.
8. **Wipe-and-reindex is the canonical migration path.** No backward-compatibility shims, no dual-read code, no stored-format version detectors. See § 2 "Single-operator posture" for the full premise. Reviewers must flag any compat shim as a § 3 violation.

---

## 4. The 3-question test

Apply to every proposal. All three must pass.

```
1. Can it be done WITHOUT executing the language's compiler or interpreter?
2. Can it be produced by a static second pass over the existing AST and
   symbol tables (optionally augmented by reading config files such as
   Cargo.toml, package.json, tsconfig.json, pyproject.toml, go.mod)?
3. Does it preserve the core invariants in section 3?
```

If yes to all three: **eligible**.
If no to any one: **rejected** as out-of-scope, regardless of how useful it would be.

A fourth, optional but high-priority question for prioritization:

```
4. Does it model framework or domain semantics that LSP will never cover?
   (HTTP routes, queue handlers, ORM relations, migrations, cron jobs,
    feature flags, component trees, green-thread spawns, middleware
    chains, validators, error handlers, websocket handlers, client
    routes, auth guards, cache bindings)
```

If yes: this is the strongest moat Scope has against LSP. Prioritize.

---

## 5. Hard limits — Scope will never cross these

These are not "hard for now." These are permanent. They define what Scope is by defining what Scope is not.

| Capability | Reason it is permanently out |
|---|---|
| Invoking the language's compiler or interpreter | Breaks "no toolchain", breaks speed envelope, breaks CI-minimal portability |
| Live type inference | LSP territory; cost is unbounded; perpending requires per-language type system |
| Runtime macro expansion (`cfg_*!`, proc-macros, derive macro semantics, C preprocessor) | Requires per-language macro engine; effectively requires the compiler |
| Editor-buffer state (live `didChange`, dirty buffers) | Breaks shell-callable model; requires daemon |
| Network calls during query | Breaks determinism, sandbox compatibility, offline use |
| Generic instantiation tracking | Requires type system |
| Trait/interface bound checking (`T: Send + Sync` constraint solving) | Requires type system |
| Lifetime / borrow analysis | Requires Rust compiler frontend |
| Reflection / dynamic dispatch resolution at runtime semantics | Inherently runtime, not statically decidable |
| Conditional type evaluation, mapped types, inferred return type computation (TS) | Requires TS type checker |
| Metaclass / monkey-patching / `getattr` / `__init_subclass__` resolution (Python) | Inherently runtime |
| Rename refactor with semantic guarantees | Requires exact reference set, requires type system |
| Type errors, borrow errors, lint diagnostics | Compiler/linter territory |

Anything in this table is `out-of-scope-permanent`. Document the user need elsewhere; do not propose a Scope feature for it.

---

## 6. Soft expansion zone — Scope expands freely here

These are the directions Scope can grow without breaking its identity. Sprints should pick from this list.

| Direction | Cost | Strategic value |
|---|---|---|
| **Resolution pass** marking each `edge.to_id` as `resolved` / `ambiguous` / `dangling` with confidence (`high` / `medium` / `low`) | medium | +10–30% precision across all languages, additive to schema, no parser change |
| **Domain edge kinds** (30 total): R0 baseline 13 — `http_route`, `queue_handler`, `orm_relation`, `migration`, `cron`, `feature_flag`, `green_thread_spawn`, `renders`, `awaits_on`, `hook_use`, `inherits_from`, `channel_send`, `channel_recv`; Tier 1 — `middleware`, `validates_with`, `error_handler`, `websocket_handler`, `client_route`; Tier 2 — `auth_guard`, `cache_binding`, `runtime_task_spawn`, `route_mount`, `store_select`; Tier 3 — `sse_stream`, `signal_handler`, `cancel_token`, `lazy_load`, `query_binding`, `os_process_spawn`, `os_thread_spawn`. (Exhaustive list + 4-kind concurrency taxonomy in `ENFORCEMENT-MAP.md` R0.) | low per kind (schema migration + small parser) | Strongest moat versus LSP; LSP will never cover this |
| **Config-file readers** (Cargo.toml, package.json, tsconfig.json, pyproject.toml, go.mod) for module hierarchy, workspace members, path aliases, external import marking | low | Unlocks correct cross-file and cross-crate resolution |
| **Re-export resolution** (`pub use`, `export * from`, `export {x} from`, `__all__`) via static text following | low | Fixes a major precision gap with no compiler involvement |
| **Doc-comment chain merging** (`///` chains, `//!` inner docs, JSDoc multi-line) | low | Improves docstring quality without semantic work |
| **Cross-project edges** (`scope link`, already on roadmap) | medium | Mono-repo and microservice graphs become first-class |
| **Confidence and provenance metadata** per edge | low | Lets LLM filter ruido; lets queries demand high-confidence only |
| **Decorator / annotation argument capture** as metadata | low | Feeds domain edges and richer sketches |
| **`.scm` query expansion** for additional symbol kinds (e.g., `mod` declarations, `macro_rules!` definitions as `kind=macro`, JSX components) | low to medium | After resolution pass and domain edges have shipped — the order matters |
| **Time-travel queries** (index per commit, `scope query @sha`) | high | Enables PR review and historical impact analysis |
| **Vector embeddings for `scope find`** (already Sprint 13) | medium | Semantic search by intent |
| **Stable cross-session symbol IDs** (already shipped: `file::name::kind::line` — line refers to declaration site, used as overload disambiguator) | — | Maintain; do not break |

---

## 7. Per-language scope and non-scope

The languages prioritized for depth are the ones the maintainer uses most: Rust, Python, Go, TypeScript. Ruby, Java, and C# are supported at surface level — they are registered in `scope-core/src/languages/dispatch.rs`, but they are not depth targets and earn only bug-fix maintenance unless the language playbook's adoption flow promotes them.

For each, the IN list is eligible for sprint work; the OUT list is rejected by the hard limits in section 5 and should not be revisited.

### Multi-version posture for languages

A single language plugin handles **every version of the language that its pinned tree-sitter grammar parses**. Tree-sitter grammars are typically a syntactic superset across language major versions — the same `tree-sitter-ruby` parses Ruby 2.x and Ruby 3.x, the same `tree-sitter-python` parses 3.8 through 3.12, etc. Newer-version syntax (Ruby 3.x pattern matching, Python 3.10 `match`/`case`, TypeScript 5.x decorators) parses cleanly in older sources where the construct simply does not appear.

Two consequences:

1. **No version-specific branching inside language plugins.** Rule C2 in `LANGUAGE-PLAYBOOK.md` Step 4 forbids it: a plugin does not read `.ruby-version`, `pyproject.toml`'s `python_requires`, `tsconfig.json`'s `target`, or any equivalent to alter its extraction. The plugin captures syntax; it does not interpret semantics that shifted between versions (e.g., Python 2 `print` statement vs Python 3 `print` function — the grammar handles both shapes; the plugin treats them as the syntax it sees).
2. **Multiple grammar versions of the same language are not supported simultaneously.** `Cargo.toml` pins one grammar per language. A grammar bump moves all sources to the new grammar; there is no per-project dispatch among grammar versions. If a future language release ships a truly incompatible grammar (rare), the choice is to bump and lose the old, or stay and lose the new. In practice tree-sitter grammars stay backwards-compatible.

Framework-version handling is **deliberately asymmetric**: framework plugins are expected to branch by framework version (Rails 5 vs 7, Express 4 vs 5) because framework patterns diverge meaningfully across versions; that mechanism is in `FRAMEWORK-PLAYBOOK.md` Step 3 and `ENFORCEMENT-MAP.md` R5 (`Detection.version`). The contrast: language semantics are the compiler's territory (out of Scope per section 5); framework patterns are the maintainer's working surface (in scope per section 6).

### Rust

**In scope (eligible for sprint work):**
- `pub use` chain following via static text resolution
- `mod` declaration to file map (already partially done)
- `#[derive(Trait)]` to `implements` edge (purely syntactic)
- `///` chain and `//!` inner-doc merging into a single docstring
- `async fn`, `unsafe fn`, `const fn` as metadata flags
- Multi-letter generic param filtering (`Item`, `Output`, `T1`) extending the existing single-letter filter
- Workspace member resolution via `Cargo.toml`
- `macro_rules!` definitions registered as `kind=macro` (definition only, not expansion)
- `use ... as` alias capture

**Out of scope (permanent):**
- `cfg_*!` macro body expansion (this is finding G1; never deepen)
- `macro_rules!` arm matching and expansion
- Trait bound checking and constraint solving
- Lifetime analysis
- Borrow checker semantics
- Generic instantiation resolution
- `impl Trait` return type resolution

### Python

**In scope:**
- Decorators with arguments (`@app.route("/foo")`, `@task(queue="x")`) captured as metadata and feeding domain edges
- `__all__` export-list resolution
- Type hints captured as `references_type` edges
- `__init__.py` module hierarchy
- Class attributes captured as fields (kind=`property`)
- `pyproject.toml` dependency graph for marking external imports
- Domain edges for Django (`urls.py` to view), FastAPI, Flask, Celery

**Out of scope (permanent):**
- `getattr` / `setattr` dynamic resolution
- Metaclass behavior
- Runtime monkey-patching
- `__init_subclass__` resolution
- Type narrowing via `isinstance`

### Go

**In scope:**
- Interface satisfaction via **static method-set comparison only** — collect M's declared methods (name, parameter list as text, return list as text); collect I's declared methods (same shape); emit an `implements` edge with `confidence='medium'` when M's set covers I's set as plain text. **Out of bounds even within this row**: pointer-vs-value receiver semantics, embedded-interface promotion across packages, generic type parameters in either side, and any edge case requiring Go's actual method-set computation. Those are type-system territory and are rejected by section 5's "Trait/interface bound checking" hard limit. The IN row above is the syntactic shadow of the type-system rule, not the rule itself; a Go plugin that grew into "actual method-set semantics" is rejected as out-of-scope-permanent regardless of how useful it would be.
- Type embedding to method-promotion edges
- `go func()` to `green_thread_spawn` edge kind (renamed from `goroutine_spawn`; same semantics — stackful M:N green thread — but the new name applies cleanly across Erlang/Elixir processes, JVM virtual threads, and any other stackful runtime)
- Channel send/receive edges
- Build tag awareness (filter indexed files by `+build` / `//go:build`)
- `go.mod` workspace and module resolution
- Domain edges for `gin`, `echo`, `gorilla/mux`, `sqlx`, `gorm`

**Out of scope (permanent):**
- Generic instantiation resolution
- `reflect` package runtime resolution
- Runtime interface assertion outcomes

### TypeScript

**In scope:**
- JSX to `renders` edges (component tree)
- React hook usage edges
- Decorator targets (`@Controller`, `@Injectable`, `@Component`) feeding domain edges
- `export * from` and `export {x} from` re-export resolution
- Type-only imports filtered out of `imports` edges
- `tsconfig.json` `paths` aliases for module resolution
- Domain edges for Express, NestJS, Next.js (pages and app routes), Prisma, TypeORM

**Out of scope (permanent):**
- Inferred return type computation
- Conditional type evaluation (`T extends U ? A : B`)
- Mapped type resolution
- Generic constraint solving
- Decorator factory return-type tracking

### Ruby (surface)

Surface-level support per `src/languages/ruby.rs`. Symbol kinds (function, class, method, etc.) and universal edges (calls, imports, contains, references_type, extends) are extracted; no Ruby-specific depth work is funded. Promotion to a depth target requires triggers per the LANGUAGE-PLAYBOOK adoption flow.

### Java (surface)

Surface-level support per `src/languages/java.rs`. Same posture as Ruby — universal edges only, no Java-specific depth (e.g., no annotation-driven Spring routing, no generic wildcard handling). Promotion requires triggers.

### C# (surface)

Surface-level support per `src/languages/csharp.rs`. Same posture as Ruby and Java — universal edges only, no C#-specific depth (e.g., no LINQ provider analysis, no async state-machine awareness). Promotion requires triggers.

---

## 8. What Scope retains versus LSP

Even if Scope reaches near-LSP semantic depth in the soft-expansion zone, these differentiators remain. They are the reason Scope exists separately from LSP and the reason agents will continue to call Scope first.

1. **Shell-callable, no daemon.** Any agent in any sandbox can invoke Scope.
2. **Polyglot single graph.** Cross-language queries (`flow ReactComponent DjangoView`) impossible in LSP without manual glue.
3. **Persistent and portable.** `.scope/` is committable; teams clone and query immediately.
4. **No toolchain required.** CI runners and minimal containers run Scope; they cannot run language servers without language installs.
5. **Tolerant of broken code.** Tree-sitter recovers; LSP servers stall.
6. **LLM-shaped output.** Token-budgeted; LSP returns IDE-shaped JSON-RPC requiring an adapter and re-aggregation.
7. **Cross-cutting queries native.** `flow`, `trace`, `impact`, `rdeps`, `diff --ref main` are primitives, not compositions.
8. **Intent search.** FTS5 with BM25 plus planned vectors over name + doc + path + callers; LSP `workspace/symbol` is fuzzy on names only.
9. **Curated ranking for LLM.** Importance-tier boost, vendor de-rank, generic-name de-rank.
10. **Custom domain edge kinds.** HTTP routes, queue handlers, ORM relations — LSP will never model these.
11. **Git-aware.** `scope diff --ref main`. LSP is atemporal.
12. **Workspace federation.** Multi-project monorepo as one graph; LSP is mono-project.
13. **Memory profile.** SQLite in MBs versus rust-analyzer in GBs.
14. **Stable cross-session IDs.** `file::name::kind::line` (the implementation in `scope-core/src/parser.rs` includes the declaration line as a uniqueness disambiguator for overloaded names; the line refers to the declaration site, so the ID survives across sessions and across edits to other parts of the file). Citable in tickets, PRs, logs.
15. **Read-only and side-effect-free.** Sandbox-compatible.
16. **MCP-native.** Already shipping `scope-mcp`.
17. **Time-travel potential.** Per-commit indices.

These are the moats. Sprints should reinforce them, not erode them.

---

## 9. Strategic order of operations

Before any per-language `.scm` refinement work, three cross-cutting investments deliver more value with less code:

1. **Resolution pass.** Mark each edge as `resolved` / `ambiguous` / `dangling` with a confidence level. Additive to schema. Single feature, single sprint, all-language win.
2. **Domain edge kinds.** Schema migration adding the framework-domain edges in section 6. Each framework parser is small. Largest moat versus LSP per unit of code.
3. **Config-file readers.** Cargo.toml, package.json, tsconfig.json, pyproject.toml, go.mod. Without these, import resolution is blind. With them, cross-file precision climbs sharply.

Only after these ship do per-language `.scm` extensions earn their seat. The reason: a better `.scm` query inside a system without resolution still produces noisy edges. The same query inside a system with resolution produces high-confidence edges. The order multiplies the value.

---

## 10. How to use this charter

When a sprint, feature request, or refactor is proposed:

1. State the proposal in one sentence.
2. Run the 3-question test from section 4.
3. Check the hard limits in section 5. If a hit, stop. The proposal is out-of-scope-permanent. Do not negotiate.
4. Locate the proposal in section 6 (general) or section 7 (per-language). If absent, propose an addition to this charter as part of the sprint.
5. Confirm it does not violate the core invariants in section 3.
6. Ship.

When a question recurs ("should Scope do X?"), answer it from this document. Do not re-derive. Do not re-debate. If the document is silent or ambiguous, amend the document explicitly; do not let the ambiguity propagate.

---

## 11. Amending this charter

This document changes only via an explicit charter-amendment commit:

```
docs(charter): <one-line summary of the change>
```

The commit message body must state:
- What changed
- Why it changed
- What the previous position was

Charter changes are versioned by commit; there is no separate version number on this file. The charter at any point in history is the charter at that commit.

---

## Appendix A — Architectural ceilings (schema invariants)

The structural limits below are why the hard limits in section 5 are hard. Each is a present-state schema invariant carried by [`ENFORCEMENT-MAP.md`](ENFORCEMENT-MAP.md) R-entries.

- **`symbols.kind` is a closed `CHECK` list of 13 kinds** (`function`, `class`, `method`, `interface`, `struct`, `enum`, `const`, `type`, `property`, `variant`, `macro`, `module`, `trait`) — per [R0](ENFORCEMENT-MAP.md#r0--schema-closures). Rust traits map to `trait` and Ruby modules to `module`, not coerced into `interface`. Adding a new kind requires a schema migration.
- **`edges.kind` is a closed `CHECK` list of 38 kinds** — 8 universal (`calls`, `imports`, `extends`, `implements`, `instantiates`, `references`, `references_type`, `contains`) + 30 domain edges (section 6) — per [R0](ENFORCEMENT-MAP.md#r0--schema-closures). Adding a kind requires a schema migration.
- **`edges` PK is the surrogate `edge_id`** with a non-unique covering index on `(from_id, to_id, kind)`. Multiple call sites between the same pair are preserved per-line; two HTTP routes can bind to the same handler ([R0](ENFORCEMENT-MAP.md#r0--schema-closures)).
- **`symbol.id` format is `file::name::kind::line`** (`scope-core/src/parser.rs`), where `line` is the declaration line, used as a uniqueness disambiguator. The line component is required for overload disambiguation; the ID is stable across sessions because the declaration line moves only when the symbol itself is edited.
- **`edges.to_id` is intentionally not a foreign key.** The schema tolerates dangling references; the resolution pass ([R3](ENFORCEMENT-MAP.md#r3--pipeline-ordering-via-type-state)) attaches an explicit `status` of `Resolved` / `Ambiguous` / `Dangling` rather than smoothing collisions.
- **`metadata` is a free-form `TEXT` JSON column.** Useful as escape hatch; not query-able for relational joins. Scope-specific structured fields graduate to columns when they prove their weight.
- **No `trait LanguagePlugin`.** Per-language behaviour lives on `LanguageId` inherent methods (per [R7](ENFORCEMENT-MAP.md#r7--indexer-level-dispatch-enforcement)). The resolution pass lives outside the extractor surface, after extraction, in the indexer pipeline.
- **`symbol_name_from_id` does not text-parse on miss.** The resolved-vs-dangling distinction is carried by the `status` column, not encoded into the id string ([R3](ENFORCEMENT-MAP.md#r3--pipeline-ordering-via-type-state) acceptance bullet 5).

The soft-expansion zone in section 6 lives entirely within these ceilings. The hard limits in section 5 are precisely the things that would require breaking them.
