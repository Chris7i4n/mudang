# Fixture — eof-in-decorator

- **Category**: EOF inside decorator argument list (Python-specific
  variant exercising decorator-call recovery).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 4–7 (inclusive, 1-indexed).
  The `@lru_cache(...)` decorator opens on line 4 with `maxsize=128,`
  — the closing `)` never appears. Python's implicit line
  continuation inside brackets causes the blank line on line 5 and
  the `expensive_op` function declaration on lines 6–7 to be
  consumed inside the open decorator call's recovery span; the
  second decorator on line 10 and `cheaper_op` function parse
  cleanly after the recovery boundary.
- **Rationale**: Decorator argument lists are a routine mid-edit
  trip point — threading one more keyword argument (or a trailing
  comma) into an existing `@cache` / `@lru_cache` and forgetting
  the `)`. Recovery must preserve the well-formed tail.

## Parseable prefix

Lines 1–3 produce the `lru_cache` import. Lines 10–12 produce the
second decorator + `cheaper_op` function after the recovery
boundary. The harness's "≥ 1 symbol" floor passes on `cheaper_op`.

## Failure point

Line 4: `@lru_cache(maxsize=128,` opens the decorator call; the
closing `)` never appears. The blank line and `expensive_op`
declaration are consumed inside the open call's recovery span.
