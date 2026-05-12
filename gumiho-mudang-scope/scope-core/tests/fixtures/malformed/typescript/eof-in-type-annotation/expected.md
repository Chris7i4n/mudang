# Fixture — eof-in-type-annotation

- **Category**: EOF inside type annotation angle brackets
  (TypeScript-specific variant exercising the parameterised-type
  recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 2–7 (inclusive, 1-indexed).
  The nested generic on line 2 (`Map<string, Map<string,`) opens
  two `<` brackets and never closes either with `>` — tree-sitter
  recovery cannot resolve the type expression, the field name,
  or the surrounding declaration, and flags the region from the
  broken type through end of file.
- **Rationale**: TypeScript generics are LL(1)-ambiguous (the
  `<` token can mean "less than" or "type-argument open") and
  recovery through nested angle brackets is a known stress point.
  A field declaration whose type never closes its generics
  swallows the constructor + class tail — recovery must surface
  the region honestly.

## Parseable prefix

Line 1 produces the `Registry` class header. The harness's
"≥ 1 symbol" floor passes on the class symbol alone; the field
declaration is part of the broken generic and is not expected to
surface, and the constructor is consumed inside the recovery
region.

## Failure point

Line 2: `private map: Map<string, Map<string,` — the outer
`Map<` and inner `Map<` both open type-argument lists and neither
closes with `>`. The line break does not close the brackets —
tree-sitter treats the rest of the file (constructor body + class
closing brace) as part of the broken declaration.
