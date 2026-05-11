# Refactor Status

Live state of `ARCHITECTURAL-REFACTOR.md`. Append-only log; the snapshot tables at the top reflect the current state.

Until the refactor closes (Phase E acceptance — see `ARCHITECTURAL-REFACTOR.md` "Acceptance for the refactor as a whole"), feature work is paused.

## Phases

| Phase | Moves | Status | Started | Shipped | Notes |
|---|---|---|---|---|---|
| A — Schema and storage | R0, R1 | in-progress | 2026-05-11 | — | first batch; everything else depends on it |
| B — Plugin layer | R2, R3, R4, R7, R9, R11, R12 | unstarted | — | — | depends on Phase A |
| C — Framework layer | R5 | unstarted | — | — | lands when framework infrastructure is first introduced; design constraint until then |
| D — Output and audit | R8, R10 | unstarted | — | — | adds detection layer for D2/E1 |
| E — Test harness | R6 | unstarted | — | — | last; depends on plugins in final shape |

## Moves

| ID | Title | Phase | Status | Commit | Date | Notes |
|---|---|---|---|---|---|---|
| R0 | Schema closures | A | in-progress | — | — | adds confidence/status/producer/pattern_id/args_text, surrogate edge_id PK, contains + 30 domain edge kinds (R0 baseline 13 + Tier 1 5 + Tier 2 5 + Tier 3 7; final whitelist 38), `goroutine_spawn` renamed to `green_thread_spawn`, 4-kind concurrency split, 3 new symbol kinds, skipped_ranges. No in-place migration — old indexes are wiped and re-indexed |
| R1 | Typed edge insertion API | A | in-progress | — | — | seals Edge; EdgeBuilder is sole producer of RawEdge; no .status() at extraction |
| R2 | LanguagePlugin output type closure | B | unstarted | — | — | plugin returns RawCaptures; Extractor layer converts to Edge::builder() calls |
| R3 | Pipeline ordering via type-state | B | unstarted | — | — | extract → resolve → write; resolution is sole producer of status |
| R4 | WorkspaceContext typed access (split) | B | unstarted | — | — | LanguageWorkspaceContext / FrameworkWorkspaceContext split; mechanical safeguard for C2 |
| R5 | FrameworkPlugin operates on Symbols and Edges | C | unstarted | — | — | graph-only via metadata; no .scm per framework; DetectedVersion/ResolvedVersion/UnknownVersionPolicy |
| R6 | Malformed-source test harness | E | unstarted | — | — | skipped_ranges populated; integration test gate per language |
| R7 | Indexer-level dispatch enforcement | B | unstarted | — | — | dispatch by extension+shebang only; plugin cannot self-activate |
| R8 | Confidence audit subcommand | D | unstarted | — | — | scope audit confidence; precision-only (recall via integration tests) |
| R9 | Immutable source guarantee | B | unstarted | — | — | no &mut at plugin layer for source-related types |
| R10 | Typed output schema | D | unstarted | — | — | output structs have no diagnostic-shaped fields |
| R11 | Macro definition-only by trait shape | B | unstarted | — | — | LanguagePlugin trait has no expand_*/evaluate_* methods |
| R12 | Type-system-free trait audit + spawn denylist | B | unstarted | — | — | trait-shape audit + Command::new denylist in plugin paths |

## Stubs outstanding

Stubs are intentional, time-bounded shortcuts that land in an earlier R-move to satisfy compilation or downstream-sprint prerequisites, with their **wholesale replacement** scheduled in a specific later R-move. They are tracked here so nothing is left behind.

Rules:

- **Append on stub introduction.** The sprint that lands the stub adds a row in the same commit that introduces the stub code.
- **Strike on stub retirement.** The sprint whose R-move retires the stub wholesale removes the row in the same commit that lands the retiring code, and appends a log entry below.
- **Phase-close gate.** A phase row in the snapshot above cannot transition to `shipped` while any row in this table is assigned a retiring R-move that belongs to that phase.
- **Refactor-acceptance gate.** Phase E acceptance — and therefore [`ARCHITECTURAL-REFACTOR.md` § Acceptance for the refactor as a whole](./ARCHITECTURAL-REFACTOR.md#acceptance-for-the-refactor-as-a-whole) — does not hold while any row remains in this table. Stubs are by definition incompatible with refactor closure.
- **No silent extension.** A sprint that needs behaviour from a stubbed code path beyond the stub's contract must escalate via [`sprints/README.md` § 3 ambiguity protocol](./sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc). Patching the stub in place is forbidden; only the retiring R-move replaces it wholesale.

| Stub | Introduced by | Retired by | Source-doc anchor | Status |
|---|---|---|---|---|
| `resolver:phase-a` (trivial workspace-name lookup in `scope-graph::resolver::resolve_stub`, exposed via `Graph::resolve`/`Graph::resolve_batch`) | R1 (sprint 0001) | R3 (sprint 0003) | [`ARCHITECTURAL-REFACTOR.md` § R1 → Phase A resolver stub](./ARCHITECTURAL-REFACTOR.md#r1--typed-edge-insertion-api) | introduced 2026-05-11 |

## Status values

- `unstarted` — no commits referenced yet.
- `in-progress` — commits exist on a branch but the move's acceptance criteria are not all met on main.
- `shipped` — merged to main with every acceptance bullet from `ARCHITECTURAL-REFACTOR.md` for that move demonstrated.

A move's acceptance criteria are listed in `ARCHITECTURAL-REFACTOR.md` under that move's section. A move cannot transition to `shipped` until every acceptance bullet has been demonstrated.

A phase transitions to `shipped` only when every move it owns is `shipped`. Partial phase shipment is rejected per the atomic-phase rule (`ARCHITECTURAL-REFACTOR.md` "Phase order").

## Update protocol

Every status transition adds a row to the log below. Update the snapshot tables at the top in the same commit. Stub introductions and retirements use the same log channel:

```
- YYYY-MM-DD | <id> | <old status> → <new status> | commit <sha> | notes: ...
- YYYY-MM-DD | stub:<short-name> | introduced | commit <sha> | notes: retiring R-move = R<n>; anchor = ARCHITECTURAL-REFACTOR.md § R<m>
- YYYY-MM-DD | stub:<short-name> | retired | commit <sha> | notes: wholesale replacement landed in R<n>
```

## Log

- 2026-05-11 | R0 | unstarted → in-progress | branch refactor/sprint-0001-schema-storage | notes: sprint 0001 opened
- 2026-05-11 | R1 | unstarted → in-progress | branch refactor/sprint-0001-schema-storage | notes: sprint 0001 opened
- 2026-05-11 | stub:resolver:phase-a | introduced | commit (R1 implementation commit on sprint branch) | notes: retiring R-move = R3; anchor = ARCHITECTURAL-REFACTOR.md § R1 → Phase A resolver stub
