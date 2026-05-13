# Framework Decisions

Verdict log for framework adoptions. Format defined in `FRAMEWORK-PLAYBOOK.md` Step 2.

## Format

```
## YYYY-MM-DD — Framework: <name>

**Path**: trigger-driven | maintainer-asserted
**Trigger count** (Path A only): N (entries in FRAMEWORK-TRIGGERS.md from <start> to <end>)
**Active projects** (Path B only): [name + last-touched date per project]
**Verdict**: BUILD | DEFER | REJECT
**ROI worksheet**: [paste]
**Strategy** (BUILD only): A (latest only) | B (multi-version) | C (decline)
**Notes**: [reasoning, edge cases, caveats]
```

Note: the **adoption path** field (`Path: trigger-driven | maintainer-asserted`) is distinct from the **version strategy** field (`Strategy: A | B | C`). Both happen to use "A/B" labels but mean different things — adoption path governs *whether* to build; version strategy governs *which versions* to support once building.

DEFER entries are re-evaluated 90 days after the original verdict; record the re-evaluation as a new dated entry that references the original.

A BUILD verdict authorizes implementation per `FRAMEWORK-PLAYBOOK.md` Step 5. The unknown-version policy and `applies_to_languages` list are recorded in the per-framework doc (`docs/frameworks/<name>.md`), not here. Framework adoption resumes only after `ENFORCEMENT-MAP.md` Phase C (R5) ships, since R5 owns the FrameworkPlugin trait shape.

The `Path` field is mandatory. Path B (maintainer-asserted) skips the trigger log but not the ROI worksheet; the active-projects field is the substitute for trigger evidence.

## Entries

(no entries yet)
