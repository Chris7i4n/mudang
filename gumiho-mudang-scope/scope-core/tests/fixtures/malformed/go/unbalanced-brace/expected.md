# Fixture — unbalanced-brace

- **Category**: unbalanced delimiters (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 12–15 (inclusive, 1-indexed).
  The `Restock` method body opens at line 9 with `{` but the
  matching `}` is missing — the `Drain` method declaration on
  line 13 is consumed inside the still-open `Restock` body and
  tree-sitter recovery flags the region from the missing brace
  through end of file.
- **Rationale**: A dropped closing brace is the second-most common
  "broken mid-edit" shape after truncation. Go's `gofmt` discipline
  makes this less frequent than in C-family code, but tree-sitter
  recovery still has to surface the swallowed declaration rather
  than silently dropping `Drain`.

## Parseable prefix

Lines 1–11 produce the `acme` package declaration, the `fmt`
import, the struct `Inventory`, and the `Restock` method signature
with its body up through `fmt.Println("restocked")`. The harness's
"≥ 1 symbol" floor passes on struct + method.

## Failure point

Line 9 opens `{` for `Restock`; the matching `}` never appears.
Line 13's `func (i *Inventory) Drain()` declaration is consumed
inside the still-open `Restock` body, triggering tree-sitter
recovery from the misaligned declaration through line 15.
