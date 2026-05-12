# Fixture — eof-in-macro-body

- **Category**: EOF inside macro invocation body (Rust-specific
  variant exercising the `mac! { ... }` recovery surface — macro
  invocations have a deliberately permissive token-tree grammar).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 2–9 (inclusive, 1-indexed).
  The `routes! {` macro body opens on line 2 with three route
  declarations across lines 3–5 and the closing `}` never appears.
  Tree-sitter token-tree recovery sees the `pub fn health_handler`
  declaration on lines 7–9 as additional token-tree content inside
  the still-open macro body; the surrounding `build_router`
  function `{` from line 1 also never closes, compounding the
  recovery region.
- **Rationale**: Macro invocation bodies use a permissive token-
  tree grammar — anything balanced is accepted. The recovery
  surface for an unbalanced macro body is therefore distinct from
  ordinary item-level recovery and merits its own fixture.
  Recovery must surface the runaway region rather than silently
  swallowing the trailing function.

## Parseable prefix

Line 1 produces the `build_router` function signature with its
opening `{`. The harness's "≥ 1 symbol" floor passes on
`build_router`.

## Failure point

Line 2: `routes! {` opens the macro body; the matching `}` never
appears. Tree-sitter consumes the route arms, the blank line,
and the entire `health_handler` function declaration as token-
tree content inside the still-open macro body.
