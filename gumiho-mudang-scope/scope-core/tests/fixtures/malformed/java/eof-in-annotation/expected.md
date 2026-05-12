# Fixture — eof-in-annotation

- **Category**: EOF inside annotation argument list (Java-specific
  variant exercising annotation-element-value-pair recovery).
- **Reason expected**: `tree_sitter_error`.
- **Skipped range expected**: lines 6–7 (inclusive, 1-indexed).
  The `@Route(...)` annotation opens on line 6 with `methods = {"GET",`
  — the array literal lacks its closing `}` and the annotation
  itself lacks its closing `)`. The newline terminates the annotation
  span as ERROR, swallowing line 7's `health` method declaration
  into the unrecoverable region; the second annotation on line 9 and
  `version` method on line 10 parse cleanly after the recovery
  boundary.
- **Rationale**: Annotation element-value pairs are a routine
  mid-edit trip point in Java — threading one more value into an
  existing `@Route` and forgetting one of the closing tokens
  (`}` for the array, `)` for the annotation). Recovery must
  preserve the well-formed tail.

## Parseable prefix

Lines 1–5 produce the package declaration, the `Route` import, and
the `Endpoints` class header. Lines 9–10 produce the second `@Route`
annotation and the `version` method after the recovery boundary.
The harness's "≥ 1 symbol" floor passes on class + `version`.

## Failure point

Line 6: `@Route(path = "/health", methods = {"GET",` — the array
literal lacks its closing `}`, the annotation lacks its closing `)`.
The newline terminates the annotation span as ERROR; line 7's
`health` method declaration is consumed into the unrecoverable
region.
