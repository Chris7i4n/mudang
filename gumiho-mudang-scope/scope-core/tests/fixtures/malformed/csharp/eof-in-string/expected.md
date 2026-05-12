# Fixture — eof-in-string

- **Category**: EOF inside string literal (shared base set).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 9–11 (inclusive, 1-indexed).
  The string opened on line 9 after `welcome to ` never closes —
  tree-sitter consumes the trailing `;`, the method closing `}` on
  line 10, and the class closing `}` on line 11 as part of the
  unterminated string token, then flags the region as ERROR at EOF.
- **Rationale**: Forgotten closing quote is a routine paste/edit
  accident. The unterminated string swallows the rest of the file
  unless the grammar bounds it — recovery must surface the swallowed
  region so the indexer does not silently drop the method.

## Parseable prefix

Lines 1–8 produce the namespace declaration, class `Greeter`,
property `Greeting`, and the `Salutation` method signature with its
opening brace. The harness's "≥ 1 symbol" floor passes on class +
property + method.

## Failure point

Line 9: `return "Hi, " + name + " — welcome to ;` — the literal that
starts after `welcome to ` is opened with `"` but never closed.
Everything from there to EOF is consumed as string content; the
parser surfaces an ERROR at EOF for the unterminated token.
