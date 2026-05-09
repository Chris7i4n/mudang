# Schema Migration UX

Companion to `ARCHITECTURAL-REFACTOR.md` R0 (mechanics) and `CHARTER.md` §3 invariant 3 ("commit-able, survives across sessions and machines").

R0 specifies the mechanical side: SQLite `PRAGMA user_version` 0 → 1, atomic migration, `scope status` refusal. This document specifies the **user-facing** side: what a user sees, what a user does, and how a team workflow stays coherent across schema bumps.

If the binary's `EXPECTED_SCHEMA_VERSION` and the index's `user_version` agree, no UX exists — everything is silent. UX matters only at mismatch.

---

## Mismatch states

| Index `user_version` | Binary `EXPECTED_SCHEMA_VERSION` | State | Action |
|---|---|---|---|
| N | N | match | none — silent |
| < N | N | older index | forward migration (in place, atomic) |
| > N | N | newer index | refusal — index was written by a future binary |
| 0 (legacy) | ≥ 1 | pre-R0 index | forward migration applies; legacy rows backfilled conservatively |
| 0 (legacy) | 0 | pre-R0 binary + pre-R0 index | match (today's state) |

---

## Forward migration (older index)

When the binary opens an index whose `user_version` is less than `EXPECTED_SCHEMA_VERSION`:

1. **Detect the mismatch in `scope status`** and on every command that reads `.scope/`. Print exactly two lines:
   ```
   index schema 0 is older than binary's expected 1
   migration available: run `scope migrate` (or `scope index` to rebuild from source)
   ```
2. **Block reads** until migration runs. Reading would lie — the new binary expects columns the old schema does not have. Exit code `2`.
3. **`scope migrate` runs the atomic migration** (R0's transaction). Output:
   ```
   migrating .scope/ from schema 0 to 1
     · backfilling 12,341 edge rows with conservative defaults (confidence=low, status=dangling, producer=legacy_backfill)
     · adding skipped_ranges to 487 file_hashes rows
     · bumping PRAGMA user_version 0 → 1
   migration complete
   ```
4. **`scope index` (full rebuild)** is the alternative: re-extract from source files. Slower but produces honest tiers from R-move-aware extractors. Recommended after migration when time permits — replaces conservative `legacy_backfill` rows with real tiers.

The migration is atomic. If it fails partway (Ctrl-C, disk full, OS kill), the SQLite transaction rolls back; the index is unchanged. The binary refuses to operate against a half-migrated index because that state is unreachable.

---

## Refusal (newer index)

When the binary opens an index whose `user_version` is greater than `EXPECTED_SCHEMA_VERSION`:

1. **Refuse all read and write operations.** Backward migration is not implemented; downgrading the binary while keeping the index would corrupt the agreement between schema constraints and binary behavior.
2. **Print on every invoked subcommand**:
   ```
   index schema 2 was written by a newer binary; this binary expects schema 1
   options:
     · upgrade scope (cargo install scope, or your platform equivalent)
     · rebuild from source: rm -rf .scope/ && scope index
   ```
3. **Exit code `2`** so wrappers (CI, agents) can branch on it deterministically.

The refusal does not delete the index. The user decides — upgrade or rebuild. Refusal is silent only after the first informative print; subsequent silent refusals lose information, so every refused subcommand prints the same two lines.

---

## Pre-R0 index (legacy)

The current schema is `user_version = 0` (unset, treated as 0 by R0's introspection logic). On the first R0-bearing binary:

- Detection: `user_version` is 0; `EXPECTED_SCHEMA_VERSION` is 1.
- Treatment: identical to "older index" — forward migration via `scope migrate`.
- Backfill: legacy rows get
  - `confidence='low'`
  - `status='dangling'`
  - `producer='legacy_backfill'`
  - `pattern_id='legacy'`
  - `capture_id=NULL`
  - `skipped_ranges='[]'`
- Recommendation in the migration output: re-index when convenient (`scope index`) to replace conservative defaults with honest tiers.

---

## Atomic interruption

Migrations run inside a single SQLite transaction. Three interruption cases:

| Case | Outcome |
|---|---|
| Ctrl-C during migration | Transaction rolls back; index is identical to pre-migration state |
| Disk full during migration | Transaction rolls back; index is identical to pre-migration state |
| Process killed (OOM, SIGKILL) | SQLite WAL is consistent; on next open, journal is replayed; index is identical to pre-migration state |

After any interrupted migration, re-run `scope migrate`. There is no "partial" state to repair.

---

## Team workflow (`.scope/` committed)

`.scope/` is committable per `CHARTER.md` §3 invariant 3. Schema bumps cross this invariant unless the workflow handles them.

### Recommended pattern

1. **One person upgrades the binary first** and runs `scope migrate` (or `scope index` for a clean rebuild).
2. **They commit the migrated `.scope/`** with a commit message like:
   ```
   chore(scope): migrate index schema 0 → 1
   ```
3. **Team members pull**:
   - **Already on the matching binary** → migration is a no-op; they see the new SQLite contents and continue working.
   - **Still on the older binary** → their `scope` calls **refuse** (the index is now schema 1; their binary expects 0). The two-line message points them to upgrade. Once they upgrade, migration is a no-op (the index is already at schema 1).

The asymmetry — refuse newer, migrate older — is deliberate. It enforces a one-way ratchet: once the team has migrated, no one accidentally downgrades and queries silently against an index they cannot understand.

### Team coordination signal

For larger teams, the schema-migration commit is a coordination point: it tells everyone they need to upgrade their binary before their next `scope` call. Treating it as a normal commit is fine; the refusal message handles the case where someone misses the signal.

### Anti-patterns

- **Do not commit a half-migrated `.scope/`.** Migrations are atomic; if interrupted, the index is unchanged. Always re-run.
- **Do not edit `.scope/` files manually.** The schema is a contract enforced by the binary; manual edits invalidate it.
- **Do not bypass the refusal** with `--force` flags. There is no `--force` option for schema mismatch — adding one would silently corrupt indices.

---

## Error message format

Every refusal or migration-required message follows the same shape:

```
<one-line summary of the mismatch>
<one-line action with concrete command>
```

Two lines. No ASCII art. No banners. Agents parse the second line; humans read both.

Exit codes:

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | generic error (existing) |
| 2 | schema mismatch — caller must run `scope migrate` or upgrade the binary |

CI / agent wrappers branch on code `2` to surface a remediation prompt rather than treating it as a generic failure.

---

## Schema-version log per bump

Each schema bump after the initial 0 → 1 adds a section here describing:

- Which R-move added the version
- What columns / constraints changed
- What the conservative backfill looks like for the new columns
- Whether forward migration is in place or rebuild-only

The first bump (0 → 1) is fully described in `ARCHITECTURAL-REFACTOR.md` R0. Subsequent bumps reference the R-move that added them and append a section here.

### Schema 0 → 1 (R0)

- Owner: R0
- Forward migration: in place via `scope migrate`
- New columns: `edges.edge_id`, `edges.confidence`, `edges.status`, `edges.producer`, `edges.pattern_id`, `edges.capture_id`, `edges.framework`, `file_hashes.skipped_ranges`
- New whitelist entries: 14 net-new edge kinds (`contains` + 13 domain), 3 net-new symbol kinds (`macro`, `module`, `trait`)
- Conservative backfill: see "Pre-R0 index" above

---

## What this document does not cover

- **Backward migrations.** Not implemented. If a user genuinely needs to downgrade, they delete `.scope/` and rebuild with the older binary. The newer binary's commit history is the only record of what was lost.
- **Partial migrations.** Not allowed. Each migration is atomic per R0.
- **Schema-version coordination across multiple `.scope/` instances** (e.g., `scope link` cross-project). Deferred until `scope link` ships per `POST-REFACTOR-PLAN.md`.

---

## Cross-references

- `CHARTER.md` §3 invariant 3 — `.scope/` portability claim.
- `ARCHITECTURAL-REFACTOR.md` R0 — mechanical side: schema, migration, status acceptance.
- `REFACTOR-STATUS.md` § "Schema version" — current and target versions.
- `GLOSSARY.md` § "Schema and migration" — term definitions.
- `src/commands/status.rs` — `StatusData.schema_version` field (introduced by R0).
- `src/commands/migrate.rs` — `scope migrate` subcommand (introduced by R0).
