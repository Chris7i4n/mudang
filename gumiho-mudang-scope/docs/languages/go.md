# Language: Go

## Tree-sitter grammar

- **Crate / package**: `tree-sitter-go` (workspace-pinned version)
- **Source**: <https://github.com/tree-sitter/tree-sitter-go>
- **License**: MIT
- **Maturity assessment**: stable.
- **Known grammar gaps**: none material at surface level.

## Depth target

- **Level**: surface
- **Post-refactor depth queue**: no

## Symbol kinds emitted

`function`, `method`, `struct` (via type declarations), `interface`, `type` (alias), `const`.

## Edge kinds emitted

`calls`, `imports`, `extends` (struct embedding).

Pattern catalog (per `queries/go/edges.scm`):

| Pattern | pattern_id |
|---|---|
| `import "fmt"` | `imports.path` |
| `processPayment(...)` | `calls.function` |
| `s.Handle()` / `fmt.Println()` | `calls.method` |
| struct embedding (`{ Logger }`) | `extends.embedding` |

## Universal boundaries — compliance log

- **A1 / A2 / A3** (type system): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) forbids `fn infer_*` / `fn solve_*` / `fn narrow_*` / `fn resolve_overload_*` in the scanned plugin / extractor paths. Go's generics are captured as text; method-set computation is the compiler's job, never the plugin's.
- **B1**: discipline-only per the universal class-3 list.
- **B2** (no runtime / dynamic resolution): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` forbids `fn evaluate_*`.
- **B3**: trivially compliant — tree-sitter parser recovery scanner active.
- **C1** (no macro expansion): **mechanically enforced by R11** — the trait-shape audit forbids `fn expand_*`. Go has no macros; the rule's enforcement layer applies uniformly across languages.
- **C2** (no version-specific compiler-quirk modelling): **mechanically enforced after R4**. `LanguageWorkspaceContext` has no `go_directive` accessor; reading it from the language layer is a compile error.
- **D1**: trivially compliant.
- **D2** (no best-guess fallback resolution): **mechanically enforced after R3**. Extractor emits `RawEdge` with `Confidence::Medium`; resolver assigns `status`; Ambiguous emits one row per candidate; `confidence` preserved verbatim.
- **D3** (no symbol-id collision resolution by guessing): mechanically enforced via R0 + R3.
- **E1**: trivially compliant.
- **E2** (no metadata interpretation in plugin): **mechanically enforced by R2**. Plugin returns `RawCaptures`; all three reserved metadata keys (`decorators`, `annotations`, `template_calls`) are **omitted** — Go has no decorator / annotation / template-component AST surface.
- **E3**: trivially compliant.
- **F1** (no multi-pass semantic analysis in plugin): **mechanically enforced by R3**.
- **F2** (no write-back to source): **mechanically enforced by R9** — `scripts/audit_immutable.sh` forbids `&mut` on source-related types at the plugin / extractor surface.
- **F3** (no embedded-format parsing beyond tree-sitter): trivially compliant.
- **F4** (no language detection by content sniffing): mechanically enforced after R7 — dispatch is compile-time const, the plugin is invoked by extension only (`.go`).

No `NEEDS REVIEW` outstanding for D2 / D3 / E2 / F1.

## Known gotchas

1. Struct embedding is captured as `extends.embedding` (Go's closest analogue to inheritance, even though Go's type system does not formally have inheritance). This is a syntactic mapping; semantic interpretation belongs to whatever consumer reads the edge.

## Test fixtures

- `gumiho-mudang-scope/tests/fixtures/languages/go_lang/`
- `gumiho-mudang-scope/tests/integration/test_go_lang.rs`
