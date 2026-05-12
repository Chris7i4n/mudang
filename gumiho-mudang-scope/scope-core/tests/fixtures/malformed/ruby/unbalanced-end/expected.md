# Fixture — unbalanced-end

- **Category**: unbalanced `end` (Ruby-specific variant of the
  shared "unbalanced delimiters" base — Ruby uses keyword `end`
  rather than `}` for block closure).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 7–13 (inclusive, 1-indexed).
  The `restock` method opens with `def restock(amount)` on line 5
  but no matching `end` appears before the next `def drain` keyword
  on line 8 — tree-sitter recovery sees the `drain` declaration
  parsed inside the still-open `restock` body and the cascading
  `end` keywords at lines 10–13 cannot reconcile the block stack.
- **Rationale**: Dropped `end` keyword is Ruby's equivalent of the
  C-family dropped closing brace — a routine mid-edit accident,
  especially when adding a new method between two existing ones.
  Recovery must surface the misaligned block stack.

## Parseable prefix

Lines 1–6 produce the `Acme` module, the `Inventory` class, the
`attr_accessor :count` declaration, and the `restock` method
signature with its body up through `@count += amount`. The harness's
"≥ 1 symbol" floor passes on module + class + method.

## Failure point

Line 5 opens `def restock(amount)`; the matching `end` never appears
before line 8's `def drain` keyword. The cascading `end` tokens at
lines 10–13 cannot reconcile the still-open `restock` body with the
surrounding class and module — tree-sitter flags the region from
the misaligned `def drain` through end of file.
