# Language: Rust

## Tree-sitter grammar

- **Crate / package**: `tree-sitter-rust` (workspace-pinned version)
- **Source**: <https://github.com/tree-sitter/tree-sitter-rust>
- **License**: MIT
- **Maturity assessment**: stable. Upstream cadence is moderate, breaking changes are rare in the surface this plugin uses.
- **Known grammar gaps**:
  - `macro_rules!` bodies are opaque under tree-sitter — content inside `macro_rules!` is captured as a token tree, not a parsed expression. The plugin records a `plugin_skip:rust:unparseable_macro_body` skipped range for any unparseable body (R6 / sprint 0008 will exercise this).
  - `attribute_item` attaches as a PRECEDING SIBLING of the item node, not as a direct child — relevant to the `annotations` metadata population (handled by the chunk-3b sibling walk in `scope-core/src/languages/rust_lang.rs::extract_metadata`).

## Depth target

- **Level**: surface
- **Post-refactor depth queue**: no
- **Promotion / demotion history**: empty.

## Symbol kinds emitted

`function`, `struct`, `enum`, `trait` (post-R0; pre-R0 was coerced to `interface`), `type` (alias), `const`, `method`, `property` (struct fields where useful).

## Edge kinds emitted

`calls`, `imports`, `references_type`, `extends` (none today; reserved), `implements` (`impl Trait for Type`).

Pattern catalog (per `queries/rust/edges.scm` + `scope-core/src/extract/rust_lang.rs`):

| Pattern | pattern_id |
|---|---|
| use `path::Item` | `imports.path` |
| use `path::Item as Alias` | `imports.aliased` |
| `func(...)` | `calls.function` |
| `Type::func(...)` | `calls.function.scoped` |
| `obj.method(...)` | `calls.method` |
| `name!(...)` | `calls.macro` |
| `path::name!(...)` | `calls.macro.scoped` |
| struct field `T` | `references_type.field` |
| fn param `T` | `references_type.param` |
| fn return `T` | `references_type.return` |
| match arm `T { .. }` | `references.match.struct` |
| match arm `T(..)` | `references.match.tuple` |
| `impl Trait for Type` | `implements.trait_impl_block` |

## Universal boundaries — compliance log

R-move shorthand: [R0](../ARCHITECTURAL-REFACTOR.md#r0--schema-closures), [R1](../ARCHITECTURAL-REFACTOR.md#r1--typed-edge-insertion-api), [R2](../ARCHITECTURAL-REFACTOR.md#r2--languageplugin-output-type-closure), [R3](../ARCHITECTURAL-REFACTOR.md#r3--pipeline-ordering-via-type-state), [R4](../ARCHITECTURAL-REFACTOR.md#r4--workspacecontext-typed-access-split), [R7](../ARCHITECTURAL-REFACTOR.md#r7--indexer-level-dispatch-enforcement).

- **A1** (no type inference): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) forbids `fn infer_*` in the scanned plugin / extractor paths. The historical `LanguageId::infer_symbol_kind` and `is_likely_generic_param`-callers were renamed (the former to `symbol_kind_for_node` in the same sprint) — the function is a static node-kind lookup, not inference. Rust type identifiers are captured as text via `type_ref` captures only; single uppercase letters are filtered as generic params (`is_likely_generic_param`).
- **A2** (no constraint solving): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` forbids `fn solve_*`. Trait bounds in generic params are captured as text, never resolved.
- **A3** (no type-system name resolution): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` forbids `fn narrow_*` / `fn resolve_overload_*`. No method dispatch on type. Method calls captured by syntactic position only.
- **B1** (no flow analysis): discipline-only per the universal class-3 list.
- **B2** (no runtime / dynamic resolution): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` forbids `fn evaluate_*`. Rust has no runtime dispatch surface for the plugin to model.
- **B3** (no assumption of valid syntax): tree-sitter parser-recovery scanner emits `tree_sitter_error:syntax_error` / `tree_sitter_error:missing_node` skipped ranges; plugin never panics. R6 harness (sprint 0008) is the enforcement gate.
- **C1** (no macro expansion): **mechanically enforced after R11 sprint 0004** — `scripts/audit_trait_shape.sh` forbids `fn expand_*` in the scanned plugin / extractor paths; combined with the chunk-7 extractor closure (the extractor is the only `EdgeKind`-aware site and has no expander entry point), expansion is unreachable from the plugin layer. At the data level: `macro_invocation` captures only the macro name (`@macro_name`); the body is not walked. `macro_rules!` bodies are recorded as plugin skips. Macro symbols (`kind: macro`) and `calls.macro` / `calls.macro.scoped` edges land per R0 / R2.
- **C2** (no version-specific compiler-quirk modelling): **mechanically enforced after R4**. `LanguageWorkspaceContext` (the language-facing context trait) has no accessor for `edition`; reading it from the language layer is a compile error. The `cargo_toml` reader inside `scope-core/src/workspace/` belongs to the indexer-side context, not the plugin-side, and `audit_context_shape.sh` (active CI gate) pins this.
- **D1** (no cross-file resolution beyond config): trivially compliant — resolution is R3's job, not the extractor's.
- **D2** (no best-guess fallback resolution): **mechanically enforced after R3**. The extractor emits a `RawEdge` with the candidate name and a `confidence` value derived from pattern precision (`Confidence::Medium` for the syntactic patterns above). The R3 resolver (`scope-graph::resolve::Resolver`) sets `status='Resolved' | 'Ambiguous' | 'Dangling'` based on the symbols-table lookup outcome; on `Ambiguous` it emits one row per candidate target (`R3 acceptance bullet 3`). The extractor's `confidence` passes through `verbatim` (R3 acceptance bullet 4), and `confidence` × `status` are orthogonal columns. There is no code path that downgrades `Confidence` or picks a single candidate during resolution.
- **D3** (no symbol-id collision resolution by guessing): mechanically enforced via R0 surrogate `edge_id` PK + R3 multi-row `Ambiguous`. Collisions are representable, not smoothed.
- **E1** (no semantic correctness assertions): trivially compliant — no diagnostic fields anywhere in the output schema (R10's audit will pin this in sprint 0006).
- **E2** (no metadata interpretation in plugin): **mechanically enforced after R2 chunk 7**. Plugin returns `RawCaptures { matches, metadata, skipped_ranges }`; metadata `annotations` reserved-key entries carry `{name, args_text?}` verbatim from `attribute_item` (e.g., `#[derive(Debug)]` → `{name: "derive", args_text: "(Debug)"}`). The plugin does not interpret the argument list. `decorators` and `template_calls` keys are **omitted** entirely (no Rust AST surface for either).
- **E3** (no heuristic optimization for hot paths): trivially compliant.
- **F1** (no multi-pass semantic analysis in plugin): **mechanically enforced after R3 typestate**. The pipeline ordering `extract → resolve → write` is encoded by `RawCaptures → RawEdge → InsertableEdge → Graph`; only the resolver module owns `InsertableEdge`'s constructor (R3 chunk 6 typestate gate). The plugin layer cannot loop back to re-walk with resolution information.
- **F2** (no write-back to source): **mechanically enforced after R9 sprint 0004** — `scripts/audit_immutable.sh` (gate `ci-immutable`) forbids `&mut str` / `&mut String` / `&mut tree_sitter::Tree` / `&mut Tree` / `&mut Source*` in the scanned plugin / extractor paths. Source data crosses the plugin surface read-only.
- **F3** (no embedded-format parsing beyond tree-sitter): trivially compliant — attribute args captured as raw text, never parsed as TOML/YAML.
- **F4** (no language detection by content sniffing): mechanically enforced after R7 — dispatch is compile-time const, the plugin is invoked by extension only (`.rs`).

No `NEEDS REVIEW` outstanding for D2 / D3 / E2 / F1.

## Known gotchas

1. `is_likely_generic_param` filters single-uppercase-letter type references because tree-sitter-rust uses `type_identifier` for both generic param names and concrete types. Trade-off: ~24% precision win on `references_type` against the vanishingly rare real one-letter Rust type.
2. `attribute_item` is a PRECEDING SIBLING of the item, not a direct child; the chunk-3b sibling-walk implementation is correct. A future grammar bump that changes attachment will require updating `scope-core/src/languages/rust_lang.rs::extract_metadata`.
3. `impl Trait for Type` requires walking the parsed tree (not the edges query) because the extractor needs symbol-list lookup for `from_id` resolution. `scope-core/src/extract/rust_lang.rs::extract_rust_trait_impl_edges` owns this.

## Test fixtures

- `gumiho-mudang-scope/tests/fixtures/languages/rust_lang/`
- `gumiho-mudang-scope/tests/integration/test_rust_lang.rs`
