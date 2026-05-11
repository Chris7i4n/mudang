# Substrate-Primary Workflow

How to operate a dev environment in which `mudang` is the **primary**
navigation layer. Reading and editing files directly is the fallback,
not the default.

This is the operational counterpart to `SCOPE-LSP-COMPOSITION.md`. The
composition doc describes the design contract; this doc describes the
daily habit that lets the design pay off in tokens, time, and accuracy.

---

## 1. The bet

The hypothesis driving substrate-primary use:

1. **Token economics** — every navigation answered from the index is
   one to two orders of magnitude cheaper than a raw file read.
2. **Iteration economics** — every misread Scope avoids saves one
   feedback loop with the user.
3. **Onboarding economics** — a new repo becomes a navigable graph in
   minutes, not hours.

The bet is only rational when three conditions hold:

- **abundant RAM** — LSP servers, ONNX embeddings, and LanceDB cache all
  warm at the same time;
- **disciplined tool order** — substrate first, file reads second;
- **complete capability map** — no falling back to `grep` because the
  index gap was unknown (Section 13 of the composition doc closes this).

This document targets dev environments that meet the RAM condition and
walks through the discipline and the map.

---

## 2. Token math

Estimated tokens per query, raw-shell path vs mudang path.

| Question | Raw path | Tokens | Mudang path | Tokens | Compression |
|----------|----------|--------|--------------|--------|-------------|
| What does X do? | `cat` / Read | 2 000–5 000 | `mudang summary X` | 30 | 60–160× |
| Outline of class X | Read | 3 000–5 000 | `mudang sketch X` | 180 | 17–28× |
| Who calls X? | `rg` + N Reads | 6 000–12 000 | `mudang refs X` | 150 | 40–80× |
| What breaks if I change X? | many Reads + grep | 15 000–30 000 | `mudang impact X` | 300 | 50–100× |
| Architectural overview | Read 10 files | 25 000–40 000 | `mudang map` | 500–1 000 | 25–80× |
| Code that does Y? | `rg` + Read | 8 000–15 000 | `mudang find "Y"` | 100 | 80–150× |
| Type at position | impossible from raw shell | — | `mudang type-at` | 80 | ∞ |
| Symbol body only | Read + manual slice | 1 000–3 000 | `mudang source X` | symbol-sized | 5–10× |
| Full context on X | many tools | 12 000–25 000 | `mudang explain X` | 600 | 20–40× |

Sessional estimate at 50 queries / session:

- raw path: 250 000–500 000 tokens consumed
- substrate-primary: 10 000–20 000 tokens consumed
- delta: ~250 000–470 000 tokens / session

At Opus prompt rates this is ~$0.75–$1.40 saved per session in tokens
alone. The iteration savings (fewer misreads → fewer retries) are
typically larger than the per-query savings.

---

## 3. Setup

### 3.1 RAM budget

A polyglot machine with every server warm at once:

| Component | Typical RAM |
|-----------|-------------|
| rust-analyzer (per active workspace) | 3–5 GB |
| tsserver | 1–2 GB |
| pyright | 1–2 GB |
| gopls | 0.5–1 GB |
| jdtls | 3–4 GB |
| ruby-lsp / solargraph | 0.5 GB |
| ONNX Runtime + bge-small | 0.5–1 GB |
| LanceDB hot cache | 0.5–2 GB |
| Mudang cache + scope SQLite WAL | 0.5 GB |
| **Peak (polyglot heavy)** | **10–18 GB** |

If the machine has < 32 GB RAM, restrict warm servers to the active
stack and let `idle_timeout_seconds` reclaim memory. The cost is a
cold-start hit (5–30 s for rust-analyzer) the first time the inactive
stack is queried.

#### Reference profile: 32 GB RAM + 8 GB GPU

Common dev-machine spec. Comfortable for substrate-primary work as long
as warm servers are scoped to the **active stack** rather than every
configured language.

Active-stack assumptions: at most 2 LSP servers warm at a time
(e.g. `rust-analyzer + tsserver`), with other languages lazy-spawned on
first query.

| Component | RAM | VRAM |
|-----------|-----|------|
| OS + editor + browser baseline | 6–10 GB | — |
| rust-analyzer (active workspace) | 3–5 GB | — |
| tsserver (active workspace) | 1–2 GB | — |
| ONNX Runtime + bge-base | — | ~600 MB |
| LanceDB hot cache | 1 GB | — |
| Mudang cache + scope SQLite WAL | 0.3 GB | — |
| **Peak (active 2-lang)** | **~12–18 GB** | **~600 MB** |

Margin: 14–20 GB RAM and 7+ GB VRAM unused. Polyglot bursts
(rust + ts + python + go warm simultaneously) hit ~12 GB CPU RAM — still
inside budget without thrashing.

GPU specifically enables:

- moving ONNX inference off CPU entirely;
- using a bigger embedding model than `bge-small` (see §3.2);
- batching embedding work without back-pressure on the indexer.

Heavy servers (`jdtls`, multiple rust-analyzers, several pyrights) are
the failure mode — keep those lazy.

### 3.2 Embedding stack

Recommended local stack:

- **Model**: `bge-small-en-v1.5` (384-dim, ~30 MB weights).
  Multilingual variant exists; switch via config.
- **Runtime**: `ort` (ONNX Runtime crate). Cold-load 80–150 ms; warm
  inference ~5–15 ms per symbol on CPU, sub-ms on GPU.
- **Store**: LanceDB sidecar at `.mudang/lance/`.
- **First-index cost**: ~5–10 min on 100 k symbols.
- **Incremental**: dedupe by SHA-256 of `build_embedding_text`; only
  changed symbols re-embed.

Pin criteria (per `docs/todos/0004-onnx-and-lancedb-distinction.md`):
the `(provider, model, dim)` tuple is fixed once. Re-embed everything
on swap.

Model-choice guidance:

| Model | Dim | Recall | Inference | When to pick |
|-------|-----|--------|-----------|--------------|
| bge-small-en-v1.5 | 384 | ~95 % of base | 5–15 ms CPU, <1 ms GPU | default; no GPU available |
| bge-base-en-v1.5 | 768 | baseline | 15–30 ms CPU, ~1–2 ms GPU | **recommended when GPU ≥ 4 GB VRAM** |
| bge-large-en-v1.5 | 1024 | best | 30–60 ms CPU, ~3–5 ms GPU | GPU ≥ 6 GB VRAM and recall-bound queries |
| voyage-3 (API) | 1024 | strong | network-bound | offline / cost concerns rule it out |

With an 8 GB GPU, `bge-base` is the sweet spot: ~5–10 % recall improvement
over `bge-small` on intent queries, inference cost negligible, VRAM
footprint trivial.

### 3.3 Auto-level config

`.mudang/config.toml` for a RAM-rich profile:

```toml
[lsp]
default_level         = "auto"
idle_timeout_seconds  = 86400      # 24 h — keep servers warm
request_timeout_seconds = 60
warm_at_startup       = true

[lsp.rust]
binary = "rust-analyzer"
init_options.cargo.allFeatures      = true
init_options.checkOnSave.command    = "clippy"

[lsp.typescript]
binary = "typescript-language-server"

[lsp.python]
binary = "pyright-langserver"

[lsp.go]
binary = "gopls"

[embeddings]
runtime    = "onnx"
provider   = "bge-small-en-v1.5"
store      = "lance"
model_path = "~/.cache/mudang/models/bge-small"
batch_size = 64
device     = "cpu"                # or "metal" / "cuda"

[output]
default_format   = "auto"          # tty: human; pipe: json
max_refs         = 50
max_impact_depth = 4
```

A constrained-RAM profile lowers `idle_timeout_seconds` to 300 and
disables `warm_at_startup`.

#### Profile: 32 GB RAM + 8 GB GPU

Concrete config for the reference machine in §3.1. Two languages warm
at startup, everything else lazy, embeddings on GPU.

```toml
[lsp]
default_level           = "auto"
idle_timeout_seconds    = 3600     # 1 h reclaim
request_timeout_seconds = 60
warm_at_startup         = true     # applies only to languages with their own warm_at_startup=true

[lsp.rust]
binary = "rust-analyzer"
warm_at_startup = true
init_options.cargo.allFeatures        = true
init_options.checkOnSave.command      = "clippy"
init_options.lru.capacity             = 128       # cap memory

[lsp.typescript]
binary = "typescript-language-server"
warm_at_startup = true
init_options.maxTsServerMemory        = 2048      # MB cap

[lsp.python]
binary = "pyright-langserver"
warm_at_startup       = false                     # lazy
idle_timeout_seconds  = 600                       # 10 min

[lsp.go]
binary = "gopls"
warm_at_startup       = false
idle_timeout_seconds  = 600

[lsp.java]
binary = "jdtls"
warm_at_startup       = false                     # 3–4 GB; on-demand only
idle_timeout_seconds  = 300

[embeddings]
runtime    = "onnx"
provider   = "bge-base-en-v1.5"                   # GPU justifies the bump
store      = "lance"
model_path = "~/.cache/mudang/models/bge-base"
batch_size = 128                                  # GPU tolerates bigger batches
device     = "cuda"                               # or "metal" on macOS

[output]
default_format   = "auto"
max_refs         = 50
max_impact_depth = 4
```

Knobs that matter under this profile:

- **`idle_timeout_seconds = 3600`** — reclaims memory if you context-switch
  to a different stack for an hour. Cold-start hit only if you bounce back.
- **`warm_at_startup` per language** — pay startup cost only for the 2
  languages you live in.
- **`init_options.lru.capacity` / `maxTsServerMemory`** — caps individual
  servers so a runaway rust-analyzer doesn't eat 8 GB on a large workspace.
- **`device = "cuda"` (or `"metal"`)** — embedding inference off CPU; the
  indexer's batch path stops being CPU-bound on `bge-base`.

Adjust `warm_at_startup` lists when you change primary stack
(e.g. swap `typescript` → `python` for a sprint on a Django service).

### 3.4 Warmup

Per-workspace warmup (run once on `cd` or at shell startup):

```bash
mudang lsp warm-all       # spawn every configured server
mudang index --watch &    # start the composer daemon (file watcher + event bus)
```

The `--watch` daemon is the composer running long-lived; it owns the
file-change event bus, warm LSP servers, and (after phase E) the AST
cache. See `docs/ARCHITECTURE.md` §3.4 and
`docs/todos/0005-delete-scope-watcher.md` — the scope crate no longer
owns a watcher; the responsibility moved to the composer.

CI warmup before strict queries:

```bash
mudang lsp warm rust go
mudang verify --sample 100 --strict   # gates the build on graph health
```

---

## 4. Workflow combos

### 4.1 Onboarding a new repo

```bash
mudang setup --preload     # init + first index + CLAUDE.md snippet
mudang lsp warm-all
mudang map > /tmp/overview.md
```

One-time cost: 1–3 minutes. After that the substrate is the entry point
for every navigation question.

### 4.2 Pre-refactor analysis

```bash
mudang sketch PaymentService
mudang refs PaymentService --strict
mudang impact PaymentService --depth 3 --strict
mudang test-impact PaymentService
mudang dead-code src/payments --strict
```

Blast radius, test coverage, and unused-symbol cleanup collected before
any edit. Aggregate cost ~1 200 tokens.

### 4.3 Debugging

```bash
mudang find "where auth tokens expire"
mudang trace validateToken
mudang impact validateToken --strict
mudang explain validateToken
```

Intent search → reachability → blast radius → full context. Total ~600
tokens vs ~15 000+ raw.

### 4.4 Code review

```bash
mudang since main
mudang symbols-since main --public-only
mudang verify --since main
```

Structural diff, public-API delta, and graph-health sanity check
scoped to the diff.

### 4.5 Cross-language flow tracing

```bash
mudang flow ApiController DjangoView --depth 5
```

Pure Scope. LSP does not see cross-language edges. Charter §8 moat.

### 4.6 TDD loop

```bash
mudang find-tests processPayment
mudang runnables src/payments/service.ts
mudang test-impact processPayment
```

Tests covering the symbol → how to run them → which others are also
affected by a change.

### 4.7 API-surface management

```bash
mudang api-surface src/payments
mudang deprecation src/payments
mudang symbols-since main --public-only
```

Public boundary, deprecation usage, and public-API delta — one pass.

### 4.8 Live diagnostics sweep

```bash
mudang health
mudang health --since main
```

Cross-language compile / lint state without opening files.

---

## 5. Tool order discipline

The single behavioural rule for substrate-primary use.

### 5.1 Hard order

| Goal | First tool | Fallback (only when first fails) |
|------|-----------|----------------------------------|
| "Where is the code that …?" | `mudang find` | `rg` (after `find` returns nothing) |
| "What's in this class / module?" | `mudang sketch` | Read (only when changes span > 50 % of file) |
| "Who uses X?" | `mudang refs --strict` | `rg` (after `refs` says zero in `--strict`) |
| "What breaks if I change X?" | `mudang impact --strict` | manual reading — explicit dead end |
| "Show me the body of X" | `mudang source` | Read (only when symbol is dominant of file) |
| "Type of X at position" | `mudang type-at` | none — refuses without LSP |
| "Full context on X" | `mudang explain` | composition of the above |
| "Tests for X" | `mudang find-tests` | `rg "X"` in `tests/` — explicit dead end |

### 5.2 What Read is still for

- non-code files: JSON / TOML / Markdown / config / `.env.example`;
- log files and build output;
- whole-file edits where > 50 % of the file is changing;
- generated code not in Scope's index.

### 5.3 What `rg` / `grep` / `find` are still for

- patterns in non-code files;
- emergency fallback when `mudang find` is genuinely empty;
- shell-level glob patterns memorised in muscle memory.

For navigation of code symbols: never.

---

## 6. Agent integration

The agent (Claude Code or another LLM client) must follow the same
discipline. The simplest mechanism is a project-local `CLAUDE.md` skill
snippet.

### 6.1 CLAUDE.md snippet

```markdown
## Code navigation protocol (mudang substrate-primary)

This repo has mudang installed. Before any file read for navigation:

1. `mudang map` once per session for the global mental model.
2. `mudang find "<intent>"` for "where is the code that does X?".
3. `mudang sketch <symbol>` for "what's in this class / module?".
4. `mudang refs <symbol> --strict` for "who uses this?".
5. `mudang impact <symbol> --strict` before any structural edit.
6. `mudang source <symbol>` for the symbol body (not the whole file).
7. `mudang explain <symbol>` when you want the one-shot context dump.

Read is allowed for:
- full-file inspection when changes will span > 50 % of the file,
- non-code files (JSON / Markdown / config),
- when mudang explicitly returns no result for that symbol.

Never use `rg` / `grep` / `find` for navigating code symbols. Use
`mudang find` or `mudang refs`.

When in doubt: substrate first, file reads second.
```

### 6.2 Skill registration

Place at `~/.claude/skills/mudang/SKILL.md` for project-agnostic use,
or inside the repo's `.claude/skills/` for repo-local. The agent picks
it up automatically when registered.

The user-level `code-navigation` skill referenced in `~/.claude/CLAUDE.md`
already nudges agents toward Scope. Mudang's CLI is the same binary —
substituting `mudang ...` for `scope ...` in the skill text completes
the integration.

### 6.3 Pre-bash hook (optional)

A shell hook that intercepts `rg <pattern>` when the pattern looks like
a symbol name and prints a hint:

```
$ rg PaymentService
hint: this looks like a symbol name. consider:
  mudang refs PaymentService
  mudang sketch PaymentService
  mudang find "PaymentService"
proceeding with rg ↓
```

The hook does not block, only nudges. Habit forming, not enforcement.

---

## 7. Risks

### 7.1 Tool drift

Largest risk is behavioural — the dev or agent reverts to `cat` / `rg`
/ Read out of habit. Mitigations:

- the CLAUDE.md snippet,
- the pre-bash nudge hook,
- session-level tool-call metering: log the ratio of `mudang *` calls
  vs raw Read / `rg`. When Read >> mudang in a session, surface a
  reminder.

### 7.2 Stale index

If a non-mudang-aware editor writes files, the Scope graph drifts. Once
the graph is wrong, every query is silently wrong.

Mitigations:

- `mudang index --watch` daemon always running;
- file-watcher debounce 100–300 ms;
- `mudang status` reports stale files explicitly;
- CI fails when `mudang status --json` reports any stale entry.

### 7.3 Cold start under workspace switching

Switching workspaces frequently makes `rust-analyzer` cold-start every
time, defeating the warm-server assumption.

Mitigations:

- pin a small set of "primary" workspaces with resident servers;
- `idle_timeout_seconds = 86 400` for primary workspaces;
- `mudang lsp pool` to list / manage residents.

### 7.4 Embedding model upgrade

Switching `bge-small` → `bge-large` means reindex of every embedding.
LanceDB schema is pinned to `(provider, model, dim)` (TODO 0004).

Mitigations:

- treat the choice as one-time;
- run reindex as a background batch when upgrading;
- never delete the old vector table until the new one is verified.

### 7.5 Server inconsistency

`--strict` must be a hard error when LSP is unavailable, not a silent
degrade to Level 0. The whole point of strict mode is the semantic
guarantee.

Mitigations:

- `--strict` exits non-zero when any required server is unavailable;
- JSON output reports `lsp_status: unavailable` when degrading
  non-strict modes;
- `mudang verify --sample` regression-tests graph health on a schedule.

### 7.6 Provenance leak

Caller code that consumes mudang JSON output must respect provenance
tags. A `lsp only — runtime-conditional` row in `implementers` is not
the same as a confirmed impl. Stripping the tag on the consumer side
turns a hedged claim into a hard claim.

Mitigation: every JSON consumer in this stack must surface the
`provenance` field. CI lint for missing-provenance accesses.

---

## 8. ROI verification (30-day checklist)

After 30 days of substrate-primary use, verify the bet:

1. **Token usage** — sum of tool-call tokens per session, compared to
   pre-substrate baseline. Expectation: 60–80 % reduction on
   navigation work.
2. **Iteration count** — PRs / commits to converge on a feature.
   Expectation: 30–50 % fewer iterations on complex tasks.
3. **Time-to-understand** — self-reported time to feel oriented in an
   unfamiliar module. Expectation: ~70 % reduction.
4. **Mis-navigation incidents** — cases where the agent read the wrong
   file or trusted a wrong reference. Expectation: ~0 with `--strict`.
5. **Warm-hit ratio** — fraction of LSP queries hitting a warm server
   vs triggering a cold start. Expectation: > 95 % on the primary
   stack.

If the numbers do not move, the setup is not actually substrate-primary.
Re-check tool order discipline and CLAUDE.md adherence.

---

## 9. Failure modes

The substrate is only as reliable as its weakest link. Three concrete
failure modes:

1. **Index lies** — `.mudang/graph.db` is stale; queries return ghost
   data. Detected by `mudang status` and CI.
2. **Server lies** — LSP server cached old state. Detected by
   `mudang verify --sample`; fixed by `mudang lsp restart <lang>`.
3. **Discipline lies** — tool order is not actually followed. Detected
   by metering tool calls per session.

The bet in §1 only pays off when all three hold.

---

## 10. Relation to other docs

- **SCOPE-LSP-COMPOSITION.md** — the design contract this workflow
  presumes (especially Sections 2, 3, 4, 13, 14).
- **gumiho-mudang-scope/docs/CHARTER.md** — the invariants Scope keeps,
  on which index reliability rests.
- **gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md** — the
  unfinished work that turns "Scope is mostly right" into "Scope is
  honest about what it knows."
- **docs/todos/0004-onnx-and-lancedb-distinction.md** — the embedding
  stack assumptions.

Substrate-primary is only fully realised after Scope Phase E + post-
refactor vector embeddings + the LSP composition layer ship. The
current state delivers Level 0 with FTS5. Adoption is incremental as
those pieces land.
