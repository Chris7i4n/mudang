# Fixture — eof-in-generics-angle

- **Category**: EOF inside generic type-argument angle brackets
  (Rust-specific variant exercising the parameterised-type
  recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 4–9 (inclusive, 1-indexed).
  The nested generic on line 4 (`HashMap<String, HashMap<String,`)
  opens two `<` brackets and never closes either with `>` —
  tree-sitter recovery cannot resolve the type expression, the
  field name, or the surrounding declaration, and flags the
  region from the broken type through end of file.
- **Rationale**: Rust generics are LL(1)-ambiguous (the `<` token
  can mean "less than" or "type-argument open") and recovery
  through nested angle brackets is a known stress point. A field
  declaration whose type never closes its generics swallows the
  associated `impl`-style block — recovery must surface the
  region honestly.

## Parseable prefix

Lines 1–3 produce the `HashMap` import and the `Registry` struct
header. The harness's "≥ 1 symbol" floor passes on the struct
symbol alone; the field declaration is part of the broken generic
and is not expected to surface, and the `new` method is consumed
inside the recovery region.

## Failure point

Line 4: `map: HashMap<String, HashMap<String,` — the outer
`HashMap<` and inner `HashMap<` both open type-argument lists and
neither closes with `>`. The line break does not close the
brackets — tree-sitter treats the rest of the file (the `new`
method body + the struct closing brace) as part of the broken
declaration.
