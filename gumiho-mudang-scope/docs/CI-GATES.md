# CI Gates

Single source of truth for the quality gates the architectural refactor turns on. Each gate is owned by a refactor move (`ARCHITECTURAL-REFACTOR.md` R0–R12) and ships in that move's phase per `REFACTOR-STATUS.md`.

When a gate is active, the script or test path listed below is the **authoritative form**. The `justfile` recipes are convenience wrappers; this document is canonical. If `justfile` is removed or renamed, the script paths in this table remain the contract.

---

## Gate inventory

| Gate | Owner | Mechanism | Fails on | Local invocation | Status |
|---|---|---|---|---|---|
| Edge sealed | R1 | grep gate over `src/` | `Edge {` literal outside `core::graph` compiles | `just ci-edge-sealed` | planned (Phase A) |
| Builder requires fields | R1 | compile-fail test in `tests/compile_fail/` | `EdgeBuilder::build()` succeeds without `confidence` / `producer` / `pattern_id` | `just test-builder` | planned (Phase A) |
| Builder forbids status | R1 | compile-fail test | `EdgeBuilder::status(...)` compiles | `just test-builder` | planned (Phase A) |
| Insertable typestate | R3 | compile-fail test | `Graph::insert(RawEdge)` compiles; `InsertableEdge` constructor reachable outside resolver | `just test-typestate` | planned (Phase B) |
| Schema-version refusal | R0 | integration test in `tests/integration/schema_version.rs` | `scope status` runs against index whose `user_version > EXPECTED_SCHEMA_VERSION` | `just test-schema` | planned (Phase A) |
| Trait-shape audit | R12 | `scripts/audit_trait_shape.sh` | `LanguagePlugin` / `Extractor` has method named `infer_*`, `evaluate_*`, `solve_*`, `narrow_*`, `resolve_overload_*`, `expand_*` | `just ci-trait-shape` | planned (Phase B) |
| Process-spawn denylist | R12 | `scripts/audit_no_spawn.sh` | `Command::new(`, `process::Command`, `std::process::Command` appears in `src/languages/`, `src/frameworks/`, `src/core/parser.rs`, `src/core/extract*.rs`, `src/core/resolve*.rs` (excluding allowlist-tagged sites) | `just ci-no-spawn` | planned (Phase B) |
| Network denylist | R12 | `scripts/audit_no_network.sh` | `std::net::*`, `reqwest`, `hyper`, `tokio::net`, `ureq` linked into plugin / extractor / query paths | `just ci-no-network` | planned (Phase B) |
| Immutable source | R9 | `scripts/audit_immutable.sh` | `&mut` on source-related types (`&mut str`, `&mut Tree`, `&mut Source*`) at plugin trait surface | `just ci-immutable` | planned (Phase B) |
| WorkspaceContext shape | R4 | `scripts/audit_context_shape.sh` | `LanguageWorkspaceContext` exposes `edition`, `target`, `python_requires`, `go_directive`, `tsconfig_target`, `framework_versions` | `just ci-context-shape` | planned (Phase B) |
| No filesystem in plugin | R4 | grep gate | `std::fs::*`, `std::path::PathBuf::from`, `File::open` constructors in plugin code | `just ci-no-fs` | planned (Phase B) |
| Indexer-only dispatch | R7 | grep gate | plugin code reads file content for self-activation (no `read_to_string` etc. in plugin trait impls) | `just ci-dispatch` | planned (Phase B) |
| No `.scm` per framework | R5 | grep gate | `queries/<lang>/frameworks/` directory exists | `just ci-no-framework-scm` | planned (Phase C) |
| Pattern catalog audit | R5 | `scripts/audit_patterns.sh` | `Pattern` in `ALL_PATTERNS` has empty `id`, missing `available_in`, or unreferenced predicate fn | `just ci-patterns` | planned (Phase C) |
| Output schema audit | R10 | `scripts/audit_output_schema.sh` | output struct has field named `error`, `warning`, `diagnostic`, `is_valid`, `lint`, `correctness` | `just ci-output-schema` | planned (Phase D) |
| Macro definition-only | R11 | trait-shape audit (subset of R12) | trait method named `expand_*` or signature returning expanded source text | `just ci-trait-shape` | planned (Phase B) |
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
ci-gates: ci-trait-shape ci-no-spawn ci-no-network ci-immutable ci-context-shape ci-no-fs ci-dispatch ci-edge-sealed ci-no-framework-scm ci-patterns ci-output-schema test-builder test-typestate test-schema test-malformed

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

test-schema:
    cargo test --test schema_version

test-malformed:
    cargo test --test malformed_sources

audit-confidence:
    cargo run --release -- audit confidence
```

If `justfile` is later replaced by a `scripts/ci.sh` or removed entirely, the inventory above continues to define every gate. The recipes mirror the doc; the doc does not mirror the recipes.

---

## Allowlist convention (R12 spawn / network / fs gates)

When a legitimate exception exists (e.g., `Command::new("scope")` self-invocation in `src/commands/setup.rs:39` for the `scope setup` flow), the call site carries:

```rust
// scope:audit-allow process-spawn — self-invocation for `scope setup`
Command::new("scope")
```

Allowlist tags:

- `scope:audit-allow process-spawn`
- `scope:audit-allow network`
- `scope:audit-allow filesystem`

The audit script greps for the exact tag immediately preceding the call. Removing or renaming a tag fails the gate. Adding a new tag requires charter-amendment-grade rationale in the commit body — process-spawn / network / filesystem are the closest things in the codebase to charter hard limits.

The current allowlist is enumerated in each script's header comment. The doc does not enumerate sites — sites move; tags are the contract.

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
