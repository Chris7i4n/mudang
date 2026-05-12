# Fixture — eof-in-raw-string

- **Category**: EOF inside raw string literal (Rust-specific
  variant of the shared "EOF inside string" base — exercises the
  `r#"..."#` recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 1–8 (inclusive, 1-indexed).
  The `r#"` raw string opens on line 1 and the closing `"#` never
  appears — tree-sitter consumes the multi-line banner content,
  the blank line, the `print_banner` function declaration, and
  the closing `}` as a single unterminated raw-string token;
  ERROR surfaces at EOF.
- **Rationale**: Rust raw strings can span lines and use a
  hash-paired delimiter to escape internal quotes — a routine
  source of "swallowed tail" recovery cases when the closing
  `"#` is dropped. Recovery must surface the runaway region so
  the indexer does not silently drop the `print_banner` symbol.

## Parseable prefix

There is no parseable prefix in the conventional sense — the
broken raw string consumes the file from line 1. The harness's
"≥ 1 symbol" floor relies on the indexer's behaviour when a file
contains only a broken declaration — if the indexer ever changes
this behaviour, the assertion fails honestly rather than silently
passing.

## Failure point

Line 1: `pub const BANNER: &str = r#"` opens the raw string with
`r#"`. No matching `"#` appears before EOF. Tree-sitter consumes
the banner body, the blank line, the `print_banner` function,
and the closing `}` as a single unterminated `raw_string_literal`
token.
