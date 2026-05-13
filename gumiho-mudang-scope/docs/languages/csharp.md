# Language: C#

## Tree-sitter grammar

- **Crate / package**: `tree-sitter-c-sharp` (workspace-pinned version)
- **Source**: <https://github.com/tree-sitter/tree-sitter-c-sharp>
- **License**: MIT
- **Maturity assessment**: stable.
- **Known grammar gaps**: `using static` is captured as a regular `using_directive`; no semantic distinction is preserved.

## Depth target

- **Level**: surface
- **Depth queue**: no

## Symbol kinds emitted

`class`, `interface`, `struct`, `method`, `property`, `const`, `enum`.

## Edge kinds emitted

`calls`, `imports`, `instantiates`, `implements` (covers both extends and implements at the syntactic level — the base list does not distinguish), `references`.

Pattern catalog (per `queries/csharp/edges.scm`):

| Pattern | pattern_id |
|---|---|
| `using X;` | `imports.identifier` |
| `using X.Y.Z;` | `imports.qualified` |
| `_logger.Info(...)` | `calls.method` |
| `DoSomething(...)` | `calls.function` |
| `new Foo()` | `instantiates.class` |
| `this.Method()` | `calls.method.this` |
| `base.Method()` | `calls.method.base` |
| base list (identifier) | `implements.base_list` |
| base list (qualified) | `implements.base_list.qualified` |
| `case PaymentStatus.Pending:` | `references.switch.member` |

## Universal boundaries — compliance log

- **A1 / A2 / A3** (type system): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) forbids any `fn infer_*` / `fn solve_*` / `fn narrow_*` / `fn resolve_overload_*` in the scanned plugin / extractor paths. C# generics are not solved; method-resolution semantics are not modelled.
- **B1**: discipline-only per the universal class-3 list ([`ENFORCEMENT-MAP.md` § Discipline-only rules](../ENFORCEMENT-MAP.md#discipline-only-rules)).
- **B2** (no runtime / dynamic resolution): **mechanically enforced by R12** — `scripts/audit_trait_shape.sh` forbids `fn evaluate_*`. C# `dynamic` dispatch is captured by syntactic position only.
- **B3**: trivially compliant — tree-sitter parser recovery scanner active.
- **C1** (no macro / template expansion): **mechanically enforced by R11** — the same trait-shape audit forbids `fn expand_*`. C# has no macro system; the rule's enforcement layer applies uniformly across languages.
- **C2** (no version-specific compiler-quirk modelling): **mechanically enforced on the plugin-facing trait surface** — no `TargetFramework` accessor in `LanguageWorkspaceContext`, pinned by `audit_context_shape.sh`. The `.csproj` reader added in sprint 0003 lives **indexer-side** behind that trait boundary; per the R4 indexer-side carveout it may expose a `<TargetFramework>` extraction function that indexer-side consumers — the R8 audit emit's `lang_version` field, shipped in sprint 0003 (d) — call directly. Plugins never reach that function; the C2 line stays at the trait, not at the reader.
- **D1**: trivially compliant.
- **D2** (no best-guess fallback resolution): **mechanically enforced after R3**.
- **D3** (no symbol-id collision resolution by guessing): mechanically enforced via R0 + R3.
- **E1**: trivially compliant.
- **E2** (no metadata interpretation in plugin): **mechanically enforced by R2**. Plugin returns `RawCaptures`; `annotations` reserved key carries `{name, args_text?}` verbatim from `[Attribute(...)]` syntax (e.g., `[HttpGet("/users")]`). `decorators` and `template_calls` keys are **omitted**.
- **E3**: trivially compliant.
- **F1** (no multi-pass semantic analysis in plugin): **mechanically enforced by R3**.
- **F2** (no write-back to source): **mechanically enforced by R9** — `scripts/audit_immutable.sh` (gate `ci-immutable`) forbids `&mut str` / `&mut String` / `&mut tree_sitter::Tree` / `&mut Tree` / `&mut Source*` in the scanned plugin / extractor paths.
- **F3** (no embedded-format parsing beyond tree-sitter): trivially compliant.
- **F4** (no language detection by content sniffing): mechanically enforced after R7 — dispatch is compile-time const, the plugin is invoked by extension only.

No `NEEDS REVIEW` outstanding for D2 / D3 / E2 / F1.

## Known gotchas

1. The base list (`(base_list ...)` in C#) does not syntactically distinguish a class superclass from an implemented interface; both are emitted as `implements`. Consumers that need the distinction must consult the symbol's `kind` (class vs interface vs struct) on the target side.
2. `lang_version` detector (sprint 0003 (d), indexer-side carveout per R4): C# has no fixed manifest filename — every project folder owns its own `*.csproj`. `csproj::extract_target_framework` scans the file's directory for any `.csproj` (alphabetical order for determinism, first match wins) and reads `<TargetFramework>` → `<TargetFrameworks>` (first moniker of the semicolon-separated list) → `<TargetFrameworkVersion>` (legacy non-SDK pre-`.NET Core`). Multi-target projects therefore report **only the primary** moniker; callers needing the full set must read the raw element. Property placeholders (`$(NetVersion)`) flow through verbatim, unresolved — MSBuild evaluation is out of scope. Source: `scope-core/src/workspace/csproj.rs`.

## Test fixtures

- `gumiho-mudang-scope/tests/fixtures/languages/csharp/`
- `gumiho-mudang-scope/tests/integration/test_csharp.rs`
