# Sprint NNNN — <phase>: <slug>

> **Source of truth**: link into the doc that owns the rule(s) this sprint enforces (charter section, playbook step, architecture doc, etc.). Sprints **reference** rules — they never restate them.
> **Phase**: <letter> (single-sprint | first-of-N | middle-of-N | last-of-N | acceptance-only). State whether the sprint is atomic on its own or part of a multi-sprint phase that merges atomically.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

One paragraph. What this sprint changes, why now, what unblocks downstream. Avoid restating rules.

## Scope owned this sprint

- **<move-id or deliverable id>** ([source link](../<doc>.md#<anchor>))
- …

For acceptance-only sprints (no new code-shape change, only verification): record "None — acceptance gate" and list the gates verified.

## Prerequisites

- Predecessor sprint(s) shipped — name them and the rows in any state-tracking doc that must show `shipped`.
- Any source-doc amendment that has to land on `main` before this sprint's branch opens — see § Ambiguities resolved.

## Charter alignment

Every sprint maps onto charter sections explicitly. State which:

- **Hard limits** ([`CHARTER.md` § Hard limits](../CHARTER.md)) — which rule(s) this sprint mechanises / detects / leaves to discipline.
- **Soft expansion zone** ([`CHARTER.md` § Soft expansion](../CHARTER.md)) — which expansion row this sprint lands.
- **Per-language IN/OUT** ([`CHARTER.md` § Per-language scope](../CHARTER.md)) — touched languages.
- **Invariants** ([`CHARTER.md` § Core invariants](../CHARTER.md)) — invariants this sprint preserves or strengthens.

## Deliverables

Mirror each owned item's acceptance section from the source-of-truth doc. Every checkbox is a **pointer**; the content lives in the linked source.

### <item id> acceptance ([source](../<doc>.md#<anchor>))

- [ ] Concrete, demonstrable bullet 1 (acceptance from source doc).
- [ ] Concrete, demonstrable bullet 2.
- [ ] …

### <item id> implementation deliverables ([source target state](../<doc>.md#<anchor>))

- [ ] Implementation step 1.
- [ ] Implementation step 2.
- [ ] …

Repeat per owned item.

---

## Ambiguities resolved before this sprint opens

If any cross-doc ambiguity was surfaced during planning, record the resolution here as a pointer to the `main` commit that landed the amendment. Sprint branches never carry rule amendments (per `README.md` § 3).

If none: drop this section.

---

## CI gates activated in this sprint

Rows in [`CI-GATES.md` § Gate inventory](../CI-GATES.md) that flip `planned` → `active` in this sprint.

- [ ] **<gate name>** (`just <recipe>`) — `planned` → `active`.
- …

If sprint flips no gates: state "none".

## Glossary terms touched

Terms in [`GLOSSARY.md`](../GLOSSARY.md) that gain new behaviour or are first referenced by this sprint. A sprint never edits the glossary — if a new term emerges, halt and add it via the glossary's own commit channel before resuming.

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and [`README.md` § Branch protocol](./README.md#5-branch-protocol).

- **Branch**: `<prefix>/sprint-NNNN-<slug>`, cut from the correct upstream.
- **Base**: `main` (single-sprint phase) or `<prefix>/phase-<letter>` (multi-sprint phase).
- **Open**: flip status row(s) in the state-tracking doc to `in-progress`. Append log entry per item noting branch name.
- **Codex review** (mandatory per `README.md` § 9 Role 1): run the canonical command before the close commit. Attach report to PR body. Cross-reference the codex report against the sprint's acceptance bullets. Address blockers.
- **Close**:
  - Single-sprint phase / direct-to-main: flip item rows to `shipped` with commit SHA + date. Flip phase row in same commit.
  - Inside a phase integration branch: leave rows `in-progress`; record acceptance demonstration in PR body. `shipped` is reserved for the phase-close commit that reaches `main`.
- **Merge**: rebase-merge or squash-merge per the convention chosen at sprint plan opening.

## Definition of done

All of the following hold simultaneously:

1. Every checkbox in **Deliverables** is checked.
2. Every CI gate listed above is `active` in `CI-GATES.md`.
3. The state-tracking doc shows every owned item + its phase row as `shipped` (single-sprint phase) or `in-progress` pending phase close (multi-sprint phase).
4. The codex review report is attached to the PR body with each focus bullet either addressed or recorded as Non-blocker / Rejected with rationale.
5. No regressions in earlier sprints' acceptance bullets.
6. Any stub or transitional shim this sprint introduces is registered in its tracking table in the same commit; any stub this sprint retires is struck in the same commit.

## Out of scope for this sprint

- Future sprints' R-moves / deliverables — name them explicitly.
- Anything queued for after the larger initiative closes (e.g. a post-refactor / post-milestone plan doc).
- Anything that requires charter / playbook amendment — those go through the explicit amendment channels each governing document defines.
