# Fixture — truncated-mid-function

- **Category**: truncated mid-function (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 8–9 (inclusive, 1-indexed).
  The `for` loop body opens on line 8 but the `sum +=` on line 9
  has no right-hand side; the source EOFs before the assignment,
  the loop, the method body, or the `impl` block can close.
- **Rationale**: Mid-edit truncation is the canonical "branch
  saved while typing" shape. Rust does not have automatic
  semicolon insertion; EOF after `+=` surfaces an ERROR region
  rather than recovering as a no-op statement.

## Parseable prefix

Lines 1–7 produce the struct `OrderProcessor` with its field
`total` and the `impl OrderProcessor` block containing the
`compute_total` method signature and the `let mut sum = 0i64;`
initialiser. The harness's "≥ 1 symbol" floor passes on struct +
function.

## Failure point

Line 9: `sum +=` — assignment operator has no right-hand-side
expression and the source EOFs at the end of the line without
closing the `for` body, the method body, or the `impl` block.
