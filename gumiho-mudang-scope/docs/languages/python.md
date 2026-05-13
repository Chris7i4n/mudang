# Language: Python

## Tree-sitter grammar

- **Crate / package**: `tree-sitter-python` (workspace-pinned version)
- **Source**: <https://github.com/tree-sitter/tree-sitter-python>
- **License**: MIT
- **Maturity assessment**: stable.
- **Known grammar gaps**: f-string interior expressions are captured as a single token stream; no recursive AST inside them (rule C1 makes this acceptable — definitions captured, expansions not).

## Depth target

- **Level**: surface
- **Post-refactor depth queue**: no

## Symbol kinds emitted

`function`, `class`, `method`, `const`, `type` (`TypeAlias`/`type` statements where the grammar exposes them), `property` (decorated `@property` methods).

## Edge kinds emitted

`calls`, `imports`, `extends`.

Pattern catalog (per `queries/python/edges.scm`):

| Pattern | pattern_id |
|---|---|
| `import os` | `imports.module` |
| `from x import y` | `imports.from` |
| `foo(...)` | `calls.function` |
| `obj.bar(...)` | `calls.method` |
| `class Foo(Bar):` | `extends.class` |

## Universal boundaries — compliance log

- **A1 / A2 / A3** (type system): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) forbids `fn infer_*` / `fn solve_*` / `fn narrow_*` / `fn resolve_overload_*` in the scanned plugin / extractor paths. The `infer_access(name)` helper that mapped Python naming convention to `access_kind` was renamed `access_from_name` in the same sprint to satisfy the audit — the function does textual classification, never inference. Python type hints are captured as text via `references_type` only at positions the query covers; overload resolution is the type-checker's job.
- **B1**: discipline-only per the universal class-3 list.
- **B2** (no runtime / dynamic resolution): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` forbids `fn evaluate_*`. `getattr` / `setattr` / `eval` are captured as syntactic call sites only.
- **B3** (no assumption of valid syntax): tree-sitter parser-recovery scanner active.
- **C1** (no macro / template expansion): **mechanically enforced after R11 sprint 0004** — the trait-shape audit forbids `fn expand_*`. Python has no macro system; decorator factories' return values are not modelled (the playbook's "definitions captured, expansions not" rule).
- **C2** (no version-specific compiler-quirk modelling): **mechanically enforced after R4**. `LanguageWorkspaceContext` has no `python_requires` accessor; reading it from the language layer is a compile error. `audit_context_shape.sh` (active CI gate) pins this. A single plugin handles every Python version the pinned tree-sitter grammar parses.
- **D1**: trivially compliant.
- **D2** (no best-guess fallback resolution): **mechanically enforced after R3**. Extractor emits `RawEdge` with `Confidence::Medium`; resolver assigns `status` in `{Resolved, Ambiguous, Dangling}` based on symbols-table lookup; Ambiguous emits one row per candidate target; `confidence` is preserved verbatim through resolution.
- **D3** (no symbol-id collision resolution by guessing): mechanically enforced via R0 surrogate `edge_id` PK + R3 multi-row `Ambiguous`.
- **E1**: trivially compliant.
- **E2** (no metadata interpretation in plugin): **mechanically enforced after R2 chunk 7**. Plugin returns `RawCaptures`; `decorators` reserved key carries `{name, args_text?}` verbatim from `@<decorator>(...)` syntax. The plugin does not evaluate decorator factory return values (rule C1) and does not interpret argument lists (rule E2). `annotations` and `template_calls` keys are **omitted** (no Python AST surface — Jinja `{% include %}` lives in templates, not Python source).
- **E3**: trivially compliant.
- **F1** (no multi-pass semantic analysis in plugin): **mechanically enforced after R3 typestate** — pipeline ordering `extract → resolve → write` encoded via `RawCaptures → RawEdge → InsertableEdge → Graph`; only `scope_graph::resolve` can construct `InsertableEdge` (R3 chunk 6 typestate gate).
- **F2** (no write-back to source): **mechanically enforced after R9 sprint 0004** — `scripts/audit_immutable.sh` forbids `&mut` on source-related types at the plugin / extractor surface.
- **F3** (no embedded-format parsing beyond tree-sitter): trivially compliant.
- **F4** (no language detection by content sniffing): mechanically enforced after R7 — dispatch is compile-time const, the plugin is invoked by extension only (`.py`, `.pyi`).

No `NEEDS REVIEW` outstanding for D2 / D3 / E2 / F1.

## Known gotchas

1. Module-level imports use the synthetic `{file_path}::__module__::function` `from_id` form. The historical dual-source fallback was retired at refactor close — the resolver now filters the synthetic ID strictly to `kind='imports'` (see `scope-graph/src/graph.rs::find_deps`).

## Test fixtures

- `gumiho-mudang-scope/tests/fixtures/languages/python/`
- `gumiho-mudang-scope/tests/integration/test_python.rs`
