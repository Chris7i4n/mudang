# Language: Java

## Tree-sitter grammar

- **Crate / package**: `tree-sitter-java` (workspace-pinned version)
- **Source**: <https://github.com/tree-sitter/tree-sitter-java>
- **License**: MIT
- **Maturity assessment**: stable.

## Depth target

- **Level**: surface
- **Post-refactor depth queue**: no

## Symbol kinds emitted

`class`, `interface`, `method`, `function` (static methods of utility classes are still `method`), `const`, `enum`.

## Edge kinds emitted

`calls`, `imports`, `instantiates`, `extends`, `implements`, `references_type`, `references` (switch-case enum constants).

Pattern catalog (per `queries/java/edges.scm`):

| Pattern | pattern_id |
|---|---|
| `import com.x.Y` | `imports.scoped` |
| `service.process()` | `calls.method` |
| `process()` | `calls.function` |
| `this.method()` | `calls.method.this` |
| `super.method()` | `calls.method.super` |
| `new Foo()` | `instantiates.class` |
| `class C extends B` | `extends.class` |
| `class C implements I` | `implements.interface` |
| `interface I extends J` | `extends.interface` |
| field type `T` | `references_type.field` |
| param type `T` | `references_type.param` |
| `case SUCCESS:` | `references.switch.enum` |

## Universal boundaries — compliance log

- **A1 / A2 / A3** (type system): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) forbids `fn infer_*` / `fn solve_*` / `fn narrow_*` / `fn resolve_overload_*` in the scanned plugin / extractor paths. Java generics are captured as text; the language layer never resolves bounds or overload candidates.
- **B1**: discipline-only per the universal class-3 list.
- **B2** (no runtime / dynamic resolution): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` forbids `fn evaluate_*`. Java reflection (`Class.forName`, `Method.invoke`) is captured by syntactic position only.
- **B3**: trivially compliant — tree-sitter parser recovery scanner active.
- **C1** (no macro expansion): **mechanically enforced by R11** — the trait-shape audit forbids `fn expand_*`. Java has no macros; annotation processors run at compile time outside scope's surface.
- **C2** (no version-specific compiler-quirk modelling): **mechanically enforced after R4** — no Java-version accessor in `LanguageWorkspaceContext`.
- **D1**: trivially compliant.
- **D2** (no best-guess fallback resolution): **mechanically enforced after R3**.
- **D3** (no symbol-id collision resolution by guessing): mechanically enforced via R0 + R3.
- **E1**: trivially compliant.
- **E2** (no metadata interpretation in plugin): **mechanically enforced by R2**. Plugin returns `RawCaptures`; `annotations` reserved key carries `{name, args_text?}` verbatim from `@Annotation(...)` syntax (e.g., `@Component`, `@RequestMapping("/users")`). `decorators` and `template_calls` keys are **omitted** (Java has annotations, not decorators or templates).
- **E3**: trivially compliant.
- **F1** (no multi-pass semantic analysis in plugin): **mechanically enforced by R3**.
- **F2** (no write-back to source): **mechanically enforced by R9** — `scripts/audit_immutable.sh` forbids `&mut` on source-related types at the plugin / extractor surface.
- **F3** (no embedded-format parsing beyond tree-sitter): trivially compliant.
- **F4** (no language detection by content sniffing): mechanically enforced after R7 — dispatch is compile-time const, the plugin is invoked by extension only (`.java`).

No `NEEDS REVIEW` outstanding for D2 / D3 / E2 / F1.

## Test fixtures

- `gumiho-mudang-scope/tests/fixtures/languages/java/`
- `gumiho-mudang-scope/tests/integration/test_java.rs`
