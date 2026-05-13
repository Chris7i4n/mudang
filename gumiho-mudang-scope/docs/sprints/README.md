# Sprints — scope methodology

Sequential delivery contract used by every scope sprint. Durable for every initiative — past and future — that runs against this codebase.

This document is the **methodology**. The per-sprint index (which sprint is next, which is in flight, which item owns which doc) lives wherever the active initiative tracks state. Historical sprint plans live in git history.

---

## Governing documents (consult first, always)

These documents are **law**, not suggestion. Every sprint links into them instead of copying their content. A sprint that contradicts any of them is invalid; a sprint that adds a rule not in them is invalid.

| Doc | Owns |
|---|---|
| [`CHARTER.md`](../CHARTER.md) | Mission, hard limits, soft expansion zone, per-language IN/OUT, amendment rule |
| [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) | Rule→implementation map (R-entries). Every sprint that changes an enforcement updates the matching R-entry in the same commit (§7.5) |
| [`LANGUAGE-PLAYBOOK.md`](../LANGUAGE-PLAYBOOK.md) | Universal language-plugin boundaries, adoption flow |
| [`FRAMEWORK-PLAYBOOK.md`](../FRAMEWORK-PLAYBOOK.md) | Framework adoption flow, version strategies, gotcha catalogue |
| [`GLOSSARY.md`](../GLOSSARY.md) | Term definitions (one source of truth) |
| [`CI-GATES.md`](../CI-GATES.md) | CI gate inventory, allowlist convention |
| [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) | Work queue eligible against the current architecture |

A sprint pulls its rules from these. If a sprint surfaces a rule that does not live in any of them, the sprint **halts** under § 3 Ambiguity protocol — the rule is decided on `main` first, then the sprint resumes.

---

## Sprint template

Every new sprint copies [`_TEMPLATE.md`](./_TEMPLATE.md) and fills it in. The template is the durable contract for sprint-doc shape; deviations are surfaced in PR review and adjusted to match.

---

## Rules of engagement

### 1. Linear order, no parallel sprints — atomic shipment to `main`

Sprints are sequential. Two sprints from different initiatives never run simultaneously. Two sprints inside the same multi-sprint phase ship in the listed order.

**Atomic shipment is preserved at the `main`-branch level.** When an initiative declares "all moves in a phase land together or none land," that mandate is honoured via **phase integration branches** (§5): any phase with more than one sprint lands its sprints onto a long-lived integration branch first, and only the **completed phase** merges to `main`.

Single-sprint phases (the sprint **is** the phase) merge directly to `main`. Structural / acceptance-only sprints (carrying no new code-shape change — e.g. crate decomposition prerequisite, charter sweep gate) merge directly to `main` because they have no multi-sprint phase to integrate.

Partial phase closure is rejected because it creates the very instability the multi-sprint phase exists to eliminate.

### 2. Source of truth is the linked doc, never the sprint

Each sprint lists deliverables as **pointers** into the relevant rule's acceptance bullets in the governing doc (charter section, R-move, playbook step, etc.). If a sprint and the source document disagree, the source document wins and the sprint is amended.

Sprints **never** restate rules. They restate **scope and ordering**. This prevents the "documentation drifts from code" failure mode where two documents claim the same rule and they fall out of sync.

### 3. Ambiguity protocol — consult the human, amend the source doc

If during implementation any of the following hold:

- An acceptance bullet is ambiguous or contradicts another doc.
- A sprint deliverable refers to behavior not specified in the linked source document.
- A type signature is required by one move but defined in another that is gated to a later phase.
- A rule in a playbook does not state how it interacts with the move shipping in the current sprint.
- A charter invariant could be read two ways.

**Halt. Consult the human. Do not invent. Do not infer. Do not pick the plausible reading and proceed.**

The cost of pausing is a question. The cost of inventing is a refactor of the refactor.

When an ambiguity is resolved, the resolution is committed to the **source-of-truth document** — `CHARTER.md`, the playbook, the architecture doc, etc. — using its own amendment channel (`docs(charter): …`, `docs(architecture): …`, etc.). The sprint branch is **never** the place where a rule is decided; sprint commits implement rules that the source doc already states. Sprint-local rule amendments are invalid because they would create a parallel rule surface and reintroduce the drift this protocol exists to prevent.

### 4. Reporting hooks

Every sprint reports to whatever state-tracking doc the active initiative uses. For an initiative without a dedicated state-tracking doc, the sprint's reporting reduces to its PR body + the merge commit on `main`.

Hook shape (instantiate against the active state doc):

- **At sprint start**: flip the affected row(s) in the snapshot table from `unstarted` → `in-progress`. Append a log entry:
  ```
  - YYYY-MM-DD | <id> | unstarted → in-progress | branch <name> | notes: sprint NNNN opened
  ```
- **At sprint close for a sprint that merges directly to `main`**: flip the row(s) to `shipped`. Append a log entry per move:
  ```
  - YYYY-MM-DD | <id> | in-progress → shipped | commit <sha> | notes: sprint NNNN closed; acceptance bullets X, Y, Z demonstrated
  ```
- **At sprint close inside a phase integration branch**: do **not** flip the row(s) to `shipped`. `shipped` means "merged to `main`". The sprint PR demonstrates acceptance on the integration branch; rows remain `in-progress` until the whole phase merges to `main`.
- **At phase close for a multi-sprint phase**: flip every row in that phase from `in-progress` → `shipped`, flip the phase row to `shipped`, and append log entries in the phase-close commit.

For a direct-to-`main` sprint, the snapshot tables and the log live in the same commit as the closing sprint deliverable. For a multi-sprint phase, the sprint-close commits record acceptance without marking rows `shipped`; the snapshot and log transition to `shipped` lives in the phase-close commit that merges to `main`.

### 5. Branch protocol — linear, incremental, atomic phase shipment

The linear-order rule (§1) and the atomic-phase mandate are enforced not only by reading discipline but also by **git topology**. Sprints run on their own branches; multi-sprint phases run on **phase integration branches** so that the **phase** is what merges to `main`, not the individual sprint.

#### Two branch kinds

- **Sprint branch** — `<prefix>/sprint-NNNN-<slug>`. One per sprint. The `<prefix>` reflects the initiative (e.g. `refactor/sprint-…` for the closed architectural refactor; a future initiative picks its own prefix at planning).
- **Phase integration branch** — `<prefix>/phase-<letter>`. One per multi-sprint phase.

#### Branch naming examples

```
<prefix>/sprint-0001-<slug>                → merges to main (single-sprint phase)
<prefix>/phase-b                           → integration branch
  <prefix>/sprint-0002-<slug>              → merges to <prefix>/phase-b
  <prefix>/sprint-0003-<slug>              → merges to <prefix>/phase-b
  <prefix>/sprint-0004-<slug>              → merges to <prefix>/phase-b
  → <prefix>/phase-b then merges to main   (phase atomic close)
```

#### Lifecycle — single-sprint phase

```
                       (main)
                          │
                          │  cut sprint branch
                          ▼
            <prefix>/sprint-NNNN-<slug>
              ── implementation commits ────
              ── CI-gate activation commit ─
              ── state-tracking transition ─
                          │
                          │  PR + review + CI green → merge to main
                          ▼
                       (main, sprint NNNN merged; phase closed if applicable)
```

#### Lifecycle — multi-sprint phase

```
                       (main, predecessor phase closed)
                          │
                          │  cut phase integration branch
                          ▼
            <prefix>/phase-<letter>
                ▲                  ▲                  ▲
                │ PR               │ PR               │ PR
            sprint-NNNN        sprint-NNNN+1      sprint-NNNN+2
            (cut from          (cut from          (cut from
             phase branch)      phase branch       phase branch
                                after NNNN         after NNNN+1
                                merged)            merged)

            After last sprint PR merges to phase branch:
                phase's full acceptance set is demonstrated.
                Phase-close commit lands on the phase branch
                (flips every move row + phase row in state doc).

            <prefix>/phase-<letter> ── PR + review + CI green → merge to main
                          │
                          ▼
                       (main, phase closed atomically)
```

#### Hard rules

- **One sprint branch open at a time.** No parallel sprint branches. Sprint N+1 does not branch until sprint N merges to its target (`main` for single-sprint phases; the phase integration branch for multi-sprint phases).
- **Base is the correct upstream.** A sprint branch in a multi-sprint phase branches off the phase integration branch, not `main`. Confusing the base reintroduces non-atomic phase shipment.
- **Phase integration branch only carries that phase's sprints.** No unrelated work merges into it. It is short-lived (only as long as the phase is in progress) and disappears after its PR merges to `main`.
- **No merging `main` *into* the sprint branch or the integration branch.** If `main` advances during a phase (hot-fix), rebase the integration branch onto the new `main`, then rebase each open sprint branch onto the updated integration branch. Merge commits from `main` would obscure which commits belong to which move.
- **Sprint branches never live longer than their sprint.** No stashing, no parking. A stalled sprint is re-scoped, not carried.
- **Ambiguity resolution commits land first, on the correct upstream.** Per §3, ambiguities are resolved by an amendment to the source-doc on `main` **before** any sprint or phase branch opens for the affected work. Sprint branches do not carry rule amendments.

#### Commit ordering inside a sprint branch

A sprint branch's history reads chronologically as:

1. *(Optional)* Pre-sprint setup commits (fixtures stubbed, scaffolding).
2. **Implementation commits.** Each commit message uses the initiative's commit-type prefix (e.g. `refactor(scope): <summary>` for the architectural refactor; future initiatives pick their own) and references the owned item ID in the body.
3. **CI-gate activation commit.** Flips the affected rows in `CI-GATES.md` from `planned` → `active`, lands the audit scripts and `justfile` recipes, wires CI to call them. Message: `ci(<initiative>): activate gates for sprint NNNN`.
4. **Codex review checkpoint.** Run `codex review --base <upstream>` (upstream = `main` for direct-to-main sprints; the phase integration branch for multi-sprint phases) with the focus checklist defined in §9. Follow-up commits addressing review findings use `<type>(scope): address codex review — <summary>` (or `fix(scope): …` for bugs the review caught). The review report itself is attached to the PR body, not committed.
5. **State-tracking transition commit.** For a sprint that merges directly to `main`, flips rows to `shipped` and appends log entries in the active state doc. Inside a phase integration branch, leaves rows `in-progress` and records acceptance in the sprint PR / commit body; `shipped` is reserved for the phase-close commit that reaches `main`. Message: `chore(<state-doc>): sprint NNNN close`.

#### Phase-close commit (multi-sprint phase only)

When the last sprint in a multi-sprint phase has merged to its integration branch, one additional commit lands on the integration branch before the integration-branch PR opens against `main`:

- **Phase-close commit.** Flips every row in the phase from `in-progress` → `shipped`, flips the **phase row** in the state-tracking doc from `in-progress` → `shipped`, appends move and phase-close log entries, and confirms every acceptance bullet for the phase is demonstrated. Message: `chore(<state-doc>): phase <letter> close`.

The integration-branch PR's merge commit on `main` is the official phase close. Use **rebase-merge** (preserves the linear history of sprint closes inside the phase) or **squash-merge** (one phase commit on `main`) — pick one and stick with it across the initiative.

#### Single-sprint phase close

For a single-sprint phase, the sprint **is** the phase. The sprint's state-tracking transition commit flips both the move row(s) **and** the phase row in the same commit. No phase integration branch is needed.

#### Rebase, not merge, for cleanup

Inside any branch (sprint or integration), history may be tidied via interactive rebase **before** PR opens. After PR opens, no force-pushes except to address review feedback.

When merging:

- Sprint PR → `main` (single-sprint phases): squash-merge or rebase-merge.
- Sprint PR → integration branch (multi-sprint phases): rebase-merge (so the integration branch shows each sprint as its own block of commits in chronological order).
- Integration PR → `main`: choose rebase-merge or squash-merge once at the start of the initiative and stick with it.

#### Hot-fixes during an open sprint or phase

Hot-fixes to `main` while sprint work is open should be rare. When they happen:

1. Hot-fix lands on `main` via `fix/<short-slug>`.
2. The open phase integration branch rebases onto the updated `main`.
3. Every open sprint branch inside the phase rebases onto the updated integration branch.
4. Work continues.

If the hot-fix touches code the open sprint is restructuring, halt (per §3) and consult the human.

### 6. Commit-message conventions

- Sprint-scope implementation: `<type>(scope): <short summary>` where `<type>` matches the initiative's convention (`refactor`, `feat`, `fix`, etc.).
- Doc-only updates linked to a sprint: `docs(<initiative>): <short summary>`.
- Reporting hook commits: `chore(<state-doc>): sprint NNNN <open|close>`.
- CI gate activation: `ci(<initiative>): activate gates for sprint NNNN`.
- Codex review follow-up: `<type>(scope): address codex review — <summary>` (or `fix(scope): <summary>` for bugs caught by the review). The review report itself is attached to the PR body, never committed.
- Hot-fixes during open sprint: `fix(scope): <short summary>`.
- Charter amendments (rare; only after the ambiguity protocol resolves in favor of changing the law): `docs(charter): <one-line summary>` per [`CHARTER.md` § Amending this charter](../CHARTER.md#11-amending-this-charter).

### 7. CI gate activation

Each sprint lists the CI gates from [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory) it is responsible for flipping from `planned` → `active`. The flip lands in the same commit as the gate's script and the recipe's wiring. Status column drift is not tolerated: doc and CI agree, always.

### 7.5 Enforcement-map update

Every sprint that introduces or changes a mechanical / detectable enforcement updates [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) in the **same commit** that ships the code. A sprint that adds a new audit script, a new trait-shape ban, a new compile-time schema constraint, a new typed-API closure, or a new const-fn dispatch — without registering it in the rule→enforcement map — does not close.

Two update shapes:

- **Refinement** — the technique is already represented by an existing `### R<n>` entry. Edit the entry's "Durable contract", "Where in the tree", or "CI gates" lines in place. Inventory-table rows are updated in the same commit if the class shifts (mechanical / detectable / discipline).
- **New technique** — the technique is genuinely new. Append a `### R<next>` section after the highest existing R-ID. Fill in: rules it enforces, durable contract, where in the tree, CI gates. The next free R-ID is the integer one after the highest existing R-ID; the choice is mechanical, not editorial. If the new technique brings a new universal rule into mechanical/detectable enforcement, the inventory tables gain a row in the same commit.

The class-3 universal list (`B1`, `C2`, `E3` per [`ENFORCEMENT-MAP.md` § Discipline-only rules](../ENFORCEMENT-MAP.md#discipline-only-rules)) is fixed; expanding it requires a charter amendment.

Sprint reviewers check the diff for code touching audit scripts, schema definitions, trait surfaces, typed APIs, or dispatch against the matching `ENFORCEMENT-MAP.md` edit. Missing the update is a blocker, not a defer-able finding.

### 8. Out of scope for any sprint

Anything in [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) is governed by that doc's own gate language and prioritisation. A sprint that proposes an item from that queue without invoking the queue's gate is invalid.

Also out: anything that requires breaking a charter invariant ([`CHARTER.md` § 3](../CHARTER.md#3-core-invariants--must-never-break)) or crosses a hard limit ([`CHARTER.md` § 5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)). These are non-negotiable.

### 9. Codex consultation protocol

Codex (OpenAI GPT-5-class model, invoked via the `codex` CLI installed as a Claude Code plugin) plays **two roles** in scope sprints — both bounded, neither overriding the §3 ambiguity protocol or the source-of-truth documents.

#### Role 1 — Mandatory sprint review checkpoint

Every sprint runs a `codex review` pass before its state-tracking transition commit and before the PR opens. The pass is **mandatory**; a sprint that skips it does not close.

**Canonical command shape**:

```bash
codex review \
  -c model="gpt-5.5" \
  -c model_reasoning_effort="medium" \
  --base <upstream> \
  --title "sprint NNNN — <slug>"
```

Where `<upstream>` is `main` for direct-to-main sprints and the phase integration branch for multi-sprint phases.

**CLI-shape note** (codex CLI v0.130.0): `--base`, `--commit`, and `--uncommitted` are each mutually exclusive with the `[PROMPT]` positional argument. The diff-source flag (`--base`) and an inline review prompt cannot both be passed in a single invocation. Sprints use **diff-source mode**: codex receives the precise diff scope via `--base <upstream>` and produces its default review report against the diff. The acceptance-bullet checklist that would otherwise be the prompt body lives in the **PR description** instead — the sprint author and the human reviewer cross-reference the codex report against that checklist when merging.

**Review focus checklist** (added verbatim to the PR description under `### Codex review focus`, one bullet per area; the human reviewer uses this to verify codex's report covered each area):

```
- Source-doc acceptance bullets (list bullets explicitly from the linked governing doc)
- CHARTER.md §3 invariants at risk in this sprint
- CHARTER.md §5 hard limits this sprint approaches
- CI-GATES.md gates this sprint flips from planned → active
- Sprint NNNN's Deliverables and Definition of done sections
- Active state-tracking doc — any stub / shim row this sprint introduces is registered; any retired in same commit is struck
```

The checklist is **mechanical** — every item is either covered by codex's report (annotate "✓ codex addressed: <one-line citation>") or surfaced as a gap and addressed in a follow-up commit.

**Why these flags**:

- `model = "gpt-5.5"` — locked across the initiative so review consistency is preserved across sprints. A model swap mid-initiative changes the review character and breaks the cross-sprint baseline; any change is a charter-amendment-grade decision recorded on `main` before the next sprint opens.
- `model_reasoning_effort = "medium"` — review work is read-and-flag against explicit acceptance bullets, not generative design. The default `xhigh` is wasted overhead at this scope; `medium` is the sweet spot for cross-doc consistency checks. If a specific sprint surfaces a deep cross-move interaction that medium misses, the sprint may override to `high` for that single invocation and record the override in the PR body.
- The `gpt-5.5-medium` model **variant** is **not** used: the ChatGPT-account-backed Codex CLI rejects it (`invalid_request_error: not supported when using Codex with a ChatGPT account`). Plain `gpt-5.5` combined with `model_reasoning_effort = "medium"` — not the rejected `model = "gpt-5.5-medium"` variant.

**Outcome handling**:

- The review report is attached to the PR body (under a `### Codex review` heading) so reviewers see what Codex flagged and how it was addressed.
- Findings are categorised by the sprint author:
  - **Blocker** — sprint cannot close until addressed. Follow-up commit lands on the sprint branch.
  - **Non-blocker** — recorded in the PR body with rationale for deferral; tracked in `POST-REFACTOR-PLAN.md` if it survives the initiative's horizon.
  - **Rejected** — Codex misread the contract; record the counter-argument in the PR body so the rationale is durable.
- **Codex is not authority**. If the review surfaces a rule ambiguity, the §3 ambiguity protocol takes over — the rule is resolved by amending the source-of-truth document, not by what Codex said.

#### Role 2 — Implementation-doubt consultation

During implementation, if a concrete coding question arises (idiom choice, Rust pattern, SQL constraint shape, test-harness wiring) the sprint author may consult Codex via `codex exec "<question>"` as a second opinion.

Boundaries:

- **Consultation, not delegation.** Codex's answer is treated like a Stack Overflow reply: useful input, not a contract.
- **No rule decisions.** If the question is "what should the rule be?", that is an ambiguity (§3) and goes to the human and the source doc, never to Codex.
- **No silent application.** A Codex suggestion that materially shapes the implementation is recorded in the PR body so reviewers can see the input that fed the design.
- **Sandboxed by default.** `codex exec` runs in the Codex sandbox (`workspace-write`), not the maintainer's full shell — it can read the workspace and write to `/tmp`, not execute arbitrary commands on the host.

#### What Codex is not used for

- **Charter or playbook amendments.** Rule changes go through the amendment channels each governing document defines.
- **CI gate decisions.** A gate's status (`planned` / `active` / `disabled`) is decided by the move owning it and recorded in `CI-GATES.md` — not by Codex's opinion about whether the gate is worth it.
- **Verdicts in `LANGUAGE-DECISIONS.md` or `FRAMEWORK-DECISIONS.md`.** Adoption verdicts are the maintainer's via the playbook flow.
- **State-tracking transitions.** Status flips reflect demonstrated acceptance, not Codex's assessment.

#### Why Codex at all

Two reasons recorded so the protocol is not arbitrary:

1. **Cross-model review surface area.** A second model trained on a different corpus surfaces blindspots that a single-model loop misses. The cost is `codex review` runtime; the benefit is catching contract drift before it reaches `main`.
2. **Implementation-detail bandwidth.** When the question is purely "how do I write this Rust idiom", consulting Codex offloads work that does not require human judgement, leaving the human's attention for §3 ambiguities — where it actually matters.

---

## Glossary anchors used across sprints

For quick reference. The authoritative definitions live in [`GLOSSARY.md`](../GLOSSARY.md).

- [Class 1 / mechanical · Class 2 / detectable · Class 3 / discipline-only](../GLOSSARY.md#architecture)
- [`RawCaptures` · `RawEdge` · `InsertableEdge` · `EdgeBuilder`](../GLOSSARY.md#refactor-types)
- [`LanguageWorkspaceContext` · `FrameworkWorkspaceContext`](../GLOSSARY.md#workspace-context)
- [`DetectedVersion` · `ResolvedVersion` · `UnknownVersionPolicy` · `VersionReq` · `available_in`](../GLOSSARY.md#versioning)
- [`Confidence` · `status` · orthogonality · cleanest-signal filter](../GLOSSARY.md#confidence-and-status-orthogonal)
- [`StatusData` · `file_hashes.skipped_ranges` · Surrogate PK · `edges.args_text` · Schema bumps](../GLOSSARY.md#schema)
- [`LanguagePlugin` · `FrameworkPlugin` · `Extractor` · `LanguageId` · `EdgeKind` · reserved metadata keys · 4-kind concurrency split](../GLOSSARY.md#plugin-shapes)

---

## What this document is not

- Not a substitute for any governing doc. Acceptance criteria live in the source-of-truth doc; sprints reference them, they do not redefine them.
- Not a calendar. Sprints have order; they do not have dates. The work is bounded but not time-boxed.
- Not a feature backlog. Features are queued in [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) and gated per that doc's own rules.
- Not a place to amend charter or playbook rules. Amendments go through the explicit-commit channels each governing document defines.
