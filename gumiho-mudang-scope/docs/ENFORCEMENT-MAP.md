# Enforcement map — rules and their implementation

This document maps every rule from [`CHARTER.md`](CHARTER.md), [`LANGUAGE-PLAYBOOK.md`](LANGUAGE-PLAYBOOK.md), and [`FRAMEWORK-PLAYBOOK.md`](FRAMEWORK-PLAYBOOK.md) to the code, schema, and CI gate that enforces — or detects — it. The charter and playbooks state **what** the architecture forbids; this document states **how** the architecture forbids it.

Each rule sits in exactly one of three classes ([§ Three classes of constraint](#three-classes-of-constraint)). The inventory tables file every rule against its class and its **R-entry**; the R-entries ([§ Rule enforcements](#rule-enforcements)) carry the durable contract, the path in the tree, and the CI gate names.

## How to use this document

- A rule is mechanical / detectable / discipline-only — the [inventory tables](#inventory-of-constraints) tell you which.
- An R-entry (`### R0` … `### R<n>`) is the durable anchor external docs deep-link to. The R-IDs are stable identifiers, not chronology.
- A CI gate name maps to its script + recipe via [`CI-GATES.md`](CI-GATES.md).

## How to extend this document

This document grows with the architecture. Every sprint that introduces or changes a mechanical / detectable enforcement updates the matching R-entry in the **same commit that ships the code**, and lands a new `### R<next>` section if the technique is genuinely new. The end-of-sprint update is **mandatory** — see [`sprints/README.md` § Enforcement-map update](sprints/README.md#75-enforcement-map-update). The next free R-ID is the integer one after the highest existing R-ID; the choice is mechanical, not editorial.

Companion docs:

| Doc | Owns |
|---|---|
| [`CHARTER.md`](CHARTER.md) | Mission, hard limits, soft expansion, per-language IN/OUT, amendment rule |
| [`LANGUAGE-PLAYBOOK.md`](LANGUAGE-PLAYBOOK.md) | 18 universal language-plugin boundaries, language-adoption flow |
| [`FRAMEWORK-PLAYBOOK.md`](FRAMEWORK-PLAYBOOK.md) | Framework adoption flow, version strategies, 15 gotcha categories |
| [`CI-GATES.md`](CI-GATES.md) | Every CI gate referenced below — script paths, recipe names, allowlist convention |
| [`sprints/README.md`](sprints/README.md) | Sprint methodology and the end-of-sprint update gate that keeps this document live |
| [`POST-REFACTOR-PLAN.md`](POST-REFACTOR-PLAN.md) | Work queue eligible against the current architecture |
| [`GLOSSARY.md`](GLOSSARY.md) | Term definitions |

---

## Three classes of constraint

Every rule from charter, language playbook, and framework playbook sits in exactly one of three classes:

1. **Mechanically enforceable** — the architecture makes the violation impossible. The offending code does not compile, or the offending output cannot be produced through the public API.
2. **Mechanically detectable** — the architecture allows the violation to compile and run, but a test or audit catches it before merge or before release.
3. **Discipline only** — the architecture cannot prevent or detect the violation; only review and judgment catch it. Items in this class are explicitly listed and bounded.

Class 3 is reserved for rules that genuinely require human judgment. The class-3 universal list is short and explicit: **B1, C2, E3** (see [§ Discipline-only rules](#discipline-only-rules)).

---

## Inventory of constraints

These tables map every universal rule to its enforcement class and the R-entry + audit that closes it. The R-entry column points into [§ Rule enforcements](#rule-enforcements); the audit column maps to [`CI-GATES.md`](CI-GATES.md).

### Charter hard limits ([CHARTER §5](CHARTER.md#5-hard-limits--scope-will-never-cross-these))

| Rule | Class | Enforcement |
|---|---|---|
| No compiler/interpreter invocation | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) process-spawn denylist (`scripts/audit_no_spawn.sh`) |
| No live type inference | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) trait-shape lint (`scripts/audit_trait_shape.sh`) + [R8](#r8--confidence-audit-subcommand) audit |
| No macro/template/preprocessor expansion | detectable | [R11](#r11--macro-definition-only-by-trait-shape) trait-shape lint + [R8](#r8--confidence-audit-subcommand) audit |
| No editor-buffer state | mechanical | filesystem-only indexer; no LSP-style mutable buffer protocol exists |
| No network at query time | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) network-denylist gate (`scripts/audit_no_network.sh`) |
| No generic instantiation tracking | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) + [R8](#r8--confidence-audit-subcommand) |
| No trait-bound checking | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) + [R8](#r8--confidence-audit-subcommand) |
| No lifetime / borrow analysis | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) + [R8](#r8--confidence-audit-subcommand) |
| No reflection / dynamic dispatch resolution | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) + [R8](#r8--confidence-audit-subcommand) |
| No conditional-type evaluation | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) + [R8](#r8--confidence-audit-subcommand) |
| No metaclass / monkey-patching resolution | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) + [R8](#r8--confidence-audit-subcommand) |
| No semantic rename refactor | mechanical | no write path in plugin layer |
| No type errors / borrow errors / lint diagnostics | mechanical | [R10](#r10--typed-output-schema) — typed output schema rejects diagnostic-shaped fields |

### 18 language-plugin boundaries ([LANGUAGE-PLAYBOOK Step 4](LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-rules-every-language-plugin-respects))

| Rule | Class | Enforcement |
|---|---|---|
| A1 no type inference | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) lint + [R8](#r8--confidence-audit-subcommand) audit |
| A2 no constraint solving | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) lint + [R8](#r8--confidence-audit-subcommand) audit |
| A3 no type-system name resolution | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) lint + [R8](#r8--confidence-audit-subcommand) audit |
| B1 no flow analysis | discipline | judgment-bound; [R8](#r8--confidence-audit-subcommand) catches symptoms |
| B2 no runtime / dynamic resolution | detectable | [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) lint + [R8](#r8--confidence-audit-subcommand) audit |
| B3 no assumption of valid syntax | detectable | [R6](#r6--malformed-source-test-harness) — malformed-source harness + `skipped_ranges` schema |
| C1 no macro/template expansion | detectable | [R11](#r11--macro-definition-only-by-trait-shape) trait-shape lint |
| C2 no version-specific quirks | discipline | judgment-bound; [R4](#r4--workspacecontext-typed-access-split-per-layer) closes the leakage path |
| D1 no cross-file resolution beyond config | mechanical | [R4](#r4--workspacecontext-typed-access-split-per-layer) — `LanguageWorkspaceContext` is the only path |
| D2 no best-guess fallback | mechanical | [R0](#r0--schema-closures) + [R1](#r1--typed-edge-insertion-api) + [R3](#r3--pipeline-ordering-via-type-state) — confidence/status required at insertion, resolution mandatory |
| D3 no collision resolution by guessing | mechanical | [R0](#r0--schema-closures) surrogate PK + `status='ambiguous'` representable; [R1](#r1--typed-edge-insertion-api) forces explicit confidence |
| E1 no semantic correctness assertions | mechanical | [R10](#r10--typed-output-schema) — output schema has no diagnostic fields |
| E2 no metadata interpretation in language plugin | mechanical | [R5](#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata) — `FrameworkPlugin` trait shape forbids AST access |
| E3 no heuristic hot-path optimization | discipline | judgment-bound |
| F1 no multi-pass semantic analysis in plugin | mechanical | [R3](#r3--pipeline-ordering-via-type-state) — pipeline order via type-state |
| F2 no write-back to source | mechanical | [R9](#r9--immutable-source-guarantee) — immutable references everywhere |
| F3 no file-format parsing beyond tree-sitter | mechanical | [R4](#r4--workspacecontext-typed-access-split-per-layer) — config readers are the only path; plugin trait does not expose file IO |
| F4 no content sniffing | mechanical | [R7](#r7--indexer-level-dispatch-enforcement) — indexer dispatches by extension+shebang via const-fn match |

### Why detectable, not mechanical, for trait-shape rules

A name-based audit ([R12](#r12--type-system-free-trait-audit--process-spawn-denylist) trait-shape) catches a method called `infer_type_at`, but does not catch a helper method named `compute_X_for_Y` that performs the same work. The process-spawn denylist catches `Command::new("rustc")` literally but not `Command::new(env::var("CC")?)` — the binary name resolves at runtime. True mechanical enforcement would require module isolation (separate crate with no `tree_sitter` dep on the inference path), an explicit dependency denylist, or a sandboxed plugin runtime — feasible follow-ups queued in [`POST-REFACTOR-PLAN.md`](POST-REFACTOR-PLAN.md).

Enforcement is the **combination** of three layers: trait-shape audit, process-spawn denylist, and [R8](#r8--confidence-audit-subcommand) confidence audit. The trait-shape and spawn audits catch sloppy implementations at PR time; the [R8](#r8--confidence-audit-subcommand) audit catches the symptom (overconfident edges) when a clean-but-forbidden implementation evades both gates by being correctly named and dynamically invoked. **Honest framing**: a determined plugin author can still write a correctly-named helper and a runtime-resolved compiler call that [R8](#r8--confidence-audit-subcommand) cannot easily reach if its precision is high — that residual surface is why the discipline-only universal list (B1, C2, E3) is short but not empty, and why the `detectable` label is best-effort rather than exhaustive. Detection is the gate, not prevention; rules listed as `detectable` are enforced by the combined audits **for typical violations**, with a small unobserved area for the determined-evasion case that falls back to code review.

### Framework playbook gotcha categories ([FRAMEWORK-PLAYBOOK Step 4](FRAMEWORK-PLAYBOOK.md))

The 15 gotcha categories are **per-instance decisions**, not universal rules. They are recorded in `docs/frameworks/<name>.md` via the template walkthrough table; a framework plugin is not shippable until every category has an explicit decision. The architecture's contribution is indirect: enforcing E2 mechanically ([R5](#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata) graph-only via metadata) gives framework plugins parsed `Symbol.metadata` and `Edge` rows, never raw AST, which prevents whole categories of accidental cross-layer leakage and makes per-instance walkthrough tractable.

---

## Rule enforcements

Each entry below has: **ID, rules it enforces, durable contract, where to find it in the tree, CI gates**. The R-ID is the durable anchor — external docs ([`CHARTER.md`](CHARTER.md), [`GLOSSARY.md`](GLOSSARY.md), playbooks, [`POST-REFACTOR-PLAN.md`](POST-REFACTOR-PLAN.md), [`AUDIT-LABEL-SCHEMA.md`](AUDIT-LABEL-SCHEMA.md)) deep-link into these section anchors.

### R0 — Schema closures

- **Enforces**: D2, D3, plus identity, provenance, and partial-index recording.
- **Durable contract**:
  - **Identity**: `edges.edge_id INTEGER PRIMARY KEY AUTOINCREMENT`. Non-unique covering index on `(from_id, to_id, kind)`. Multiple edges of the same kind between the same pair are allowed — each row carries its own line and provenance.
  - **Confidence and status**:
    - `edges.confidence TEXT NOT NULL CHECK (confidence IN ('high','medium','low'))`.
    - `edges.status TEXT NOT NULL CHECK (status IN ('resolved','ambiguous','dangling'))`.
  - **Provenance**:
    - `edges.producer TEXT NOT NULL` — identifier of the producing plugin or layer (`rust_lang`, `python`, `framework:flask`, `resolution`, …).
    - `edges.pattern_id TEXT NOT NULL` — short slug naming the pattern that produced the edge (`calls.method`, `imports.use`, `http_route.decorator_literal`, …). Used by [R8](#r8--confidence-audit-subcommand) to localize tier drift to a specific pattern.
    - `edges.capture_id TEXT NULL` — tree-sitter capture name when applicable.
    - `edges.framework TEXT NULL` — populated only for framework-derived edges.
    - `edges.args_text TEXT NULL` — call-site / declaration-site argument literal as written in source, capped at 2 KB with `[truncated]` marker (the 2 KB cap is queued for removal by [`POST-REFACTOR-PLAN.md` § Priority 2 — Honesty audit](POST-REFACTOR-PLAN.md#priority-2-immediately-post-refactor--honesty-audit-eliminate-non-essential-approximations)).
  - **Edge kind whitelist (38 total)**: 8 universal (`calls`, `imports`, `extends`, `implements`, `instantiates`, `references`, `references_type`, `contains`) + 30 domain. Domain split: baseline (13) — `http_route`, `queue_handler`, `orm_relation`, `green_thread_spawn`, `renders`, `hook_use`, `inherits_from`, `migration`, `cron`, `feature_flag`, `awaits_on`, `channel_send`, `channel_recv`. Tier 1 (5) — `middleware`, `validates_with`, `error_handler`, `websocket_handler`, `client_route`. Tier 2 (5) — `auth_guard`, `cache_binding`, `runtime_task_spawn`, `route_mount`, `store_select`. Tier 3 (7) — `sse_stream`, `signal_handler`, `cancel_token`, `lazy_load`, `query_binding`, `os_process_spawn`, `os_thread_spawn`.
  - **No generic primitive edges** (`decorator_call`, `annotation_call`, `template_render`, `hook_call`) — primitives live in `symbols.metadata` per [R5](#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata); only domain edges are top-level rows.
  - **4-kind concurrency split** (`os_process_spawn` / `os_thread_spawn` / `green_thread_spawn` / `runtime_task_spawn`) records operational differences in stack ownership, scheduler, address space, and sync-block safety. A producer-side plugin picks one based on what the runtime actually does, not on the surface API spelling.
  - **Symbol kind whitelist (13)**: `function`, `class`, `method`, `interface`, `struct`, `enum`, `const`, `type`, `property`, `variant`, `macro`, `module`, `trait`.
  - **Symbols metadata structured fields** (JSON shape, no schema change): `decorators`, `annotations`, `template_calls` — reserved keys for framework consumption ([R5](#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata)). Other free-form keys may exist. Hooks-style detection is intentionally **not** a reserved metadata key; framework plugins apply regex matchers themselves at the framework layer (per [R5](#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata)).
  - **Partial-index recording**: `file_hashes.skipped_ranges TEXT NOT NULL DEFAULT '[]'` — JSON array `[{start_line, end_line, reason}]`.
- **Where in the tree**: `gumiho-mudang-scope/scope-graph/src/sql/schema.sql`; `scope-core/src/edge.rs` (RawEdge / EdgeBuilder); `scope-core/src/types.rs` (Symbol).
- **Migration policy**: wipe-and-reindex ([CHARTER §2](CHARTER.md#2-architectural-stance) + [§3 invariant 8](CHARTER.md#3-core-invariants--must-never-break)). `rm -rf .scope/ && mudang index` rebuilds from source.

### R1 — Typed edge insertion API

- **Enforces**: D2, D3, E1, E2.
- **Durable contract**:
  - **`RawEdge` is sealed**. Fields are `pub(crate)` inside `scope-core/src/edge.rs`. Callers outside the module cannot construct `RawEdge` directly. CI gate `grep_edge_sealed.sh` blocks struct-literal construction outside `scope-core/src/{edge,types}.rs`.
  - **`EdgeBuilder` is the sole producer of `RawEdge`**. Typestate requires `from`, `to`, `kind`, `confidence`, `producer`, `pattern_id` before `.build()`. Missing any required field is a compile-time error. `capture_id` and `framework` are optional. **The builder does not accept `status`** — `status` is the resolution layer's output ([R3](#r3--pipeline-ordering-via-type-state)), never the extractor's.
  - **`Graph` storage API accepts only `InsertableEdge`** — no `insert_edge(RawEdge)` overload. `RawEdge` is not insertable; only the resolution layer ([R3](#r3--pipeline-ordering-via-type-state)) converts `RawEdge` → `InsertableEdge` and assigns `status` based on lookup outcome.
  - **No short-circuit at extraction**: even when the extractor knows the target is unambiguous, resolution still runs and assigns `status=Resolved`. Resolution is the only path that touches `status`.
- **Where in the tree**: `gumiho-mudang-scope/scope-core/src/edge.rs` (RawEdge, EdgeBuilder); `scope-graph/src/lib.rs` (Insertable trait); `scope-graph/src/resolve/mod.rs` (InsertableEdge constructor).
- **CI gates**: Edge sealed (`grep_edge_sealed.sh`), Builder requires fields + Builder forbids status (`compile_fail_builder` tests in `scope-core/tests/compile_fail/builder/`).

### R2 — LanguagePlugin output type closure

- **Enforces**: A1, A2, A3, B2, C1, E1, E2, F1, plus the plugin-skip channel for B3 ([R6](#r6--malformed-source-test-harness)).
- **Durable contract**:
  - Per-language extractors return `RawCaptures { matches, metadata, skipped_ranges }`. Plugins do not directly emit edges; they cannot directly write confidence values. The conversion layer lives at `scope-core/src/extract/` and is the only place that knows about `EdgeKind`.
  - `RawCaptures.skipped_ranges` is the **plugin-driven skip channel** for B3 / [R6](#r6--malformed-source-test-harness) (e.g. macro bodies the plugin cannot interpret). The indexer concatenates plugin-reported skips with its own tree-sitter-error skips before writing `file_hashes.skipped_ranges`.
  - `make_edge(from, to, kind, pattern_id, file_path, line)` is the sole construction site for `RawEdge` inside per-language extractors. No `EdgeKind` decision lives outside the extractor surface.
  - **No method on the extraction interface implies inference, expansion, resolution, or evaluation** — names are caught by [R12](#r12--type-system-free-trait-audit--process-spawn-denylist)'s trait-shape audit.
- **Where in the tree**: `gumiho-mudang-scope/scope-core/src/extract/<lang>.rs` (7 per-language extractors); `scope-core/src/extract/mod.rs` (typed dispatch `extract_edges(lang, file_path, &RawCaptures) → Vec<RawEdge>`); `scope-index/src/indexer.rs` (thin orchestrator).

### R3 — Pipeline ordering via type-state

- **Enforces**: F1, D2.
- **Durable contract**:
  - Pipeline is `extract → resolve → write`, encoded in the type system. `RawCaptures → RawEdge → InsertableEdge → Graph`.
  - **Resolver is sole producer of `status`** (`Resolved` / `Ambiguous` / `Dangling`). Resolution **does not** override the extractor's confidence; tier reflects pattern precision and is independent of lookup outcome.
  - **Multi-row Ambiguous**: when the resolver finds N candidate targets, it emits **N rows** of `InsertableEdge`, one per candidate, each `status = Ambiguous`. Candidate set preserved on disk as evidence; resolver does not invent a tiebreak. Collapsing at the producer is a charter-amendment-grade decision.
  - `Graph::insert_resolved_edges` accepts only `&[InsertableEdge]`. `InsertableEdge::new` and struct-literal construction are callable only inside `scope_graph::resolve` (typestate compile-error airtight).
  - Resolver lives in `scope-graph/src/resolve/` because resolution needs `Graph` read access and must be sole constructor of `InsertableEdge`.
- **Where in the tree**: `gumiho-mudang-scope/scope-graph/src/resolve/mod.rs`.
- **CI gates**: Insertable typestate (`compile_fail_typestate` in `scope-graph/tests/`).

### R4 — WorkspaceContext typed access (split per layer)

- **Enforces**: D1, F3, C2 (closes the leakage path).
- **Durable contract**:
  - `LanguageWorkspaceContext` (visible to language plugins) exposes only: `package_for_file`, `dependencies`, `is_workspace_internal`, `module_layout`. **No accessor for** `edition`, `target`, `python_requires`, `go_directive`, `tsconfig_target`, `framework_versions` — those would tempt C2 violations.
  - `FrameworkWorkspaceContext: LanguageWorkspaceContext` adds `framework_versions()` (typed map: framework name → `DetectedVersion`) and `lockfile()`. Still no raw filesystem; still no language-version fields.
  - Config readers (Cargo.toml, package.json, pyproject.toml, Gemfile.lock, …) live workspace-side (`scope-core/src/workspace/`), never on the plugin surface.
  - Adding a method to `LanguageWorkspaceContext` exposing version-coupled fields is a charter-amendment-grade change.
- **Where in the tree**: `gumiho-mudang-scope/scope-core/src/workspace_context.rs` (traits); `scope-core/src/workspace/` (config readers).
- **CI gates**: WorkspaceContext shape (`audit_context_shape.sh`); No filesystem in plugin (`grep_no_fs.sh`).

### R5 — FrameworkPlugin operates on Symbols and Edges, not AST (graph-only via metadata)

- **Enforces**: E2, F1.
- **Durable contract** (model B — eager metadata):
  - `FrameworkPlugin` consumes `&[Symbol]` (with parsed `metadata` JSON) and `&[RawEdge]`. It does not see tree-sitter nodes, source text, or filesystem paths.
  - **Language plugins populate `symbols.metadata` with structured primitives** (the three reserved keys: `decorators`, `annotations`, `template_calls` — schema in [R0](#r0--schema-closures)). They do not emit generic primitive edges (would pollute the graph for projects that use no framework). Metadata sits on the symbol; if no framework matches, no derived edge is created.
  - **FrameworkPlugin is a predicate** over the graph that emits domain edges when its predicate matches. Not a tree-sitter query; no `.scm` of its own.
  - **No `queries/<lang>/frameworks/<name>.scm` files exist.** All framework knowledge lives in predicate code; all AST extraction lives in the language plugin.
  - **Trait shape**: `fn detect(&self, ctx: &dyn FrameworkWorkspaceContext) -> Detection`; `fn unknown_version_policy(&self) -> UnknownVersionPolicy`; `fn match_edges(&self, symbols: &[Symbol], edges: &[RawEdge], version: ResolvedVersion) -> Vec<RawEdge>`. No `tree_sitter::*` types anywhere in the trait. No `&Path`, no `&str` source.
  - **Framework version is first-class**: `Detection.version: DetectedVersion` (`Resolved(semver) | Indeterminate | NoVersionConcept`). `UnknownVersionPolicy` (`Skip | StableOnlyLowConfidence | AssumeLatest`) governs `Indeterminate` handling. `ResolvedVersion` is what predicates see (`Detected | Fallback | Assumed | Versionless`).
  - **Deliberate asymmetry with language layer**: language plugins MUST NOT branch on language version (C2); framework plugins MUST branch on framework version (frameworks ship breaking changes between releases).
  - **Pattern catalog organization**: per-framework module at `scope-core/src/frameworks/<name>/` with `patterns.rs` (`ALL_PATTERNS: &[Pattern]`, each with `available_in: VersionReq`), `predicates.rs` (the matching fns), `fixtures/<v>/`.
  - **Cross-language match prevention** (mechanical): indexer applies `symbols.iter().filter(|s| detection.applies_to_languages.contains(&s.language))` before invoking `match_edges`. A Ruby framework cannot see Python symbols, even if a Python decorator shares a name with a Ruby callback.
  - **Workspace-level detection**: `detect()` runs once per workspace. Per-sub-root version detection for npm/Python multi-package monorepos is a known limitation; recall regressions caught by integration-test fixtures pinned to specific versions.
- **Where in the tree**: `gumiho-mudang-scope/scope-core/src/frameworks/` (trait + dispatch); `scope-core/tests/synthetic_framework/` (trait-surface acceptance fixture).
- **CI gates**: No framework SCM (`audit_no_framework_scm.sh`); Pattern catalog audit (`audit_patterns.sh`); [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) trait-shape audit extended to `frameworks/`.

### R6 — Malformed-source test harness

- **Enforces**: B3.
- **Durable contract**:
  - Schema field `file_hashes.skipped_ranges` (from [R0](#r0--schema-closures)) carries JSON array `[{start_line, end_line, reason}]`.
  - **Indexer behavior**: tree-sitter `ERROR` / `MISSING` node regions recorded with reason families: `tree_sitter_error:syntax_error`, `tree_sitter_error:missing_node`, `plugin_skip:<plugin>:<rationale>`.
  - **Fixture set**: hand-crafted-synthetic, 5-fixture floor per supported `LanguageId`, at `gumiho-mudang-scope/scope-core/tests/fixtures/malformed/<language_slug>/<case>/`.
  - **Integration test**: walks every fixture and asserts no panic, parseable prefix produces ≥ 1 symbol, `skipped_ranges` non-empty when partially malformed, `insta` snapshot pins the recorded reason + range per fixture.
  - **Out of scope**: invalid UTF-8 at the byte level. Recording such files honestly is a separate initiative with its own trigger.
- **Where in the tree**: `gumiho-mudang-scope/scope-core/tests/malformed_sources.rs`; `scope-core/tests/fixtures/malformed/<lang>/<case>/`; `scope-core/tests/snapshots/malformed_sources/*.snap`.
- **CI gates**: Malformed-source harness (`just test-malformed`).

### R7 — Indexer-level dispatch enforcement

- **Enforces**: F4.
- **Durable contract** (compile-time dispatch, no runtime election, no trait, no per-language unit struct):
  - `enum LanguageId` in `scope-core/src/languages/id.rs`, exhaustive over the seven supported languages (TypeScript, CSharp, Python, Go, Java, Rust, Ruby). Per-language behaviour lives in `impl LanguageId` match arms that delegate to per-language modules. There is **no** `trait LanguagePlugin` and **no** `*Plugin` unit structs.
  - Adding a `LanguageId` variant without adding match arms = compile error.
  - `register_languages!` macro in `scope-core/src/languages/dispatch.rs` generates `const fn dispatch_extension` / `const fn dispatch_shebang` and a `const _: () = assert_no_extension_overlap(...)` block — extension overlap panics at compile time.
  - DB `symbols.language` and `edges.producer` text slugs preserved verbatim by `LanguageId::as_str()`.
- **Where in the tree**: `gumiho-mudang-scope/scope-core/src/languages/id.rs` (enum + exhaustive impl); `scope-core/src/languages/dispatch.rs` (`register_languages!` + const-fn dispatch).
- **CI gates**: Indexer-only dispatch (`grep_dispatch.sh`).

### R8 — Confidence audit subcommand

- **Enforces**: detection layer for D2, E1 — **precision side only**.
- **Durable contract**:
  - `scope audit confidence` samples N edges per `(kind, confidence)` combination and produces a precision report per `(kind, tier, producer, pattern_id)`.
  - **Tier targets**: high ≥ 95%, medium ≥ 70%, low no minimum. Any tier below target surfaces offenders.
  - **Sampling**: default `N = 30` per cell, `--sample-size N` override, deterministic xorshift64 via `--seed N`.
  - **Two-phase labelling channel** (the contract is the JSONL file; any external labeller can plug in):
    1. `scope audit confidence --emit-sample <path>` writes unlabelled, seed-pinned JSONL — one edge per line with `label: null`.
    2. Anything fills the `label` slot — hand, LLM, LSP cross-check, hybrid.
    3. `scope audit confidence --label <path>` reads the labelled file and produces the precision report.
  - **Auditor immutability**: hard mechanical SHA-256 drift gate via `Graph::check_audit_freshness` (no `--allow-drift` escape). The auditor never mutates source-derived tables.
  - **Output format**: default `--format json` carrying `schema_version`, `disclaimer`, `sample_schema_doc`, `report[]`. `--format tsv` is a convenience view.
  - **JSONL sample schema** (full doc in [`AUDIT-LABEL-SCHEMA.md`](AUDIT-LABEL-SCHEMA.md)): `schema_version: "1"`, fields `edge_id`, `kind`, `confidence`, `producer`, `pattern_id`, `from`, `to`, `source_snippet`, `lang_version`, `label`. `lang_version` is reserved-for-future; emits always `null`. Bump to `"2"` is scheduled in [`POST-REFACTOR-PLAN.md` § Priority 1 sub-item (g)](POST-REFACTOR-PLAN.md#priority-1-immediately-post-refactor--self-correction-cycle).
  - **What R8 measures and what it does not**: precision only. **Not recall** — a predicate that simply stops matching emits zero rows and zero rows are not sampleable. Recall regressions caught by integration-test fixtures with expected edge counts (snapshot via `insta`).
- **Where in the tree**: `gumiho-mudang-cli/src/commands/audit.rs` (subcommand surface — extraction to `scope-audit` sub-crate queued in [`POST-REFACTOR-PLAN.md` § Priority 3 — Layering audit](POST-REFACTOR-PLAN.md#priority-3-immediately-post-refactor--layering-audit-thin-cli-fat-library)); `scope-graph/src/audit.rs` (`AuditEdgeRow`, `AuditFreshness`, `check_audit_freshness`); `scope-core/tests/fixtures/reference/<lang>/audit-samples/` (committed labelled corpus).
- **CI gates**: Confidence audit (`just audit-confidence` runs the integration suite).

### R9 — Immutable source guarantee

- **Enforces**: F2, F1.
- **Durable contract**: every plugin trait method takes `&str`, `&Tree`, and the appropriate context trait (`&dyn LanguageWorkspaceContext` / `&dyn FrameworkWorkspaceContext`) — all immutable references only. No `&mut` reaches the plugin layer for source-related types. Allowlist tag `// scope:audit-allow mutable-source — <rationale>`.
- **Where in the tree**: signature audit lives in `scripts/audit_immutable.sh`; affected surfaces in `scope-core/src/{languages,extract,parser.rs}`.
- **CI gates**: Immutable source (`audit_immutable.sh`).

### R10 — Typed output schema

- **Enforces**: E1.
- **Durable contract**:
  - Every `--json` CLI path emits `JsonOutput<TypedView>` with zero `serde_json::json!()` macro sites and zero `serde_json::Value` ad-hoc trees in `gumiho-mudang-cli/src/{commands,output}`.
  - Every plain-text renderer is `impl fmt::Display` on a typed view struct in `output/formatter.rs`. Zero `pub fn print_*` free functions; zero residual `println!` / `eprintln!` outside `Display` bodies.
  - Banned field names on output structs: `error`, `warning`, `diagnostic`, `is_valid`, `lint`, `correctness`.
- **Where in the tree**: `gumiho-mudang-cli/src/output/formatter.rs`; `gumiho-mudang-cli/src/output/schema/`; `gumiho-mudang-cli/src/commands/*.rs`.
- **CI gates**: Output schema audit (`audit_output_schema.sh`).

### R11 — Macro definition-only by trait shape

- **Enforces**: C1.
- **Durable contract**: no function named `expand_*` in `scope-core/src/languages/` or `scope-core/src/extract/`. Macro symbols index under `kind: macro` (definition only). Macro invocations land as `calls.macro` / `calls.macro.scoped` edges to the macro symbol — never expanded. Because there is no `LanguagePlugin` trait to inspect (per [R7](#r7--indexer-level-dispatch-enforcement) the plugin surface is `impl LanguageId` arms + per-language modules), the negative shape is enforced over `impl LanguageId` arms + per-language modules + per-extractor modules, with the negative-trait-shape doc block on `id.rs` module header.
- **Where in the tree**: subset of [R12](#r12--type-system-free-trait-audit--process-spawn-denylist) trait-shape audit (`expand_*` arm of `audit_trait_shape.sh`).

### R12 — Type-system-free trait audit + process-spawn denylist

- **Enforces**: A1, A2, A3, B2, "no compiler/interpreter invocation" charter row, charter type-system limits.
- **Durable contract** (three audit scripts):
  - **Trait-shape audit** (`audit_trait_shape.sh`): no function named `infer_*`, `evaluate_*`, `solve_*`, `narrow_*`, `resolve_overload_*`, `expand_*` in scanned paths.
  - **Process-spawn denylist** (`audit_no_spawn.sh`): no `Command::new(` / `process::Command` / `std::process::Command` / `Command as <Alias>` in plugin / extractor / resolver / query paths (excluding `// scope:audit-allow process-spawn` tagged sites).
  - **Network denylist** (`audit_no_network.sh`): no `std::net::*` / `tokio::net::*` / `reqwest::` / `hyper::` / `ureq::` references in the same path-filtered set.
  - The only legitimate `Command::new("scope")` in the workspace lives in `gumiho-mudang-cli/src/commands/setup.rs` — outside the audit scope by path filter.
- **Honest limit**: trait-shape audit catches obvious names but not a helper named `compute_X_for_Y` that performs the same forbidden work. The process-spawn denylist catches literal `Command::new(` introductions but not a runtime-resolved `Command::new(env::var("CC")?)`. This is why A1–A3 + B2 + "no compiler/interpreter invocation" are `detectable`, not `mechanical`. [R8](#r8--confidence-audit-subcommand) is the symptom-side safety net.
- **Where in the tree**: `scripts/audit_trait_shape.sh`, `scripts/audit_no_spawn.sh`, `scripts/audit_no_network.sh`.

---

## Discipline-only rules

These three universal rules cannot be enforced by code; they require human judgment.

- **B1 (no flow analysis)** — flow analysis takes many syntactic forms; static detection of "is this plugin doing flow analysis" is itself an unbounded analysis problem. Safety net: code review against the explicit rule, plus [R8](#r8--confidence-audit-subcommand) catches the symptoms (overconfident edges) when the root cause is missed.
- **C2 (no version-specific compiler-quirk modelling)** — distinguishing "syntactic capture" from "semantic interpretation tied to a language version" requires knowledge of the specific language. Safety net: code review against the explicit rule. [R4](#r4--workspacecontext-typed-access-split-per-layer) closes the **leakage path** mechanically (the trait surface omits language-version fields), so a plugin that wanted to drift into C2 cannot reach the inputs — but the rule itself (don't interpret version-specific semantics from the syntax tree) is discipline-bound.
- **E3 (no heuristic hot-path optimization)** — distinguishing "honest fast path" from "approximate fast path" requires reading the implementation's intent. Safety net: code review against the explicit rule.

These three are explicit, listed, and bounded. Reviewers know exactly what to check. No other universal rule is delegated to discipline.

### Per-instance decisions (not universal rules)

The 15 framework-gotcha categories from [FRAMEWORK-PLAYBOOK Step 4](FRAMEWORK-PLAYBOOK.md) and the rule-temptation entries in language plugin docs are **per-instance decisions logged in the per-plugin templates** (`docs/frameworks/_TEMPLATE.md`, `docs/languages/_TEMPLATE.md`). They are not universal rules and they are not in any class of the inventory. The contract: every adopted plugin has a complete walkthrough table with explicit decisions. [R5](#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata)'s trait shape and [R0](#r0--schema-closures)'s metadata schema make those walkthroughs tractable but do not encode the decisions themselves — the decisions are framework-specific and live with the framework's doc.

---

## Where new work goes

Feature work — per-language depth, framework rollout, vector embeddings, time-travel queries, `mudang link`, `.js`/`.jsx` indexing, the self-correction cycle, the honesty audit, the layering audit — is queued in [`POST-REFACTOR-PLAN.md`](POST-REFACTOR-PLAN.md). Each item respects its own gate (language adoption flow per [`LANGUAGE-PLAYBOOK.md`](LANGUAGE-PLAYBOOK.md); framework adoption flow per [`FRAMEWORK-PLAYBOOK.md`](FRAMEWORK-PLAYBOOK.md); trigger frequency for cross-cutting items).
