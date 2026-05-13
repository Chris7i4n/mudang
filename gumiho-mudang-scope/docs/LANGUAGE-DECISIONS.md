# Language Decisions

Verdict log for language adoptions. Format defined in `LANGUAGE-PLAYBOOK.md` Step 2 ("with the same format as `FRAMEWORK-DECISIONS.md`"), adapted for languages.

## Format

```
## YYYY-MM-DD — Language: <name>

**Path**: trigger-driven | maintainer-asserted
**Trigger count** (Path A only): N (entries in LANGUAGE-TRIGGERS.md from <start> to <end>)
**Active projects** (Path B only): [name + last-touched date per project]
**Verdict**: BUILD | DEFER | REJECT
**ROI worksheet**: [paste]
**Depth target** (BUILD only): surface | depth
**Notes**: [reasoning, edge cases, caveats]
```

DEFER entries are re-evaluated 90 days after the original verdict; record the re-evaluation as a new dated entry that references the original.

A BUILD verdict authorizes implementation per `LANGUAGE-PLAYBOOK.md` Step 5 within the 18 universal boundaries (Step 4). Depth feature work resumes only after `ENFORCEMENT-MAP.md` ships (`BACKLOG.md` gate); surface-only work is unblocked.

The `Path` field is mandatory. Path B (maintainer-asserted) skips the trigger log but not the ROI worksheet; the active-projects field is the substitute for trigger evidence.

## Entries

(no entries yet)
