# Fixture — truncated-mid-function

- **Category**: truncated mid-function (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 6–7 (inclusive, 1-indexed).
  The `for` body opens but the `running +=` on line 7 has no
  right-hand side; the source EOFs before the assignment, the loop,
  or the method can close.
- **Rationale**: Mid-edit truncation is the canonical "branch saved
  while typing" shape. `+=` requires an RHS in Python; EOF after
  the operator surfaces an ERROR region rather than recovering as
  a no-op statement.

## Parseable prefix

Lines 1–6 produce the class `OrderProcessor`, the class attribute
`total`, and the `compute_total` method signature with its body up
through the `for item in items:` header and the `running = 0`
initialiser. The harness's "≥ 1 symbol" floor passes on class +
method.

## Failure point

Line 7: `running +=` — assignment operator has no right-hand-side
expression and the source EOFs at the end of the line without
closing the `for` body or the method.
