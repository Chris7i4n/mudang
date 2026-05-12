# Language: Ruby

## Tree-sitter grammar

- **Crate / package**: `tree-sitter-ruby` (workspace-pinned version)
- **Source**: <https://github.com/tree-sitter/tree-sitter-ruby>
- **License**: MIT
- **Maturity assessment**: stable.
- **Known grammar gaps**: dynamic metaprogramming (`define_method` with dynamic arguments, `instance_eval` blocks) intentionally not modelled per rule C1 / B2.

## Depth target

- **Level**: surface
- **Post-refactor depth queue**: no

## Symbol kinds emitted

`class`, `module` (post-R0; pre-R0 was coerced to `interface`), `method`, `function` (top-level `def` outside class context), `const`.

## Edge kinds emitted

`calls`, `imports`, `instantiates`, `extends`, `implements` (mixins via `include` / `prepend` / `extend`), `references_type`, `references`.

Ruby dispatches by semantic capture name rather than by `pattern_index` — the extractor branches on `import.method`, `extends.parent`, `meta.method`, etc.

Pattern catalog (per `queries/ruby/edges.scm` + `scope-core/src/extract/ruby.rs`):

| Pattern | pattern_id |
|---|---|
| `require "..."`, `require_relative`, `load` | `imports.require` |
| `autoload :X, "..."` | `imports.autoload` |
| `ClassName.new` | `instantiates.new` |
| `class Child < Parent` | `extends.class` |
| `include Module` | `implements.mixin.include` |
| `prepend Module` | `implements.mixin.prepend` |
| `extend Module` | `implements.mixin.extend` |
| constant in expression position | `references_type.constant` |
| `send :method` / `public_send` / `__send__` | `calls.meta.send` |
| `define_method :name` | `references.meta.define_method` |
| `const_get "Name"` | `references_type.meta.const_get` |
| `super` | `calls.super` |
| `yield` | `calls.yield` |
| `receiver.method` | `calls.method` |
| `method_name` | `calls.function` |

## Universal boundaries — compliance log

- **A1 / A2 / A3** (type system): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) forbids `fn infer_*` / `fn solve_*` / `fn narrow_*` / `fn resolve_overload_*` in the scanned plugin / extractor paths. The `infer_visibility(node, source)` helper that mapped Ruby AST shape to public/private/protected was renamed `visibility_for_node` in the same sprint to satisfy the audit — the function does syntactic walk, not inference. Ruby is dynamically typed; no type-system work is even attempted in the plugin.
- **B1**: discipline-only per the universal class-3 list.
- **B2** (no runtime / dynamic resolution): **mechanically enforced after R12 sprint 0004** — `scripts/audit_trait_shape.sh` forbids `fn evaluate_*`. **Conservatively compliant at the data level**: `send` / `public_send` / `__send__` / `define_method` / `const_get` literals are emitted as edges only when the argument is a literal symbol or string AND the literal is not dynamic (`is_dynamic_ruby_literal`: rejects strings containing `#{`, `\`, or newlines). Dynamic arguments are intentionally not matched. Tradeoff: literal-only metaprogramming is high-precision low-recall, which fits the playbook's "honest ambiguity beats false certainty" principle.
- **B3** (no assumption of valid syntax): tree-sitter parser-recovery scanner active.
- **C1** (no macro / template expansion): **mechanically enforced after R11 sprint 0004** — `scripts/audit_trait_shape.sh` forbids `fn expand_*`. Hook-style `define_method` literal capture is the documented data-level exception per the playbook's "definitions captured, expansions not" rule; the plugin captures the definition site, never evaluates the block body.
- **C2** (no version-specific compiler-quirk modelling): **mechanically enforced after R4** — no Ruby-version accessor in `LanguageWorkspaceContext`.
- **D1**: trivially compliant.
- **D2** (no best-guess fallback resolution): **mechanically enforced after R3**. Extractor emits `RawEdge` with `Confidence::Medium`; resolver assigns `status` ∈ `{Resolved, Ambiguous, Dangling}`; Ambiguous emits one row per candidate; `confidence` preserved verbatim.
- **D3** (no symbol-id collision resolution by guessing): mechanically enforced via R0 + R3.
- **E1**: trivially compliant.
- **E2** (no metadata interpretation in plugin): **mechanically enforced after R2 chunk 7**. Plugin returns `RawCaptures`; all three reserved metadata keys (`decorators`, `annotations`, `template_calls`) are **omitted** — Ruby has no native decorator / annotation syntax (mixins are edges, not metadata) and template surfaces (ERB, Slim, Haml) live in separate template files.
- **E3**: trivially compliant.
- **F1** (no multi-pass semantic analysis in plugin): **mechanically enforced after R3 typestate**.
- **F2** (no write-back to source): **mechanically enforced after R9 sprint 0004** — `scripts/audit_immutable.sh` forbids `&mut` on source-related types at the plugin / extractor surface.
- **F3** (no embedded-format parsing beyond tree-sitter): trivially compliant.
- **F4** (no language detection by content sniffing): mechanically enforced after R7 — dispatch is compile-time const, the plugin is invoked by extension only (`.rb`).

No `NEEDS REVIEW` outstanding for D2 / D3 / E2 / F1.

## Known gotchas

1. The query `(call method: (_) @call.name) @call.node` also structurally matches receiver calls, imports, mixins, and metaprogramming — the extractor validates call text with `is_plain_ruby_call` and `is_reserved_edge_call` to suppress double-emission across the narrower patterns. Adding new narrow patterns must also extend `is_reserved_edge_call` to prevent double-counting.
2. `clean_ruby_literal` strips `:` symbol prefix and matching quotes; `clean_ruby_edge_name` strips leading `::` from constants. Both are conservative — round-tripping the source form is not guaranteed.

## Test fixtures

- `gumiho-mudang-scope/tests/fixtures/languages/ruby/`
- `gumiho-mudang-scope/tests/integration/test_ruby.rs`
