# Mudang docs index

Top-level documentation for the `gumiho-mudang` monorepo. Read in
the order below for a clean mental model.

---

## Entry points

| Read first | Why |
|------------|-----|
| `ROADMAP.md` | The immutable phase ordering (A → E). Sets context for everything else. |
| `ARCHITECTURE.md` | Crate map, lib-first principle, composer role, boundary contract. |
| `SCOPE-LSP-COMPOSITION.md` | The design contract for composing scope + LSP. Largest doc; section index below. |
| `SUBSTRATE-PRIMARY.md` | Daily workflow built on the architecture; RAM / GPU profiles; agent integration. |
| `NOTIFY-API.md` | File-change events, cascade flow, daemon protocol, tier 2 wiring. |
| `CROSS-LANG-STITCHING.md` | Composer mechanism for joining edges across language plugins via anchor strings (URL, queue, topic, env var). |
| `EDIT-LAYER.md` | Phase E preliminary: AST edit primitives, safety gates, AST cache. |

---

## Document map

### `ROADMAP.md`

Five phases (A scope refactor → B LSP basic-RPC → C composer + notify
+ diagnostics → D LanceDB + GPU embedder → E CodeStruct AST edit).
Acceptance criteria per phase. Amendment rules. Hard rule: no
reordering without explicit PR.

### `ARCHITECTURE.md`

- §1 lib-first principle
- §2 crate map (scope / lsp / edit / composer / CLI) + §2.2 scope
  decomposition into 5 sub-crates
- §3 composer crate surface
- §4 unified `file_changed` event diagram
- §5 LSP deliberately small surface
- §6 CLI deliberately small surface
- §7 external crate usage example
- §8 boundary contract (code-aware only; not fs/shell/git/net)
- §9 cross-refs

### `SCOPE-LSP-COMPOSITION.md`

The largest doc (~2 150 lines). Composition contract.

- §1 operating principle; §1.2 **the five operating modes**;
  §1.3 **three internal scope query surfaces** (graph / FTS / vector)
- §2 four-level routing model + auto-level selection
- §3 capability map (scope gaps ↔ LSP methods; scope-only capabilities)
- §4 ten real query cases (A–J)
- §5 composition flow (sequence diagrams); §5.4 **merge algorithm**
  (compose-merge vs compose-backfill)
- §6 cache model
- §7 LSP server lifecycle (composer-managed)
- §8 provenance tags + JSON shape
- §9 configuration
- §10 CI / offline behaviour
- §11 what this doc is not
- §12 open questions
- §13 **full LSP capability matrix** (~35 methods)
- §14 extended composition catalog (cases K–Z) including §14.5
  **Case AA tier 2 embeddings** and §14.6 **Case BB load reducer**
- §15 server-specific compositions; §15.7 per-language capability
  matrix
- §16 sub-measured limits
- §17 **decision tree** (consolidated)
- §18 cost / latency budget

### `SUBSTRATE-PRIMARY.md`

Daily workflow built on substrate-primary thesis.

- §1 the bet; §2 token math
- §3 setup: §3.1 RAM budget (+ 32 GB / 8 GB GPU reference profile);
  §3.2 embedding stack (bge-small / base / large with GPU column);
  §3.3 auto-level config (RAM-rich, 32 GB + GPU, constrained profiles)
- §4 workflow combos (onboarding, refactor, debug, review, cross-lang,
  TDD, API surface, diagnostics)
- §5 tool order discipline (hard table)
- §6 agent integration (CLAUDE.md snippet, skill registration)
- §7 risks; §8 ROI verification; §9 failure modes
- §10 relation to other docs

### `NOTIFY-API.md`

File-change event API.

- §1 motivation (drift problem)
- §2 three usage modes (lib / CLI-daemon / CLI-one-off) + decision matrix
- §3 CLI surface
- §4 IPC protocol (line-delimited JSON over Unix socket)
- §5 Rust API (`Notifier`)
- §6 cascade flow (8 steps with diagram)
- §7 cascade levels (none / graph / full)
- §8 event taxonomy (11 event kinds)
- §9 **tier 2 integration** (LSP-enriches-Scope wiring)
- §10 workflows (7 real use cases)
- §11 guarantees
- §12 failure modes
- §13 daemon lifecycle
- §14 configuration
- §15 security
- §16 what this API is not
- §17 cross-refs

### `CROSS-LANG-STITCHING.md`

Composer-side anchor-string JOIN that turns the polyglot graph into
end-to-end cross-language relationships.

- §1 purpose; §2 layer ownership (lang plugin / framework plugin /
  scope / composer / LSP)
- §3 anchor types (HTTP, WS, bg job, pubsub, env, table, GraphQL,
  flag, gRPC)
- §4 normalisers per anchor (§4.1 URL + method with template-param
  canonicalisation)
- §5 algorithm + §5.1 cache (`.mudang/composer-cache/stitch.sqlite`,
  notify-driven invalidation)
- §6 confidence policy (`high` / `medium` / `low` / `drop`)
- §7 failure modes (dynamic URL, microservices, wildcard route,
  version mismatch)
- §8 required scope fields (post-R0; 0009 absorbed into R0)
- §9 composer public API (`flow`, `stitched_edges`,
  `unresolved_anchors`)
- §10 coverage estimates per anchor
- §11 non-goals (type shape match, runtime confirmation, cross-repo)
- §12 cross-refs

### `EDIT-LAYER.md` (phase E preliminary)

AST structural editing layer.

- §1 what this layer is
- §2 why a separate crate
- §3 inspiration from CodeStruct (paper) without source reuse
- §4 hybrid routing structural vs semantic
- §5 **five safety gates** (dry-run / pre-parse / pre-post diagnostic
  / post-edit reindex / atomic apply)
- §6 eight-step edit flow
- §7 **AST cache** (resident; math, project sizes, modes,
  cold-start, eviction, capabilities enabled)
- §8 EditEngine surface
- §9 capability gaps vs LSP-only edits
- §10 license path (no CodeStruct port; reimplement from paper)
- §11 engineering cost estimate
- §12 open questions
- §13 cross-refs
- §14 non-goals

---

## TODO directory

Pending decisions captured during the rename audit and the
architecture discussion. See `todos/README.md` for the index.

| #    | Task                                                                          | Status |
|------|-------------------------------------------------------------------------------|--------|
| 0001 | Rename index directory `.scope/` → `.mudang/`                                 | TODO   |
| 0002 | Rename workspace manifest `scope-workspace.toml` → `mudang-workspace.toml`    | TODO   |
| 0003 | Update GitHub URLs once the new repository is published                       | TODO   |
| 0004 | Clarify ONNX vs LanceDB roles in the embeddings stack                         | TODO   |
| 0005 | Delete `gumiho-mudang-scope/src/core/watcher.rs`                              | TODO   |
| 0006 | Split `gumiho-mudang-scope` into focused sub-crates                            | TODO   |
| 0007 | Create `gumiho-mudang-composer` crate                                          | TODO   |
| 0008 | Constrain `gumiho-mudang-lsp` to basic-RPC primitives                          | TODO   |
| 0009 | Expand R0 domain edge kinds to cover Rails/Tokio/Axum/React patterns          | ABSORBED into R0 (ships in scope sprint 0001) |

---

## Related docs outside this directory

- `gumiho-mudang-scope/docs/CHARTER.md` — scope's read-only invariants.
- `gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` — scope's
  internal R-moves (R0–R12) executed during phase A.
- `gumiho-mudang-scope/docs/POST-REFACTOR-PLAN.md` — what scope ships
  after R-moves complete.

---

## Reading order by goal

- **"I want the whole picture"**: `ROADMAP.md` → `ARCHITECTURE.md` →
  `SCOPE-LSP-COMPOSITION.md` §1–§4 → `SUBSTRATE-PRIMARY.md` §1–§3.
- **"I want to implement phase A or B"**: scope's own docs +
  `ARCHITECTURE.md` §2.2 + `docs/todos/0006`–`0008`.
- **"I want to implement phase C"**: `ARCHITECTURE.md` §3 + §4,
  `SCOPE-LSP-COMPOSITION.md` §17 (decision tree), `NOTIFY-API.md`,
  `CROSS-LANG-STITCHING.md`, `docs/todos/0005`, `0007`.
- **"I want to implement phase D"**: `SCOPE-LSP-COMPOSITION.md` §14.5
  + `docs/todos/0004` + `SUBSTRATE-PRIMARY.md` §3.2.
- **"I want to implement phase E"**: `EDIT-LAYER.md` (this directory).
- **"I want to use mudang every day"**: `SUBSTRATE-PRIMARY.md`.
- **"I'm writing an external consumer"**: `ARCHITECTURE.md` §7 +
  `NOTIFY-API.md` §2.1.
