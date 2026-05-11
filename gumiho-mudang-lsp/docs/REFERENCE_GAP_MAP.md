# Reference Gap Map

This document maps the TypeScript LSP implementation used as the
reference design to the current Rust implementation in
`gumiho-mudang-lsp`.

The goal is not to clone the TypeScript code mechanically. The goal is to
identify which behavior matters for `mudang`, which behavior already
exists in Rust, and which gaps must be closed before the LSP crate can be
treated as a reliable semantic oracle.

## Reference Modules

The TypeScript reference is organized around these responsibilities:

- `config.ts`: loads LSP server definitions from plugins.
- `manager.ts`: owns a singleton manager, async initialization state,
  reinitialization, and shutdown.
- `LSPClient.ts`: owns process spawn, JSON-RPC transport, initialization,
  notifications, requests, crash handling, and server capabilities.
- `LSPServerInstance.ts`: owns one server lifecycle, initialization
  parameters, health, retry behavior, and request forwarding.
- `LSPServerManager.ts`: owns all server instances, extension routing,
  lazy start, file sync, and server-initiated request handlers.
- `passiveFeedback.ts`: converts LSP diagnostics into the product's
  diagnostic attachment format.
- `LSPDiagnosticRegistry.ts`: stores pending diagnostics, deduplicates
  them, limits volume, and tracks delivered diagnostics across turns.
- `LSPTool.ts`: maps user-facing semantic operations to raw LSP methods
  and formats LSP results.

The Rust crate currently has these equivalents:

- `registry.rs`: static built-in server registry.
- `settings.rs`: `.athena/lsp.json` enabled server settings.
- `client.rs`: process and JSON-RPC client.
- `jsonrpc.rs`: manual LSP JSON-RPC codec.
- `instance.rs`: server lifecycle wrapper.
- `manager.rs`: multi-server manager and file synchronization.
- `diagnostics.rs`: in-memory diagnostic registry.
- `status.rs`: compact status widget formatting.
- `types.rs`: local diagnostic and operation/capability types.

## Summary

| Area | Rust Status | Gap |
| --- | --- | --- |
| Process transport | Partial | Manual codec works, but lacks stderr capture, spawn-event handling, request handlers, and richer connection errors. |
| Initialize handshake | Partial | Rust sends a smaller initialize payload and discards real server capabilities. |
| Capability source | Missing | Rust uses static presets instead of `initialize_result.capabilities`. |
| Server config | Different | Rust uses built-in registry plus enabled IDs, not plugin-defined server config. |
| Language IDs | Incomplete | Rust uses human language names instead of canonical LSP language IDs. |
| Lazy startup | Missing | Rust requires explicit server startup before file operations can route. |
| Crash recovery | Partial | Rust records error state but does not restart on next use. |
| Server requests | Missing | Rust parses but ignores server-initiated requests. |
| Transient retry | Missing | Rust does not retry `ContentModified` / `-32801`. |
| File sync | Partial | Basic file sync exists, but versioning and language IDs need correction. |
| Diagnostics | Partial | Dedupe and limits exist, but conversion and product integration are missing. |
| Semantic tool surface | Intentionally external | Rust should expose raw canonical LSP methods; LSPTool-style operation composition belongs above this crate. |
| CLI integration | Missing | `mudang` declares the crate but does not route commands through it. |
| Tests | Partial | Unit tests and one real smoke example exist, but no scripted fake-LSP integration suite. |

## Capability Truth

### Reference Behavior

The TypeScript client stores the real `ServerCapabilities` from the
`initialize` response. The instance and tool layers can then reason from
the actual server response instead of assuming support from registry
metadata.

### Rust Behavior

Rust returns the raw `initialize` result from `LspClient::initialize`,
but `LspServerInstance::spawn` only checks whether initialization
succeeded and discards the result. The registry supplies static
`LspCapabilities` presets such as `FULL`, `NO_CALL_HIERARCHY`, and
`BASIC`.

### Required Closure

- Store the raw `initialize` result on `LspServerInstance`.
- Parse `initialize_result.capabilities` into runtime capability state.
- Treat static registry capabilities as bootstrap hints only.
- Expose observed capabilities through the manager.
- Route raw protocol methods through observed capabilities.
- If a server does not advertise a provider, do not claim support.
- Add tests where static registry assumptions differ from real server
  capabilities.

See [CAPABILITY_TRUTH.md](CAPABILITY_TRUTH.md) for the detailed target
model.

## Initialize Parameters

### Reference Behavior

The TypeScript instance sends:

- `processId`;
- `initializationOptions`;
- `workspaceFolders`;
- `rootPath`;
- `rootUri`;
- workspace capabilities;
- text document capabilities;
- diagnostic metadata support;
- hover content formats;
- definition link support;
- call hierarchy support;
- `general.positionEncodings`.

This broad payload exists because real servers often need more than
`rootUri`. Examples include Pyright, gopls, Vue, and
typescript-language-server.

### Rust Behavior

Rust sends:

- `processId`;
- `rootUri`;
- a small `textDocument` capability set.

It does not send `workspaceFolders`, `rootPath`, `initializationOptions`,
workspace capability flags, position encodings, hover content formats, or
definition link support.

### Required Closure

- Add an initialization config type that can carry workspace folder,
  initialization options, and optional server-specific settings.
- Send both modern and compatibility root fields where useful:
  `workspaceFolders`, `rootUri`, and possibly `rootPath`.
- Add `general.positionEncodings` and model the selected encoding.
- Keep advertised client capabilities honest: do not advertise dynamic
  registration until handlers exist.

## Server-Initiated Requests

### Reference Behavior

The TypeScript client supports `onRequest`. The manager registers a
`workspace/configuration` handler and returns `null` per requested item,
because some servers ask for configuration even when the client does not
advertise support.

### Rust Behavior

The JSON-RPC codec parses server-initiated requests, but `LspClient`
ignores `IncomingMessage::Request`.

### Required Closure

- Add request handler registration to `LspClient`.
- Send JSON-RPC responses for handled server requests.
- Return method-not-found or a controlled null response for unhandled
  server requests instead of silently dropping them.
- Register at least `workspace/configuration`.
- Decide explicitly whether to support `client/registerCapability`.

## Process And Transport Robustness

### Reference Behavior

The TypeScript client:

- waits for successful process spawn;
- captures stderr for debugging;
- tracks intentional stopping;
- handles process `error`, `exit`, stdin errors, connection errors, and
  connection close;
- disposes connection resources during stop;
- preserves start errors for diagnostics.

### Rust Behavior

Rust:

- spawns the process through `tokio::process::Command`;
- pipes stdin/stdout;
- discards stderr;
- treats `ConnectionClosed` as crash;
- aborts the reader task and kills the process on stop.

### Required Closure

- Capture stderr and expose it through tracing or last-error context.
- Distinguish intentional shutdown from crash.
- Propagate codec errors beyond `ConnectionClosed`.
- Track process exit status where available.
- Remove stale pending requests on crash and notify their waiters.

## Lifecycle, Lazy Start, And Restart

### Reference Behavior

The TypeScript manager creates server instances during manager
initialization, but starts them lazily on first file use. If a server is
in `error`, `ensureServerStarted` tries to start it again, with a
crash-recovery limit.

Manual `restart()` also exists and enforces `maxRestarts`.

### Rust Behavior

Rust starts a server only when `LspServerManager::start` is called
explicitly. File operations route only to already running instances.
Crash state is recorded, and delayed restart logic only emits a callback.
The manager does not perform automatic restart.

### Required Closure

- Add `ensure_server_started(file_path)` to the manager.
- Allow `open_file`, `change_file`, and `send_request` to lazily start
  the selected server.
- Retry startup when a server is in `Error`, bounded by restart limits.
- Define the distinction between manual restart count and crash recovery
  count.
- Make delayed restart either real behavior or remove it.

## Server Configuration Source

### Reference Behavior

The TypeScript implementation loads LSP servers from plugins. Each server
can define:

- command;
- args;
- extension-to-language mapping;
- transport;
- environment variables;
- initialization options;
- settings;
- workspace folder;
- startup timeout;
- max restarts.

### Rust Behavior

Rust has a built-in static registry and an `.athena/lsp.json` file with
enabled server IDs. It detects installed commands using `which`.

### Required Closure

The Rust crate does not need to copy the plugin model if `mudang` does
not need plugins. It does need enough configuration to avoid hard-coded
semantic mistakes.

Minimum required additions:

- canonical extension-to-language mapping;
- per-server initialization options;
- per-server environment variables;
- per-server workspace root override;
- per-server startup timeout override;
- a way to disable or prefer overlapping servers.

## Language IDs

### Reference Behavior

The TypeScript manager derives `languageId` from
`extensionToLanguage[ext]`. That mapping stores canonical LSP language
IDs such as `rust`, `typescript`, and `python`.

### Rust Behavior

Rust uses the first value in `LspServerDef.languages`, which currently
contains human-readable names such as `Rust`, `TypeScript`, and
`Python`.

### Required Closure

- Add a canonical `extension_language_ids` mapping to `LspServerDef`.
- Use that mapping for `textDocument/didOpen`.
- Keep human-readable names separate from protocol language IDs.
- Add tests for `.rs -> rust`, `.ts -> typescript`, `.py -> python`,
  and framework-specific extensions such as `.vue`.

## File Synchronization

### Reference Behavior

The TypeScript manager:

- lazily starts a server before `didOpen`;
- resolves file paths to file URLs with standard URL helpers;
- skips duplicate `didOpen`;
- sends `didChange` only after `didOpen`;
- sends `didSave` and `didClose`;
- tracks open files by URI and server name.

### Rust Behavior

Rust implements the same basic notifications and tracks opened files by
URI and server ID. It does not lazily start servers and uses a simple
`file://{path}` formatter.

### Required Closure

- Use a proper file URL conversion API instead of formatting strings.
- Increment document versions on change.
- Clear delivered diagnostics for a file when content changes.
- Lazy-start before `didOpen`.
- Use canonical language IDs.

## Transient Request Retry

### Reference Behavior

The TypeScript instance retries LSP requests on error code `-32801`
(`ContentModified`) with exponential backoff. This matters for
rust-analyzer and other servers while they are indexing or reconciling
document state.

### Rust Behavior

Rust request forwarding has no retry behavior.

### Required Closure

- Preserve JSON-RPC error code, not just the message.
- Detect `-32801`.
- Retry with bounded exponential backoff.
- Keep non-transient errors non-retryable.
- Add fake-LSP tests for transient error then success.

## Diagnostics

### Reference Behavior

The TypeScript diagnostics path:

- converts LSP diagnostics into product diagnostic attachments;
- normalizes severity from numeric LSP values to display strings;
- decodes file URIs;
- drops empty diagnostic batches before delivery;
- deduplicates within batch and across turns;
- limits per-file and total volume;
- tracks failures in the diagnostic handler.

### Rust Behavior

Rust stores LSP diagnostic structs, deduplicates them, limits volume, and
clears delivered state when a server sends an empty diagnostic batch. It
does not convert diagnostics into a CLI or shared mudang output shape.

### Required Closure

- Add a diagnostic conversion layer for mudang output.
- Include server ID and producer metadata.
- Decode file URIs consistently.
- Decide whether empty diagnostics should be delivered as "cleared" state
  or only used to reset deduplication.
- Integrate diagnostics with CLI output or semantic cache.

## Raw Protocol Surface Versus Tool Operations

### Reference Behavior

The TypeScript `LSPTool` maps user-facing operations to raw LSP methods.
Those operation names are a product/tool contract, not protocol names.

The left side of each mapping is a mudang-facing operation name, not a
canonical LSP method. The right side is the canonical LSP JSON-RPC
method string.

| Mudang Operation | LSP Request Method |
| --- | --- |
| `goToDefinition` | `textDocument/definition` |
| `findReferences` | `textDocument/references` |
| `hover` | `textDocument/hover` |
| `documentSymbol` | `textDocument/documentSymbol` |
| `workspaceSymbol` | `workspace/symbol` |
| `goToImplementation` | `textDocument/implementation` |
| `prepareCallHierarchy` | `textDocument/prepareCallHierarchy` |
| `incomingCalls` | `textDocument/prepareCallHierarchy`, then `callHierarchy/incomingCalls` |
| `outgoingCalls` | `textDocument/prepareCallHierarchy`, then `callHierarchy/outgoingCalls` |

It also opens files before requests, filters gitignored locations, and
formats responses for the caller.

### Rust Behavior

Rust exposes raw `send_request(method, params)`. Callers must know LSP
method names, parameter shapes, response shapes, and formatting rules.

### Required Closure

- Keep `gumiho-mudang-lsp` centered on raw canonical LSP method calls.
- Add method-level runtime capability checks before sending capability-
  gated methods.
- Preserve protocol-shaped params and results at this layer.
- Do not implement `LSPTool`-style composed operations in this crate.
- Let higher layers normalize `Location`, `LocationLink`, hover markup,
  call hierarchy flows, ignored paths, and product output schemas.

## Singleton And Application Integration

### Reference Behavior

The TypeScript `manager.ts` owns process-wide initialization state:

- `not-started`;
- `pending`;
- `success`;
- `failed`;
- generation counter for stale initialization;
- wait-for-initialization;
- reinitialize after plugin refresh;
- shutdown cleanup.

### Rust Behavior

Rust exposes library structs. There is no process-wide singleton and no
CLI integration.

### Required Closure

For a CLI-first tool, Rust probably does not need a long-lived singleton.
It does need an explicit integration boundary:

- a CLI command or flag that creates a manager;
- graceful shutdown on command exit;
- clear behavior when LSP startup is pending or failed;
- explicit opt-in so syntactic commands remain fast and offline.

## Testing

### Reference Behavior

The TypeScript implementation is designed around production concerns:
plugin loading, lazy start, request handlers, diagnostics, and tool
formatting.

### Rust Behavior

Rust has focused unit tests and one real `rust-analyzer` smoke example.

### Required Closure

- Add a scripted fake LSP server for deterministic integration tests.
- Test initialize capability parsing.
- Test `workspace/configuration` responses.
- Test lazy start.
- Test crash then recovery on next use.
- Test `-32801` retry.
- Test canonical language IDs.
- Test diagnostics conversion and deduplication.
- Keep the real server smoke example as optional manual validation.

## Priority Order

1. Capability truth from `initialize_result.capabilities`.
2. Canonical language IDs.
3. Server-initiated request handling, starting with
   `workspace/configuration`.
4. Stronger initialize payload with workspace folders and initialization
   options.
5. Lazy `ensure_server_started`.
6. Request retry for `-32801`.
7. Raw protocol method support helpers and capability-gated dispatch.
8. Diagnostics conversion and CLI integration.
9. Fake-LSP integration test harness.

## Non-Goals

- Do not make `gumiho-mudang-lsp` an LSP server.
- Do not reimplement language type systems.
- Do not treat static registry capability presets as authoritative.
- Do not make LSP part of the default syntactic hot path.
- Do not put user-facing semantic operation composition in
  `gumiho-mudang-lsp`.
