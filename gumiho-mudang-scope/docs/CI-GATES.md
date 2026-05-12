# CI Gates

Single source of truth for the quality gates the architectural refactor turns on. Each gate is owned by a refactor move (`ARCHITECTURAL-REFACTOR.md` R0–R12) and ships in that move's phase per `REFACTOR-STATUS.md`.

When a gate is active, the script or test path listed below is the **authoritative form**. The `justfile` recipes are convenience wrappers; this document is canonical. If `justfile` is removed or renamed, the script paths in this table remain the contract.

---

## Gate inventory

| Gate | Owner | Mechanism | Fails on | Local invocation | Status |
|---|---|---|---|---|---|
| Edge sealed | R1 | `scripts/grep_edge_sealed.sh` (grep gate) | `Edge {` / `RawEdge {` / `InsertableEdge {` literal outside `scope-core/src/{edge,types}.rs` | `just ci-edge-sealed` | active (sprint 0001 — 2026-05-11) |
| Builder requires fields | R1 | compile-fail test `scope-core/tests/compile_fail/builder/missing_*.rs` | `EdgeBuilder::build()` succeeds without `confidence` / `producer` / `pattern_id` | `just test-builder` | active (sprint 0001 — 2026-05-11) |
| Builder forbids status | R1 | compile-fail test `scope-core/tests/compile_fail/builder/no_status_method.rs` | `EdgeBuilder::status(...)` compiles | `just test-builder` | active (sprint 0001 — 2026-05-11) |
| Insertable typestate | R3 | compile-fail test `scope-graph/tests/compile_fail/typestate/{insert_raw_edge,insertable_new_is_private,insertable_fields_private}.rs` | `Graph::insert_*` accepts `&[RawEdge]`; `InsertableEdge::new` callable outside `scope_graph::resolve`; `InsertableEdge { … }` struct-literal accepted outside the module | `just test-typestate` | active (sprint 0003 — 2026-05-11) |
| Trait-shape audit | R12 | `scripts/audit_trait_shape.sh` (grep gate over `gumiho-mudang-scope/scope-core/src/languages/` + `scope-core/src/extract/`; the post-A.4 plugin surface is `impl LanguageId` arms + per-language and per-extractor free functions, not a trait) | function named `infer_*`, `evaluate_*`, `solve_*`, `narrow_*`, `resolve_overload_*`, `expand_*` in the scanned paths (comment lines excluded; the name is the contract, no allowlist tag) | `just ci-trait-shape` | active (sprint 0004 — 2026-05-11) |
| Process-spawn denylist | R12 | `scripts/audit_no_spawn.sh` (grep gate, source-text scan) | `Command::new(` / `process::Command` / `std::process::Command` literal in `gumiho-mudang-scope/scope-core/src/languages/`, `gumiho-mudang-scope/scope-core/src/frameworks/` (when introduced in sprint 0005), `gumiho-mudang-scope/scope-core/src/parser.rs`, `gumiho-mudang-scope/scope-core/src/extract/`, `gumiho-mudang-scope/scope-graph/src/resolve/` (excluding allowlist-tagged sites) | `just ci-no-spawn` | active (sprint 0004 — 2026-05-11) |
| Network denylist | R12 | `scripts/audit_no_network.sh` (grep gate, source-text scan; `cargo-deny` is **not** wired as a refactor gate — dep-tree hygiene is tooling, not the R12 contract) | `std::net::*` / `tokio::net::*` / `reqwest::` / `hyper::` / `ureq::` symbol references in the path-filtered set used by the process-spawn gate (plugin, extractor, resolver, query) excluding allowlist-tagged sites | `just ci-no-network` | active (sprint 0004 — 2026-05-11) |
| Immutable source | R9 | `scripts/audit_immutable.sh` (grep gate, source-text scan) | `&mut str` / `&mut String` / `&mut tree_sitter::Tree` / `&mut Tree` / `&mut Source*` token in `gumiho-mudang-scope/scope-core/src/languages/`, `scope-core/src/extract/`, `scope-core/src/parser.rs` (excluding allowlist-tagged sites with the `scope:audit-allow mutable-source` tag) | `just ci-immutable` | active (sprint 0004 — 2026-05-11) |
| WorkspaceContext shape | R4 | `scripts/audit_context_shape.sh` | `LanguageWorkspaceContext` exposes `edition`, `target`, `python_requires`, `go_directive`, `tsconfig_target`, `framework_versions` | `just ci-context-shape` | active (sprint 0002 — 2026-05-11) |
| No filesystem in plugin | R4 | `scripts/grep_no_fs.sh` | `std::fs::*`, `std::path::PathBuf::from`, `File::open` constructors in plugin code (`scope-core/src/languages/`) without `// scope:audit-allow filesystem` tag | `just ci-no-fs` | active (sprint 0002 — 2026-05-11) |
| Indexer-only dispatch | R7 | `scripts/grep_dispatch.sh` | content readers (`read_to_string`, `read_to_end`, `read_line`, `BufRead`) in `scope-core/src/languages/`, or `register_languages!` / `dispatch_extension` / `dispatch_shebang` defined outside `languages/dispatch.rs` | `just ci-dispatch` | active (sprint 0002 — 2026-05-11) |
| No `.scm` per framework | R5 | grep gate | `queries/<lang>/frameworks/` directory exists | `just ci-no-framework-scm` | planned (Phase C) |
| Pattern catalog audit | R5 | `scripts/audit_patterns.sh` | `Pattern` in `ALL_PATTERNS` has empty `id`, missing `available_in`, or unreferenced predicate fn | `just ci-patterns` | planned (Phase C) |
| Output schema audit | R10 | `scripts/audit_output_schema.sh` | output struct has field named `error`, `warning`, `diagnostic`, `is_valid`, `lint`, `correctness` | `just ci-output-schema` | planned (Phase D) |
| Macro definition-only | R11 | trait-shape audit (subset of R12) | function named `expand_*` in `gumiho-mudang-scope/scope-core/src/languages/` or `scope-core/src/extract/` (the trait-shape gate's `expand_*` arm) | `just ci-trait-shape` | active (sprint 0004 — 2026-05-11) |
| Malformed-source harness | R6 | `cargo test --test malformed_sources` | plugin panics on any fixture; partial-malformed fixture produces empty `skipped_ranges`; snapshot diff on recorded reason / range | `just test-malformed` | planned (Phase E) |
| Confidence audit | R8 | `scope audit confidence` (subcommand) | precision below tier target (`high ≥ 95%`, `medium ≥ 70%`, `low` no minimum) per `(kind, producer, pattern_id)` | `just audit-confidence` | planned (Phase D) |

---

## Status legend

- `planned` — gate spec is in `ARCHITECTURAL-REFACTOR.md`; not yet implemented. Phase column ties shipping to its R-move.
- `active` — script + recipe land on main; CI runs it; failures block merge.
- `disabled` — gate exists but is currently bypassed; record reason in commit body. Disabled is never silent — the doc row's status column reflects it.

---

## Authority

This document is the contract. Implementation order for any gate:

1. R-move ships per `REFACTOR-STATUS.md`.
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
ci-gates: ci-trait-shape ci-no-spawn ci-no-network ci-immutable ci-context-shape ci-no-fs ci-dispatch ci-edge-sealed ci-no-framework-scm ci-patterns ci-output-schema test-builder test-typestate test-malformed

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
    cargo run --release -- audit confidence
```

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

The current allowlist is enumerated in each script's header comment. The doc does not enumerate sites — sites move; tags are the contract. As of sprint 0004, the audited paths contain **zero** allowlist entries; the only in-tree `Command::new("scope")` self-invocation lives in `gumiho-mudang-cli/src/commands/setup.rs` (out of audit scope by path).

---

## Where to look when a gate fails

- **Compile error in `cargo test --test compile_fail_*`** → check the move's typestate / sealed-struct rules in `ARCHITECTURAL-REFACTOR.md`.
- **`cargo test` failure** → check the move's acceptance bullets.
- **Grep-gate failure** → re-read this document's row for the gate; the "Fails on" column states the rule.
- **`scope audit confidence` failure** → R8's tier targets are violated; localize via `(producer, pattern_id)` in the report.

---

## Adding a new gate

1. Append a row to the inventory (`planned`).
2. Reference the owning R-move (or open a new R-move via amendment to `ARCHITECTURAL-REFACTOR.md` if no existing move owns the rule).
3. Author the script / test at the listed path.
4. Add the recipe.
5. Wire CI.
6. Flip status to `active` in the same commit.

Gates that do not appear in this inventory do not exist for the purpose of contract review.
