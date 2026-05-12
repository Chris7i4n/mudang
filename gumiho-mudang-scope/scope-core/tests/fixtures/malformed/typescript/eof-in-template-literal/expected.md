# Fixture — eof-in-template-literal

- **Category**: EOF inside template literal (TypeScript-specific
  variant of the shared "EOF inside string" base — exercises the
  multi-line backtick-template recovery surface, including a
  well-formed `${...}` interpolation embedded inside the still-
  open literal).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 2–10 (inclusive, 1-indexed).
  The template literal opens with a backtick on line 2 and never
  closes — tree-sitter consumes the multi-line HTML-like body
  (including the well-formed `${name}` interpolation), the blank
  line, and the entire `rebuild` function declaration as part of
  the unterminated template literal; ERROR surfaces at EOF.
- **Rationale**: Template literals are TypeScript's multi-line
  string surface and the routine source of "swallowed tail"
  recovery cases when the closing backtick is dropped. The
  embedded `${...}` interpolation is a useful complication for
  the recovery surface because tree-sitter must distinguish
  "balanced interpolation expression inside still-open literal"
  from "literal closed and expression context resumed".

## Parseable prefix

Line 1 produces the `buildHtml` function signature with its
opening `{`. The harness's "≥ 1 symbol" floor passes on
`buildHtml`.

## Failure point

Line 2: `return \`` opens the template literal with a backtick;
no matching backtick appears before EOF. Tree-sitter consumes the
HTML-like body, the well-formed `${name}` interpolation, the
blank line, and the entire `rebuild` function declaration as
template-literal content.
