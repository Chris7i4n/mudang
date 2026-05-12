# Fixture — eof-in-string

- **Category**: EOF inside string literal (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 7–9 (inclusive, 1-indexed).
  The string opened after `welcome to ` on line 7 never closes —
  Java's regular string literals do not span newlines, so the
  closing quote must appear on the same line; tree-sitter surfaces
  ERROR for the unterminated token from line 7 through the trailing
  `}` tokens it then consumes as part of the malformed region.
- **Rationale**: Forgotten closing quote is a routine paste/edit
  accident. Recovery must surface the swallowed region so the
  indexer does not silently drop the method or class tail.

## Parseable prefix

Lines 1–6 produce the package declaration, the class `Greeter`, the
field `greeting`, and the `salutation` method signature with its
opening brace. The harness's "≥ 1 symbol" floor passes on class +
field + method.

## Failure point

Line 7: `return "Hi, " + name + " — welcome to ;` — the literal
opened after `welcome to ` is never closed with `"`. Java's string
literal is single-line; the unterminated token surfaces as ERROR
and the trailing `}` on line 8 and `}` on line 9 are consumed in
the recovery region.
