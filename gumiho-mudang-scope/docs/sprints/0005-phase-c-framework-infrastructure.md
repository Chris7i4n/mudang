# Sprint 0005 — Phase C: Framework infrastructure

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R5](../ARCHITECTURAL-REFACTOR.md#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata).
> **Phase**: C. Single-sprint phase.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Land the `FrameworkPlugin` trait, the graph-only matching model
(predicates over `&[Symbol]` + `&[Edge]`, never AST), the version
detection types (`DetectedVersion` / `ResolvedVersion` /
`UnknownVersionPolicy`), the pattern catalog organization, and the
indexer-level cross-language pre-filter.

This sprint introduces the infrastructure but does **not** adopt any
specific framework. Per `ARCHITECTURAL-REFACTOR.md` § Phase C:
"Lands when framework infrastructure is first introduced. Until then,
R5 is a design constraint, not a code change." Concrete frameworks
(Flask, Rails, Tokio, Express, Axum, NestJS, …) are adopted **after**
the refactor closes, following
[`FRAMEWORK-PLAYBOOK.md`](../FRAMEWORK-PLAYBOOK.md) — i.e., in the
[`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) queue.

## R-moves shipped this sprint

- **R5 — FrameworkPlugin operates on Symbols and Edges, not AST
  (graph-only via metadata)** ([§ R5](../ARCHITECTURAL-REFACTOR.md#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata))

## Prerequisites

- Phase B `shipped`: sprints 0002, 0003, 0004 must all be `shipped` in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md).
  - R2's `Symbol.metadata` population (`decorators`, `annotations`,
    `template_calls`) is the input substrate of R5 predicates.
  - R3's resolution pipeline is what R5 predicates feed `EdgeBuilder`
    outputs into.
  - R4's `FrameworkWorkspaceContext` is what R5's `detect()` consumes.
  - R12's trait-shape audit must already exist so R5's `FrameworkPlugin`
    trait can be added to its audit list at land time.

## Charter alignment

- **Mission** ([`CHARTER.md` §1](../CHARTER.md#1-mission)):
  framework-awareness is the strongest moat vs LSP — fourth-question
  priority in [`CHARTER.md` §4](../CHARTER.md#4-the-3-question-test).
- **Soft expansion zone** ([`CHARTER.md` §6](../CHARTER.md#6-soft-expansion-zone--scope-expands-freely-here)):
  domain edges (`http_route`, `queue_handler`, `orm_relation`, etc.)
  emit only when a framework predicate matches. The schema for them
  landed in R0; the matching machinery lands here.
- **Universal language boundaries**
  ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  R5 closes **E2** mechanically. Framework plugins consume parsed
  `Symbol.metadata` and `Edge` rows; they never see raw AST.
- **Asymmetry with language version**
  ([`CHARTER.md` §7 Multi-version posture](../CHARTER.md#7-per-language-scope-and-non-scope)):
  framework predicates branch on framework version; language plugins
  never branch on language version (C2). The R4 split (sprint 0002)
  is the mechanical safeguard that keeps these two layers separate.
- **Framework adoption procedure**: this sprint lands infrastructure
  only. Per-framework adoption follows
  [`FRAMEWORK-PLAYBOOK.md`](../FRAMEWORK-PLAYBOOK.md) — Step 1
  (adoption trigger), Step 2 (ROI worksheet), Step 3 (version
  strategy), Step 4 (15-category walkthrough), Step 5 (implementation
  order). No framework is added in this sprint.

## Deliverables

### R5 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata))

- [ ] `FrameworkPlugin` trait inspection shows **no** `tree_sitter::*`
      types and **no** `&Path` / `&str`-source parameters.
- [ ] `Detection.applies_to_languages: Vec<LanguageId>` is a
      required field; an empty list with `detected: true` is rejected
      by CI gate. See [`GLOSSARY.md` · `Detection`](../GLOSSARY.md#refactor-types)
      and [`GLOSSARY.md` · `LanguageId`](../GLOSSARY.md#plugin-shapes).
- [ ] `unknown_version_policy()` is a required method on
      `FrameworkPlugin`.
- [ ] **No `queries/<lang>/frameworks/`** directory exists; CI grep
      gate (`just ci-no-framework-scm`) enforces.
- [ ] Indexer applies the cross-language pre-filter before invoking
      `match_edges`:
      `symbols.iter().filter(|s| detection.applies_to_languages.contains(&s.language))`
      and the same for edges joined through their endpoints.
- [ ] `match_edges` returns `Vec<EdgeBuilder>`; the resolver (R3,
      sprint 0003) finishes the conversion to `InsertableEdge`. The
      framework plugin never inserts directly.
- [ ] **Version detection types** land per the R5 target-state code
      block:
      - `DetectedVersion::Resolved(semver::Version)` — lockfile-equivalent
        with version-coercion rule recorded in the per-framework doc.
      - `DetectedVersion::Indeterminate` — range-only manifest, beta
        tag, unparseable manifest. Routed to `unknown_version_policy()`.
      - `DetectedVersion::NoVersionConcept` — framework lacks versioned
        releases.
      - `ResolvedVersion::Detected(v)` / `Fallback` / `Assumed(v)` /
        `Versionless` — what `match_edges` actually receives.
      - `UnknownVersionPolicy::Skip` / `StableOnlyLowConfidence` /
        `AssumeLatest(v)`.
- [ ] **Pattern catalog shape** per the R5 target-state code block:
      `Pattern { id, edge_kind, available_in: semver::VersionReq,
      predicate: fn(&[Symbol], &[Edge]) -> Vec<EdgeBuilder> }`.
- [ ] **Pattern audit** (`scripts/audit_patterns.sh`) runs and flags:
      empty `id`, missing `available_in`, unreferenced predicate fn.
      CI gate flips `planned` → `active`.
- [ ] **Version coercion layer** documented (recorded per-framework
      when a framework lands). The layer is part of the infrastructure;
      its **rules** are framework-specific and go in
      `docs/frameworks/<name>.md`. None are added in this sprint
      because no framework is adopted in this sprint.

### Integration test deliverables ([source](../ARCHITECTURAL-REFACTOR.md#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata))

The acceptance bullets list integration tests; with no framework yet
adopted, these tests run against a **synthetic test framework** that
exercises the infrastructure without committing scope to maintain any
real-world framework:

- [ ] Synthetic framework with language metadata correctly populated →
      emits expected domain edges. Removing the metadata → zero edges
      (graph-only contract).
- [ ] Synthetic framework with mixed-language fixture verifies the
      cross-language pre-filter: a Python decorator name shared with a
      Ruby callback does not match against the Ruby-only synthetic
      framework's predicate.
- [ ] Synthetic framework with version pinned outside its declared
      `available_in` set → zero edges.
- [ ] Synthetic framework with no resolvable version → behavior
      matches declared `unknown_version_policy()`:
      `Skip` → zero edges; `StableOnlyLowConfidence` → only fallback
      patterns with `confidence=low`; `AssumeLatest(v)` → latest-version
      pattern set with `producer='framework:<name>:assumed_<v>'`.
- [ ] Cross-crate fixture: a workspace where crate A produces a
      "queue message" and crate B consumes it produces a cross-crate
      `queue_handler` edge under the synthetic framework's predicate
      (validates cross-app graph queries inside one workspace).

---

## Ambiguities to clarify before code lands

Each ambiguity below is resolved by an amendment to the cited
source-of-truth document on `main` **before** this sprint's branch
opens.

1. **Synthetic test framework location and shape.** The R5 integration
   tests need a framework to exercise; no real framework is being
   adopted here. Whether the synthetic framework lives at
   `tests/synthetic_framework/`, `scope-core/src/frameworks/_test/`
   (post-sprint-0000 sub-crate path), or behind a `#[cfg(test)]` module
   is unspecified. Resolution amends `ARCHITECTURAL-REFACTOR.md` R5
   "Target state".

2. **Cross-language fixture maintenance burden.** The cross-language
   pre-filter integration test requires a fixture with two languages
   sharing a decorator-like name. Whether this fixture lives under
   `tests/fixtures/frameworks/_pre_filter/` or alongside the per-language
   fixture trees is unspecified. Resolution amends
   `ARCHITECTURAL-REFACTOR.md` R5 "Acceptance".

3. **`FrameworkWorkspaceContext` visibility flip — mandatory.** Sprint
   0002 landed `FrameworkWorkspaceContext` as `pub(crate)` in
   `scope-core` per the resolution recorded in
   `ARCHITECTURAL-REFACTOR.md` § R4 "Target state". **This sprint
   widens it to `pub` in the same commit that lands the first
   `FrameworkPlugin` impl** — not a later commit, not a separate PR.
   The flip is mechanical (one keyword) and unconditional. It is not a
   stub and is not tracked in `REFACTOR-STATUS.md` § Stubs outstanding;
   it is legitimate visibility staging. Sprint-close checklist verifies
   the keyword changed and that no Phase B sprint accidentally widened
   it earlier (check by `grep -n 'pub trait FrameworkWorkspaceContext'
   scope-core/src/` — must appear only in the R5 first-impl commit).

4. **`LanguageId` and `.js`/`.jsx`.** R5 says
   `applies_to_languages: Vec<LanguageId>`. Today
   `LanguageId` does not include JavaScript
   ([`FRAMEWORK-PLAYBOOK.md` Step 4 → Language scope](../FRAMEWORK-PLAYBOOK.md#language-scope)).
   **Cheap-path** (extend `TypeScriptPlugin::extensions()` to include
   `js|jsx`) and **strict-path** (new `JavaScript` variant) are
   post-refactor decisions per
   [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md). If R5 acceptance
   demands JS coverage, resolution amends R5 "Acceptance" to declare
   JS-shaped tests deferred to post-refactor.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **No `.scm` per framework** (`just ci-no-framework-scm`) —
      `planned` → `active`.
- [ ] **Pattern catalog audit** (`just ci-patterns`) — `planned` →
      `active`.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`FrameworkPlugin`, `LanguageId`](../GLOSSARY.md#plugin-shapes)
- [`Pattern`, `Detection`](../GLOSSARY.md#refactor-types)
- [`DetectedVersion`, `ResolvedVersion`, `UnknownVersionPolicy`,
  `VersionReq`, `available_in`, Version coercion](../GLOSSARY.md#versioning)
- [`FrameworkWorkspaceContext`](../GLOSSARY.md#workspace-context)

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0005-framework-infrastructure`, cut from
  `main` after Phase B's integration branch merged.
- **Base**: `main` directly — Phase C has a single R-move sprint, so
  no phase integration branch is needed (per
  [`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-atomic-phase-shipment)).
- **Open**: flip R5 row in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append log entry noting branch name.
- **Codex review**: before the `REFACTOR-STATUS.md` transition commit,
  run the canonical command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base main`
  - `--title "sprint 0005 — R5"`
  - Prompt focus: R5 acceptance bullets, E2 mechanical enforcement
    (graph-only via metadata), DetectedVersion / ResolvedVersion /
    UnknownVersionPolicy correctness, cross-language pre-filter, CI
    gates this sprint activates (No `.scm` per framework, Pattern
    catalog audit).
  Attach report to PR body; address blockers.
- **Close**: flip R5 to `shipped`. **In the same commit**, flip the
  **Phase C** row in the phase snapshot table to `shipped` (this is
  Phase C's only sprint).
- **Merge**: squash-merge or rebase-merge to `main`.

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. Four ambiguities above are resolved before code lands.
3. The two CI gates listed above are `active` in `CI-GATES.md` and CI.
4. `REFACTOR-STATUS.md` shows R5 `shipped` and Phase C `shipped`.
5. `FRAMEWORK-PLAYBOOK.md` Step 5 (implementation order within a
   framework) is followable end-to-end against the new infrastructure —
   without yet adopting any real framework.
6. The trait-shape audit (R12, sprint 0004) is extended to cover
   `FrameworkPlugin` so that future framework adoptions do not regress
   the negative trait shape.

## Out of scope for this sprint

- Adopting any specific framework (Flask, Rails, Tokio, Express, Axum,
  NestJS, Prisma, TypeORM, Celery, gin, echo, etc.). Adoption follows
  [`FRAMEWORK-PLAYBOOK.md`](../FRAMEWORK-PLAYBOOK.md) and waits for
  [`POST-REFACTOR-PLAN.md`](../POST-REFACTOR-PLAN.md) — i.e., after
  Phase E acceptance.
- Per-sub-root version detection (npm/Python multi-package monorepos).
  Trigger-deferred per [`FRAMEWORK-PLAYBOOK.md` Step 3 →
  Workspace-uniform-version assumption](../FRAMEWORK-PLAYBOOK.md#cross-workspace-queries-cross-app-single-workspace).
- `.js`/`.jsx` indexing path decisions
  ([`POST-REFACTOR-PLAN.md` § Items deliberately deferred](../POST-REFACTOR-PLAN.md#items-deliberately-deferred-beyond-this-plan)).
- Output schema and confidence audit — sprint 0006 (R10, R8).
- Malformed-source harness — sprint 0007 (R6).
