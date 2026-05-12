# Fixture — truncated-mid-method

- **Category**: truncated mid-function (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 7–8 (inclusive, 1-indexed).
  The `items.each do |item|` block opens but the `sum +=` on line 8
  has no right-hand side; the source EOFs before the assignment, the
  block, the method, the class, or the module can close.
- **Rationale**: Mid-edit truncation is the canonical "branch saved
  while typing" shape. Ruby's `+=` requires a right-hand side; EOF
  after the operator surfaces an ERROR region rather than recovering
  as a no-op statement.

## Parseable prefix

Lines 1–6 produce the `Acme` module, the `OrderProcessor` class,
the `attr_accessor :total` declaration, and the `compute_total`
method signature with the `sum = 0` initialiser. The harness's
"≥ 1 symbol" floor passes on module + class + method.

## Failure point

Line 8: `sum +=` — assignment operator has no right-hand-side
expression and the source EOFs at the end of the line without
closing the block, the method, the class, or the module.
