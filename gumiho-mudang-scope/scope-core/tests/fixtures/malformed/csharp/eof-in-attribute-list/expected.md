# Fixture — eof-in-attribute-list

- **Category**: EOF inside attribute argument list (C#-specific
  variant — exercises bracketed-call recovery in attribute position).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 5–6 (inclusive, 1-indexed).
  The `[Route(...)]` attribute opens on line 5 with an array literal
  `new[] { "GET",` that never closes its `}`, and the attribute
  itself never closes its `)]`. Tree-sitter recovery flags the
  attribute span plus the immediately following `Health` method
  declaration as ERROR; the second attribute and `Version` method
  parse cleanly after the recovery boundary.
- **Rationale**: Attribute argument lists are a common mid-edit
  trip point in C# — adding `Methods = new[] { ... }` to an existing
  `[Route]` and forgetting one of the three closing tokens
  (`}` for the array, `)` for the call, `]` for the attribute).
  Recovery must preserve the well-formed tail.

## Parseable prefix

Lines 1–4 produce the namespace declaration and the `Endpoints`
class header. Lines 8–10 produce the second `Route` attribute and
the `Version` method declaration after the recovery boundary. The
harness's "≥ 1 symbol" floor passes on class + `Version`.

## Failure point

Line 5: `[Route("/health", Methods = new[] { "GET",` — the array
literal lacks its closing `}`, the call lacks its closing `)`, and
the attribute lacks its closing `]`. The newline terminates the
attribute span as ERROR, swallowing line 6's `Health` method
declaration into the unrecoverable region.
