# Fixture — eof-in-generics-angle

- **Category**: EOF inside generic type-argument angle brackets
  (Java-specific variant exercising the parameterised-type recovery
  surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 7–12 (inclusive, 1-indexed).
  The nested generic on line 7 (`Map<String, Map<String,`) opens
  two `<` brackets and never closes either with `>` — tree-sitter
  recovery cannot resolve the type expression, the field name, or
  the surrounding declaration, and flags the region from the broken
  type through end of file.
- **Rationale**: Java generics are LL(1)-ambiguous (the `<` token
  can mean "less than" or "type-argument open") and recovery
  through nested angle brackets is a known stress point. A field
  declaration whose type never closes its generics swallows the
  constructor + remaining class body — recovery must surface the
  region rather than silently treat the constructor as nothing.

## Parseable prefix

Lines 1–6 produce the package declaration, the `Map` / `HashMap`
imports, and the `Registry` class header. The harness's "≥ 1 symbol"
floor passes on the class symbol alone; the field declaration is
part of the broken generic and is not expected to surface, and the
constructor is consumed inside the recovery region.

## Failure point

Line 7: `private Map<String, Map<String,` — the outer `Map<` and
inner `Map<` both open type-argument lists and neither closes with
`>`. The line break does not close the brackets — tree-sitter
treats the rest of the file (constructor + class closing brace) as
part of the broken declaration.
