# Fixture — eof-in-string-interpolation

- **Category**: EOF inside string interpolation (Ruby-specific
  variant exercising the `"#{...}"` recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 4–11 (inclusive, 1-indexed).
  The interpolation opens on line 4 with `#{name.upcase` — the
  closing `}` never appears. Tree-sitter consumes the newline, the
  `end` keyword on line 5, and every following token as part of
  the still-open interpolation expression; the surrounding `"`
  never closes either, so the runaway region spans through end of
  file.
- **Rationale**: String interpolation is the canonical Ruby
  "expression embedded in literal" surface — a forgotten closing
  `}` not only breaks the interpolation, it also leaves the
  surrounding string unclosed. The cascading recovery is exactly
  what the harness must surface honestly.

## Parseable prefix

Lines 1–3 produce the `Acme` module, the `Greeter` class, and the
`salute` method signature. The harness's "≥ 1 symbol" floor passes
on module + class + method.

## Failure point

Line 4: `"Hello, #{name.upcase` — the interpolation opens with
`#{` after `"Hello, ` and the closing `}` never appears. The
surrounding `"` also never closes. Tree-sitter consumes the rest
of the file (the well-formed `farewell` method, the cascading
`end` keywords) as part of the still-open interpolation +
unterminated string.
