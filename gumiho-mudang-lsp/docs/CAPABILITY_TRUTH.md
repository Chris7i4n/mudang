# Capability Truth

This document defines the target capability model for
`gumiho-mudang-lsp`.

The current implementation uses static capability presets in the server
registry. That is useful as a temporary bootstrap mechanism, but it is
not acceptable as the long-term source of truth.

## Rule

The authoritative source for LSP feature support is the actual server
state observed at runtime:

1. the server's `initialize` response;
2. any later dynamic registrations that the client explicitly supports;
3. any later dynamic unregistrations that the client explicitly supports.

Static registry metadata must never override observed server
capabilities.

## Why This Matters

LSP server capability support changes across:

- server versions;
- language ecosystems;
- project configuration;
- workspace roots;
- installed plugins or extensions;
- command-line flags;
- initialization options.

Because of that, a static preset such as `FULL` can become stale. It can
also be wrong for a specific workspace even if it was true for another
workspace.

If mudang claims an operation is available when the initialized server
did not advertise it, the output becomes an inference. LSP integration
must avoid that.

## Registry Metadata

The registry may contain bootstrap metadata:

- server ID;
- display name;
- command;
- args;
- install hint;
- supported file extensions;
- canonical language IDs;
- root markers;
- skip markers;
- required binaries.

The registry may also contain "expected capabilities" for documentation
or preflight display, but those must be labeled as expectations or hints.
They must not drive runtime operation support after initialization.

## Runtime Capability State

Each running server instance should store a runtime capability record:

- raw `initialize` result;
- raw `serverInfo`, if provided;
- raw `capabilities`;
- parsed protocol method support;
- capability source metadata;
- whether dynamic registration is supported by this client;
- any observed dynamic registrations;
- any observed dynamic unregistrations.

The manager should expose this state so callers can inspect which
protocol methods are actually available for a file.

## Protocol Method Mapping

The parsed protocol method support should be derived from LSP server
capabilities, not from static registry presets.

This crate should use canonical LSP JSON-RPC method names as its public
protocol surface. User-facing operation names such as
`goToDefinition`, `incomingCalls`, or `outgoingCalls` belong in higher
layers.

| LSP Request Method | Server Capability Field |
| --- | --- |
| `textDocument/hover` | `hoverProvider` |
| `textDocument/definition` | `definitionProvider` |
| `textDocument/references` | `referencesProvider` |
| `textDocument/documentSymbol` | `documentSymbolProvider` |
| `workspace/symbol` | `workspaceSymbolProvider` |
| `textDocument/implementation` | `implementationProvider` |
| `textDocument/prepareCallHierarchy` | `callHierarchyProvider` |
| `callHierarchy/incomingCalls` | gated by `callHierarchyProvider`; requires caller-provided `CallHierarchyItem` |
| `callHierarchy/outgoingCalls` | gated by `callHierarchyProvider`; requires caller-provided `CallHierarchyItem` |

Provider fields may be booleans or option objects. Any truthy provider
object should be interpreted as support for the corresponding provider.
Missing, `false`, or `null` provider fields should be interpreted as no
support unless dynamic registration later adds support.

`callHierarchy/incomingCalls` and `callHierarchy/outgoingCalls` are raw
LSP methods, but their params require a `CallHierarchyItem`. The caller
is responsible for obtaining that item, usually by first calling
`textDocument/prepareCallHierarchy`.

## Dynamic Registration

The current Rust client does not support dynamic registration.

Until dynamic registration is implemented:

- advertise `dynamicRegistration: false`;
- do not treat `client/registerCapability` as supported;
- do not claim support for providers that are absent from the initialize
  response;
- respond to unsupported server requests in a controlled way instead of
  silently ignoring them.

If dynamic registration is implemented later:

- handle `client/registerCapability`;
- handle `client/unregisterCapability`;
- update runtime capability state;
- record capability source as `dynamic`;
- test add/remove flows with a fake LSP server.

## Capability Source Labels

Every operation support decision should be explainable.

Suggested labels:

- `initialize`: observed in the server's initialize response;
- `dynamic`: observed through dynamic registration;
- `unsupported`: not advertised by the server;
- `unknown`: server is not initialized or capability state is unavailable;
- `registry_hint`: static registry expectation, only valid before runtime
  observation.

`registry_hint` must not be used as runtime truth once initialization has
completed.

## Failure Policy

If initialization fails, the server has no runtime capabilities.

If initialization succeeds but capability parsing fails:

- keep the raw initialize result for debugging;
- mark parsed protocol method support as `unknown`;
- do not silently fall back to `FULL`;
- surface the parsing failure in status or diagnostics;
- prefer refusing a protocol request over pretending support exists.

## API Direction

The Rust API should move toward this shape:

- `LspServerInstance::initialize_result()`;
- `LspServerInstance::server_capabilities()`;
- `LspServerInstance::method_support(method)`;
- `LspServerManager::capabilities_for_file(path)`;
- `LspServerManager::supports_method(path, method)`;
- raw protocol request methods that check `supports_method` when the
  method has a capability gate.

The exact names can change, but the invariant should not: callers must be
able to ask what the initialized server actually supports.

## Tests Required

The capability implementation should be backed by deterministic fake-LSP
tests:

- server advertises hover only;
- server advertises provider options objects instead of booleans;
- server omits a provider;
- server returns `null` provider values;
- static registry says `FULL` but initialize response does not;
- initialize response says support exists despite conservative registry
  hint;
- dynamic registration request arrives while dynamic registration is not
  supported;
- malformed capability response does not fall back to `FULL`.

The key regression test is:

> Runtime capabilities must follow the initialized server response even
> when registry presets disagree.
