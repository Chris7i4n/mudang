# scope-audit-labeller-llm

LLM-backed labeller for the Scope audit loop. Reads v2 JSONL records from stdin, asks a model whether each extracted edge is correctly labelled, writes labelled v2 JSONL to stdout. Lives in the [`gumiho-mudang-labeller/`](../) sibling workspace — never enters the Scope binary.

## Role in the pipeline

```
scope audit confidence --emit-sample sample.jsonl
    →  scope-audit-labeller-llm < sample.jsonl > sample.labelled.jsonl
    →  scope audit confidence --label sample.labelled.jsonl
```

The crate's `LlmLabeller<P>` wraps any `Provider` (HTTP transport to a chat-completions API) and exposes the `Labeller` trait the audit loop calls one record at a time. Composability lives at the trait level — sprint 0012's hybrid composer holds an `LlmLabeller` and forwards records.

## Architecture

Three layers, top-down:

1. **`LlmLabeller<P>`** (`src/labeller.rs`) — implements `Labeller`. Per record: render the prompt → call the provider → parse the response → copy verdict fields onto the record → stamp `labeller_id`. Errors at any step become an **abstain** record (the seven labeller-fillable fields stay `null`, `labeller_id` still stamped, diagnostic line written to stderr).
2. **`Provider` trait** (`src/provider.rs`) — the transport seam. One method: `complete(&Prompt) -> Result<ProviderResponse, Self::Error>`. Retry / rate-limit handling lives **inside** the provider; by the time `complete` returns, the bounded retry policy has already run.
3. **Concrete providers** (`src/providers/`) — each behind its own cargo feature.

## Provider features

| Feature | Default | Provider | Endpoint |
|---|---|---|---|
| `deepseek` | yes | DeepSeek chat-completions | `https://api.deepseek.com/chat/completions` |

Additional providers (Anthropic, OpenAI, Gemini, local llama.cpp / ollama) are deferred to follow-up sprints. Each lands as its own cargo feature without touching existing builds.

`cargo add scope-audit-labeller-llm` selects the DeepSeek feature out of the box. Operators wanting a different provider build with `--no-default-features --features <provider>`.

## DeepSeek configuration

Set `DEEPSEEK_API_KEY` in the environment. Optional overrides:

```rust
DeepSeekProvider::from_env()?
    .with_model("deepseek-reasoner")           // default: "deepseek-chat"
    .with_endpoint("https://example.com/v1")   // default: api.deepseek.com
    .with_max_retries(2)                       // default: 4
    .with_base_backoff(Duration::from_millis(250));  // default: 500 ms
```

Retry policy: bounded exponential backoff on HTTP 429, HTTP 5xx, and transport-layer errors (DNS / TCP / TLS). After `max_retries`, the provider surfaces a `DeepSeekError` which `LlmLabeller` catches and turns into an abstain record.

## `labeller_id` convention

Stamped onto every record: `llm:<provider_id>:<model_id>`. For the default DeepSeek configuration: `llm:deepseek:deepseek-chat`. The `provider_id` and `model_id` come from `Provider::provider_id` and `Provider::model_id` respectively; a custom provider impl picks its own constants.

## Prompt template

The system prompt commits the model to emitting a single JSON object whose keys mirror the seven v2 labeller-fillable columns exactly:

```json
{
  "label": true,
  "evidence": {"reasoning": "exact match"},
  "target_proposed": null,
  "kind_proposed": null,
  "confidence_proposed": null,
  "reasoning_text": "extractor is correct",
  "lang_version_evidence": null
}
```

Inputs the user message exposes to the model: `kind`, `from`, `to`, `extractor_confidence`, `producer`, `pattern_id`, `lang_version`, `source_snippet`. `producer_captured_args` is reserved for a future schema extension and not yet sent.

`label: null` means abstain. The system prompt explicitly tells the model that a confident wrong answer is worse than an honest abstain.

## Running the binary

Build:

```sh
cd gumiho-mudang-labeller
cargo build --release -p scope-audit-labeller-llm
```

Run:

```sh
export DEEPSEEK_API_KEY=sk-...
scope audit confidence --emit-sample sample.jsonl
./target/release/scope-audit-labeller-llm < sample.jsonl > sample.labelled.jsonl
scope audit confidence --label sample.labelled.jsonl
```

Exit codes:

- `0` — every input record was processed (`Ok` from `run_labeller`). Individual records may be abstains; abstain is not a process-level error.
- `1` — pipeline-level failure (JSONL parse error on the input stream, IO error writing stdout). Stderr carries the diagnostic.
- `2` — startup failure (no `DEEPSEEK_API_KEY`, or built with no provider feature enabled). Stderr carries the diagnostic.

## Testing

Default (no network):

```sh
cargo test -p scope-audit-labeller-llm
```

Covers prompt rendering, verdict parsing, `LlmLabeller<MockProvider>` end-to-end pipeline, abstain-on-error resilience, binary-without-API-key smoke test.

Live DeepSeek round trip (opt-in, costs one API call):

```sh
DEEPSEEK_API_KEY=sk-... cargo test \
    -p scope-audit-labeller-llm \
    --features live-deepseek-tests \
    --test live_deepseek
```

Two-layer gate: the `live-deepseek-tests` cargo feature gates compilation, and the test additionally no-ops when `DEEPSEEK_API_KEY` is unset. Default `cargo test --workspace` therefore never reaches the network.

## Throughput

Order-of-magnitude only; not a regression gate. Rough numbers on `deepseek-chat` with `temperature = 0` and default retries: ≈40–80 records / minute single-threaded. Limited by per-call model latency, not transport. Parallelism is left to a future sprint (would need the `Labeller` trait to grow a batched method or the runner to fan out — neither shape is committed).

## Adding a new provider

1. Add the feature to `Cargo.toml`'s `[features]` block. Gate the provider's optional dependencies on it.
2. Create `src/providers/<name>.rs`. Implement `Provider` with stable `provider_id` and `model_id` strings.
3. Wire the feature gate in `src/providers/mod.rs` (`#[cfg(feature = "<name>")] pub mod <name>;`).
4. Add an integration test against the live endpoint behind a `live-<name>-tests` feature + env-var gate, mirroring `live_deepseek.rs`.
5. Document the feature row in the table above.

`LlmLabeller`, the prompt template, and the verdict parser are provider-agnostic by construction — no changes needed when a new provider lands.

## Charter alignment

This crate lives in the sibling labeller workspace; the root Scope workspace is `[workspace] exclude`-d from it. The R14 `labeller-workspace-isolation` gate enforces the boundary in both directions on every CI run. Adding an LLM provider here never adds a network dependency to the Scope binary — that is the whole point of the workspace split.
