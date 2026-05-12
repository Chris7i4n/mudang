# Fixture — truncated-mid-function

- **Category**: truncated mid-function (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 6–7 (inclusive, 1-indexed).
  The `for` loop body opens on line 6 but the `sum +=` on line 7
  has no right-hand side; the source EOFs before the assignment,
  the loop, the method body, or the class can close.
- **Rationale**: Mid-edit truncation is the canonical "branch
  saved while typing" shape. JavaScript / TypeScript automatic
  semicolon insertion does not fire after `+=`, so EOF after the
  operator surfaces an ERROR region rather than recovering as a
  no-op statement.

## Parseable prefix

Lines 1–5 produce the class `OrderProcessor`, the class field
`total`, and the `computeTotal` method signature with its body up
through the `let sum = 0;` initialiser. The harness's "≥ 1 symbol"
floor passes on class + method.

## Failure point

Line 7: `sum +=` — assignment operator has no right-hand-side
expression and the source EOFs at the end of the line without
closing the `for` body, the method body, or the class.
