# Sprint 0004 — Phase B: Trait closure and audit gates

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R9](../ARCHITECTURAL-REFACTOR.md#r9--immutable-source-guarantee), [§ R11](../ARCHITECTURAL-REFACTOR.md#r11--macro-definition-only-by-trait-shape), [§ R12](../ARCHITECTURAL-REFACTOR.md#r12--type-system-free-trait-audit--process-spawn-denylist).
> **Phase**: B (third and final sprint of three). Phase B closes when
> this sprint is merged into `refactor/phase-b`, the phase-close commit
> lands, and `refactor/phase-b` merges to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Close Phase B by landing the audit-side safeguards on the plugin layer
that sprints 0002 and 0003 restructured. Three audits:

- **R9** — no `&mut` reaches the plugin layer for source-related types.
- **R11** — macro handling is definition-only; no `expand_*` trait
  method exists.
- **R12** — type-system-free trait audit + process-spawn / network
  denylist on plugin paths.

After this sprint's phase-close path reaches `main`, Phase B is
`shipped`: the language-plugin layer is mechanically closed against
every rule that the refactor's inventory
([`ARCHITECTURAL-REFACTOR.md` § Inventory of constraints](../ARCHITECTURAL-REFACTOR.md#inventory-of-constraints-and-current-enforcement))
labels `mechanical` or `detectable`, with the residual surface
acknowledged in
[§ Why detectable, not mechanical](../ARCHITECTURAL-REFACTOR.md#why-detectable-not-mechanical-for-trait-shape-rules).

## R-moves owned by this sprint

- **R9 — Immutable source guarantee** ([§ R9](../ARCHITECTURAL-REFACTOR.md#r9--immutable-source-guarantee))
- **R11 — Macro definition-only by trait shape** ([§ R11](../ARCHITECTURAL-REFACTOR.md#r11--macro-definition-only-by-trait-shape))
- **R12 — Type-system-free trait audit + process-spawn denylist** ([§ R12](../ARCHITECTURAL-REFACTOR.md#r12--type-system-free-trait-audit--process-spawn-denylist))

## Prerequisites

- Sprint 0003 merged into `refactor/phase-b`: R2 (the post-refactor
  `LanguagePlugin` trait shape) is what R11 and R12 audit. Without R2
  the trait shape is the legacy pre-R2 shape and the audits cannot pass.
  R2/R3 remain `in-progress` until the Phase B integration branch
  merges to `main`.

## Charter alignment

- **Hard limits** ([`CHARTER.md` §5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)):
  - "No compiler/interpreter invocation" — process-spawn denylist (R12)
    is the detection layer.
  - "No network at query time" — network denylist (R12) is the detection
    layer.
  - "No live type inference" / "No generic instantiation tracking" /
    "No trait-bound checking" / "No reflection / dynamic dispatch
    resolution" / "No conditional-type evaluation" / "No
    metaclass / monkey-patching resolution" — trait-shape audit (R12)
    is the detection layer.
  - "Runtime macro expansion" — trait-shape audit (R11) ensures no
    `expand_*` method exists on `LanguagePlugin`.
- **Universal language boundaries**
  ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  - **A1, A2, A3, B2** — trait-shape audit covers them.
  - **C1** — R11 covers macro/template/preprocessor expansion shape.
  - **F2** — immutable-source audit covers write-back-to-source.

## Deliverables

### R9 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r9--immutable-source-guarantee))

- [ ] Every plugin trait method takes `&str`, `&Tree`, and the
      appropriate context trait (`&dyn LanguageWorkspaceContext` for
      language plugins, `&dyn FrameworkWorkspaceContext` for framework
      plugins per the R4 split merged into `refactor/phase-b` in sprint
      0002) — all immutable references.
- [ ] No `&mut` reaches the plugin layer for any source-related type
      (`&mut str`, `&mut Tree`, `&mut Source*`).
- [ ] Static lint / grep-based CI gate (`scripts/audit_immutable.sh`)
      runs and is `active` in CI.

### R11 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r11--macro-definition-only-by-trait-shape))

- [ ] Trait inspection of `LanguagePlugin` (post-R2 shape) shows **no**
      method named `expand_*` or `evaluate_*` or anything implying
      expansion.
- [ ] Macro definitions are indexed as `Symbol { kind: macro }` (the
      `macro` symbol kind landed in R0, sprint 0001).
- [ ] Macro invocation sites are recorded as `references` edges from
      the call site to the macro symbol — never expanded.

### R12 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r12--type-system-free-trait-audit--process-spawn-denylist))

- [ ] **Trait-shape audit** (`scripts/audit_trait_shape.sh`): no method
      whose name implies inference / evaluation / narrowing / overload
      resolution / expansion exists on `LanguagePlugin` or the
      `Extractor` layer
      ([sprint 0003 § Deliverables](./0003-phase-b-typed-plugin-and-resolution.md#deliverables)).
- [ ] **Process-spawn denylist** (`scripts/audit_no_spawn.sh`): no
      `Command::new(`, `process::Command`, or `std::process::Command`
      appears in `src/languages/`, `src/frameworks/`, `src/core/parser.rs`,
      `src/core/extract*.rs`, or `src/core/resolve*.rs` (excluding
      allowlist-tagged sites — see [`CI-GATES.md` § Allowlist convention](../CI-GATES.md#allowlist-convention-r12-spawn--network--fs-gates)).
- [ ] **Network denylist** (`scripts/audit_no_network.sh`): no
      `std::net::*`, `reqwest`, `hyper`, `tokio::net`, or `ureq` linked
      into plugin / extractor / query paths.
- [ ] The current legitimate `Command::new("scope")` self-invocation in
      `src/commands/setup.rs:39` carries the
      `// scope:audit-allow process-spawn — self-invocation for scope setup`
      tag and is the **only** allowlist entry.
- [ ] The negative trait shape (the methods that must not exist) is
      documented as a comment in the trait module so future readers see
      it without grepping the audit script.

---

## Ambiguities to clarify before code lands

Each ambiguity below is resolved by an amendment to the cited
source-of-truth document on `main` **before** this sprint's branch
opens.

1. **Network denylist enforcement target.** `CI-GATES.md` notes the
   gate's current target is the dependency tree, with `grep + cargo-deny`
   as the implementation. Whether `cargo-deny` is wired in this sprint
   or in a follow-up is not specified. Halt and ask: ship pure grep now,
   or invest in `cargo-deny` integration as part of this sprint?
   Resolution amends `CI-GATES.md` Network-denylist row.

2. **Allowlist tag location.** `R12` says the tag immediately precedes
   the call site. If a call is wrapped in a helper function, the tag
   could live on either the helper or the call within the helper. Halt
   and ask the human for the canonical placement; resolution amends
   `CI-GATES.md § Allowlist convention`.

3. **Extractor / resolver path filter.** The process-spawn denylist
   mentions `src/core/extract*.rs` and `src/core/resolve*.rs` as paths,
   but after sprint 0000 the code lives under sub-crate paths
   (`scope-core/`, `scope-index/`, `scope-graph/`). Sprint 0003's
   ambiguities about extractor and resolver location feed into this
   gate. Resolution amends `CI-GATES.md` Process-spawn-denylist row's
   "Fails on" column with the updated sub-crate paths.

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **Trait-shape audit** (`just ci-trait-shape`) — `planned` →
      `active`.
- [ ] **Process-spawn denylist** (`just ci-no-spawn`) — `planned` →
      `active`.
- [ ] **Network denylist** (`just ci-no-network`) — `planned` →
      `active`.
- [ ] **Immutable source** (`just ci-immutable`) — `planned` →
      `active`.
- [ ] **Macro definition-only** — already covered by trait-shape audit
      per `CI-GATES.md` row ("trait-shape audit (subset of R12)").
      Confirm the audit script's pattern list catches `expand_*` /
      signature returning expanded source text.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`LanguagePlugin`, `Extractor`](../GLOSSARY.md#plugin-shapes)
- [Class 1 / mechanical, Class 2 / detectable, Class 3 /
  discipline-only](../GLOSSARY.md#architecture)
- [Gate, Gate status, Allowlist tag](../GLOSSARY.md#ci-gates)

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0004-trait-closure-audits`, cut from
  `refactor/phase-b` after sprint 0003 merged into it.
- **Base**: `refactor/phase-b`, **not** `main`.
- **Open**: flip R9, R11, R12 rows in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append log entries noting branch name.
- **Codex review (sprint scope)**: before the sprint-close commit, run
  the canonical command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base refactor/phase-b`
  - `--title "sprint 0004 — R9+R11+R12"`
  - Prompt focus: R9 / R11 / R12 acceptance bullets, A1–A3 + B2 + C1
    + F2 detection layer, the universal class-3 list (B1, C2, E3),
    the four CI gates this sprint activates (Trait-shape,
    Spawn-denylist, Network-denylist, Immutable source).
- **Close (sprint branch)**: demonstrate R9/R11/R12 acceptance on the
  sprint branch. R9, R11, and R12 remain `in-progress` in
  `REFACTOR-STATUS.md` until the Phase B phase-close commit.
  Rebase-merge sprint branch into `refactor/phase-b`.
- **Codex review (phase scope)**: before opening the integration PR,
  run a second canonical pass with:
  - `--base main`
  - `--title "Phase B integration"`
  - **`-c model_reasoning_effort="high"`** override (Phase B integration
    review crosses seven R-moves and their interactions — the explicit
    medium→high override authorised in
    [`README.md` § 9 — Why these flags](./README.md#role-1--mandatory-sprint-review-checkpoint);
    record override in the integration PR body).
  - Prompt focus: Phase B § Acceptance set in
    `ARCHITECTURAL-REFACTOR.md`, cross-R-move interactions (R3 resolver
    consumes R4 context; R12 audit covers R2 post-shape trait; etc.).
  Both reports attach to the integration PR body.
- **Phase-close commit (on `refactor/phase-b`)**: once sprint 0004 is
  merged into the integration branch and the phase-scope Codex review
  has been addressed, the Phase B acceptance set is demonstrated. Add
  a separate phase-close commit on `refactor/phase-b` flipping every
  Phase B R-move row (R2, R3, R4, R7, R9, R11, R12) from
  `in-progress` → `shipped`, flipping the **Phase B row** in the
  snapshot from `in-progress` → `shipped`, and appending R-move plus
  phase-close log entries. Message: `chore(refactor-status): phase B close`.
- **Integration merge**: open PR `refactor/phase-b` → `main`. After CI
  green and review, merge (rebase-merge preserves sprint structure;
  squash-merge collapses Phase B into one commit). The merge commit on
  `main` is the official Phase B close.
- **Next**: sprint 0005 is cut from post-merge `main`.

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. Three ambiguities above are resolved before code lands.
3. Four CI gates listed above are `active` in `CI-GATES.md` and CI.
4. After the Phase B integration branch merges to `main`,
   `REFACTOR-STATUS.md` shows every Phase B R-move (R2, R3, R4, R7,
   R9, R11, R12) and **Phase B** as `shipped`.
5. Each `docs/languages/<name>.md` compliance log
   ([`LANGUAGE-PLAYBOOK.md` Step 6](../LANGUAGE-PLAYBOOK.md#step-6--per-language-doc-template))
   records: A1, A2, A3, B2, C1, F2 mechanically enforced (via R9, R11,
   R12). Only B1, C2, E3 remain `discipline-only` per the universal
   class-3 list ([`ARCHITECTURAL-REFACTOR.md` § What remains
   discipline-only after the refactor](../ARCHITECTURAL-REFACTOR.md#what-remains-discipline-only-after-the-refactor)).
6. No `NEEDS REVIEW` entry left for any rule labeled `mechanical` or
   `detectable` in the refactor's inventory tables, on any active
   language plugin.

## Out of scope for this sprint

- Framework infrastructure — sprint 0005 (R5).
- Output schema, confidence audit — sprint 0006 (R10, R8).
- Malformed-source harness — sprint 0007 (R6).
- Phase C is not unlocked merely by Phase B closing; Phase C
  ([`ARCHITECTURAL-REFACTOR.md` § Phase C](../ARCHITECTURAL-REFACTOR.md#phase-c--framework-layer-closure))
  lands "when framework infrastructure is first introduced".
  Sprint 0005 is the moment that decision is acted on.
