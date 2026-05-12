# Fixture — missing-close-tag-jsx

- **Category**: missing close tag in JSX (TS/TSX-specific variant
  exercising the JSX element-recovery surface — fills the
  language-specific slot the README per-language category table
  reserves for TypeScript).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 5–9 (inclusive, 1-indexed).
  The `<div className="container">` opening JSX tag on line 5 is
  never closed with `</div>` — tree-sitter's JSX recovery sees
  the well-formed `<p>...</p>` element inside the still-open
  `<div>`, then encounters the parenthesis on line 8 and the
  method's closing brace on line 9 without a matching JSX close
  tag; recovery flags the region from the open tag through the
  malformed paren-and-brace tail. The second function on lines
  11–13 parses cleanly after the recovery boundary.
- **Rationale**: JSX is a TypeScript surface that does not exist
  in any other language we support and the canonical mid-edit
  failure mode is a dropped closing tag (especially when
  refactoring a wrapper). Recovery must preserve the well-formed
  second component.

## Parseable prefix

Lines 1–4 produce the `React` import and the `Greeter` function
signature with its `return (` opening. Lines 11–13 produce the
`Farewell` function after the recovery boundary. The harness's
"≥ 1 symbol" floor passes on `Greeter` + `Farewell`.

## Failure point

Line 5: `<div className="container">` opens a JSX element; the
matching `</div>` never appears. The inner `<p>...</p>` is well-
formed, but the outer `<div>` swallows the trailing `);` on line
8 and the function's `}` on line 9 as JSX content; tree-sitter
recovery boundary lands before line 11's second function
declaration.
