# Fixture — unbalanced-brace

- **Category**: unbalanced delimiters (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 8–12 (inclusive, 1-indexed).
  The `restock` method body opens at line 6 with `{` but the
  matching `}` is missing — the `drain` method declaration on
  line 9 is consumed inside the still-open `restock` body and
  tree-sitter recovery flags the region from the missing brace
  through end of file.
- **Rationale**: Dropped closing brace is the second-most common
  "broken mid-edit" shape after truncation. Recovery must surface
  symbols for the parseable prefix and flag the misaligned tail.

## Parseable prefix

Lines 1–7 produce the struct `Inventory` with its field `count`
and the `impl Inventory` block containing the `restock` method
signature with the `self.count += amount;` statement parsed inside
its body. The harness's "≥ 1 symbol" floor passes on struct +
function.

## Failure point

Line 6 opens `{` for `restock`; the matching `}` never appears.
Line 9's `pub fn drain` declaration is consumed inside the still-
open `restock` body, triggering tree-sitter recovery from the
mismatched declaration through line 12.
