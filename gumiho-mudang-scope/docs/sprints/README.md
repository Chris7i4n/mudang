# Sprints — scope architectural refactor

Linear, sequential delivery plan for `ARCHITECTURAL-REFACTOR.md` R0–R12.
Each sprint maps onto one phase (A–E) of the refactor; sprints within a
phase land in the order listed, and the phase row in
[`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) only flips to `shipped`
when **every** R-move it owns is `shipped`.

This document is the **only** sprint index for the refactor. It does not
restate the rules; it points at the documents that own them.

---

## Governing documents (consult first, always)

These documents are **law**, not suggestion. Every sprint links into them
instead of copying their content. A sprint that contradicts any of them is
invalid; a sprint that adds a rule not in them is invalid.

| Doc | Owns |
|---|---|
| [`CHARTER.md`](../CHARTER.md) | Mission, hard limits, soft expansion zone, per-language IN/OUT, amendment rule |
| [`ARCHITECTURAL-REFACTOR.md`](../ARCHITECTURAL-REFACTOR.md) | R0–R12 moves, three classes of constraint, phase order, acceptance criteria |
| [`LANGUAGE-PLAYBOOK.md`](../LANGUAGE-PLAYBOOK.md) | 18 universal language-plugin boundaries, adoption flow |
| [`FRAMEWORK-PLAYBOOK.md`](../FRAMEWORK-PLAYBOOK.md) | Framework adoption flow, version strategies, 15 gotcha categories |
| [`GLOSSARY.md`](../GLOSSARY.md) | Term definitions (one source of truth) |
| [`CI-GATES.md`](../CI-GATES.md) | CI gate inventory, allowlist convention |
| [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) | Live state of every move and phase; reporting target |
| [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) | Work queued **after** Phase E acceptance; no sprint here starts before then |

---

## Sprint index

Land in this order. No sprint starts before its predecessor is closed.

Sprint 0000 is a **structural prerequisite** (crate decomposition) owned
by mudang's umbrella docs, not by `ARCHITECTURAL-REFACTOR.md`. Sprints
0001–0007 are the R-move sprints. Sprint 0008 is the final acceptance gate (charter sweep + shim retirement) — no new R-moves, but no refactor close without it.

| # | Sprint | Phase | R-moves | Status |
|---|---|---|---|---|
| 0000 | [Crate decomposition](./0000-phase-a-crate-decomposition.md) | A (internal, pre-R0) | — (structural) | shipped (2026-05-11) |
| 0001 | [Schema and storage](./0001-phase-a-schema-and-storage.md) | A | R0, R1 | shipped (2026-05-11) |
| 0002 | [Dispatch and workspace context](./0002-phase-b-dispatch-and-workspace-context.md) | B | R7, R4 | in-progress (closed on `refactor/phase-b`; not yet on `main`) |
| 0003 | [Typed plugin output and resolution pipeline](./0003-phase-b-typed-plugin-and-resolution.md) | B | R2, R3 | in-progress (chunks 1, 2, 3a, 3b + codex review fixes on `refactor/sprint-0003-typed-plugin-resolution`) |
| 0004 | [Trait closure and audit gates](./0004-phase-b-trait-closure-and-audits.md) | B | R9, R11, R12 | unstarted |
| 0005 | [Framework infrastructure](./0005-phase-c-framework-infrastructure.md) | C | R5 | unstarted |
| 0006 | [Output schema and confidence audit](./0006-phase-d-output-and-audit.md) | D | R10, R8 | unstarted |
| 0007 | [Malformed-source harness](./0007-phase-e-malformed-source-harness.md) | E | R6 | unstarted |
| 0008 | [Charter sweep and shim retirement](./0008-phase-e-charter-sweep.md) | E | — (acceptance gate) | unstarted |

After sprint 0008 closes and the full-refactor acceptance criteria in
[`ARCHITECTURAL-REFACTOR.md` § Acceptance for the refactor as a whole](../ARCHITECTURAL-REFACTOR.md#acceptance-for-the-refactor-as-a-whole)
hold, `POST-REFACTOR-PLAN.md`'s queue becomes eligible. Nothing from
that document starts earlier.

---

## Rules of engagement

### 1. Linear order, no parallel sprints — atomic phase shipment to `main`

Sprints are sequential. Two sprints from different phases never run
simultaneously. Two sprints inside the same phase ship in the listed
order.

**Atomic phase shipment is preserved at the `main`-branch level.**
[`ARCHITECTURAL-REFACTOR.md` § Phase order](../ARCHITECTURAL-REFACTOR.md#phase-order)
mandates that "each phase ships atomically: all moves in the phase
land together or none land." This sprint plan honours that mandate via
**phase integration branches** (§5 below): any phase with more than
one R-move sprint (today: Phase B) lands its sprints onto a
long-lived integration branch first, and only the **completed phase**
merges to `main`. The phase row in
[`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) flips to `shipped` in
the same commit as the phase-integration-branch merge to `main`.

Single-sprint phases (today: A's R-move sprint 0001, C, D, E) merge
directly to `main` because the sprint **is** the phase. Sprint 0000
(structural, pre-R0) merges directly to `main` because it carries no
R-move; it lands before Phase A's atomic shipment starts.

Partial phase closure is rejected because it creates the very
instability the refactor exists to eliminate.

### 2. Source of truth is the linked doc, never the sprint

Each sprint lists deliverables as **pointers** into the relevant R-move's
acceptance bullets in `ARCHITECTURAL-REFACTOR.md`. If a sprint and the
source document disagree, the source document wins and the sprint is
amended.

Sprints **never** restate rules. They restate **scope and ordering**.
This prevents the "documentation drifts from code" failure mode where
two documents claim the same rule and they fall out of sync.

### 3. Ambiguity protocol — consult the human, amend the source doc

If during implementation any of the following hold:

- An R-move's acceptance bullet is ambiguous or contradicts another doc.
- A sprint deliverable refers to behavior not specified in the linked
  source document.
- A type signature is required by one R-move but defined in another that
  is gated to a later phase.
- A rule in `LANGUAGE-PLAYBOOK.md` or `FRAMEWORK-PLAYBOOK.md` does not
  state how it interacts with the R-move shipping in the current sprint.
- A charter invariant could be read two ways.

**Halt. Consult the human. Do not invent. Do not infer. Do not pick the
plausible reading and proceed.**

The cost of pausing is a question. The cost of inventing is a refactor
of the refactor.

When an ambiguity is resolved, the resolution is committed to the
**source-of-truth document** — `CHARTER.md`,
`ARCHITECTURAL-REFACTOR.md`, `LANGUAGE-PLAYBOOK.md`,
`FRAMEWORK-PLAYBOOK.md`, `docs/ARCHITECTURE.md`, or `docs/ROADMAP.md`,
depending on which doc owns the rule — using its own amendment
channel (`docs(charter): …`, `docs(refactor): …`, etc.). The sprint
branch is **never** the place where a rule is decided; sprint commits
implement rules that the source doc already states. Sprint-local rule
amendments are invalid because they would create a parallel rule surface
and reintroduce the drift this protocol exists to prevent.

### 4. Reporting hooks

Every R-move sprint reports to
[`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md). Sprint 0000 is the
exception: it carries no R-move and reports through
[`docs/todos/0006-split-scope-crate.md`](../../../docs/todos/0006-split-scope-crate.md)
instead.

The hooks are:

- **At sprint start**: flip the affected R-move row(s) in the snapshot
  table from `unstarted` → `in-progress`. Append a log entry:
  ```
  - YYYY-MM-DD | RN | unstarted → in-progress | branch <name> | notes: sprint NNNN opened
  ```
- **At sprint close for a sprint that merges directly to `main`**:
  flip the row(s) to `shipped`. Append a log entry per R-move:
  ```
  - YYYY-MM-DD | RN | in-progress → shipped | commit <sha> | notes: sprint NNNN closed; acceptance bullets X, Y, Z demonstrated
  ```
- **At sprint close inside a phase integration branch**: do **not** flip
  the R-move row(s) to `shipped`. `shipped` means "merged to `main`"
  per `REFACTOR-STATUS.md`. The sprint PR demonstrates acceptance on
  the integration branch; the rows remain `in-progress` until the whole
  phase merges to `main`.
- **At phase close for a multi-sprint phase**: flip every R-move row in
  that phase from `in-progress` → `shipped`, flip the phase row to
  `shipped`, and append log entries in the phase-close commit.

For a direct-to-`main` sprint, the snapshot tables and the log live in
the same commit as the closing sprint deliverable. For a multi-sprint
phase, the sprint-close commits record acceptance without marking rows
`shipped`; the snapshot and log transition to `shipped` lives in the
phase-close commit that merges to `main`.

### 5. Branch protocol — linear, incremental, atomic phase shipment

The linear-order rule (§1) and the atomic-phase mandate are enforced
not only by reading discipline but also by **git topology**. Sprints
run on their own branches; multi-sprint phases run on **phase
integration branches** so that the **phase** is what merges to `main`,
not the individual sprint.

#### Two branch kinds

- **Sprint branch** — `refactor/sprint-NNNN-<slug>`. One per sprint.
- **Phase integration branch** — `refactor/phase-<letter>`. One per
  multi-sprint phase. Today only Phase B uses one; A, C, D, E are each
  a single R-move sprint and merge directly to `main`.

#### Branch naming

```
refactor/sprint-0000-crate-decomposition           → merges to main
refactor/sprint-0001-schema-storage                → merges to main (Phase A R-move)
refactor/phase-b                                   → integration branch
  refactor/sprint-0002-dispatch-workspace-context  → merges to refactor/phase-b
  refactor/sprint-0003-typed-plugin-resolution     → merges to refactor/phase-b
  refactor/sprint-0004-trait-closure-audits        → merges to refactor/phase-b
  → refactor/phase-b then merges to main           (Phase B atomic close)
refactor/sprint-0005-framework-infrastructure      → merges to main (Phase C)
refactor/sprint-0006-output-audit                  → merges to main (Phase D)
refactor/sprint-0007-malformed-harness             → merges to main (Phase E)
```

#### Lifecycle — single-sprint phase (A R-move, C, D, E) and sprint 0000

```
                       (main)
                          │
                          │  cut sprint branch
                          ▼
            refactor/sprint-NNNN-<slug>
              ── R-move implementation commits ──
              ── CI-gate activation commit ─────
              ── REFACTOR-STATUS.md transition ──
                          │
                          │  PR + review + CI green → merge to main
                          ▼
                       (main, sprint NNNN merged; phase closed if applicable)
```

#### Lifecycle — multi-sprint phase (B)

```
                       (main, sprint 0001 merged → Phase A closed)
                          │
                          │  cut Phase B integration branch
                          ▼
            refactor/phase-b
                ▲                  ▲                  ▲
                │                  │                  │
                │ PR               │ PR               │ PR
                │                  │                  │
            sprint-0002        sprint-0003        sprint-0004
            (cut from          (cut from          (cut from
             phase-b)           phase-b after      phase-b after
                                0002 merged)       0003 merged)

            After 0004 PR merges to phase-b:
                Phase B's full acceptance set is demonstrated on
                refactor/phase-b. Then:

            refactor/phase-b ── PR + review + CI green → merge to main
                          │
                          ▼
                       (main, Phase B closed atomically)
```

#### Hard rules

- **One sprint branch open at a time.** No parallel sprint branches.
  Sprint N+1 does not branch until sprint N merges to its target
  (`main` for single-sprint phases and sprint 0000;
  `refactor/phase-<letter>` for multi-sprint phases).
- **Base is the correct upstream.** A sprint branch in a multi-sprint
  phase branches off `refactor/phase-<letter>`, not `main`. A
  single-sprint-phase sprint branches off `main`. Confusing the base
  reintroduces non-atomic phase shipment.
- **Phase integration branch only carries that phase's sprints.** No
  unrelated work merges into `refactor/phase-b`. It is short-lived
  (only as long as Phase B is in progress) and disappears after its
  PR merges to `main`.
- **No merging `main` *into* the sprint branch or the integration
  branch.** If `main` advances during a phase (hot-fix), rebase the
  integration branch onto the new `main`, then rebase each open sprint
  branch onto the updated integration branch. Merge commits from `main`
  would obscure which commits belong to which R-move.
- **Sprint branches never live longer than their sprint.** No stashing,
  no parking. A stalled sprint is re-scoped, not carried.
- **Ambiguity resolution commits land first, on the correct upstream.**
  Per §3, ambiguities are resolved by an amendment to the source-doc
  on `main` **before** any sprint or phase branch opens for the
  affected work. Sprint branches do not carry rule amendments.

#### Commit ordering inside a sprint branch

A sprint branch's history reads chronologically as:

1. *(Optional)* Pre-sprint setup commits (fixtures stubbed, scaffolding).
2. **R-move implementation commits.** Each commit message uses
   `refactor(scope): <summary>` and references the R-move ID in the
   body (`R0:`, `R1:`, etc.).
3. **CI-gate activation commit.** Flips the affected rows in
   `CI-GATES.md` from `planned` → `active`, lands the audit scripts
   and `justfile` recipes, wires CI to call them. Message:
   `ci(refactor): activate gates for sprint NNNN`.
4. **Codex review checkpoint.** Run `codex review --base <upstream>`
   (upstream = `main` for direct-to-main sprints; `refactor/phase-b`
   for Phase B sprints) with a prompt that names the R-move(s) owned
   and the acceptance bullets in `ARCHITECTURAL-REFACTOR.md`. See
   §9 below for the full protocol. Follow-up commits addressing review
   findings use `refactor(scope): address codex review — <summary>`
   (or `fix(scope): …` for bugs the review caught). The review report
   itself is attached to the PR body, not committed.
5. **REFACTOR-STATUS.md transition commit.** For a sprint that merges
   directly to `main`, flips R-move rows to `shipped` and appends log
   entries. Inside a phase integration branch, leaves the R-move rows
   `in-progress` and records acceptance in the sprint PR / commit body;
   `shipped` is reserved for the phase-close commit that reaches
   `main`. Message: `chore(refactor-status): sprint NNNN close`.

#### Phase-close commit (multi-sprint phase only)

When the last sprint in a multi-sprint phase has merged to its
integration branch, one additional commit lands on the integration
branch before the integration-branch PR opens against `main`:

- **Phase-close commit.** Flips every R-move row in the phase from
  `in-progress` → `shipped`, flips the **phase row** in
  `REFACTOR-STATUS.md` snapshot from `in-progress` → `shipped`, appends
  R-move and phase-close log entries, and confirms every R-move
  acceptance bullet for the phase is demonstrated. Message:
  `chore(refactor-status): phase <letter> close`.

The integration-branch PR's merge commit on `main` is the official
phase close. Use **rebase-merge** (preserves the linear history of
sprint closes inside the phase) or **squash-merge** (one phase commit
on `main`) — pick one and stick with it across the refactor.

#### Single-sprint phase close

For Phase A's R-move (sprint 0001), Phase C (sprint 0005), Phase D
(sprint 0006), and Phase E (sprint 0007), the sprint **is** the
phase. The sprint's REFACTOR-STATUS.md transition commit flips both
the R-move row(s) **and** the phase row in the same commit. No
phase integration branch is needed.

#### Rebase, not merge, for cleanup

Inside any branch (sprint or integration), history may be tidied via
interactive rebase **before** PR opens. After PR opens, no force-pushes
except to address review feedback.

When merging:

- Sprint PR → `main` (single-sprint phases / sprint 0000): squash-merge
  or rebase-merge.
- Sprint PR → integration branch (multi-sprint phases): rebase-merge
  (so the integration branch shows each sprint as its own block of
  commits in chronological order).
- Integration PR → `main`: choose rebase-merge or squash-merge once at
  the start of the refactor and stick with it.

#### Hot-fixes during an open sprint or phase

Hot-fixes to `main` while refactor work is open should be rare. When
they happen:

1. Hot-fix lands on `main` via `fix/<short-slug>`.
2. The open phase integration branch rebases onto the updated `main`.
3. Every open sprint branch inside the phase rebases onto the updated
   integration branch.
4. Work continues.

If the hot-fix touches code the open sprint is restructuring, halt
(per §3) and consult the human.

#### Visualizing the linear history

After all eight sprints close (one for crate decomposition + seven for
R-moves), the `main` branch's `git log --oneline` shows the refactor
as the five phase closes, plus the structural sprint 0000 if history is
not squashed, in order:

```
…  chore(refactor-status): phase E close       (sprint 0007)
…  chore(refactor-status): phase D close       (sprint 0006)
…  chore(refactor-status): phase C close       (sprint 0005)
…  chore(refactor-status): phase B close       (Phase B integration merge)
…  chore(refactor-status): sprint 0001 close   (Phase A's R-move sprint)
…  chore(refactor-status): sprint 0000 close   (crate decomposition)
```

Any deviation from this shape is a process failure.

### 6. Commit-message conventions

- Sprint-scope refactor work: `refactor(scope): <short summary>`.
- Doc-only updates linked to a sprint: `docs(refactor): <short summary>`.
- Reporting hook commits: `chore(refactor-status): sprint NNNN <open|close>`.
- CI gate activation: `ci(refactor): activate gates for sprint NNNN`.
- Codex review follow-up: `refactor(scope): address codex review — <summary>`
  (or `fix(scope): <summary>` for bugs caught by the review). The review
  report itself is attached to the PR body, never committed.
- Hot-fixes during open sprint: `fix(scope): <short summary>`.
- Charter amendments (rare; only after the ambiguity protocol resolves
  in favor of changing the law): `docs(charter): <one-line summary>` per
  [`CHARTER.md` § Amending this charter](../CHARTER.md#11-amending-this-charter).

### 7. CI gate activation

Each sprint lists the CI gates from
[`CI-GATES.md`](../CI-GATES.md#gate-inventory) it is responsible for
flipping from `planned` → `active`. The flip lands in the same commit
as the gate's script and the recipe's wiring. Status column drift is
not tolerated: doc and CI agree, always.

### 8. Out of scope for the refactor sprints

Anything from [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) — per-language
depth, framework rollout, vector embeddings, time-travel queries, `scope link`,
`.js`/`.jsx` indexing — is **out** until Phase E acceptance holds. A sprint
that proposes any of these items is invalid; file it in the post-refactor
queue.

Also out: anything that requires breaking a charter invariant (`CHARTER.md`
§3) or crosses a hard limit (`CHARTER.md` §5). These are non-negotiable.

### 9. Codex consultation protocol

Codex (OpenAI GPT-5-class model, invoked via the `codex` CLI installed
as a Claude Code plugin) plays **two roles** in the refactor sprints —
both bounded, neither overriding the §3 ambiguity protocol or the
source-of-truth documents.

#### Role 1 — Mandatory sprint review checkpoint

Every sprint runs a `codex review` pass before its
`REFACTOR-STATUS.md` transition commit and before the PR opens.
The pass is **mandatory**; a sprint that skips it does not close.

**Canonical command shape** (model and reasoning effort locked across
the refactor):

```bash
codex review \
  -c model="gpt-5.5" \
  -c model_reasoning_effort="medium" \
  --base <upstream> \
  --title "sprint NNNN — <slug>"
```

Where `<upstream>` is `main` for direct-to-main sprints and
`refactor/phase-b` for Phase B sprints.

**CLI-shape note** (codex CLI v0.130.0): `--base`, `--commit`, and
`--uncommitted` are each mutually exclusive with the `[PROMPT]`
positional argument. The diff-source flag (`--base`) and an inline
review prompt cannot both be passed in a single invocation. The
refactor opts for **diff-source mode**: codex receives the precise
diff scope via `--base <upstream>` and produces its default review
report against the diff. The acceptance-bullet checklist that would
otherwise be the prompt body lives in the **PR description**
instead (see "Review focus checklist" below) — the sprint author
and the human reviewer cross-reference the codex report against
that checklist when merging.

**Review focus checklist** (added verbatim to the PR description
under `### Codex review focus`, one bullet per area; the human
reviewer uses this to verify codex's report covered each area):

```
- ARCHITECTURAL-REFACTOR.md § R<n> Acceptance bullets (list bullets explicitly)
- CHARTER.md §3 invariants at risk in this sprint
- CHARTER.md §5 hard limits this sprint approaches
- CI-GATES.md gates this sprint flips from planned → active
- Sprint NNNN's Deliverables and Definition of done sections
- REFACTOR-STATUS.md § Stubs outstanding — any stub this sprint
  introduces is registered; any stub this sprint retires is struck
```

The checklist is **mechanical** — every item is either covered by
codex's report (annotate "✓ codex addressed: <one-line citation>")
or surfaced as a gap and addressed in a follow-up commit
(`refactor(scope): address codex review — <summary>`).

**Why these flags**:

- `model = "gpt-5.5"` — locked across the whole refactor so review
  consistency is preserved across sprints. A model swap mid-refactor
  changes the review character and breaks the cross-sprint baseline;
  any change is a charter-amendment-grade decision recorded on `main`
  before the next sprint opens.
- `model_reasoning_effort = "medium"` — review work is read-and-flag
  against explicit acceptance bullets, not generative design. The
  default `xhigh` is wasted overhead at this scope; `medium` is the
  sweet spot for cross-doc consistency checks. If a specific sprint
  surfaces a deep cross-R-move interaction that medium misses
  (candidate: Phase B integration review in sprint 0004), the sprint
  may override to `high` for that single invocation and record the
  override in the PR body.
- The `gpt-5.5-medium` model **variant** is **not** used: the
  ChatGPT-account-backed Codex CLI rejects it (`invalid_request_error:
  not supported when using Codex with a ChatGPT account`). Plain
  `gpt-5.5` is the canonical choice for this account configuration.
  Note the distinction: `model = "gpt-5.5"` (plain) combined with
  `model_reasoning_effort = "medium"` — not the rejected
  `model = "gpt-5.5-medium"` variant.

**Outcome handling**:

- The review report is attached to the PR body (under a `### Codex
  review` heading) so reviewers see what Codex flagged and how it was
  addressed.
- Findings are categorised by the sprint author:
  - **Blocker** — sprint cannot close until addressed. Follow-up
    commit lands on the sprint branch (`refactor(scope): address
    codex review — <summary>` or `fix(scope): …`).
  - **Non-blocker** — recorded in the PR body with rationale for
    deferral; tracked in `POST-REFACTOR-PLAN.md` if it survives the
    refactor's horizon.
  - **Rejected** — Codex misread the contract; record the
    counter-argument in the PR body so the rationale is durable.
- **Codex is not authority**. If the review surfaces a rule
  ambiguity, the §3 ambiguity protocol takes over — the rule is
  resolved by amending the source-of-truth document, not by what
  Codex said.

#### Role 2 — Implementation-doubt consultation

During implementation, if a concrete coding question arises (idiom
choice, Rust pattern, SQL constraint shape, test-harness wiring) the
sprint author may consult Codex via `codex exec "<question>"` as a
second opinion.

Boundaries:

- **Consultation, not delegation.** Codex's answer is treated like a
  Stack Overflow reply: useful input, not a contract.
- **No rule decisions.** If the question is "what should the rule
  be?", that is an ambiguity (§3) and goes to the human and the
  source doc, never to Codex.
- **No silent application.** A Codex suggestion that materially
  shapes the implementation is recorded in the PR body so reviewers
  can see the input that fed the design.
- **Sandboxed by default.** `codex exec` runs in the Codex sandbox
  (`workspace-write`), not the maintainer's full shell — it can read
  the workspace and write to `/tmp`, not execute arbitrary commands
  on the host.

#### What Codex is not used for

- **Charter or playbook amendments.** Rule changes go through the
  amendment channels each governing document defines.
- **CI gate decisions.** A gate's status (`planned` / `active` /
  `disabled`) is decided by the R-move owning it and recorded in
  `CI-GATES.md` — not by Codex's opinion about whether the gate is
  worth it.
- **Verdicts in `LANGUAGE-DECISIONS.md` or `FRAMEWORK-DECISIONS.md`.**
  Adoption verdicts are the maintainer's via the playbook flow.
- **REFACTOR-STATUS.md transitions.** Status flips reflect demonstrated
  acceptance, not Codex's assessment.

#### Why Codex at all

Two reasons recorded so the protocol is not arbitrary:

1. **Cross-model review surface area.** A second model trained on a
   different corpus surfaces blindspots that a single-model loop
   misses. The cost is `codex review` runtime; the benefit is
   catching contract drift before it reaches `main`.
2. **Implementation-detail bandwidth.** When the question is
   purely "how do I write this Rust idiom", consulting Codex offloads
   work that does not require human judgement, leaving the human's
   attention for §3 ambiguities — where it actually matters.

---

## Glossary anchors used across sprints

For quick reference. The authoritative definitions live in
[`GLOSSARY.md`](../GLOSSARY.md).

- [Class 1 / mechanical · Class 2 / detectable · Class 3 / discipline-only](../GLOSSARY.md#architecture)
- [R-move · Phase · Hard limit · Soft expansion zone](../GLOSSARY.md#architecture)
- [`RawCaptures` · `RawEdge` · `InsertableEdge` · `EdgeBuilder` · `Edge`](../GLOSSARY.md#refactor-types)
- [`LanguageWorkspaceContext` · `FrameworkWorkspaceContext`](../GLOSSARY.md#workspace-context)
- [`DetectedVersion` · `ResolvedVersion` · `UnknownVersionPolicy` · `VersionReq` · `available_in`](../GLOSSARY.md#versioning)
- [`Confidence` · `status` · orthogonality · cleanest-signal filter](../GLOSSARY.md#confidence-and-status-orthogonal)
- [`StatusData` · `file_hashes.skipped_ranges` · Surrogate PK · `edges.args_text` · Schema bumps](../GLOSSARY.md#schema)
- [`LanguagePlugin` · `FrameworkPlugin` · `Extractor` · `LanguageId` · `EdgeKind` · reserved metadata keys · 4-kind concurrency split](../GLOSSARY.md#plugin-shapes)

---

## What this document is not

- Not a substitute for `ARCHITECTURAL-REFACTOR.md`. The R-moves' acceptance
  criteria live there; sprints reference them, they do not redefine them.
- Not a calendar. Sprints have order; they do not have dates. The work is
  bounded but not time-boxed
  ([`ARCHITECTURAL-REFACTOR.md` § What this document does not contain](../ARCHITECTURAL-REFACTOR.md#what-this-document-does-not-contain)).
- Not a feature backlog. Features are queued in
  [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) and unblock only
  after Phase E acceptance.
- Not a place to amend charter or playbook rules. Amendments go through
  the explicit-commit channels each governing document defines.
