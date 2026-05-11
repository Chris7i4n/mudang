# 0003 — Update GitHub URLs to the new repository

- **Status:** TODO
- **Decision:** replace every URL pointing at the legacy `scope` repo.
- **Tracking:** _<new repo URL to be added once published>_

## Background

Several documents currently link to the legacy GitHub URL
`https://github.com/rynhardt-potgieter/scope`. Once the new
`gumiho-mudang` repository is published, every such reference
must be updated.

During the rename audit, these URLs were either left as
`<TODO: new repo URL>` placeholders or removed where they were
purely cosmetic.

## Files containing legacy URLs (pre-audit)

- `gumiho-mudang-cli/skills/README.md` — installation `curl`
  commands (2 URLs).
- `~/.claude/CLAUDE.md` (user's private global config) — Scope CLI
  installed reference (1 URL).
- Any future install scripts, README badges, or release notes.

## Action

When the new URL is known, update every placeholder and lingering
legacy URL in a single PR. Cross-reference this file's tracking
link to confirm the planned destination.
