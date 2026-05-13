# gumiho-mudang

Code intelligence oracle that **gumiho** consults to understand code.

In Korean folklore, the **mudang** (무당) is a shamanic intermediary — keeper of knowledge, divinator, bridge between worlds. Within the gumiho ecosystem, mudang is the entity the LLM consults to answer code questions: structural, semantic, or both.

The mudang practices two divination techniques in parallel:

| Technique | Crate | Worldview |
|---|---|---|
| Structural reading (bones, static text) | `gumiho-mudang-scope` | Tree-sitter ASTs over local source. Fast, sound, syntactic. Limit: cross-file resolution falls to nominal matching. |
| Oracle consultation (calling specialist spirits) | `gumiho-mudang-lsp` | LSP servers (rust-analyzer, pyright, tsserver, ...). Slow, semantic, authoritative. Used when structural reading cannot answer. |

The mudang chooses the technique per question. Structural reading answers most questions in milliseconds. Oracle consultation answers the harder ones at the cost of latency and a live LSP toolchain.

## Crates

```
gumiho-mudang-scope/      # syntactic engine: tree-sitter + SQLite + per-language plugins
gumiho-mudang-lsp/        # semantic oracle: LSP manager, capabilities, lifecycle
gumiho-mudang-cli/        # binary "mudang": user-facing commands, routing, cache
```

Each `gumiho-*` is a self-contained project. The prefix names what the project is **for** within the spirit's ecosystem — not a hierarchy.

## Install

```bash
just install              # installs the `mudang` binary into ~/.cargo/bin
mudang --help
```

`just` recipes for build / test / install / uninstall live in [`justfile`](justfile). The CI-gate recipes (`just gate`, `just gate-refactor`) are canonical contributor tooling — see [`gumiho-mudang-scope/CONTRIBUTING.md`](gumiho-mudang-scope/CONTRIBUTING.md).

## Reading the architecture

- [`gumiho-mudang-scope/docs/README.md`](gumiho-mudang-scope/docs/README.md) — entry point for the scope architecture (charter, enforcement map, playbooks, sprints, CI gates, glossary). Start here for any structural question about the syntactic engine.
- [`gumiho-mudang-scope/CONTRIBUTING.md`](gumiho-mudang-scope/CONTRIBUTING.md) — contributor on-ramp; threads the governing docs with prereqs, pre-commit checklist, snapshot workflow, fixture-corpus pointers, change → test mapping, sprint methodology, and codex review setup.
- [`docs/README.md`](docs/README.md) — repo-wide cross-cutting docs (LSP composition, edit layer, notify API, substrate, architecture overview).
