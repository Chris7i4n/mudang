# Fixture — truncated-mid-method

- **Category**: truncated mid-function (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 10–12 (inclusive, 1-indexed).
  The `foreach` block opens but the `sum +=` expression has no
  right-hand side and the source EOFs before the method body or class
  brace can close.
- **Rationale**: Method body truncates mid-expression — the canonical
  "branch saved while typing" shape. Parser recovery must mark the
  unterminated block as an ERROR region and the indexer must record
  the range honestly rather than silently dropping the method.

## Parseable prefix

Lines 1–9 produce the namespace declaration, class `OrderProcessor`,
property `Total`, and the `ComputeTotal` method signature with its
opening brace and `int sum = 0;` statement. The harness's
"≥ 1 symbol" floor passes on class + property + method.

## Failure point

Line 11: `sum +=` — expression has no right-hand side; the source
EOFs at the end of line 12 without closing the `foreach` block, the
method body, or the class.
