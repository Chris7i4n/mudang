# Sprint 0006 — Phase D: Output schema and confidence audit

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R10](../ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema) and [§ R8](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand).
> **Phase**: D. Atomic — both R-moves ship together or neither ships.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Close the output side and the detection layer for D2 / E1:

- **R10** locks output schemas as typed structs with no diagnostic
  fields, mechanically enforcing **E1** (no semantic correctness
  assertions) at the output boundary.
- **R8** lands the `scope audit confidence` subcommand — a
  **precision** report per `(kind, tier, producer, pattern_id)`.
  The R8 audit is the symptom-side safety net for the detection-class
  rules (A1–A3, B2 in the inventory) that the trait-shape audit
  ([sprint 0004, R12](./0004-phase-b-trait-closure-and-audits.md))
  cannot catch when a determined plugin author uses correctly-named
  helpers or runtime-resolved spawns.

## R-moves shipped this sprint

- **R10 — Typed output schema** ([§ R10](../ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema))
- **R8 — Confidence audit subcommand** ([§ R8](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand))

## Prerequisites

- Phase C `shipped`: sprint 0005 must close before R8 because R8
  samples precision **per `producer`** and **per `pattern_id`**, and
  framework predicates contribute their own rows.
- Phase B `shipped`: R8's tier targets assume the post-R3 status column
  and the post-R0 `producer` / `pattern_id` columns.
- Phase A `shipped`: R10's typed-struct conversion replaces the legacy
  string-concatenation formatters; R0's schema columns are the input.

## Charter alignment

- **Hard limits** ([`CHARTER.md` §5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)):
  "No type / borrow / lint diagnostics" — R10 is the mechanical closure
  via output-struct shape ("no field named `error`, `warning`,
  `diagnostic`, `is_valid`, etc.").
- **Universal language boundaries**
  ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  - **E1** (no semantic correctness assertions) — mechanical after R10.
  - **A1, A2, A3, B2** (detectable per the inventory) — R8 is the
    symptom-side detection that catches what the trait-shape audit
    cannot
    ([`ARCHITECTURAL-REFACTOR.md` § Why detectable, not mechanical](../ARCHITECTURAL-REFACTOR.md#why-detectable-not-mechanical-for-trait-shape-rules)).
- **Honest framing** ([`ARCHITECTURAL-REFACTOR.md` § R8 → What R8
  measures and what it does not](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand)):
  R8 measures **precision only**. Recall regressions are caught by
  integration-test snapshots and per-framework doc walkthroughs, not
  by this subcommand. The subcommand's help text and report header
  must state this verbatim.

## Deliverables

### R10 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema))

- [ ] Output schemas are typed structs (`SymbolSketch`, `EdgeSummary`,
      `CompactView`, and any others currently driven by raw string
      concatenation). Formatters serialize structs; they do not
      concatenate strings.
- [ ] Output-schema audit (`scripts/audit_output_schema.sh`) catches
      fields named `error`, `warning`, `diagnostic`, `is_valid`,
      `lint`, `correctness`.
- [ ] Existing output formats (`sketch`, `summary`, `compact`, `json`)
      preserve their token budgets — the typed shape does not balloon
      output.

### R8 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand))

- [ ] `scope audit confidence` subcommand exists, runs against the
      reference fixture corpus, and produces a parseable precision
      report per `(kind, tier, producer, pattern_id)`.
- [ ] Tier targets enforced: `high ≥ 95%`, `medium ≥ 70%`, `low` has
      no minimum. Offenders are identifiable to specific plugins and
      patterns via `(producer, pattern_id)`.
- [ ] Help text and report header **both** state:
      *"precision report; recall is measured by integration-test
      snapshots, not this subcommand."*
- [ ] The reference fixture corpus is committed and version-controlled
      under `tests/fixtures/reference/` (or wherever the human selects
      — see ambiguity below).
- [ ] Sampling protocol documented: N edges per `(kind, confidence)`
      combination; sampling seeded for reproducibility; the sample is
      manually labelled correct/incorrect (or LLM-labelled with
      explicit prompt).

---

## Ambiguities to clarify before code lands

Each ambiguity below is resolved by an amendment to the cited
source-of-truth document on `main` **before** this sprint's branch
opens. Resolutions amend `ARCHITECTURAL-REFACTOR.md` R8 or R10
"Target state" / "Acceptance" as relevant.

1. **Reference fixture corpus location and provenance.** ✅ Resolved
   on main via [`ARCHITECTURAL-REFACTOR.md` § R8 → Operational shape
   → Reference fixture corpus](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand).
   Corpus lives at `gumiho-mudang-scope/scope-core/tests/fixtures/reference/<language_slug>/`,
   one subtree per supported `LanguageId`. Provenance: real anonymized
   snippets from the maintainer's projects (per `LANGUAGE-PLAYBOOK.md`
   Step 5). Anonymization rules — strip secrets, replace proprietary
   identifiers with shape-preserving placeholders — recorded in
   `tests/fixtures/reference/README.md` (authored alongside the first
   fixture).
2. **Sampling size N.** ✅ Resolved on main via [§ R8 → Operational
   shape → Sampling protocol](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand).
   Default `N = 30` per `(kind, confidence)`; `--sample-size N`
   override; `--seed N` for reproducibility (default fixed
   compile-time constant). 30 chosen as binomial-proportion lower
   bound for early tier-drift signal.
3. **Labelling channel.** ✅ Resolved on main via [§ R8 →
   Operational shape → Labelling channel](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand).
   Manual labelling is the default and only channel shipped in R8.
   Two-phase workflow: `--emit-sample <path>` writes unlabelled
   sample, maintainer fills `label` slot, `--label <path>` reads
   labelled file and produces precision report. LLM-assisted
   labelling deferred post-refactor per `POST-REFACTOR-PLAN.md` §
   Items deliberately deferred.
4. **Output format for the precision report.** ✅ Resolved on main
   via [§ R8 → Operational shape → Output format](../ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand).
   Default `--format json` with top-level `schema_version: "1"` field
   plus a `report` array of `(kind, tier, producer, pattern_id,
   sample_size, correct_count, precision)` rows. `--format tsv`
   available for shell pipelines (same columns). JSON shape is the
   contract.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **Output schema audit** (`just ci-output-schema`) — `planned`
      → `active`.
- [ ] **Confidence audit** (`just audit-confidence`) — `planned`
      → `active`. Per `CI-GATES.md`, this fails the build when
      precision is below the tier target.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`Confidence`, `status`, orthogonality, cleanest-signal filter](../GLOSSARY.md#confidence-and-status-orthogonal)
- [`Producer`, `pattern_id`](../GLOSSARY.md#refactor-types)
- [`scope audit confidence`, `scope audit coverage` (planned)](../GLOSSARY.md#subcommands)
- [Gate, Gate status](../GLOSSARY.md#ci-gates)

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0006-output-audit`, cut from `main`
  after Phase C merged.
- **Base**: `main` directly — Phase D ships R10 + R8 atomically inside
  this single sprint, so no phase integration branch is needed.
- **Open**: flip R10 and R8 rows in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append log entries noting branch name.
- **Codex review**: before the `REFACTOR-STATUS.md` transition commit,
  run the canonical command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base main`
  - `--title "sprint 0006 — R10+R8"`
  - Prompt focus: R10 and R8 acceptance bullets, E1 mechanical
    enforcement (no diagnostic-shaped output fields), R8's
    precision-only framing (recall caught elsewhere), CI gates this
    sprint activates (Output schema audit, Confidence audit).
  Attach report to PR body; address blockers.
- **Close**: flip R10 and R8 to `shipped`. **In the same commit**,
  flip the **Phase D** row in the phase snapshot table to `shipped`.
- **Merge**: squash-merge or rebase-merge to `main`.

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. Four ambiguities above are resolved before code lands.
3. The two CI gates listed above are `active` in `CI-GATES.md` and CI.
4. `REFACTOR-STATUS.md` shows R10 and R8 `shipped`; Phase D `shipped`.
5. `scope audit confidence --help` exists and prints the precision-only
   disclaimer.
6. The first run of `scope audit confidence` against the reference
   corpus produces a report that the human reviews; any offenders are
   either acknowledged (with confidence downgrade or pattern fix) or
   accepted with rationale recorded.

## Out of scope for this sprint

- `scope audit coverage` subcommand — explicitly post-refactor
  ([`POST-REFACTOR-PLAN.md` § Items deliberately deferred](../POST-REFACTOR-PLAN.md#items-deliberately-deferred-beyond-this-plan)).
- Malformed-source harness — sprint 0007 (R6).
- Per-language depth feature work — post-refactor.
- Performance regression measurement — handled by the refactor-as-a-whole
  acceptance criterion ("< 10% regression from pre-refactor baseline")
  enforced at Phase E close (sprint 0007 § Definition of done).
