# CI Gates

Single source of truth for the quality gates the architecture enforces. Each gate is owned by an R-entry in [`ENFORCEMENT-MAP.md`](ENFORCEMENT-MAP.md). Per-gate provenance lives in this doc's Status column and in git history.

When a gate is active, the script or test path listed below is the **authoritative form**. The `justfile` recipes are convenience wrappers; this document is canonical. If `justfile` is removed or renamed, the script paths in this table remain the contract.

---

## Gate inventory

| Gate | Owner | Mechanism | Fails on | Local invocation | Status |
|---|---|---|---|---|---|
| Edge sealed | R1 | `scripts/grep_edge_sealed.sh` (grep gate) | `Edge {` / `RawEdge {` / `InsertableEdge {` literal outside `scope-core/src/{edge,types}.rs` | `just ci-edge-sealed` | active |
| Builder requires fields | R1 | compile-fail test `scope-core/tests/compile_fail/builder/missing_*.rs` | `EdgeBuilder::build()` succeeds without `confidence` / `producer` / `pattern_id` | `just test-builder` | active |
| Builder forbids status | R1 | compile-fail test `scope-core/tests/compile_fail/builder/no_status_method.rs` | `EdgeBuilder::status(...)` compiles | `just test-builder` | active |
| Insertable typestate | R3 | compile-fail test `scope-graph/tests/compile_fail/typestate/{insert_raw_edge,insertable_new_is_private,insertable_fields_private}.rs` | `Graph::insert_*` accepts `&[RawEdge]`; `InsertableEdge::new` callable outside `scope_graph::resolve`; `InsertableEdge { … }` struct-literal accepted outside the module | `just test-typestate` | active |
| Trait-shape audit | R12 | `scripts/audit_trait_shape.sh` (grep gate over `gumiho-mudang-scope/scope-core/src/languages/` + `scope-core/src/extract/` + `scope-core/src/frameworks/`; the post-A.4 plugin surface is `impl LanguageId` arms + per-language / per-extractor free functions + `FrameworkPlugin` impls, not a single trait) | function named `infer_*`, `evaluate_*`, `solve_*`, `narrow_*`, `resolve_overload_*`, `expand_*` in the scanned paths (comment lines excluded; the name is the contract, no allowlist tag) | `just ci-trait-shape` | active |
| Process-spawn denylist | R12 | `scripts/audit_no_spawn.sh` (grep gate, source-text scan) | `Command::new(` / `process::Command` / `std::process::Command` literal in `gumiho-mudang-scope/scope-core/src/languages/`, `gumiho-mudang-scope/scope-core/src/frameworks/` (when introduced), `gumiho-mudang-scope/scope-core/src/parser.rs`, `gumiho-mudang-scope/scope-core/src/extract/`, `gumiho-mudang-scope/scope-graph/src/resolve/` (excluding allowlist-tagged sites) | `just ci-no-spawn` | active |
| Network denylist | R12 | `scripts/audit_no_network.sh` (grep gate, source-text scan; `cargo-deny` is **not** wired as a refactor gate — dep-tree hygiene is tooling, not the R12 contract) | `std::net::*` / `tokio::net::*` / `reqwest::` / `hyper::` / `ureq::` symbol references in the path-filtered set used by the process-spawn gate (plugin, extractor, resolver, query) excluding allowlist-tagged sites | `just ci-no-network` | active |
| Immutable source | R9 | `scripts/audit_immutable.sh` (grep gate, source-text scan) | `&mut str` / `&mut String` / `&mut tree_sitter::Tree` / `&mut Tree` / `&mut Source*` token in `gumiho-mudang-scope/scope-core/src/languages/`, `scope-core/src/extract/`, `scope-core/src/parser.rs` (excluding allowlist-tagged sites with the `scope:audit-allow mutable-source` tag) | `just ci-immutable` | active |
| WorkspaceContext shape | R4 | `scripts/audit_context_shape.sh` | `LanguageWorkspaceContext` exposes `edition`, `target`, `python_requires`, `go_directive`, `tsconfig_target`, `framework_versions` | `just ci-context-shape` | active |
| No filesystem in plugin | R4 | `scripts/grep_no_fs.sh` | `std::fs::*`, `std::path::PathBuf::from`, `File::open` constructors in plugin code (`scope-core/src/languages/`) without `// scope:audit-allow filesystem` tag | `just ci-no-fs` | active |
| Indexer-only dispatch | R7 | `scripts/grep_dispatch.sh` | content readers (`read_to_string`, `read_to_end`, `read_line`, `BufRead`) in `scope-core/src/languages/`, or `register_languages!` / `dispatch_extension` / `dispatch_shebang` defined outside `languages/dispatch.rs` | `just ci-dispatch` | active |
| No `.scm` per framework | R5 | `scripts/audit_no_framework_scm.sh` (`find -type d` for `queries/*/frameworks` and `queries/frameworks` paths anywhere under `gumiho-mudang-scope/`) | any `queries/<lang>/frameworks/` directory exists in the scope sub-crate tree | `just ci-no-framework-scm` | active |
| Pattern catalog audit | R5 | `scripts/audit_patterns.sh` (awk-extracts every `Pattern { ... }` literal — single-line or multi-line — under `scope-core/src/frameworks/` and `scope-core/tests/synthetic_framework/`; inspects each block for `id: ""` literals) | any `Pattern { ... }` literal whose `id:` slot is the empty-string literal `""` (missing `available_in` or `predicate` fail at compile time because the fields are non-`Option`) | `just ci-patterns` | active |
| Output schema audit | R10 | `scripts/audit_output_schema.sh` (awk state-machine tracks `struct <Name> { ... }` blocks under `gumiho-mudang-cli/src/output/` + `gumiho-mudang-cli/src/commands/`; emits any field declaration whose name is in the banned set) | output struct has field named `error`, `warning`, `diagnostic`, `is_valid`, `lint`, `correctness` | `just ci-output-schema` | active |
| Macro definition-only | R11 | trait-shape audit (subset of R12) | function named `expand_*` in `gumiho-mudang-scope/scope-core/src/languages/` or `scope-core/src/extract/` (the trait-shape gate's `expand_*` arm) | `just ci-trait-shape` | active |
| Malformed-source harness | R6 | `cargo test --test malformed_sources` | plugin panics on any fixture; partial-malformed fixture produces empty `skipped_ranges`; snapshot diff on recorded reason / range | `just test-malformed` | active |
| Confidence audit | R8 | `scope audit confidence` (subcommand) — integration test suite `gumiho-mudang-cli/tests/integration/test_audit_confidence.rs` exercises every surface (emit-sample / label / drift gate / schema_version reject / tier gate fail). The mechanical regression gate this row activates is the test suite; the **continuous re-audit cycle** (precision drift detection over time via committed labelled samples + edge_id-stable join key) is queued in [`BACKLOG.md` § Priority 1 — Self-correction cycle](BACKLOG.md#priority-1--self-correction-cycle). | wiring break of the audit subcommand, `SampleRecord` field-order drift (breaks external labellers), drift gate removal, tier-gate threshold loosening, schema_version bump without contract update | `just audit-confidence` | active |
| Charter sweep | charter §2 + §3 inv 8 | `scripts/gate_charter.sh` — narrow-grep gate. Ten checks, each targeting a specific shim shape: namespace-synth import path (`gumiho_mudang_scope::core::*`), `pub mod core { … }` re-namespace, dead `pub use scope_X as scope_X_crate` façade aliases, `pub type Edge = …` transitional alias, `Edge` import from scope_core/types, `INSERT OR IGNORE` in production SQL, schema-detector fn names (`has_legacy_*` / `pre_r[0-9]+_*`), `command_label: &'static str` deprecation-alias parameter, `scope impact` CLI subcommand wiring, `__module__::class` synthetic-ID fallback in graph query layer. The gate stays narrow on purpose — catching only the active shim shapes, not charter-aligned prose (loose substrings like `legacy` / `compat` / `backward` fire on migration notes, language-feature terminology, and directional terms). | any of the ten checks finds a hit (re-introduction of a forbidden shim) | `just gate-charter` | active |
| Doc-sync | R13 | `scripts/gate_doc_sync.sh` — narrow-grep gate. Initial checks (sprint 0001): **enforcement-map-paths** (every backticked file path cited in `ENFORCEMENT-MAP.md` resolves on disk), **ci-gates-recipes** (every `active` row's `just <recipe>` exists in `justfile`), **doc-relative-links** (every relative markdown link under `gumiho-mudang-scope/docs/` resolves), **cycle-docs-indexed** (`SELF-CORRECTION-CYCLE.md` + `SELF-CORRECTION-STATE.md` referenced from docs `README.md` when present). Extension protocol for later Priority 1 sprints: one new `check_<short_name>` function per drift shape, per [`SELF-CORRECTION-CYCLE.md` § Extending the doc-sync gate](SELF-CORRECTION-CYCLE.md#extending-the-doc-sync-gate). | any check finds a drift hit (named doc value ≠ named code value, or named doc surface points to a non-existent path) | `just gate-doc-sync` | active |

---

## Status legend

- `planned` — gate spec is in `ENFORCEMENT-MAP.md`; not yet implemented. The owning R-entry names the technique it enforces.
- `active` — script + recipe land on main; CI runs it; failures block merge.
- `disabled` — gate exists but is currently bypassed; record reason in commit body. Disabled is never silent — the doc row's status column reflects it.

---

## Authority

This document is the contract. Implementation order for any new gate:

1. The owning R-entry (from `ENFORCEMENT-MAP.md`, or appended as the next free `### R<n>` by the introducing sprint per [`sprints/README.md` § 7.5](sprints/README.md#75-enforcement-map-update)) ships in its own sprint.
2. Gate's script or test is authored at the path listed in the inventory.
3. `justfile` recipe is added.
4. CI workflow calls the recipe (or the script directly).
5. Status column flips `planned` → `active` in this document, in the same commit.

When a gate is active, the **script path** is the durable contract; the `justfile` recipe is convenience and may be removed without impacting CI. New gates are appended to the inventory first, then implemented.

---

## Justfile recipe shape

The recipes are thin. The doc is canonical; the recipes call out to the canonical script paths.

```just
# Run every active CI gate. CI calls this; humans can too.
ci-gates: ci-trait-shape ci-no-spawn ci-no-network ci-immutable ci-context-shape ci-no-fs ci-dispatch ci-edge-sealed ci-no-framework-scm ci-patterns ci-output-schema test-builder test-typestate test-malformed audit-confidence

ci-trait-shape:
    ./scripts/audit_trait_shape.sh

ci-no-spawn:
    ./scripts/audit_no_spawn.sh

ci-no-network:
    ./scripts/audit_no_network.sh

ci-immutable:
    ./scripts/audit_immutable.sh

ci-context-shape:
    ./scripts/audit_context_shape.sh

ci-no-fs:
    @scripts/grep_no_fs.sh

ci-dispatch:
    @scripts/grep_dispatch.sh

ci-edge-sealed:
    @scripts/grep_edge_sealed.sh

ci-no-framework-scm:
    @test ! -d queries/*/frameworks || (echo "queries/<lang>/frameworks/ is forbidden by R5" && exit 1)

ci-patterns:
    ./scripts/audit_patterns.sh

ci-output-schema:
    ./scripts/audit_output_schema.sh

test-builder:
    cargo test --test compile_fail_builder

test-typestate:
    cargo test --test compile_fail_typestate

test-malformed:
    cargo test --test malformed_sources

audit-confidence:
    cargo test -p gumiho-mudang-cli --test test_audit_confidence
```

`audit-confidence` runs the integration suite, **not** an end-to-end labelled-sample replay. The suite is the regression gate; the continuous re-audit cycle (committed labelled samples, edge_id-stable join key, precision-drift detection over time) is queued in [`BACKLOG.md` § Priority 1 — Self-correction cycle](BACKLOG.md#priority-1--self-correction-cycle).

If `justfile` is later replaced by a `scripts/ci.sh` or removed entirely, the inventory above continues to define every gate. The recipes mirror the doc; the doc does not mirror the recipes.

---

## Allowlist convention (R12 spawn / network / fs gates)

The path filter on each R12 source-text gate scans only plugin / extractor / resolver / query paths (per the "Fails on" column of each row). Call sites **outside** those paths — for example `gumiho-mudang-cli/src/commands/setup.rs:39` where the `scope setup` subcommand spawns a `scope` subprocess — are out of audit scope by **path exclusion**, not by allowlist tag. The tag mechanism exists for the rare case where a denylisted construct must legitimately live **inside** an audited path.

When such an in-scope exception exists, the call site carries:

```rust
// scope:audit-allow process-spawn — <one-line rationale>
Command::new("…")
```

Allowlist tags:

- `scope:audit-allow process-spawn`
- `scope:audit-allow network`
- `scope:audit-allow filesystem`

Tag-placement rule:

- The tag goes on the line **immediately preceding the denylisted construct itself** (the `Command::new(…)` / `std::net::…` / `std::fs::…` line that the audit grep would otherwise flag). The audit greps lines in scanned paths; the tag is matched lexically on the preceding line.
- **Wrapping a denylisted construct in a helper to evade the gate is forbidden.** If a helper inside an audited path internally calls a denylisted construct, the tag goes on the inner call (the construct itself), never on the helper's signature or its call sites. Hiding `Command::new(...)` behind `fn run_scope(args: &[&str])` and tagging `run_scope` is process-failure-grade: it silently widens the exception surface and breaks the audit's source-text contract. There is no "transitive call audit" by design — the gate operates lexically, and the tag follows the same locality.
- A construct in an audited path that lacks a tag fails the gate. Removing or renaming a tag fails the gate. Adding a new tag site requires charter-amendment-grade rationale in the commit body — process-spawn / network / filesystem are the closest things in the codebase to charter hard limits.

The current allowlist is enumerated in each script's header comment. The doc does not enumerate sites — sites move; tags are the contract.the audited paths contain **zero** allowlist entries; the only in-tree `Command::new("scope")` self-invocation lives in `gumiho-mudang-cli/src/commands/setup.rs` (out of audit scope by path).

---

## Where to look when a gate fails

- **Compile error in `cargo test --test compile_fail_*`** → check the owning R-entry's typestate / sealed-struct contract in `ENFORCEMENT-MAP.md`.
- **`cargo test` failure** → check the owning R-entry's durable contract.
- **Grep-gate failure** → re-read this document's row for the gate; the "Fails on" column states the rule.
- **`scope audit confidence` failure** → R8's tier targets are violated; localize via `(producer, pattern_id)` in the report.

---

## Adding a new gate

1. Append a row to the inventory (`planned`).
2. Reference the owning R-entry (or append a new `### R<n>` in `ENFORCEMENT-MAP.md` if no existing entry owns the rule, per the end-of-sprint update gate in `sprints/README.md` § 7.5).
3. Author the script / test at the listed path.
4. Add the recipe.
5. Wire CI.
6. Flip status to `active` in the same commit.

Gates that do not appear in this inventory do not exist for the purpose of contract review.
