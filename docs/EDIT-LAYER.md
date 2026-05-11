# Edit layer (phase E preliminary)

> **Status:** preliminary. Phase E (`docs/ROADMAP.md`) has not opened.
> This document captures design decisions made during phase A discussion
> so they are not lost. When phase E opens it expands into the full
> contract; until then the invariants here are binding.

The edit layer is the third pillar of the composition (scope + LSP +
edit), shipped as the `gumiho-mudang-edit` crate. It introduces
**structural editing** on top of the read-only graph. Scope's charter
remains untouched: `gumiho-mudang-scope` never writes.

---

## 1. What this layer is

A library that exposes AST-aware editing primitives over symbols and
files mudang's scope graph already tracks. It is orchestrated by the
composer (`gumiho-mudang-composer`) alongside scope and LSP.

Public operations (preliminary):

- `insert(symbol_id, position, content)` — `position ∈ {before, after, body}`
- `replace(symbol_id, content)`
- `remove(symbol_id)`
- `rename(symbol_id, new_name)` — routes to LSP when available;
  structural fallback otherwise
- `create_file(path, content)`
- `delete_file(path)`
- `move_file(from, to)` — composes with `workspace/willRenameFiles`

Symbol resolution reuses the composer's `resolve_symbol` machinery, so
the entry points are the same identifiers used by `mudang refs`,
`mudang impact`, `mudang sketch`.

---

## 2. Why a separate crate

- `gumiho-mudang-scope` is **read-only** by charter
  (`gumiho-mudang-scope/docs/CHARTER.md` §5). Adding edit there would
  break the audit guarantee.
- Edit needs its own invariants (this document).
- The phase A scope decomposition (`docs/todos/0006-split-scope-crate.md`)
  produces `scope-core` exposing the tree-sitter parser and language
  plugins — the edit crate depends on `scope-core` only, never on
  `scope-graph` or `scope-index`.
- License: edit may reuse external crates (e.g. ast-grep MIT) whose
  licenses must not bleed into scope.

---

## 3. Inspiration vs implementation

The design takes inspiration from **CodeStruct** (Amazon Science,
arXiv 2604.05407, ACL 2026). Their result on SWE-Bench Verified:
+1.2–5.0 % Pass@1 with 12–38 % token reduction across six LLMs. The
two-primitive shape (`readCode`, `editCode`) maps onto mudang's
already-existing `mudang source` + the new `mudang edit`.

CodeStruct's code is **CC-BY-NC-4.0** (non-commercial) and the
repository is **archived** (no PRs accepted). Direct port is therefore
forbidden. The edit layer is reimplemented from scratch using:

- `scope-core`'s tree-sitter grammars and language plugins;
- optionally `ast-grep` (MIT) as a structural search/match primitive;
- a small set of language-specific edit rules per grammar.

The paper informs the **action space design**; no source code is
copied.

---

## 4. Hybrid routing: structural vs semantic

Edit operations route to one of two backends, decided per operation:

| Edit kind | Backend | Rationale |
|-----------|---------|-----------|
| Rename across workspace | LSP `rename` / `prepareRename` | semantic correctness, re-export chains |
| Extract method / inline | LSP `codeAction` (when offered) | type inference required |
| Organize imports | LSP `codeAction` / server command | per-language conventions |
| Insert before/after symbol | **edit layer** (structural) | no type analysis needed |
| Replace symbol body | **edit layer** | scoped to one entity |
| Remove symbol | **edit layer** | scoped to one entity |
| File create / delete / move | **edit layer** + LSP `willRenameFiles` | graph + server both need to know |
| Bulk pattern transform | **edit layer** (ast-grep style) | polyglot, batch |

Semantic edits are preferred when an LSP server is reachable
**and** offers the capability. Structural edits cover the rest, plus
the offline / broken-source / no-toolchain cases LSP cannot serve.

---

## 5. The five safety gates (non-negotiable)

Every edit operation, regardless of routing backend, passes through
five gates before any byte hits disk. Failing any gate aborts the
operation with no side effect.

### 5.1 Gate 1 — Dry-run default

Every edit returns a **preview** (unified diff or workspace edit
description) unless `--apply` is passed explicitly.

```bash
mudang edit ValidateToken --op replace --content "$NEW"
# → prints diff, exits 0, file unchanged
mudang edit ValidateToken --op replace --content "$NEW" --apply
# → writes file, returns ack
```

Library API equivalent: `EditOpts { apply: false }` (default) returns
`EditPreview`; `apply: true` returns `EditAck`.

### 5.2 Gate 2 — Tree-sitter pre-parse check

The post-edit source is parsed in memory before being written. If the
parse reports a `ERROR` node that did not exist before the edit, the
operation aborts with `PARSE_REGRESSION`.

This catches the common case where an LLM-emitted replacement is
syntactically broken.

### 5.3 Gate 3 — Pre/post LSP diagnostic diff

When an LSP server is available, the composer:

1. snapshots `workspace/diagnostic` for the affected file(s) **before**
   applying;
2. applies the edit (steps 4–5);
3. snapshots diagnostics **after**;
4. if new `Error`-severity diagnostics appeared that did not exist
   before, the edit **rolls back** automatically.

Configurable: `[edit.gates] rollback_on_new_errors = true` (default).
`false` to allow edits that introduce errors (e.g. mid-refactor).

### 5.4 Gate 4 — Post-edit scope reindex

After the edit is applied, the composer notifies scope via the
file-change event bus (`docs/NOTIFY-API.md`). Scope reindexes
incrementally. If reindex fails (e.g. parser panic on a pathological
input), the edit is rolled back.

This keeps the substrate-primary thesis honest: the graph is always
consistent with disk.

### 5.5 Gate 5 — Atomic apply

The write is `tempfile + rename`:

1. write the new content to `<path>.mudang.tmp`;
2. fsync the temp file;
3. rename atomically over the original (`std::fs::rename`);
4. delete `.mudang.tmp` if anything is left over.

Crashes mid-edit leave either the old file intact or the new file
fully written — never a half-written file.

---

## 6. The eight-step edit flow

```
mudang edit <symbol> --op replace --content <text>
   │
   ▼
1. composer.resolve_symbol(<symbol>) → SymbolHandle { file, range }
   │
   ▼
2. ast_cache.get_or_parse(file) → Tree (resident; phase E §7 below)
   │
   ▼
3. tree.mutate(range, content) → Tree_new      ← in-memory, microseconds
   │
   ▼
4. gate 1: if !opts.apply → return EditPreview(diff)
   │
   ▼
5. gate 2: tree-sitter validate Tree_new
            → on PARSE_REGRESSION: abort
   │
   ▼
6. gate 3 (when LSP available):
     diag_before = lsp.diagnostic(file)
     write Tree_new.text() to tempfile + rename (gate 5 pre-apply)
     diag_after  = lsp.diagnostic(file)
     if new_errors(diag_before, diag_after):
       rollback file from .mudang.bak
       return EditAck { rolled_back: true, reason: "new errors" }
   │
   ▼
7. gate 4: composer.notify(file, cascade=full)
            (via NOTIFY-API §6 pipeline)
            on reindex failure: rollback
   │
   ▼
8. emit edit.completed event; return EditAck { stats }
```

Steps 3 and 5 happen entirely in memory (AST cache resident). Step 6
involves disk I/O. Step 7 is the cascade defined in
`docs/NOTIFY-API.md` §6.

---

## 7. AST cache (resident)

The edit layer requires an AST cache to keep edit operations cheap.
The cache lives inside the composer (`docs/ARCHITECTURE.md` §3.2) and
holds tree-sitter parse trees in RAM.

### 7.1 Memory math

Tree-sitter AST footprint, per language:

| Metric | Typical value |
|--------|---------------|
| Bytes per node | ~64–128 (struct + pointers) |
| Nodes per 1 KLOC | ~5 000–15 000 (TS dense, Go sparse) |
| AST per 1 KLOC | ~500 KB – 1.5 MB |
| + Source bytes retained | ~50–100 KB per 1 KLOC |
| **Total per 1 KLOC** | **~600 KB – 1.6 MB** |

### 7.2 Project sizes

| Project | LOC | AST resident | + source + symbols | Fits in 32 GB? |
|---------|-----|--------------|---------------------|-----------------|
| ripgrep | ~30 k | ~30 MB | ~50 MB | trivial |
| tokio | ~50 k | ~50 MB | ~80 MB | trivial |
| react codebase | ~500 k | ~500 MB | ~800 MB | yes |
| rust-analyzer | ~500 k | ~500 MB | ~800 MB | yes |
| TypeScript compiler | ~3 M | ~3 GB | ~5 GB | yes |
| Chromium / Linux | ~30 M+ | ~30 GB | ~50 GB | **no** — LRU only |

**Sweet spot for a 32 GB / 8 GB-GPU profile** (`docs/SUBSTRATE-PRIMARY.md` §3.1):
projects up to ~5–10 M LOC fit `full` mode comfortably.

### 7.3 Modes

```toml
[edit.ast_cache]
mode            = "full"        # full | lru | off
max_ram_mb      = 4096          # 4 GB ceiling
warm_at_startup = true

[edit.ast_cache.lru]              # only when mode = "lru"
target_mb                = 2048
evict_after_idle_seconds = 600
keep_dirty_resident      = true   # never evict a file with pending edit
```

| Profile | Recommended mode |
|---------|-------------------|
| 32 GB + project < 2 M LOC | `full` |
| 32 GB + project 2–10 M LOC | `full` with `max_ram_mb = 8192` or aggressive `lru` |
| 32 GB + mono-repo > 10 M LOC | `lru` mandatory |
| < 16 GB RAM | `lru` or `off` |

### 7.4 Cold-start cost (mode = full)

Parallel parse via rayon (scope already uses this pattern):

| LOC | Cold parse time (8-core) |
|-----|---------------------------|
| 100 k | 1–3 s |
| 500 k | 5–10 s |
| 2 M | 30–60 s |
| 5 M | 1–3 min |

Acceptable once per daemon lifetime.

### 7.5 Eviction and re-parse

- file changes (notify event) → drop the cached tree;
- new edit op on a dropped tree → re-parse on demand;
- `keep_dirty_resident = true` prevents LRU from evicting a tree with
  pending edits;
- per-tree memory accounting; eviction policy honours `max_ram_mb`.

### 7.6 What AST cache enables beyond edit

The resident AST also powers capabilities scope's distilled graph
cannot reach:

1. **Bulk structural transforms** — "rewrite every `match` to `if-let`
   in `src/` ", single pass over the cache.
2. **AST-level lint** — "find every `unwrap()` in production code"
   without `rg + parse`.
3. **Multi-file atomic refactor preview** — compute the full diff
   structurally before touching disk.
4. **Pattern-based search** — tree-sitter queries (`(call_expression
   function: (identifier) @name)`) executed in ms across the project.
5. **Semantic tree diff** between branches without re-parsing.

CodeStruct's design parses per call; mudang's resident cache
**eliminates** that overhead for hot files.

---

## 8. EditEngine surface (preliminary)

```rust
pub struct EditEngine {
    scope:     Arc<ScopeFacade>,        // for symbol resolution
    lsp:       Arc<LspPool>,             // for semantic routing
    ast_cache: Arc<RwLock<AstCache>>,    // resident parse trees
    bus:       Arc<EventBus>,            // notify cascade
}

impl EditEngine {
    pub fn preview(&self, op: EditOp) -> Result<EditPreview>;
    pub fn apply  (&self, op: EditOp, opts: EditOpts) -> Result<EditAck>;
}

pub enum EditOp {
    Insert { symbol: SymbolId, position: InsertPos, content: String },
    Replace { symbol: SymbolId, content: String },
    Remove  { symbol: SymbolId },
    CreateFile { path: PathBuf, content: String },
    DeleteFile { path: PathBuf },
    MoveFile   { from: PathBuf, to: PathBuf },
    Rename     { symbol: SymbolId, new_name: String },   // routes to LSP when available
}

pub enum InsertPos { Before, After, Body }

pub struct EditOpts {
    pub apply:                  bool,    // gate 1: dry-run default
    pub rollback_on_new_errors: bool,    // gate 3
    pub source:                 Option<String>,    // audit log
}

pub struct EditPreview {
    pub diff:              String,
    pub affected_files:    Vec<PathBuf>,
    pub estimated_impact:  Vec<SymbolId>,
}

pub struct EditAck {
    pub id:           String,
    pub op:           EditOp,
    pub rolled_back:  bool,
    pub rollback_reason: Option<String>,
    pub stats: EditStats,
}
```

---

## 9. Capability gaps that justify the layer (vs LSP-only edits)

LSP edits (`rename`, `codeAction`, `applyEdit`, `willRenameFiles`)
cover semantic refactors but fail in six concrete scenarios. Each is a
reason the structural edit layer exists.

| Scenario | LSP | edit layer |
|----------|-----|------------|
| No toolchain installed | ❌ | ✅ |
| Source broken mid-refactor | ❌ (stalls) | ✅ (tree-sitter tolerant) |
| Polyglot single pass | ❌ (per-server) | ✅ (tree-sitter universal) |
| Token-cheap op for agent | ❌ (verbose JSON-RPC) | ✅ ("edit fn foo") |
| Trivial edits without type analysis | ❌ (overkill) | ✅ |
| Bulk transformation across many files | ❌ (N roundtrips) | ✅ (batch) |

LSP still wins in three scenarios that the edit layer routes to it:

| Scenario | Winner |
|----------|--------|
| Rename with re-export chain | LSP |
| Refactor needing type inference (extract w/ types) | LSP |
| Semantic correctness guarantee | LSP |

---

## 10. License path (binding)

The edit layer **does not** copy CodeStruct source. The repository is
CC-BY-NC-4.0 and archived; direct reuse is blocked legally and
practically. The path is:

1. read the paper (`arXiv:2604.05407`);
2. design the action space from the paper, recording deviations;
3. implement using `scope-core`'s tree-sitter grammars and language
   plugins;
4. optionally vendor `ast-grep` (MIT) for the structural-match
   primitive;
5. all code is original.

The paper's bibtex entry stays in this doc and any user-facing docs
that describe inspiration. No CodeStruct repository fork.

---

## 11. Engineering cost estimate

| Item | Effort |
|------|--------|
| `gumiho-mudang-edit` skeleton + traits | 1 sprint |
| Insert / replace / remove + indentation per language (7) | 2–3 sprints |
| Five safety gates | 2 sprints |
| LSP routing + pre/post diagnostic | 1 sprint |
| AST cache (`full` mode + LRU) | 1 sprint |
| Charter doc finalisation + tests | 1 sprint |
| Total minimum viable (rust only) | **~3 sprints** |
| Full polyglot parity (7 languages) | ~7–8 sprints |

---

## 12. Open questions (resolve when phase E opens)

1. **Indentation policy per language** — preserve original block
   indentation, or re-indent on insert? CodeStruct re-indents.
   Mudang policy TBD.
2. **Default `--apply` policy for agent use** — keep dry-run hard
   default; allow opt-in `--auto-apply` via env var for trusted agents?
3. **Rollback strategy after gate 4 failure** — restore from
   `.mudang.bak` or replay-from-graph? Replay needs more work but
   handles concurrent edits better.
4. **Bulk operations atomicity** — is "transform all matches across N
   files" one transaction or per-file? Affects gate 5.
5. **ast-grep vendor vs reimplement** — vendoring couples our release
   cadence to ast-grep's. Decision deferred.
6. **Edit on non-code files** (markdown, JSON, TOML) — out of scope or
   covered? Tree-sitter has grammars for all three.

---

## 13. Relation to other docs

- `docs/ROADMAP.md` — phase E (when this lands).
- `docs/ARCHITECTURE.md` — §2 (`gumiho-mudang-edit` crate), §3.2
  (AST cache hosted by composer), §8 (boundary contract).
- `docs/SCOPE-LSP-COMPOSITION.md` — §13.5 (LSP editing methods that
  the routing layer prefers when available).
- `docs/SUBSTRATE-PRIMARY.md` — §3.1 (AST cache RAM budget in the
  32 GB + GPU profile), §5 (tool order discipline still treats
  `mudang edit` as primary).
- `docs/NOTIFY-API.md` — §6 (cascade flow invoked by gate 4).
- `docs/todos/0006-split-scope-crate.md` — splits `scope-core` out
  so this crate can depend on it cleanly.
- `gumiho-mudang-scope/docs/CHARTER.md` — scope's read-only
  invariants this crate explicitly does not touch.

---

## 14. Non-goals

- This layer is **not** a refactor planner. The composer's existing
  `mudang impact` answers "what breaks if I change X". The edit layer
  applies the change after the planner decides.
- This layer is **not** a build system or test runner. Affected tests
  surface via `mudang test-impact`; running them stays in `bash`.
- This layer is **not** a file-system tool. Raw `cat` / `Read` /
  `ls` / `rg` for non-graph-tracked files remain orthogonal
  (`docs/ARCHITECTURE.md` §8).
