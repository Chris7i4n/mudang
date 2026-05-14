# Labeller Interface Contract

Forward-looking requirements doc. Captures what `scope-audit-labeller-lsp` (sibling workspace `gumiho-mudang-labeller/`, [BACKLOG (b₃)](../../gumiho-mudang-scope/docs/BACKLOG.md)) needs from `gumiho-mudang-lsp` so that one labeller binary can drive **N** language servers through a **single** public interface, with the LSP crate owning routing, lifecycle, and per-server quirks internally.

The labeller is the first non-CLI consumer of this crate. This document is the spec the sprint that reopens (b₃) consumes — when the items listed under *Required Closures* are all done, the labeller becomes a thin adapter.

For the gaps that still exist today, see [`LIMITATIONS.md`](LIMITATIONS.md) and [`REFERENCE_GAP_MAP.md`](REFERENCE_GAP_MAP.md). For the capability-handling target model that this doc references, see [`CAPABILITY_TRUTH.md`](CAPABILITY_TRUTH.md).

## Consumer context

- The labeller crate lives in `gumiho-mudang-labeller/scope-audit-labeller-lsp/`. That workspace is **excluded** from the root cargo workspace; the R14 `labeller-workspace-isolation` gate ([`ENFORCEMENT-MAP.md` § R14](../../gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md)) forbids cargo `path` dependencies crossing the boundary in either direction.
- When the labeller reopens, it consumes this crate through one of the three mechanisms recorded in [`BACKLOG.md` § Priority 1 (b₃)](../../gumiho-mudang-scope/docs/BACKLOG.md#priority-1--self-correction-cycle) (separate workspace move / crates.io publish / R14 whitelist amendment). Pick before any code lands.
- The labeller is a **batch consumer**: stdin → v2 JSONL sample records → per-record (path, position, expected target) query against an LSP server → stdout → labelled v2 JSONL. No interactive surface, no editor integration, no diagnostics streaming.

## Single-interface principle

"Single interface" means **one uniform call shape across every language**, not "no parameters". The labeller still tells the crate which language it wants because the labeller knows that already — every v2 `SampleRecord` carries `producer` (`rust`, `typescript`, `python`, `go`, `java`, `csharp`, `ruby`) and `lang_version`. The crate does not infer language from file extension at the call site; extension-only routing is rejected on the grounds spelled out under *Routing key* below.

The labeller hands the crate a `(language, file_path, position, operation)` quadruple; the crate:

1. Picks the right server for that **explicit language** (filtered through `.athena/lsp.json` enabled IDs when multiple servers exist for one language), **starting it lazily if needed**.
2. Performs file sync (`didOpen` if first touch in this run; `didChange` on subsequent records into the same file; `didClose` on session drop) — see *File-sync invariant* below for the sticky-binding rule that prevents `didChange` from crossing instances.
3. Checks the server's **real** capabilities (parsed from `initialize` response, not from registry presets — see [`CAPABILITY_TRUTH.md`](CAPABILITY_TRUTH.md)).
4. Dispatches the request to a method the server supports, falling back along a documented chain when the primary method is unsupported.
5. Returns a typed result, **or** a typed abstain reason if no method on this server can answer.

The labeller's call site stays **server-agnostic** (it does not name `rust-analyzer` / `tsserver` / `gopls` — only languages). Adding a new language at the labeller side reduces to registering the new server in the LSP crate's registry (or `.athena/lsp.json` settings) and adding the language to the `Language` enum.

## Routing key

Routing by file extension alone is insufficient. The same extension serves multiple languages or multiple competing servers in the same language:

- `.h` — C, C++, Objective-C share the extension.
- `.ts` — `typescript-language-server` (Node tsserver) and `denols` (Deno) both claim it; one workspace can use either.
- `.js` — tsserver, Flow language server, ESLint-LSP, eslint can all register.
- `.py` — `pyright`, `pylsp` (python-lsp-server), `ruff-lsp`, `jedi-language-server` all serve Python with different capability sets.
- `.cs` — `omnisharp` vs `csharp-ls`.

If the manager's picker is `.find(first match by extension)`, the choice is non-deterministic in practice (depends on insertion order, on `.athena/lsp.json` enable order, on the current `instances` HashMap iteration). A second call into the same file via `didChange` may land on a different instance than the first `didOpen`, breaking server-side document version invariants and producing silent miscompares.

The labeller therefore passes `language` explicitly. The crate's routing key is the pair **`(language, file_ext)`**, with `language` as the primary discriminator. The picker rejects ambiguity inside one language with a typed error (see *Multi-server within one language* below) — never silently picks one.

### Multi-server within one language

When `.athena/lsp.json` enables two servers for the same language (e.g. both `pyright` and `pylsp`), the labeller must disambiguate at session construction. Two acceptable shapes:

- **Per-language preference.** Settings carry a per-language single preferred server ID; the manager honours it and ignores the others when answering labeller requests. The unused server may still be running for CLI consumers — the labeller-facing picker just doesn't see it.
- **Multiple managers.** The labeller constructs one `LspServerManager` per language, each given only that language's enabled servers (already filtered upstream). Routing inside a manager is then unambiguous because there is one server per language by construction.

Either shape is acceptable; the contract is that **the labeller's hot-path call never observes ambiguity**. If ambiguity is reachable, the crate surfaces it at construction time, not at first `resolve`.

### File-sync invariant — sticky `(uri, server_id)` binding

`didOpen` records `(uri, server_id)` in `opened_files`. Every subsequent `didChange` / `didSave` / `didClose` for that `uri` **must route to the same `server_id`**, regardless of how the labeller's later call would re-derive the server from `(language, file_path)`. This is what prevents the bug the routing-key section motivates: if a labeller bug or settings change made the language→server resolution drift mid-session, file-sync still goes to the instance that first saw the file. The labeller is not responsible for enforcing this — it is the manager's invariant. A mismatch between the cached `server_id` and the freshly resolved one is a typed error (`AbstainReason::FileSyncRebound { uri, original, resolved }`), not a silent rebinding.

`didClose` removes the binding; the next `didOpen` for the same uri may legitimately pick a different server (e.g. settings reloaded). Within one session-without-close, the binding is immutable.

## Required public API

### Construction

```rust
// Hypothetical post-closure shape. Naming is illustrative.
let manager = LspServerManager::new(InstanceConfig {
    request_timeout: Duration::from_secs(10),     // see Required Closure (2)
    ..Default::default()
});

// Either pass enabled defs explicitly or accept registry defaults
// filtered by .athena/lsp.json.
for def in lsp_settings.enabled_servers() {
    manager.enable(def, &workspace_root).await;
}
```

The labeller never spawns processes itself, never names a server ID in hot-path code, and never holds an `LspClient` directly. Spawn is lazy; the first request that needs a server triggers `start` + `initialize`.

### Hot-path query

```rust
// Single, server-agnostic call. `language` is required — the labeller
// already knows it from the v2 record's `producer` column and passes
// it explicitly so routing is deterministic (see Routing key).
//
// `operation` is a labeller-vocabulary enum, not an LSP method name.
// The crate decides which underlying method(s) to invoke.
let outcome: LabellerOutcome = manager.resolve(
    Language::Rust,
    &file_path,
    LspPosition { line, character },
    LabellerOperation::DefinitionOf,
).await;
```

`Language` is an enum covering the seven Scope-supported languages (`Rust`, `TypeScript`, `Python`, `Go`, `Java`, `CSharp`, `Ruby`). The labeller maps `SampleRecord.producer` → `Language` at the call site; the crate never parses `producer` itself (that string is a Scope-side contract, not an LSP-crate concern).

`LabellerOperation` enumerates the small set the labeller cares about:

- `DefinitionOf` — "where is the symbol at this position defined?" (primary `textDocument/definition`, fallback `textDocument/typeDefinition`, fallback `textDocument/declaration`).
- `ReferencesOf` — "where is this symbol referenced?" (primary `textDocument/references`).
- `IncomingCalls` / `OutgoingCalls` — call-hierarchy queries; primary path is `textDocument/prepareCallHierarchy` then `callHierarchy/incomingCalls` or `callHierarchy/outgoingCalls`.
- `HoverOf` — last-resort type/identity disambiguation; primary `textDocument/hover`.

The crate's *internal* mapping `LabellerOperation → ordered list of LSP methods` is fixed and documented; the labeller does not pick the chain.

### Outcome shape

```rust
pub enum LabellerOutcome {
    Resolved {
        server_id: String,            // for labeller_id stamping
        server_version: String,       // recovered from initialize result.serverInfo
        method_used: String,          // which LSP method actually answered
        target_uri: String,
        target_range: LspRange,
        raw_response: serde_json::Value, // for evidence column
    },
    Abstained(AbstainReason),
}

pub enum AbstainReason {
    NoServerForLanguage,              // registry has no entry / not enabled for that Language
    AmbiguousServerForLanguage {      // multiple servers enabled for one language; settings did not disambiguate
        language: Language,
        candidates: Vec<String>,
    },
    ServerNotInstalled,               // `which` lookup failed
    ServerStartupFailed { detail: String },
    NoMethodSupported,                // every method in the dispatch chain unsupported
    Timeout { method: String, after: Duration },
    Crashed { restart_attempted: bool },
    ServerReturnedNull { method: String }, // server has the capability but knows nothing
    Transport { detail: String },     // JSON-RPC error other than -32801 ContentModified
    FileSyncRebound {                 // sticky-binding invariant violated; never silently rebind
        uri: String,
        original: String,
        resolved: String,
    },
}
```

Two labeller-side decisions follow directly:

- `LabellerOutcome::Resolved` populates the v2 record's `target_proposed`, `evidence`, and `kind_proposed` columns; `labeller_id` is stamped as `lsp:<server_id>:<server_version>` per the schema doc convention ([`AUDIT-LABEL-SCHEMA.md` § `labeller_id`](../../gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md)).
- `LabellerOutcome::Abstained` clears the seven labeller-fillable columns (per the same abstain-clears-prior-verdict rule the LLM labeller followed in sprint 0010, [`scope-audit-labeller-llm/src/verdict.rs`](../../gumiho-mudang-labeller/scope-audit-labeller-llm/src/verdict.rs)) and stamps `labeller_id` so the abstain is attributable. `reasoning_text` records the `AbstainReason` variant.

### Session lifecycle

```rust
// At end of run (Drop or explicit):
manager.stop_all().await;
```

`stop_all` shuts down every started server cleanly (`shutdown` → `exit` → process drop). No labeller-side process management.

## Best-effort dispatch — contract

For each `LabellerOperation`, the crate walks the ordered method chain and stops at the first method the server's **observed** capabilities advertise. If a method advertises but returns null / empty, that counts as `ServerReturnedNull` for that method — the next method in the chain is **not** tried (the server has spoken; further methods would shadow that signal). If a method is unsupported, the next method is tried; if every method in the chain is unsupported, return `NoMethodSupported`.

Rationale: best-effort is about *capability*, not *content*. Falling through on empty content would let the labeller silently stamp a worse-quality answer (e.g. `hover` text when `definition` had the truthful "no such symbol" answer).

The dispatch chain per operation is fixed at this crate's edge — the labeller does not configure it. Future ambiguity (a new operation, a server with non-standard alternate methods) is resolved here, not duplicated at every consumer.

## Capability handling — required closures

The single-interface promise depends on capability detection being **real**, not preset. The items below are already listed in [`LIMITATIONS.md`](LIMITATIONS.md) and [`REFERENCE_GAP_MAP.md`](REFERENCE_GAP_MAP.md) under their respective sections; they are repeated here as a labeller-facing punch list so the reopening sprint can checklist them.

1. **Parse `initialize_result.capabilities`** into a runtime `ObservedCapabilities` value held on `LspServerInstance`. Static `LspCapabilities::FULL` / `NO_CALL_HIERARCHY` / `BASIC` presets become bootstrap hints only; they are not consulted after handshake. ([`CAPABILITY_TRUTH.md`](CAPABILITY_TRUTH.md) records the target model.)
2. **Per-request timeout** on `LspClient::request` (and the manager-level `send_request`). The labeller's per-record budget defaults to 10s; the value comes from `InstanceConfig::request_timeout`. On expiry, the request is cancelled and an `AbstainReason::Timeout` returned.
3. **Canonical lowercase language IDs**. `didOpen` must send `rust`, `typescript`, `python`, `go`, `java`, `csharp`, `ruby` — not the human-readable `Rust` / `TypeScript` / etc. that the registry currently surfaces. Some servers (pyright, typescript-language-server) reject the human-readable form.
4. **Server-initiated request responder**. The client must respond to `workspace/configuration`, `client/registerCapability`, and `workspace/workspaceFolders` requests with the minimal safe answer (empty config, ack registrations the client claims to support, current workspace folder). Servers that wait for a response on these will otherwise hang and the labeller will see only timeouts.
5. **Canonical `serverInfo` capture**. The `initialize` response carries `serverInfo.name` and `serverInfo.version`. Both must be stored on `LspServerInstance` and exposed via the manager so the labeller can build `labeller_id = lsp:<name>:<version>` without re-querying.
6. **`ContentModified` (-32801) transient retry**. The current crate does not retry; LSP semantics treat this as "the document changed; retry the request". For the labeller's per-record flow this should be a bounded retry inside `send_request` (e.g. up to 3 attempts) before surfacing `AbstainReason::Transport`.
7. **Workspace root upward search**. The current shallow root detection fails on monorepo-style fixtures where the sample's `file_path` sits below the manifest. The labeller hands the crate a per-record file path; the crate must walk upward to the nearest server-recognised root marker.
8. **Language-explicit routing.** `LspServerManager::resolve` takes `Language` as a required parameter (see *Hot-path query*); the manager filters its `instances` map by language **before** filtering by extension. The current `get_instance_for_file` (`registry.rs` / `manager.rs`) routes by extension only and surfaces a non-deterministic `.find(first)` choice when multiple servers cover the same extension — that picker is replaced. File-sync notifications (`didOpen` / `didChange` / `didSave` / `didClose`) thread the resolved `server_id` through the sticky binding described under *File-sync invariant*; `didChange` never re-derives the server from extension.
9. **Per-language disambiguation in `.athena/lsp.json`.** When two servers are enabled for one language, settings must carry a per-language preferred-server key (e.g. `"python": { "preferred": "pyright" }`); the manager honours it, otherwise the manager returns `AmbiguousServerForLanguage` at construction time — never silently at first `resolve`.

Items 1–5 and 8–9 are **mandatory** before the labeller can claim "single interface, multi-server". Item 6 is mandatory for any server that ever emits `ContentModified` (rust-analyzer does under load). Item 7 is mandatory for tsserver / pyright / gopls; rust-analyzer survives without it because `cargo` projects are typically the cwd.

## Non-goals (for the labeller's view of this crate)

- **Semantic composition** above the protocol layer (LSPTool-style "find incoming calls for the symbol at cursor"). The labeller does its own composition; this crate exposes raw LSP methods plus the small `LabellerOperation` dispatch over them.
- **Diagnostics integration**. The labeller does not read `textDocument/publishDiagnostics`; the diagnostics registry exists for CLI consumers and is harmless background noise to the labeller path.
- **Multi-root workspace negotiation**. Each labeller run targets a single workspace root recovered from upward search; multi-root is a CLI-side concern.
- **Persistence**. The labeller does not need diagnostics or capabilities cached across runs.
- **Dynamic registration coverage** beyond the responder in closure (4). The labeller does not register capabilities at runtime.

## Why the labeller, not the crate, owns the verdict-shape mapping

The crate returns a typed `LabellerOutcome`. Mapping that to v2 schema columns (`target_proposed`, `evidence`, `kind_proposed`, etc.) lives in the labeller crate, because:

- The wire shape of `evidence` is defined in [`AUDIT-LABEL-SCHEMA.md`](../../gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md) — the labeller workspace consumes that doc, not Scope crates. Embedding the mapping here would couple this crate to the schema doc and inherit a doc-sync gate it should not be subject to.
- The single-operator-posture rule that "the labeller owns the seven labeller-fillable columns when its `labeller_id` is stamped" (sprint 0010 codex round 1 finding, [`scope-audit-labeller-llm/src/verdict.rs:apply_to`](../../gumiho-mudang-labeller/scope-audit-labeller-llm/src/verdict.rs)) is labeller-side state hygiene, not LSP-protocol state.
- Future labellers (hybrid, ML-driven) compose **over** the LSP labeller's output. The verdict-shape mapping lives where composition happens.

## Doc-sync hook

When the labeller reopens (b₃), the reopening sprint's plan must reference this doc by path. The doc-sync gate (R13) does not currently watch this file; if a future sprint adds a check tying labeller-side code to specific sections here, register it under R13 in the same commit (see [`SELF-CORRECTION-CYCLE.md` § Extending the doc-sync gate](../../gumiho-mudang-scope/docs/SELF-CORRECTION-CYCLE.md)).

## Summary checklist (for the reopening sprint)

- [ ] Consumption mechanism decided per [`BACKLOG.md` § Priority 1 (b₃)](../../gumiho-mudang-scope/docs/BACKLOG.md#priority-1--self-correction-cycle).
- [ ] Closures 1–5 above shipped in `gumiho-mudang-lsp` (mandatory for any multi-server claim).
- [ ] Closures 8–9 shipped — language-explicit routing replaces extension-only picker; multi-server-per-language ambiguity surfaced at construction (mandatory for any multi-server claim, regardless of how many languages this sprint covers).
- [ ] Closure 6 shipped (mandatory for rust-analyzer reliability).
- [ ] Closure 7 shipped if the reopening sprint covers any server beyond rust-analyzer.
- [ ] `Language` enum landed and threaded through `resolve` + file-sync.
- [ ] `LabellerOperation` enum + dispatch chain landed in this crate.
- [ ] `LabellerOutcome` / `AbstainReason` enums landed.
- [ ] `(uri, server_id)` sticky-binding invariant enforced in the manager; `FileSyncRebound` returned on violation rather than silent rebind.
- [ ] Manager exposes `serverInfo` recovery so the labeller can stamp `lsp:<server>:<version>`.
- [ ] Labeller crate becomes a thin adapter: read v2 JSONL → map `producer` → `Language` → for each record call `manager.resolve(language, ...)` → map outcome to verdict fields → write v2 JSONL.
