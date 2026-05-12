# Fixture — eof-in-triple-string

- **Category**: EOF inside triple-quoted string (Python-specific
  variant of the shared "EOF inside string" base — exercises the
  multi-line `"""..."""` recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 1–12 (inclusive, 1-indexed).
  The triple-quoted string opens with `"""` on line 1 and never
  closes — tree-sitter consumes the docstring body, the blank line,
  and both function declarations as part of the unterminated
  string token; ERROR surfaces at EOF.
- **Rationale**: Triple-quoted strings span lines by definition —
  forgetting the closing `"""` is a routine paste/edit accident
  (especially when restructuring module-level docstrings). The
  entire file tail is swallowed unless recovery flags the runaway
  region.

## Parseable prefix

There is no parseable prefix in the conventional sense — the broken
string token consumes the file from line 1. The harness's
"≥ 1 symbol" floor passes on the module-level synthetic `__module__`
symbol that the indexer emits for every Python file regardless of
content; if the indexer's module-symbol behaviour ever changes, the
"≥ 1 symbol" assertion fails honestly rather than silently passing.

## Failure point

Line 1: `DOC = """` — the triple-quoted string opens and no
matching `"""` appears before EOF. Both `greet` and `farewell`
function declarations are consumed as string content.
