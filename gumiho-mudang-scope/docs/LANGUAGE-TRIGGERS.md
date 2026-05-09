# Language Triggers

Append-only friction log for new-language candidates. Format defined in `LANGUAGE-PLAYBOOK.md` Step 1.

## Format

```
- YYYY-MM-DD | <language> | <one-line description of friction>
```

Three fields: date, language name, one-line description. Keep entries short and honest.

## Threshold (Path A only)

5+ entries for the same language within 60 days → candidate moves to `LANGUAGE-PLAYBOOK.md` Step 2 (evaluation). The threshold is higher than for frameworks (5 vs 3) because adding a language is more expensive.

Languages adopted via the **maintainer-asserted path** (`LANGUAGE-PLAYBOOK.md` Step 1, Path B) skip this log and go directly to Step 2. The path choice is recorded in `LANGUAGE-DECISIONS.md`.

## Discipline

- **Log honestly.** A one-liner script that solved the problem is not a trigger.
- **Log immediately.** Retrospective logs miss real friction.
- **One trigger per real incident.** Do not pad to justify a language already wanted.
- **Do not log triggers for languages not currently used.** "I might learn Elixir someday" is not a trigger.

## Entries

(no entries yet)
