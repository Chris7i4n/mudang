# Refactor Status

Live state of `ARCHITECTURAL-REFACTOR.md`. Append-only log; the snapshot tables at the top reflect the current state.

Until the refactor closes (Phase E acceptance — see `ARCHITECTURAL-REFACTOR.md` "Acceptance for the refactor as a whole"), feature work is paused.

## Phases

| Phase | Moves | Status | Started | Shipped | Notes |
|---|---|---|---|---|---|
| A — Schema and storage | R0, R1 | unstarted | — | — | first batch; everything else depends on it |
| B — Plugin layer | R2, R3, R4, R7, R9, R11, R12 | unstarted | — | — | depends on Phase A |
| C — Framework layer | R5 | unstarted | — | — | lands when framework infrastructure is first introduced; design constraint until then |
| D — Output and audit | R8, R10 | unstarted | — | — | adds detection layer for D2/E1 |
| E — Test harness | R6 | unstarted | — | — | last; depends on plugins in final shape |

## Moves

| ID | Title | Phase | Status | Commit | Date | Notes |
|---|---|---|---|---|---|---|
| R0 | Schema closures | A | unstarted | — | — | adds confidence/status/producer/pattern_id, surrogate edge_id PK, contains + 13 domain edge kinds, 3 new symbol kinds, skipped_ranges, schema_version 0→1 |
| R1 | Typed edge insertion API | A | unstarted | — | — | seals Edge; EdgeBuilder is sole producer of RawEdge; no .status() at extraction |
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

## Schema version

| Version | Status | R-move that bumped | Notes |
|---|---|---|---|
| 0 | current (legacy) | — | pre-R0; SQLite `PRAGMA user_version` unset, treated as 0 by R0's StatusData refusal logic |
| 1 | target | R0 | bumps inside the R0 migration transaction |

Subsequent schema-affecting refactors increment by 1 each. Future increments are recorded here when proposed; the binary's `EXPECTED_SCHEMA_VERSION` constant in `src/commands/status.rs` (introduced by R0) is the runtime source of truth.

## Status values

- `unstarted` — no commits referenced yet.
- `in-progress` — commits exist on a branch but the move's acceptance criteria are not all met on main.
- `shipped` — merged to main with every acceptance bullet from `ARCHITECTURAL-REFACTOR.md` for that move demonstrated.

A move's acceptance criteria are listed in `ARCHITECTURAL-REFACTOR.md` under that move's section. A move cannot transition to `shipped` until every acceptance bullet has been demonstrated.

A phase transitions to `shipped` only when every move it owns is `shipped`. Partial phase shipment is rejected per the atomic-phase rule (`ARCHITECTURAL-REFACTOR.md` "Phase order").

## Update protocol

Every status transition adds a row to the log below. Update the snapshot tables at the top in the same commit.

```
- YYYY-MM-DD | <id> | <old status> → <new status> | commit <sha> | notes: ...
```

## Log

(no transitions yet — every move is `unstarted`)
