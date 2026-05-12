# Sprint 0008 — Phase E: Malformed-source harness

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R6](../ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness).
> **Phase**: E. First of two Phase E sprints. Followed by [Sprint 0009 — Charter sweep and shim retirement](./0009-phase-e-charter-sweep.md), which closes the refactor.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Land the malformed-source test harness — the gate that asserts every
plugin survives broken sources, populates `file_hashes.skipped_ranges`
honestly, and never silently drops a partially-malformed file.

Sprint 0008 lands R6. [Sprint 0009](./0009-phase-e-charter-sweep.md)
follows immediately to sweep compat shims and close the refactor. The
Phase E row in `REFACTOR-STATUS.md` stays `in-progress` until sprint
0009 closes — sprint 0008 flips **only** the R6 row.

The full-refactor acceptance criteria
([`ARCHITECTURAL-REFACTOR.md` § Acceptance for the refactor as a whole](../ARCHITECTURAL-REFACTOR.md#acceptance-for-the-refactor-as-a-whole))
are demonstrated at sprint 0009's close; [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md)
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

- **Branch**: `refactor/sprint-0008-malformed-harness`, cut from `main`
  after Phase D merged.
- **Base**: `main` directly — Phase E has one R-move sprint (this one)
  plus an acceptance-only sprint (0009), so no phase integration branch
  is needed.
- **Open**: flip R6 row in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append log entry noting branch name.
- **Codex review (sprint scope)**: before the `REFACTOR-STATUS.md`
  transition commit, run the canonical command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base main`
  - `--title "sprint 0008 — R6"`
  - Prompt focus: R6 acceptance bullets, charter §3 invariant 5
    (tree-sitter resilience), B3 detection, malformed-fixture set per
    language, malformed-source CI gate.
  The full-refactor-scope codex review fires at sprint 0009 close per
  [sprint 0009 § Reporting](./0009-phase-e-charter-sweep.md#reporting),
  not here — sprint 0008 ships R6 only; the refactor is not yet closed.
- **Close**: flip R6 to `shipped`. **Do NOT** flip the Phase E row —
  sprint 0009 (charter sweep) closes Phase E + the refactor as a whole.
  Append log entries per `README.md § 4` for the R6 transition.
- **Merge**: squash-merge or rebase-merge to `main`. After merge, sprint
  0009 opens immediately to sweep compat shims and close the refactor.
  `POST-REFACTOR-PLAN.md` eligibility unlocks at sprint 0009's close, not
  this one.

---

## Definition of done

Sprint 0008 ships R6 only; Phase E + refactor close at sprint 0009. All
of the following hold simultaneously:

1. Every checkbox in **Deliverables** above is checked.
2. Three ambiguities above are resolved before code lands.
3. The malformed-source CI gate is `active` in `CI-GATES.md` and CI.
4. Snapshot tests pin the recorded reason and range per fixture (regression
   surfaces as snapshot diff) — covered by Deliverables § R6 acceptance
   but called out here as the mechanical lock for B3 detection.
5. R6 row in [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) flips
   `in-progress → shipped`; Phase E row stays `in-progress` (closes at
   sprint 0009).
6. Sprint-scope codex review surfaces no P0 / P1 findings.

Full-refactor acceptance criteria
([`ARCHITECTURAL-REFACTOR.md` § Acceptance for the refactor as a whole](../ARCHITECTURAL-REFACTOR.md#acceptance-for-the-refactor-as-a-whole))
live in [sprint 0009 § Acceptance](./0009-phase-e-charter-sweep.md#acceptance)
and gate that sprint's close — not this one.

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
