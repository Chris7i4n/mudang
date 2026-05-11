# 0004 — Clarify ONNX vs LanceDB roles in the embeddings stack

- **Status:** TODO
- **Decision:** treat ONNX and LanceDB as separate, complementary layers. Never describe one as a replacement for the other.
- **Tracking:** _<issue / PR link to be added>_

## Decision

When introducing semantic search (replacing the FTS5 backend in
`gumiho-mudang-scope/src/core/searcher.rs`), the implementation will
involve **two orthogonal components**, not one. Internal docs,
config comments, CLI help text, and commit messages must keep the
distinction explicit so contributors don't conflate them.

## The two layers

| Layer | Responsibility | Examples |
|-------|----------------|----------|
| **Embedding runtime** | Turn text into a fixed-dimension vector. Runs the model. | ONNX Runtime (`ort` crate), `candle`, `fastembed-rs`, or remote APIs (Voyage, OpenAI, Cohere) |
| **Vector store** | Persist vectors and run ANN search (cosine / L2 / hybrid filter). | LanceDB, Qdrant, pgvector, USearch |

LanceDB does **not** run models. It stores vectors. Even when
LanceDB exposes "embedding functions" in its Python API, those
wrappers internally call ONNX Runtime, sentence-transformers, or a
remote API — they are not a substitute for the runtime layer.

## Pipeline shape

```
Symbol → embedder::build_embedding_text (already exists)
       → [embedding runtime: ONNX | candle | API]   → float32[N]
       → [vector store: LanceDB]                    → persisted with symbol_id
```

Query path:

```
"handles auth errors"
  → [same embedding runtime, same model]            → float32[N]
  → LanceDB ANN search (+ optional scalar filter)
  → JOIN symbol_id against SQLite `symbols`         → metadata, file, line
  → SearchResult
```

## Why this matters

- The current `searcher.rs:6-9` comment hints at "LanceDB +
  embeddings" as one swap. That phrasing has already caused
  confusion: LanceDB on its own does not produce vectors.
- The `[embeddings]` section of `.scope/config.toml` exposes
  `provider = "local" | "voyage" | "openai"` and `model`. Those
  fields configure the **runtime layer**, not the vector store.
  The vector store is fixed (LanceDB) once chosen.
- Swapping models or providers requires re-embedding every symbol
  because dimensions and semantics change. Vector store schema is
  pinned to a specific (provider, model, dim) tuple.

## Affected code (when implementation lands)

- **`gumiho-mudang-scope/src/core/searcher.rs`** — split into a
  `Searcher` trait with at least two backends: `Fts5Searcher`
  (current) and `LanceSearcher` (new). LanceSearcher composes an
  `Embedder` trait.
- **New `gumiho-mudang-scope/src/core/embedder.rs` split** — keep
  the existing `build_embedding_text` (text construction) and add
  an `Embedder` trait with implementations: `OnnxEmbedder`,
  `CandleEmbedder`, `RemoteEmbedder { provider: Voyage | OpenAI }`.
- **`gumiho-mudang-scope/src/config/project.rs`** — extend the
  `[embeddings]` section to spell out the two layers:
  - `runtime = "onnx" | "candle" | "api"`  (how vectors are produced)
  - `provider = "local-bge-small" | "voyage-3" | "openai-3-small"`  (which model)
  - `store = "lance" | "fts5"`  (where vectors live; `fts5` keeps current behaviour)
- **Index pipeline (`indexer.rs`)** — `index_full` and
  `index_incremental` must invoke the embedding runtime per symbol
  (batched), then hand vectors to the vector store. Both backends
  share the same `Searcher::index_symbols` signature.
- **Schema** — `.scope/lance/` (or `.mudang/lance/` after TODO
  0001 lands) for LanceDB tables. Keep `.scope/graph.db` for the
  relational graph; do **not** migrate the graph into LanceDB.
  Recursive CTEs (`find_impact`, `find_deps`, `find_call_paths`,
  `find_flow_paths`) depend on SQLite.

## Documentation rules

When writing user-facing or contributor-facing docs:

1. **Always name both layers.** "ONNX produces the vector, LanceDB
   stores and searches it."
2. **Never write "use LanceDB for embeddings."** That phrasing
   implies LanceDB does inference. It does not.
3. **Never write "ONNX replaces FTS5."** ONNX is a model runtime.
   FTS5 is replaced by LanceDB (the store), with ONNX feeding it.
4. **Pin the (provider, model, dim) tuple** anywhere the schema is
   discussed. Reindex is forced whenever any of the three change.

## Non-goals

- This TODO does **not** decide which runtime ships first (ONNX
  Runtime vs candle vs API-only). That belongs in a follow-up
  design doc with binary-size, latency, and licensing trade-offs.
- This TODO does **not** decide whether the relational graph
  migrates to LanceDB. It explicitly says it should not.

## Follow-up: Tier 2 enriched embeddings

The pipeline above produces what is referred to in
`docs/SCOPE-LSP-COMPOSITION.md` §14.5 (Case AA) as **tier 1
embeddings** — vectors derived from the syntactic embedding text
already built by `embedder::build_embedding_text`.

A second tier of embeddings is planned in which the text that goes into
the runtime is **augmented at index time with LSP-derived facts** (mode
4 in the composition doc's §1.2). The two tiers coexist in separate
LanceDB tables and are queried together with rank fusion.

This TODO captures the runtime / store distinction; tier 2 is captured
separately in the composition doc. The relationship is:

```
Symbol → build_embedding_text_v1 (syntactic)          → runtime → vector_v1 → table vectors_v1_syntactic
                ↓
        lsp::enrich_symbol (hover, inlayHint, …)
                ↓
        build_embedding_text_v2 (semantic-augmented)  → runtime → vector_v2 → table vectors_v2_enriched
```

The cache key for tier 2 must include `lsp_server_version` in addition
to the `(provider, model, dim)` tuple this TODO pins. Tier 1 ignores
LSP versioning.

When this TODO's affected-code changes land:

- the `Embedder` trait must be tier-agnostic (it just embeds whatever
  text it is given);
- the `Searcher` trait must accept a vector-table identifier (`v1` or
  `v2`) and support `query_fused(v1, v2, ...)`;
- the `[embeddings]` config section gains:
  - `tier2.enabled = true | false`,
  - `tier2.enrich_timeout_ms`,
  - `tier2.batch_budget_seconds`,
  - `tier2.skip_low_importance = true` (cost optimisation).

Tier 2 is **not** a prerequisite for shipping tier 1. Tier 1 alone
already replaces FTS5 for `find` and is the first acceptance criterion.
