# 0002 — Rename workspace manifest `scope-workspace.toml` → `mudang-workspace.toml`

- **Status:** TODO
- **Decision:** rename, no back-compat.
- **Tracking:** _<issue / PR link to be added>_

## Decision

The workspace manifest filename (currently `scope-workspace.toml`)
will be renamed to `mudang-workspace.toml`. The TOML schema itself
is unchanged — only the filename moves.

## Affected code

- **`gumiho-mudang-scope`** — workspace discovery / loading logic.
  Every glob, filename literal, and error message referencing
  `scope-workspace.toml`.
- **`gumiho-mudang-cli`** — `workspace init`, `workspace list`,
  and any error / help text mentioning the manifest filename.
- **Tests / fixtures** — workspace integration tests.

## Affected docs (already updated)

All documentation inside this monorepo references
`mudang-workspace.toml`. Code currently still reads
`scope-workspace.toml`, so docs lead code until this TODO lands.

## Migration notes

- `mudang workspace init` should fail fast if a
  `scope-workspace.toml` already exists, with a hint to rename it
  manually.
- No automatic migration. The user handles the rename.
