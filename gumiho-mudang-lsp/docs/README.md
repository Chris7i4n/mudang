# gumiho-mudang-lsp Docs

This directory documents the current LSP adapter implementation, its
known gaps, and the intended direction for closing those gaps.

## Documents

- [LIMITATIONS.md](LIMITATIONS.md) records current implementation limits.
- [REFERENCE_GAP_MAP.md](REFERENCE_GAP_MAP.md) maps the TypeScript
  reference implementation to the Rust crate, area by area.
- [CAPABILITY_TRUTH.md](CAPABILITY_TRUTH.md) defines how server
  capabilities must be treated once the implementation is completed.
- [SCOPE_OUTPUT_INTEROP.md](SCOPE_OUTPUT_INTEROP.md) records the
  forward-looking possibility — unlocked by Scope's R10 typed output
  schema — of a future composition layer consuming Scope
  output via shared Rust types or TS/typeshare-generated bindings.

## Design Principle

The authoritative source for LSP feature support is the server's real
`initialize` response, plus any later dynamic capability registration
that the client explicitly supports.

Static registry data may describe install commands, file extensions,
language IDs, and bootstrap hints. It must not be treated as the final
truth for semantic capability support.

## Surface Boundary

`gumiho-mudang-lsp` should expose the raw LSP protocol surface: canonical
JSON-RPC methods, protocol params, protocol results, lifecycle, file
sync, diagnostics, and observed server capabilities.

It should not own user-facing semantic compositions. Higher layers may
build operations such as "incoming calls for the symbol at this cursor"
by combining raw LSP methods like `textDocument/prepareCallHierarchy`
and `callHierarchy/incomingCalls`.
