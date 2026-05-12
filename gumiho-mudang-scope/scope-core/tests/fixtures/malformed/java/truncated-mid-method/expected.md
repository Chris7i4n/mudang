# Fixture — truncated-mid-method

- **Category**: truncated mid-function (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 7–9 (inclusive, 1-indexed).
  The `for` loop body opens but the `sum +=` on line 9 has no
  right-hand side; the source EOFs before the assignment, the loop,
  the method, or the class can close.
- **Rationale**: Mid-edit truncation is the canonical "branch saved
  while typing" shape. Java's lack of automatic semicolon insertion
  guarantees the ERROR region surfaces around the unterminated
  assignment.

## Parseable prefix

Lines 1–6 produce the package declaration, the class
`OrderProcessor`, the field `total`, and the `computeTotal` method
signature with its opening brace and the `int sum = 0;` initialiser.
The harness's "≥ 1 symbol" floor passes on class + field + method.

## Failure point

Line 9: `sum +=` — assignment operator has no right-hand-side
expression and the source EOFs at the end of the line without
closing the `for` body, the method, or the class.
