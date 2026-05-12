# Fixture — missing-closing-paren-on-call

- **Category**: missing closing paren on call (Go-specific variant
  exercising call-expression recovery).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 6–7 (inclusive, 1-indexed).
  The `fmt.Sprintf(...)` call opens its arguments on line 6 but the
  closing `)` never appears before the line break; the trailing `}`
  on line 7 is consumed inside the open call's recovery span, then
  the second function declaration parses cleanly after the boundary.
- **Rationale**: Forgetting a closing paren on a multi-argument call
  is a routine mid-edit accident — especially common when threading
  one more argument into an existing `Sprintf` / `Errorf` call.
  Recovery must preserve the well-formed tail (the `Farewell`
  function).

## Parseable prefix

Lines 1–5 produce the `acme` package declaration, the `fmt` import,
and the `Greet` function signature with its opening brace. Lines
9–11 produce the `Farewell` function after the recovery boundary.
The harness's "≥ 1 symbol" floor passes on the two function symbols.

## Failure point

Line 6: `return fmt.Sprintf("Hello, %s — welcome to %s", name, "Acme"`
— the call's argument list never closes its `)`. The line break
terminates the call's recovery span; line 7's `}` (intended as
`Greet`'s closing brace) is consumed inside the error region.
