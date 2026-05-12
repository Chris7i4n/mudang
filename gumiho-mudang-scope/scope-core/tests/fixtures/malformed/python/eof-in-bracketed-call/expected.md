# Fixture — eof-in-bracketed-call

- **Category**: EOF inside bracketed call (Python-specific variant
  exercising the implicit-line-continuation recovery surface —
  fills the "unbalanced delimiters" slot for the function-call
  case).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 7–13 (inclusive, 1-indexed).
  The `format_name(` call opens on line 6 with three keyword
  arguments threaded across lines 7–9; the closing `)` never
  appears. Python's implicit line continuation inside brackets
  causes tree-sitter to keep consuming — the blank line on line 10
  and the `farewell` function declaration on lines 12–13 are all
  swallowed inside the open call's recovery span.
- **Rationale**: Bracketed-call recovery is the Python equivalent
  of the C-family "missing closing paren" case — common when
  threading one more keyword argument into an existing call.
  Implicit line continuation makes the swallowed region larger
  than the visual point of failure, which is exactly the kind of
  "silent drop" the harness must surface.

## Parseable prefix

Lines 1–5 produce the `format_name` function (with its body) and
the `greet` function signature with its opening `result = ...`
line. The harness's "≥ 1 symbol" floor passes on `format_name` +
`greet`.

## Failure point

Line 6: `result = format_name(` opens the call; line 9's
`suffix="Esq."` is the last well-formed argument; the closing `)`
never appears. The blank line and the `farewell` function
declaration are consumed inside the open call's recovery span.
