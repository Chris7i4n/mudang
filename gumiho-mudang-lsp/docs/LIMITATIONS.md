# gumiho-mudang-lsp Limitations

This document records the current limitations of `gumiho-mudang-lsp`.
It describes the crate as it exists now, not the intended final design.

For a detailed comparison against the TypeScript reference
implementation, see [REFERENCE_GAP_MAP.md](REFERENCE_GAP_MAP.md). For
the target capability model, see
[CAPABILITY_TRUTH.md](CAPABILITY_TRUTH.md).

## Current Role

`gumiho-mudang-lsp` is an adapter for external language servers. It is
not an LSP server itself.

The crate can:

- spawn an external LSP process over stdio;
- perform the LSP `initialize` / `initialized` handshake;
- send raw LSP requests and notifications;
- synchronize files with `didOpen`, `didChange`, `didSave`, and
  `didClose`;
- collect `textDocument/publishDiagnostics` notifications;
- maintain per-server lifecycle state.

The crate is currently a library building block. The `mudang` CLI
declares it as a dependency, but no user-facing CLI command routes
through it yet.

## Capability Handling

Real LSP servers report their capabilities in the `initialize` response.
The client receives that response, but the instance startup path only
checks whether initialization succeeded and then discards the payload.

Current behavior:

- server capabilities used by the crate come from static registry
  presets;
- `LspCapabilities::FULL`, `NO_CALL_HIERARCHY`, and `BASIC` are manual
  assumptions;
- actual `initialize_result.capabilities` is not parsed into
  `LspCapabilities`;
- static capabilities are not validated against the server response;
- dynamic registration is not supported.

This means the registry can overstate or understate what a specific
installed server actually supports.

Target invariant: after initialization, runtime capabilities must come
from the server's real `initialize` response, not from registry presets.

## Raw Protocol Surface

The manager can forward arbitrary LSP requests to the selected server.
That raw protocol boundary is the intended direction for this crate.

`gumiho-mudang-lsp` should expose canonical LSP methods and protocol
data. It should not own user-facing composed operations.

Current gaps at this layer:

- raw requests are accepted without checking runtime server capabilities;
- request errors preserve messages but not enough structured LSP error
  data for policy decisions;
- protocol params and results are passed as raw JSON values;
- method-level support is not exposed from the manager;
- caller-facing composition, formatting, and producer tagging belong
  above this crate and are not implemented yet.

Consumers should expect to know canonical LSP method names and parameter
shapes, or use a higher-level crate/CLI layer that composes those raw
methods into product operations.

## CLI Integration

The crate is not wired into `gumiho-mudang-cli` behavior yet.

Current gaps:

- no CLI flags such as `--resolve` or `--types` use this crate;
- no command starts or stops LSP instances;
- no command merges LSP facts with the syntactic graph;
- no command drains or displays LSP diagnostics;
- no status widget is exposed through the CLI.

Until that integration exists, the LSP crate is usable through tests,
examples, or direct library calls only.

## Lifecycle And Restart Behavior

`LspServerInstance` tracks state and detects process crashes, but restart
behavior is only partially implemented.

Current behavior:

- startup failures move the instance to `Error` or `Failed`;
- crash detection records `last_error` and increments `restart_count`;
- `restart()` exists as an explicit method;
- delayed restart scheduling only emits a state callback;
- the manager does not automatically restart errored instances.

The result is a lifecycle model with useful state reporting, but without
complete self-healing behavior.

## Server-Initiated Requests

The client reads server-initiated JSON-RPC requests but currently ignores
them.

This may limit language servers that expect client responses for
requests such as:

- `workspace/configuration`;
- `client/registerCapability`;
- `workspace/workspaceFolders`;
- custom server-specific configuration requests.

The client advertises a deliberately small capability set during
initialization, but some servers may still send requests that need
responses.

## Language IDs

`didOpen` uses the first human-readable language name from the registry
as `languageId`.

Examples include `Rust`, `TypeScript`, and `Python`. Some LSP servers
expect canonical lowercase language IDs such as `rust`, `typescript`,
or `python`.

The smoke test against `rust-analyzer` works, but the current registry
format can be incompatible with stricter servers.

## Workspace And Root Detection

Root selection is shallow. The instance checks whether any root marker
exists directly under the supplied working directory and otherwise uses
that same directory.

Current gaps:

- no upward search from file path to project root;
- no multi-root workspace support;
- no workspace folder negotiation;
- no project-specific server configuration beyond enabled server IDs;
- no per-language root strategy.

## Diagnostics

Diagnostics are captured and deduplicated, but they are not integrated
with the rest of mudang yet.

Current behavior:

- diagnostics are stored in memory only;
- repeated diagnostics are suppressed across drains;
- empty diagnostic batches clear delivered state for that file;
- output is capped at 10 diagnostics per file and 30 total diagnostics;
- diagnostics are not persisted;
- diagnostics are not attached to the syntactic graph.

These limits are useful for agent-facing output volume, but they are not
yet part of a broader semantic cache.

## Registry Accuracy

The server registry is a static list of known servers, commands,
extensions, root markers, and install hints.

Current gaps:

- command availability is detected with `which`, but version compatibility
  is not checked;
- install hints are not verified;
- server capability presets are manually curated;
- extension matching chooses a running server by extension only;
- overlapping servers for the same extension are not resolved by project
  semantics beyond simple `skip_if` markers.

## Testing Coverage

The unit tests cover codec behavior, static registry lookup, settings,
diagnostic deduplication, lifecycle state transitions, and missing binary
handling.

The example smoke test validates a real `rust-analyzer` path when
`rust-analyzer` is installed.

Current gaps:

- no automated integration tests against a fake LSP server with full
  request/notification scripts;
- no CI-stable real-server matrix;
- no tests that parse real initialize capabilities;
- no tests for higher-level semantic result conversion because that layer
  intentionally belongs above this crate.

## Known Migration Issue

`gumiho-mudang-lsp/Cargo.lock` still identifies the package as
`gumiho-lsp`. That appears to be migration residue and should be
normalized before this crate is treated as fully migrated.
