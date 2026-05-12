# Fixture — truncated-mid-function

- **Category**: truncated mid-function (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 12–14 (inclusive, 1-indexed).
  The `for` body opens but `sum +=` on line 14 has no right-hand
  side; the source EOFs before the assignment, the loop, the method
  body, or the file-level block can close.
- **Rationale**: Mid-edit truncation is the most common "broken
  branch" shape. `+=` is one of the few Go operators where automatic
  semicolon insertion does not fire, so EOF after the operator is
  guaranteed to surface an ERROR region — not a recoverable
  no-op-statement.

## Parseable prefix

Lines 1–11 produce the `acme` package declaration, the `fmt` import,
the struct `Order` (with `ID` and `Total` fields), and the
`ComputeTotal` method signature with its opening brace and the
`sum := 0` initialiser. The harness's "≥ 1 symbol" floor passes on
struct + method.

## Failure point

Line 14: `sum +=` — operator has no right-hand-side expression and
the source EOFs at the end of the line without closing the `for`
loop, the method body, or any of the trailing tokens.
