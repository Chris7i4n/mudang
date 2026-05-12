# Fixture — eof-in-heredoc

- **Category**: EOF inside heredoc (Ruby-specific variant of the
  shared "EOF inside string" base — exercises the `<<~HTML ... HTML`
  recovery surface).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 4–15 (inclusive, 1-indexed).
  The `<<~HTML` heredoc opens on line 4 and the closing `HTML`
  delimiter never appears — tree-sitter consumes the HTML-like
  body, the blank line, the `reset` method declaration, and the
  cascading `end` keywords as part of the unterminated heredoc
  token; ERROR surfaces at EOF.
- **Rationale**: Ruby heredocs span lines by definition and the
  closing delimiter is a bare identifier at column zero — a routine
  source of "swallowed tail" recovery cases when the delimiter is
  mistyped or dropped. Recovery must surface the runaway region.

## Parseable prefix

Lines 1–3 produce the `Acme` module, the `TemplateLoader` class,
and the `template` method signature. The harness's "≥ 1 symbol"
floor passes on module + class + method.

## Failure point

Line 4: `      <<~HTML` opens the heredoc; the closing `HTML`
delimiter never appears. Tree-sitter consumes the body, the
following method declaration, and every `end` keyword through line
15 as part of the unterminated `heredoc_body` token.
