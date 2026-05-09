# gumiho-mudang Charter

> Top-level invariants. Per-crate charters live alongside their crates
> (e.g., `gumiho-mudang-scope/CHARTER.md`) once migration begins.

## Purpose

gumiho-mudang is the code intelligence oracle that gumiho (the LLM)
consults to answer questions about code. It composes a fast syntactic
engine with an opt-in semantic oracle. The mudang is consumed; it does
not consume.

## Invariants (top-level)

1. **Mudang is consulted, not in control.** The CLI surface answers
   questions; it never drives gumiho or another caller. Other gumiho-*
   projects are unaware of mudang's internals.
2. **Two techniques, one schema.** Syntactic and semantic results share
   one output schema. Every record is tagged with its `producer`
   (`syntactic` | `lsp:<server>` | `hybrid`) so consumers can reason
   about confidence and provenance.
3. **Syntactic is the default. LSP is opt-in.** The hot path is
   sub-second and offline. LSP integration ships behind explicit flags
   (e.g., `--resolve`, `--types`) and degrades gracefully when an LSP
   server is unavailable.
4. **One graph across N languages.** Symbols and edges share one
   schema regardless of source language. Per-language plugins translate;
   they do not fragment the graph.
5. **No interpreter, no compiler.** Static-text analysis only. Runtime
   evaluation, macro expansion, and type inference are LSP territory —
   never reimplemented inside `gumiho-mudang-scope`. (See `gumiho-mudang-scope`
   charter for the full list of universal rules.)
6. **Cache invalidation is unified.** File hashes drive both the
   syntactic graph and any LSP-derived facts cached at this layer.

## Out of scope

- Becoming an LSP server itself.
- Embedding language-specific compilers.
- Cross-process daemon mode (CLI per invocation; LSP servers managed
  per-CLI-process via `gumiho-mudang-lsp`).
- External distribution / publishing crates to crates.io.

## Naming

- Repo / project: `gumiho-mudang`.
- Binary: `mudang`.
- Crates: `gumiho-mudang-scope`, `gumiho-mudang-lsp`, `gumiho-mudang-cli`.

## Status

Skeleton. Migration pending — see `docs/MIGRATION.md` (TBD) for the
plan to import legacy `scope/` and `gumiho-lsp/` codebases.
