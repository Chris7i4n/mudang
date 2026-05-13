# Language: TypeScript

## Tree-sitter grammar

- **Crate / package**: `tree-sitter-typescript` (workspace-pinned version; uses `LANGUAGE_TYPESCRIPT`, not `LANGUAGE_TSX`)
- **Source**: <https://github.com/tree-sitter/tree-sitter-typescript>
- **License**: MIT
- **Maturity assessment**: stable.
- **Known grammar gaps**: JSX/TSX is not parsed by `LANGUAGE_TYPESCRIPT`; `.tsx` files lose component-call AST detail. Surface adoption accepts this — the `template_calls` reserved metadata key is omitted entirely for TS (absent key ≠ empty array), preserving the playbook distinction between "no AST surface" and "AST has no instances".

## Depth target

- **Level**: surface
- **Post-refactor depth queue**: no

## Symbol kinds emitted

`function`, `class`, `method`, `interface`, `type` (alias), `const`, `property`.

## Edge kinds emitted

`calls`, `imports`, `instantiates`, `extends`, `implements`, `references_type`.

Pattern catalog (per `queries/typescript/edges.scm`):

| Pattern | pattern_id |
|---|---|
| `import { x } from 'y'` | `imports.named` |
| `foo(...)` | `calls.function` |
| `obj.method(...)` | `calls.method` |
| `a.b.method(...)` | `calls.method.chained` |
| `new Foo(...)` | `instantiates.class` |
| `class C extends B` | `extends.class` |
| `class C implements I` | `implements.interface` |
| `this.method(...)` | `calls.method.this` |
| `: T` type annotation | `references_type.annotation` |

## Universal boundaries — compliance log

- **A1 / A2 / A3** (type system): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) forbids `fn infer_*` / `fn solve_*` / `fn narrow_*` / `fn resolve_overload_*` in the scanned plugin / extractor paths. Type annotations captured as text via `references_type.annotation`; conditional-type evaluation and overload resolution belong to the TypeScript compiler, never to scope.
- **B1**: discipline-only per the universal class-3 list.
- **B2** (no runtime / dynamic resolution): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` forbids `fn evaluate_*`.
- **B3** (no assumption of valid syntax): tree-sitter parser-recovery scanner active.
- **C1** (no macro / template expansion): **mechanically enforced by R11** — `scripts/audit_trait_shape.sh` forbids `fn expand_*`. TS decorator factories' return values are not modelled — the plugin captures the decorator name + raw `args_text` and stops.
- **C2** (no version-specific compiler-quirk modelling): **mechanically enforced after R4**. `LanguageWorkspaceContext` has no `tsconfig_target` accessor; reading it from the language layer is a compile error.
- **D1**: trivially compliant.
- **D2** (no best-guess fallback resolution): **mechanically enforced after R3**. Extractor emits `RawEdge` with `Confidence::Medium`; resolver assigns `status` in `{Resolved, Ambiguous, Dangling}`; Ambiguous emits one row per candidate; `confidence` preserved verbatim.
- **D3** (no symbol-id collision resolution by guessing): mechanically enforced via R0 surrogate `edge_id` PK + R3 multi-row `Ambiguous`.
- **E1**: trivially compliant.
- **E2** (no metadata interpretation in plugin): **mechanically enforced by R2**. Plugin returns `RawCaptures`; `decorators` reserved key carries `{name, args_text?}` verbatim from class-member and class-level decorators. Decorator capture uses the direct-child + preceding-sibling walk; a parent-walk fallback would bleed sibling decorators across methods. `annotations` and `template_calls` keys are **omitted** (no AST surface in `LANGUAGE_TYPESCRIPT`).
- **E3**: trivially compliant.
- **F1** (no multi-pass semantic analysis in plugin): **mechanically enforced by R3**.
- **F2** (no write-back to source): **mechanically enforced by R9** — `scripts/audit_immutable.sh` forbids `&mut` on source-related types at the plugin / extractor surface.
- **F3** (no embedded-format parsing beyond tree-sitter): trivially compliant.
- **F4** (no language detection by content sniffing): mechanically enforced after R7 — dispatch is compile-time const, the plugin is invoked by extension only (`.ts`).

No `NEEDS REVIEW` outstanding for D2 / D3 / E2 / F1.

## Known gotchas

1. Decorator capture uses tree-sitter-typescript's direct-child walk only. The parent-walk fallback was deleted in commit 733b16c after the review flagged that `class C { @A a(); @B b(); c() }` incorrectly assigned all decorators to all three methods.
2. The grammar's `LANGUAGE_TYPESCRIPT` variant does not parse JSX; this is intentional per the depth target.

## Test fixtures

- `gumiho-mudang-scope/tests/fixtures/languages/typescript/`
- `gumiho-mudang-scope/tests/integration/test_typescript.rs`
