# Fixture — mixed-indent-collapse

- **Category**: mixed indent collapse (Python-specific — fills the
  "unbalanced delimiters" slot since Python has no braces).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 6–10 (inclusive, 1-indexed).
  The `restock` method body opens with 8-space indentation on line
  6; line 7's `return self.count` is indented at 7 spaces, which is
  neither a continuation of the surrounding block nor a dedent to
  any enclosing block. Tree-sitter recovery cannot reconcile the
  indent context and flags the region from the mismatched line
  through the `drain` method declaration.
- **Rationale**: Indentation is Python's structural delimiter — a
  one-space mismatch is the canonical mid-edit failure shape and
  cannot be silently absorbed. Recovery must surface the indent
  conflict so the indexer reports the malformed region rather than
  treating the lines as if they belong to the surrounding block.

## Parseable prefix

Lines 1–5 produce the class `Inventory`, the `__init__` method (with
its body), and the `restock` method signature. The harness's
"≥ 1 symbol" floor passes on class + `__init__` + `restock`.

## Failure point

Line 7: `       return self.count` is indented at 7 spaces. Line 6
ended at 8-space indentation inside the `restock` body, and there
is no enclosing block at 7-space indentation. The indent token mix
triggers tree-sitter recovery, and lines 7–10 are flagged as
malformed structure.
