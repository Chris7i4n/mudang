# 0008 — Constrain `gumiho-mudang-lsp` to basic-RPC primitives

- **Status:** TODO (phase B)
- **Decision:** the LSP crate exposes only transport-layer primitives. All higher-level orchestration moves to the composer.
- **Tracking:** _<issue / PR link to be added>_

## Decision

`gumiho-mudang-lsp` is a small transport-layer library by design. Its
public API is approximately what `docs/ARCHITECTURE.md` §5 sketches:

- spawn / initialize / shutdown per language server;
- send any LSP request, receive any LSP response;
- subscribe to push notifications (`publishDiagnostics`, server
  custom);
- expose negotiated capabilities;
- minimal pool management (one client per language per workspace).

No "find references for X". No caching. No composition with scope. No
retry policy beyond a single attempt. No diagnostics aggregation. No
file-change handling.

## What stays out (and why)

- **per-method convenience wrappers** (`find_implementations_with_cache`,
  `rename_across_workspace`) — composition logic. Lives in the composer.
- **LSP-response caching** — composer owns `.mudang/lsp-cache/`. The
  transport crate is stateless beyond the live connection.
- **composition with the scope graph** — modes 1–5 of
  `SCOPE-LSP-COMPOSITION.md` live in the composer.
- **diagnostics aggregation across servers** — composer.
- **file-change handling** — composer's event bus.
- **automatic retry beyond one attempt** — composer applies retry
  policy per command kind.

The unifying reason: the LSP wire protocol is unstable per server.
The right place to absorb that instability is the composer's wrappers,
not the transport crate. Keeping the transport crate small means
swapping wrappers (or pinning per-server quirks) does not destabilise
the transport.

## Affected code

- the LSP crate (currently `gumiho-mudang-lsp/`) — kept small.
- all convenience methods that would be tempting to add live in
  `gumiho-mudang-composer` once phase C opens.

## Acceptance

- every method listed in `SCOPE-LSP-COMPOSITION.md` §13 is callable
  via `gumiho-mudang-lsp`'s generic `request(...)` / `notify(...)`
  pair without a dedicated wrapper;
- the crate has no dependency on `gumiho-mudang-scope`;
- the crate has no caching layer;
- cold start, idle teardown, crash recovery behaviours from
  `SCOPE-LSP-COMPOSITION.md` §7 are implemented and tested;
- the pool type (`LspPool`) exposes warm-all, restart, status — no
  semantic operations.

## Dependencies

- phase A of `docs/ROADMAP.md` complete.

## Non-goals

- this TODO does not specify which LSP server versions are pinned —
  that lives in `SCOPE-LSP-COMPOSITION.md` §15.6;
- this TODO does not block phase C: the composer can begin its
  composer-side work as soon as the basic-RPC surface stabilises.

## Discipline note

This boundary is deliberately costly to break. Adding a "small
convenience method" inside `gumiho-mudang-lsp` re-opens the question
of where caching, retries, and composition live. When tempted, the
right move is to add the wrapper in the composer crate instead, even
if it feels like duplication initially. The composer is the single
place where LSP method calls become user-facing commands.
