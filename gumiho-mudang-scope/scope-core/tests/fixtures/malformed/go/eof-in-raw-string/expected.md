# Fixture — eof-in-raw-string

- **Category**: EOF inside raw string literal (Go-specific variant
  of the shared "EOF inside string" base — exercises backtick raw
  string recovery).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 5–12 (inclusive, 1-indexed).
  The raw string opens with a backtick on line 5 and never closes —
  tree-sitter consumes the multi-line banner content, the blank line,
  the `PrintBanner` function declaration, and the closing `}` as a
  single unterminated raw-string token; ERROR surfaces at EOF.
- **Rationale**: Go raw strings span lines and ignore escape
  sequences — they are a routine source of "swallowed tail" recovery
  cases when the closing backtick is dropped. Recovery must surface
  the runaway region so the indexer does not silently treat the
  rest of the file as nothing.

## Parseable prefix

Lines 1–4 produce the `acme` package declaration and the `fmt`
import. The harness's "≥ 1 symbol" floor passes on package + import
metadata; the `Banner` variable declaration is part of the broken
statement and is not expected to surface.

## Failure point

Line 5: `var Banner = ` followed by an opening backtick — no
matching backtick appears before EOF. Tree-sitter consumes the
banner body, the blank line, the `PrintBanner` function, and the
closing brace as a single unterminated `raw_string_literal` token.
