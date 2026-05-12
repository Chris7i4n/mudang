# Fixture — unbalanced-brace

- **Category**: unbalanced delimiters (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 10–15 (inclusive, 1-indexed).
  The `Restock` method body opens at line 8 with `{` but the
  matching `}` is missing — the `Drain` method declaration on
  line 11 is then parsed inside the still-open `Restock` body and
  tree-sitter recovery flags the region from the missing brace
  through end of file.
- **Rationale**: Lost closing brace is the second-most common
  "broken mid-edit" shape after truncation. Recovery must produce
  symbols for the parseable prefix and mark the misaligned tail.

## Parseable prefix

Lines 1–9 produce the namespace declaration, class `Inventory`,
field `Count`, and the `Restock` method signature with the
`Count += 10;` statement parsed inside its body. The harness's
"≥ 1 symbol" floor passes on class + field + method.

## Failure point

Line 8 opens `{` for `Restock`; the matching `}` never appears.
Line 11's `public void Drain()` declaration is consumed inside the
still-open `Restock` body, triggering tree-sitter recovery from the
mismatched declaration through line 15.
