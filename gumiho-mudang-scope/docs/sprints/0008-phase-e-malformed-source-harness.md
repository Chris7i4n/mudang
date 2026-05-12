# Sprint 0007 — Phase E: Malformed-source harness

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R6](../ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness).
> **Phase**: E. First of two Phase E sprints. Followed by [Sprint 0008 — Charter sweep and shim retirement](./0008-phase-e-charter-sweep.md), which closes the refactor.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Close the refactor by landing the malformed-source test harness — the
gate that asserts every plugin survives broken sources, populates
`file_hashes.skipped_ranges` honestly, and never silently drops a
partially-malformed file.

Phase E ships **last** so that every plugin is already in its final
shape before the gate activates. Sprint 0007 lands the R6 malformed-
source harness; [Sprint 0008](./0008-phase-e-charter-sweep.md) then
sweeps the codebase for compat shims and closes the refactor. The
full-refactor acceptance criteria
([`ARCHITECTURAL-REFACTOR.md` § Acceptance for the refactor as a whole](../ARCHITECTURAL-REFACTOR.md#acceptance-for-the-refactor-as-a-whole))
must hold at sprint 0008's close; [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md)
becomes eligible immediately afterwards.

## R-moves shipped this sprint

- **R6 — Malformed-source test harness** ([§ R6](../ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness))

## Prerequisites

- Phases A–D `shipped` (all of sprints 0001–0006 closed).
- `file_hashes.skipped_ranges` schema landed (R0, sprint 0001).
- Plugin-driven `skipped_ranges` recording landed (R2, sprint 0003).
- Indexer-level tree-sitter-error recording (the parser side of R6's
  indexer behaviour) is already wired — confirm before this sprint
  starts; if not, the wiring lands here.

## Charter alignment

- **Invariants** ([`CHARTER.md` §3](../CHARTER.md#3-core-invariants--must-never-break)):
  invariant 5 — "Tree-sitter resilience. The index updates correctly
  even when source code does not compile. Mid-refactor, broken branches,
  generated code with gaps — all must produce a useful (if incomplete)
  index." R6 is the mechanical gate for this invariant.
- **Universal language boundaries**
  ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  - **B3** (no assumption of valid syntax) — detectable after R6.

## Deliverables

### R6 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness))

- [ ] Indexer behaviour: when tree-sitter recovery encounters an
      `ERROR` node region, the parser records the region's line range
      and reason (`tree_sitter_error`, `unrecoverable_node`,
      `plugin_skip:<plugin>:<rationale>`) into `skipped_ranges`.
- [ ] Plugin-driven skips appended with reason
      `plugin_skip:<plugin_name>:<rationale>` (the wiring landed in
      sprint 0003; this sprint verifies it).
- [ ] **Fixture set** of deliberately broken sources per language —
      minimum **5 fixtures per supported language**:
      `rust`, `python`, `go`, `typescript`, `java`, `csharp`, `ruby`.
      Categories per fixture set:
      - Truncated mid-function.
      - Unbalanced braces.
      - EOF inside a string / unterminated raw string.
      - Mixed indentation collapse (Python especially).
      - Missing closing tag in JSX (TS/TSX).
      - Per-language additions per
        [§ R6 → Fixture set](../ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness).
- [ ] **Integration test** (CI gate `cargo test --test malformed_sources`):
      - No panic on any fixture.
      - The parseable prefix produces at least one symbol.
      - `file_hashes.skipped_ranges` is **non-empty** when the file is
        partially malformed (silent skip is no longer acceptable).
      - The skipped range covers the lines that the human-readable
        expectation says should be skipped (snapshot test via
        [`insta`](https://crates.io/crates/insta)).
- [ ] Snapshot tests pin the recorded reason and range per fixture so
      future regressions surface as snapshot diffs.
- [ ] **Out of scope** (recorded explicitly per § R6):
      invalid UTF-8 at the byte level. Today the read returns an error;
      R6 does not change this. Byte-level lossy reading is
      trigger-deferred to a separate refactor
      ([`POST-REFACTOR-PLAN.md` § Items deliberately deferred](../POST-REFACTOR-PLAN.md#items-deliberately-deferred-beyond-this-plan)).

---

## Ambiguities to clarify before code lands

Each ambiguity below is resolved by an amendment to
`ARCHITECTURAL-REFACTOR.md` R6 on `main` **before** this sprint's
branch opens.

1. **Fixture provenance.** Same question as sprint 0006's reference
   corpus: anonymized real-project malformed snippets vs hand-crafted
   synthetic. R6 implies hand-crafted ("deliberately broken sources").
   Confirm placement: `tests/fixtures/malformed/<language>/<case>/`.
2. **Plugins that have no concept of a given malformation category.**
   Example: Python has no "missing closing tag in JSX" case. Whether
   each category is mandatory per language or skipped when N/A is
   unspecified. Confirm — likely "per-language minimum 5 fixtures,
   categories chosen per language's grammar".
3. **Snapshot tool.** R6 mentions `insta` indirectly; confirm `insta`
   is the chosen tool and the snapshot output path discipline.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **Malformed-source harness** (`just test-malformed`) — `planned`
      → `active`.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`SkippedRange`, `file_hashes.skipped_ranges`](../GLOSSARY.md#schema)
- [Gate, Gate status](../GLOSSARY.md#ci-gates)

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0007-malformed-harness`, cut from `main`
  after Phase D merged.
- **Base**: `main` directly — Phase E has one R-move sprint, so no
  phase integration branch is needed.
- **Open**: flip R6 row in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append log entry noting branch name.
- **Codex review (sprint scope)**: before the `REFACTOR-STATUS.md`
  transition commit, run the canonical command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base main`
  - `--title "sprint 0007 — R6"`
  - Prompt focus: R6 acceptance bullets, charter §3 invariant 5
    (tree-sitter resilience), B3 detection, malformed-fixture set per
    language, malformed-source CI gate.
- **Codex review (full refactor scope)**: after the sprint-scope review
  and before opening the PR, run a second canonical pass with:
  - `--base <pre-refactor-baseline>` (the commit immediately preceding
    sprint 0000's first commit; recorded in `REFACTOR-STATUS.md` log)
  - `--title "Refactor close"`
  - **`-c model_reasoning_effort="high"`** override (full-refactor
    review crosses every R-move; the explicit medium→high override
    authorised in
    [`README.md` § 9 — Why these flags](./README.md#role-1--mandatory-sprint-review-checkpoint);
    record override in the PR body).
  - Prompt focus: whole-refactor acceptance set in
    `ARCHITECTURAL-REFACTOR.md § Acceptance for the refactor as a whole`.
  Both reports attach to the PR body; blockers gate the sprint close.
- **Close**: flip R6 to `shipped`. **In the same commit**, flip the
  **Phase E** row in the phase snapshot table to `shipped`. **Also in
  the same commit**, add a final log entry recording that the refactor
  as a whole is `shipped` (see Definition of done below).
- **Merge**: squash-merge or rebase-merge to `main`. After merge, the
  `POST-REFACTOR-PLAN.md` queue becomes eligible — but a new branch
  for any post-refactor item follows its own naming (not `refactor/…`).

---

## Definition of done

This sprint is unique: closing it closes the **entire refactor**. All
of the following hold simultaneously, mirroring
[`ARCHITECTURAL-REFACTOR.md` § Acceptance for the refactor as a whole](../ARCHITECTURAL-REFACTOR.md#acceptance-for-the-refactor-as-a-whole):

1. Every checkbox in **Deliverables** above is checked.
2. Three ambiguities above are resolved before code lands.
3. The malformed-source CI gate is `active` in `CI-GATES.md` and CI.
4. `REFACTOR-STATUS.md` shows **every** R-move (R0–R12) and **every**
   phase (A–E) as `shipped`.
5. Every universal rule in the inventory tables
   ([`CHARTER.md` §5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)
   hard limits and
   [`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries))
   is in class 1, class 2, or the **explicit class-3 list of three**
   (B1, C2, E3). No other rule is delegated to discipline.
6. Every active language plugin's `docs/languages/<name>.md` has
   **zero** `NEEDS REVIEW` entries.
7. Every active framework plugin's `docs/frameworks/<name>.md` — none
   adopted at refactor close — has, when adopted, an explicit decision
   in every row of the 15-category walkthrough
   ([`FRAMEWORK-PLAYBOOK.md` Step 4](../FRAMEWORK-PLAYBOOK.md#step-4--gotcha-catalogue)).
   Framework adoption is post-refactor work; this gate is forward-looking.
8. Full benchmark suite shows **< 10% regression** from pre-refactor
   baseline. The baseline is the commit immediately preceding sprint
   0001's first commit; the post-refactor measurement is taken on the
   commit that closes sprint 0007.
9. `scope audit confidence` runs against the reference fixture corpus
   and produces a parseable precision report per
   `(kind, tier, producer, pattern_id)`.
10. CI pipeline includes the malformed-source gate (R6), the
    typed-trait audit (R12), and the immutable-source check (R9).

When all of the above hold, the
[`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) queue becomes
eligible. The first post-refactor sprint is **not** part of this
document — it is planned separately, against the closed architecture.

## Out of scope for this sprint

- Anything in [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md):
  per-language depth queue, framework rollout, vector embeddings,
  time-travel queries, `scope link`, `.js`/`.jsx` indexing, self-indexing,
  per-sub-root version detection, byte-level lossy reading, module
  isolation, `scope audit coverage`, optional symbol-kind renames.
- Adopting any specific framework — covered by
  [`FRAMEWORK-PLAYBOOK.md`](../FRAMEWORK-PLAYBOOK.md) and triggered by
  post-refactor work.
- Per-language depth promotion (Ruby / Java / C# surface → depth) —
  requires triggers per
  [`LANGUAGE-PLAYBOOK.md` Step 7](../LANGUAGE-PLAYBOOK.md#step-7--maintenance-triggers).
