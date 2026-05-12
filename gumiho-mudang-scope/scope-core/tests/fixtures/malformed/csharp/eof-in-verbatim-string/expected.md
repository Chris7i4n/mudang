# Fixture — eof-in-verbatim-string

- **Category**: EOF inside verbatim string literal (C#-specific
  variant of the shared "EOF inside string" base).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 5–13 (inclusive, 1-indexed).
  The `@"` verbatim string opens on line 5 and never closes — every
  byte from there to EOF (including the `Reset` method declaration
  and the class closing brace) is consumed as verbatim-string content.
- **Rationale**: C# verbatim strings (`@"..."`) span lines and do not
  interpret backslashes — they are a routine source of "swallowed
  tail" recovery cases when the closing quote is dropped. Tree-sitter
  must flag the runaway region rather than silently treating the rest
  of the file as nothing.

## Parseable prefix

Lines 1–4 produce the namespace declaration and the class
`TemplateLoader` header. The harness's "≥ 1 symbol" floor passes on
the class symbol alone; the `Template` property is part of the
broken statement and is not expected to surface.

## Failure point

Line 5: `public string Template { get; } = @"` — the verbatim string
opens with `@"` and no matching `"` appears before EOF. Tree-sitter
consumes the HTML-like body, the blank line, the `Reset` method, and
the class closing brace as a single unterminated `verbatim_string`
token; ERROR surfaces at EOF.
