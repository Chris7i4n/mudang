# Fixture — eof-in-import-block

- **Category**: EOF inside grouped-import block (Go-specific variant
  exercising the import-declaration recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 7–12 (inclusive, 1-indexed).
  The grouped import opens with `import (` on line 3 but the closing
  `)` never appears — the `Hello` function declaration on line 8 is
  consumed inside the still-open import list and tree-sitter
  recovery flags the region from the missing paren through end of
  file.
- **Rationale**: Grouped imports are the canonical Go way to declare
  multiple imports — forgetting the closing `)` (especially when
  adding one more import at the bottom) is a common mid-edit
  accident. The blank line after the last import string is the
  natural human visual cue that recovery starts there.

## Parseable prefix

Lines 1–6 produce the `acme` package declaration and the three
import strings inside the still-open import group. The harness's
"≥ 1 symbol" floor passes on the package + import metadata; the
`Hello` function declaration is part of the broken import region
and is not expected to surface as its own symbol.

## Failure point

Line 3: `import (` opens the grouped-import list; the matching `)`
never appears. The blank line at line 7 does not close the group —
tree-sitter recovery treats the `Hello` function declaration as
malformed import content from line 8 through the file's closing
brace at line 12.
