# Fixture — eof-in-regex

- **Category**: EOF inside regex literal (Ruby-specific variant
  exercising regex-recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 4–11 (inclusive, 1-indexed).
  The `/welcome to [Aa]cme` regex literal opens on line 4 and the
  closing `/` never appears — tree-sitter consumes the newline,
  the `end` keyword on line 5, the blank line, and every following
  token forward looking for a closing `/`; the open regex swallows
  the rest of the file through line 11.
- **Rationale**: Ruby regex literals are delimited by `/` and span
  is bounded only by the matching `/` — a forgotten closing slash
  swallows the rest of the file. This is a known recovery stress
  point because the parser cannot disambiguate "regex literal"
  from "division operator" without context.

## Parseable prefix

Lines 1–3 produce the `Acme` module, the `Matcher` class, and the
`find_first` method signature. The harness's "≥ 1 symbol" floor
passes on module + class + method.

## Failure point

Line 4: `text.match(/welcome to [Aa]cme` — the regex literal opens
with `/` after `text.match(` and never closes its `/`. Tree-sitter
consumes the newline and every following token forward looking for
a closing slash; the well-formed `find_all` method on lines 7–9 is
swallowed inside the runaway regex token.
