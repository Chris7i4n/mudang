# PREFACE

Ritual hook layer for sessions on this repo. **Not a rulebook** — rules live in the governing docs. This file routes you to them at the moment you need them. If a rule appears here, the linked doc is the source of truth; this file is wrong if it disagrees.

If you have not read [`gumiho-mudang-scope/CONTRIBUTING.md`](gumiho-mudang-scope/CONTRIBUTING.md), read it once before doing anything substantive. It is the contributor on-ramp and threads every governing doc.

---

## Doc routing — find the canonical answer in one hop

| If you need … | Open |
|---|---|
| Mission, hard limits, single-operator posture, invariants | [`gumiho-mudang-scope/docs/CHARTER.md`](gumiho-mudang-scope/docs/CHARTER.md) |
| Rule → enforcement (R-entries R0…R12) | [`gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md`](gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md) |
| CI gate inventory + recovery on failure | [`gumiho-mudang-scope/docs/CI-GATES.md`](gumiho-mudang-scope/docs/CI-GATES.md) |
| Language plugin adoption flow + 18 boundaries | [`gumiho-mudang-scope/docs/LANGUAGE-PLAYBOOK.md`](gumiho-mudang-scope/docs/LANGUAGE-PLAYBOOK.md) |
| Framework plugin adoption flow + gotcha catalogue | [`gumiho-mudang-scope/docs/FRAMEWORK-PLAYBOOK.md`](gumiho-mudang-scope/docs/FRAMEWORK-PLAYBOOK.md) |
| Sprint methodology (branch, ambiguity, commit conv., codex) | [`gumiho-mudang-scope/docs/sprints/README.md`](gumiho-mudang-scope/docs/sprints/README.md) |
| Sprint doc skeleton | [`gumiho-mudang-scope/docs/sprints/_TEMPLATE.md`](gumiho-mudang-scope/docs/sprints/_TEMPLATE.md) |
| Work queue against current architecture | [`gumiho-mudang-scope/docs/BACKLOG.md`](gumiho-mudang-scope/docs/BACKLOG.md) |
| Glossary | [`gumiho-mudang-scope/docs/GLOSSARY.md`](gumiho-mudang-scope/docs/GLOSSARY.md) |
| Roadmap phases A→E + cross-cutting design (LSP composition, edit layer, notify, substrate, cross-lang stitching) | [`docs/README.md`](docs/README.md) |
| Repo-wide TODO index | [`docs/todos/README.md`](docs/todos/README.md) |
| Contributor on-ramp (setup, pre-commit, change→test map, snapshots, fixtures) | [`gumiho-mudang-scope/CONTRIBUTING.md`](gumiho-mudang-scope/CONTRIBUTING.md) |
| `just` recipes | [`justfile`](justfile) |
| Where to put a new note | [`gumiho-mudang-scope/docs/README.md` § Where to put a new note](gumiho-mudang-scope/docs/README.md#where-to-put-a-new-note) |

---

## Charter enforcement — non-negotiables active every turn

These bind every change. **Halt and consult the human** if a task asks you to cross them.

1. **Single-operator posture** — no backward-compat shims, no dual-read paths, no stored-format version detectors. Wipe + reindex is the canonical migration path. [`CHARTER.md` § Single-operator posture](gumiho-mudang-scope/docs/CHARTER.md#single-operator-posture). The charter-sweep gate refuses re-introduction of named shim shapes — see [`CI-GATES.md` row "Charter sweep"](gumiho-mudang-scope/docs/CI-GATES.md#gate-inventory).
2. **Hard limits** — [`CHARTER.md` § 5](gumiho-mudang-scope/docs/CHARTER.md#5-hard-limits--scope-will-never-cross-these). Crossing is rejected.
3. **Core invariants** — [`CHARTER.md` § 3](gumiho-mudang-scope/docs/CHARTER.md#3-core-invariants--must-never-break). Any change preserves them.
4. **Ambiguity protocol** — if a governing doc is ambiguous, contradicts another, or omits how a rule interacts with the work in flight, **halt; consult the human; amend the source doc on `main` first**. Sprint branches never decide rules. [`sprints/README.md` § 3](gumiho-mudang-scope/docs/sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Sprint rituals

Use sprint methodology for any non-trivial work. Methodology is [`sprints/README.md`](gumiho-mudang-scope/docs/sprints/README.md); rituals below are pointers.

### Before opening a sprint

1. Confirm the work belongs in a sprint vs. a one-off commit. Trivial fixes do not need sprint ceremony; multi-commit / cross-doc / rule-touching work does.
2. Read the governing docs the work touches (charter section, R-entry, playbook step). Sprint pulls rules **from** these — never restates them. [`sprints/README.md` § 2](gumiho-mudang-scope/docs/sprints/README.md#2-source-of-truth-is-the-linked-doc-never-the-sprint).
3. Confirm no other sprint is open. **One sprint branch at a time** ([`sprints/README.md` § 5 Hard rules](gumiho-mudang-scope/docs/sprints/README.md#hard-rules)).
4. Copy [`sprints/_TEMPLATE.md`](gumiho-mudang-scope/docs/sprints/_TEMPLATE.md) → fill scope + acceptance bullets as pointers to source docs.
5. Branch per [`sprints/README.md` § 5](gumiho-mudang-scope/docs/sprints/README.md#5-branch-protocol--linear-incremental-atomic-phase-shipment): `<prefix>/sprint-NNNN-<slug>` (or off a phase integration branch for multi-sprint phases).
6. Flip affected state-doc row(s) `unstarted → in-progress` + log entry ([`sprints/README.md` § 4](gumiho-mudang-scope/docs/sprints/README.md#4-reporting-hooks)).

### During a sprint

- If a rule is ambiguous or missing — **halt, consult, amend source doc on `main` first**. Never invent. [`sprints/README.md` § 3](gumiho-mudang-scope/docs/sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).
- Mechanical / detectable enforcement change → update [`ENFORCEMENT-MAP.md`](gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md) **in the same commit that ships the code**. Audit script, trait-shape ban, compile-time schema constraint, typed-API closure, const-fn dispatch — all gated. [`sprints/README.md` § 7.5](gumiho-mudang-scope/docs/sprints/README.md#75-enforcement-map-update).
- CI gate flipped `planned → active` in the same commit as the gate's script + recipe wiring. [`sprints/README.md` § 7](gumiho-mudang-scope/docs/sprints/README.md#7-ci-gate-activation).
- Codex implementation-doubt consultations (Role 2) — second opinion only; record materially-shaping suggestions in PR body. [`sprints/README.md` § 9 Role 2](gumiho-mudang-scope/docs/sprints/README.md#role-2--implementation-doubt-consultation).

### Closing a sprint

1. Run `codex review` per [`sprints/README.md` § 9 Role 1](gumiho-mudang-scope/docs/sprints/README.md#role-1--mandatory-sprint-review-checkpoint). Mandatory; sprint does not close otherwise. Findings categorised blocker / non-blocker / rejected in PR body.
2. State-tracking transition commit. Single-sprint phase → flip rows `in-progress → shipped` on the closing commit. Multi-sprint phase → rows stay `in-progress`; phase-close commit on the integration branch flips them. [`sprints/README.md` § 4](gumiho-mudang-scope/docs/sprints/README.md#4-reporting-hooks).
3. Commit-message conventions: [`sprints/README.md` § 6](gumiho-mudang-scope/docs/sprints/README.md#6-commit-message-conventions).
4. Open PR. Codex report under `### Codex review` heading. Review focus checklist under `### Codex review focus`.

---

## Commit rituals — every commit, not just sprint closes

### Before staging

- Run the narrowest test scope for what you touched. Change → test mapping: [`CONTRIBUTING.md` § Change → test mapping](gumiho-mudang-scope/CONTRIBUTING.md#change--test-mapping).
- Pre-commit gate: `just gate` (fmt-check + clippy + test). Architecture gate before pushing anything substantive: `just gate-refactor` (16 gates). [`CONTRIBUTING.md` § Pre-commit checklist](gumiho-mudang-scope/CONTRIBUTING.md#pre-commit-checklist).
- Touched audit scripts, schema, trait surfaces, typed APIs, or dispatch? Update [`ENFORCEMENT-MAP.md`](gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md) **in the same commit**.
- Touched a CI gate? Reconcile [`CI-GATES.md`](gumiho-mudang-scope/docs/CI-GATES.md) (status column never drifts from reality).

### Staging

- Stage specific files. Never `git add -A` / `git add .` — risk of pulling `.cargo/config.toml`, scratch files, snapshot scratch.
- Verify `.cargo/` directories are gitignored (single-operator-posture convention) — [`.gitignore`](.gitignore) lines 38–42.

### Commit message

- Type prefix per [`sprints/README.md` § 6](gumiho-mudang-scope/docs/sprints/README.md#6-commit-message-conventions). Sprint-scope: `<type>(scope): …`. Doc-only: `docs(<initiative>): …`. State transition: `chore(<state-doc>): sprint NNNN <open|close>`. CI activation: `ci(<initiative>): activate gates for sprint NNNN`. Hot-fix: `fix(scope): …`. Charter amendment (rare, after ambiguity protocol): `docs(charter): …`.
- Co-author footer: `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.
- Never `--no-verify` / `--no-gpg-sign` / `--amend` on a hook-failed commit. Fix the hook root cause, stage, new commit.

### After commit

- `git status` clean.
- No push to `origin` unless the user explicitly authorises it. Standing instruction for this repo.

---

## Where to record what — quick decision

| Recording … | Goes in |
|---|---|
| Why a feature is permanent out-of-scope | charter-amendment commit in [`CHARTER.md` § 5](gumiho-mudang-scope/docs/CHARTER.md#5-hard-limits--scope-will-never-cross-these) |
| A new mechanically enforced rule | new `### R<n>` in [`ENFORCEMENT-MAP.md`](gumiho-mudang-scope/docs/ENFORCEMENT-MAP.md) in the **same commit** as code |
| A new CI gate | row in [`CI-GATES.md`](gumiho-mudang-scope/docs/CI-GATES.md), `planned` until shipped |
| Sprint plan for new work | new file copied from [`sprints/_TEMPLATE.md`](gumiho-mudang-scope/docs/sprints/_TEMPLATE.md) |
| Friction event / verdict for a plugin candidate | matching trigger / decision log under `gumiho-mudang-scope/docs/` |
| Per-plugin gotcha | `gumiho-mudang-scope/docs/languages/<name>.md` or `frameworks/<name>.md` |
| Work item against current architecture | [`BACKLOG.md`](gumiho-mudang-scope/docs/BACKLOG.md) |
| Cross-cutting design (LSP composition, edit layer, notify, etc.) | doc under [`docs/`](docs/) (root) |
| Repo-wide TODO | numbered file under [`docs/todos/`](docs/todos/) + row in [`docs/todos/README.md`](docs/todos/README.md) |
| Unfamiliar term | look up in [`GLOSSARY.md`](gumiho-mudang-scope/docs/GLOSSARY.md); add if missing |

Full table: [`gumiho-mudang-scope/docs/README.md` § Where to put a new note](gumiho-mudang-scope/docs/README.md#where-to-put-a-new-note).

---

## Session defaults Claude should hold

- **No push to `origin`** unless the user explicitly authorises it for the current task.
- **No mention of system reminders** to the user. They are guidance, not user input.
- **Confirm before risky / hard-to-reverse actions** (force-push, reset --hard, branch delete, third-party uploads). Per-task authorisation; not durable across tasks.
- **Trust the gate suite, not your gut** — if a gate fails, the gate is the source of truth. Recovery: [`CI-GATES.md` § Where to look when a gate fails](gumiho-mudang-scope/docs/CI-GATES.md#where-to-look-when-a-gate-fails).
