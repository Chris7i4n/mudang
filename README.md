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

## Status

Skeleton. Migration of code from `scope` and `gumiho-lsp` pending.


Each `gumiho-*` is a self-contained project. The prefix names what the project is **for** within the spirit's ecosystem — not a hierarchy.
