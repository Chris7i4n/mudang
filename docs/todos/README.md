# TODOs

Pending tasks captured during the `scope` → `mudang` rename audit.

Each TODO file records a decision made during the audit that has
not yet been executed in code. Docs were updated to reflect the
target state; code currently lags behind.

When work begins on a TODO, the user pastes the relevant issue or
PR link into the file's `Tracking` field.

| #    | Task                                                                          | Status |
|------|-------------------------------------------------------------------------------|--------|
| 0001 | Rename index directory `.scope/` → `.mudang/`                                 | TODO   |
| 0002 | Rename workspace manifest `scope-workspace.toml` → `mudang-workspace.toml`    | TODO   |
| 0003 | Update GitHub URLs once the new repository is published                       | TODO   |
| 0004 | Clarify ONNX vs LanceDB roles in the embeddings stack                         | TODO   |
| 0005 | Delete `gumiho-mudang-scope/src/core/watcher.rs` (event bus moves to composer)| TODO   |
| 0006 | Split `gumiho-mudang-scope` into focused sub-crates                            | TODO   |
| 0007 | Create `gumiho-mudang-composer` crate (canonical public API)                   | TODO   |
| 0008 | Constrain `gumiho-mudang-lsp` to basic-RPC primitives only                     | TODO   |
| 0009 | Expand R0 domain edge kinds to cover Rails/Tokio/Axum/React patterns          | ABSORBED — content now part of `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` R0; ships in scope sprint 0001 |
