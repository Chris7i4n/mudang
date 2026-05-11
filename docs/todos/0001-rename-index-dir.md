# 0001 — Rename index directory `.scope/` → `.mudang/`

- **Status:** TODO
- **Decision:** full rename, no back-compat (option b from the audit).
- **Tracking:** _<issue / PR link to be added>_

## Decision

The on-disk index directory (currently `.scope/`) will be renamed
to `.mudang/`. No fallback path will be kept. Existing
installations must regenerate the index after upgrade.

Old `.scope/` directories scattered across the user's projects will
be cleaned up manually by the user.

## Affected code

- **`gumiho-mudang-scope`** — every reference to `.scope/` in the
  engine crate: index path resolution, init logic, schema lookup,
  watcher glob patterns, SQLite file location.
- **`gumiho-mudang-cli`** — `init`, `index`, `status`, `setup`,
  watch loop, and any hard-coded `.scope/` literals in command
  handlers.
- **`gumiho-mudang-lsp`** — index discovery / loading paths.
- **Tests / fixtures** — fixture READMEs already reference
  `.mudang/`; verify fixture setup helpers create the new
  directory.
- **`.gitignore`** entries (project-local).

## Affected docs (already updated)

All documentation inside this monorepo references `.mudang/` as
part of the rename audit. Code currently still creates `.scope/`,
so docs lead code until this TODO lands.

## Migration notes

- `mudang init` should fail fast if a `.scope/` directory already
  exists in the target project, with a hint to delete it and re-run.
- No automatic migration tool. The user handles cleanup manually.
