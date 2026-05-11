# Sprint 0002 — Phase B: Dispatch and workspace context

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R7](../ARCHITECTURAL-REFACTOR.md#r7--indexer-level-dispatch-enforcement) and [§ R4](../ARCHITECTURAL-REFACTOR.md#r4--workspacecontext-typed-access-split-per-layer).
> **Phase**: B (first sprint of three). Phase B is atomic; this sprint
> opens it. Phase B closes only after sprint 0004 merges into
> `refactor/phase-b`, the phase-close commit lands, and the integration
> branch merges to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human).

---

## Goal

Lock the **inputs** of the language and framework plugin layers before
their **outputs** are restructured (sprint 0003). Dispatch (R7) is the
mechanical answer to F4 (no content sniffing); the workspace context
split (R4) is the mechanical answer to D1, F3, and the C2-leakage gap.
Both must exist before R2/R3 redefine the plugin trait, because R2/R3
trait signatures consume the context types from R4.

## R-moves owned by this sprint

- **R7 — Indexer-level dispatch enforcement** ([§ R7](../ARCHITECTURAL-REFACTOR.md#r7--indexer-level-dispatch-enforcement))
- **R4 — WorkspaceContext typed access (split per layer)** ([§ R4](../ARCHITECTURAL-REFACTOR.md#r4--workspacecontext-typed-access-split-per-layer))

## Prerequisites

- Sprint 0001 shipped: R0 and R1 must be `shipped` in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md). Without R0 the
  `producer` and `pattern_id` columns do not exist; without R1 the
  builder is not the sole producer of `RawEdge` and plugin trait
  signatures cannot enforce the post-R2 shape that R4's context types
  are about to be threaded into.

## Charter alignment

- **Hard limits** ([`CHARTER.md` §5](../CHARTER.md#5-hard-limits--scope-will-never-cross-these)):
  R4 closes the latent path by which a language plugin could read
  `edition` / `target` / `python_requires` / framework versions and
  thereby drift into version-specific semantics (C2). The mechanical
  safeguard is the split trait shape — no `LanguageWorkspaceContext`
  accessor exposes those fields.
- **Universal language boundaries** ([`LANGUAGE-PLAYBOOK.md` Step 4](../LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)):
  - **D1** (no cross-file resolution beyond config) — mechanical after R4.
  - **F3** (no file-format parsing beyond tree-sitter) — mechanical after R4.
  - **F4** (no content sniffing) — mechanical after R7.
  - **C2** (no version-specific compiler-quirk modelling) —
    still discipline-only as a rule, but the **leakage path** that
    would have let a plugin drift into it is closed by R4's split.
    The trait surface omits the version fields, so reading them in a
    language plugin is a compile error.
    [`LANGUAGE-PLAYBOOK.md` Category C](../LANGUAGE-PLAYBOOK.md#category-c--macros-templates-and-version-semantics)
    documents the asymmetry.

## Deliverables

Mirrored from each R-move's **Acceptance** section in
`ARCHITECTURAL-REFACTOR.md`.

### R7 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r7--indexer-level-dispatch-enforcement))

Per the amended R7 "Target state" — compile-time dispatch, no runtime
election:

- [ ] `LanguageId` enum lands in `scope-core/src/languages/id.rs`,
      exhaustive over the seven supported languages.
- [ ] Every plugin type exposes `const ID: LanguageId`,
      `const EXTENSIONS: &'static [&'static str]`, and
      `const SHEBANGS: &'static [&'static str]`.
- [ ] `LanguagePlugin` trait has **no** `extensions`,
      `accepts_extension`, `accepts_shebang`, or any other runtime
      opt-in method. Capability is `const`; plugin instance does not
      decide at runtime.
- [ ] `scope-core/src/languages/dispatch.rs` exists and exposes
      `pub const fn dispatch_extension(&str) -> Option<LanguageId>`,
      `pub const fn dispatch_shebang(&str) -> Option<LanguageId>`,
      and `pub fn plugin_for(LanguageId) -> &'static dyn LanguagePlugin`
      (exhaustive `match` — compiler forces an arm per variant).
- [ ] A declarative macro `register_languages!` is the single call site
      naming all seven plugins together.
- [ ] A `const _: () = assert_no_extension_overlap(...);` block in
      `dispatch.rs` fails the build if any extension is claimed by two
      plugins.
- [ ] `scope-core/src/parser.rs::detect_language` hardcoded match is
      removed; every caller goes through `dispatch_extension`.
- [ ] Plugin code cannot self-activate: it cannot open a file, read its
      contents, or decide whether to handle it. The indexer is the sole
      dispatcher (CI grep gate: `just ci-dispatch`).

### R4 acceptance ([source](../ARCHITECTURAL-REFACTOR.md#r4--workspacecontext-typed-access-split-per-layer))

- [ ] Plugin code contains no `std::fs::*` calls (CI grep gate flips
      to `active` — see below).
- [ ] Language-plugin trait inspection shows it accepts
      `&dyn LanguageWorkspaceContext` (or a generic bound thereon) and
      **never** `FrameworkWorkspaceContext`.
- [ ] Framework-plugin trait inspection shows it accepts
      `&dyn FrameworkWorkspaceContext`. (The framework trait itself
      lands in R5, sprint 0005 — but the **context trait** lands here
      so that R5 has it available.)
- [ ] `LanguageWorkspaceContext` ships as `pub` in `scope-core`.
      `FrameworkWorkspaceContext` ships as `pub(crate)` in `scope-core`
      per the amended R4 "Target state" (visibility safeguard during
      Phase B). R5 (sprint 0005) widens it to `pub` in the same commit
      that lands the first `FrameworkPlugin` impl.
- [ ] `LanguageWorkspaceContext` exposes no method whose name suggests
      version-coupled fields: no `edition`, `target`, `python_requires`,
      `go_directive`, `tsconfig_target`, `framework_versions`.
- [ ] CI grep gate enforces the negative trait shape
      (`just ci-context-shape`; see below).
- [ ] Config-file readers (Cargo.toml, package.json, pyproject.toml,
      Gemfile.lock, tsconfig.json, go.mod) populate the typed structs.
      No raw filesystem handle is reachable from either context trait.

---

## Ambiguities — resolved on main before branch opens

Both pre-branch ambiguities were resolved per `README.md` § 3 ambiguity
protocol and committed to `ARCHITECTURAL-REFACTOR.md` on `main` before
this sprint's branch opens. The resolutions are now binding:

1. **`FrameworkWorkspaceContext` visibility (R4).** Resolved: ships as
   `pub(crate)` in `scope-core` for the duration of Phase B (sprints
   0002, 0003, 0004). R5 (sprint 0005, Phase C) widens to `pub` in the
   same commit that lands the first `FrameworkPlugin` impl. Mechanical
   extension of the split-trait safeguard — compiler refuses any
   language-plugin code that tries to import or bound on the framework
   context during Phase B. The visibility flip is not a stub; not
   tracked in `REFACTOR-STATUS.md` § Stubs outstanding. See
   [`ARCHITECTURAL-REFACTOR.md` § R4](../ARCHITECTURAL-REFACTOR.md#r4--workspacecontext-typed-access-split-per-layer)
   and sprint 0005 ambiguity register.

2. **R7 dispatch shape.** Resolved: compile-time dispatch, no runtime
   election. Plugin capability is associated `const` data
   (`ID` / `EXTENSIONS` / `SHEBANGS`); `LanguageId` enum is exhaustive;
   declarative macro `register_languages!` generates `dispatch_extension`
   (`const fn`), `dispatch_shebang` (`const fn`), and `plugin_for`
   (exhaustive `match`). A `const _: () = assert_no_extension_overlap(...)`
   block fails the build on duplicate extension claims.
   `LanguagePlugin::extensions` and `parser.rs::detect_language` are
   removed. See
   [`ARCHITECTURAL-REFACTOR.md` § R7](../ARCHITECTURAL-REFACTOR.md#r7--indexer-level-dispatch-enforcement).

---

## CI gates activated in this sprint

From [`CI-GATES.md` § Gate inventory](../CI-GATES.md#gate-inventory):

- [ ] **WorkspaceContext shape** (`just ci-context-shape`) — `planned`
      → `active`. Audits that `LanguageWorkspaceContext` does **not**
      expose the version-coupled accessors listed in R4.
- [ ] **No filesystem in plugin** (`just ci-no-fs`) — `planned` →
      `active`. Greps for `std::fs::*`, `std::path::PathBuf::from`,
      `File::open` in plugin code.
- [ ] **Indexer-only dispatch** (`just ci-dispatch`) — `planned` →
      `active`. Greps for content-sniffing patterns in plugin trait
      impls.

## Glossary terms touched

From [`GLOSSARY.md`](../GLOSSARY.md):

- [`LanguageWorkspaceContext`, `FrameworkWorkspaceContext`,
  `WorkspaceContext` (historical)](../GLOSSARY.md#workspace-context)

No new terms introduced. If during implementation a new context method
needs a name, halt and add it to the glossary in its own commit before
the API ships.

## Reporting

Per [`README.md` § Reporting hooks](./README.md#4-reporting-hooks) and
[`README.md` § Branch protocol](./README.md#5-branch-protocol--linear-incremental-one-sprint-per-branch):

- **Branch**: `refactor/sprint-0002-dispatch-workspace-context`, cut
  from `refactor/phase-b` (the Phase B integration branch, itself cut
  from `main` after sprint 0001 merged).
- **Base**: `refactor/phase-b`, **not** `main`. Per
  [`README.md` § 1](./README.md#1-linear-order-no-parallel-sprints--atomic-phase-shipment-to-main)
  Phase B is atomic at the `main` level; sprints 0002/0003/0004 merge
  into the integration branch first.
- **Open**: cut `refactor/phase-b` from `main` (if not already cut by a
  prior sprint in the phase), then cut this sprint's branch from
  `refactor/phase-b`. Flip R7 and R4 rows in
  [`REFACTOR-STATUS.md`](../REFACTOR-STATUS.md) snapshot to
  `in-progress`. Append log entries noting both branch names and
  `sprint 0002 opened`.
- **Codex review**: before the sprint-close commit, run the canonical
  command from
  [`README.md` § 9 — Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint)
  with:
  - `--base refactor/phase-b`
  - `--title "sprint 0002 — R7+R4"`
  - Prompt focus: R7 and R4 acceptance bullets, C2 leakage path
    closure, charter §5 hard limits, CI gates this sprint activates
    (WorkspaceContext shape, No filesystem in plugin, Indexer-only
    dispatch).
  Attach report to PR body; address blockers.
- **Close**: demonstrate R7/R4 acceptance on the sprint branch and
  rebase-merge it into `refactor/phase-b`. R7 and R4 remain
  `in-progress` in `REFACTOR-STATUS.md` until the Phase B integration
  branch merges to `main`; `shipped` is reserved for main.
- **Merge**: rebase-merge sprint branch into `refactor/phase-b`.
  Sprint 0003's branch is cut from `refactor/phase-b` after this
  merge.

## Definition of done

1. Every checkbox in **Deliverables** above is checked.
2. The two ambiguities above are resolved before code lands; both
   resolutions are committed to `ARCHITECTURAL-REFACTOR.md` on `main`
   before this sprint's branch opens. (Done: R4 visibility = `pub(crate)`;
   R7 = compile-time const dispatch via `register_languages!`.)
3. The three CI gates listed above are `active` in `CI-GATES.md` and CI.
4. `REFACTOR-STATUS.md` shows R7 and R4 `in-progress` on
   `refactor/phase-b`; they flip to `shipped` only in the Phase B
   phase-close commit that merges to `main`.
5. Every existing language plugin (`scope-core/src/languages/rust_lang.rs`,
   `python.rs`, `go_lang.rs`, `typescript.rs`, `java.rs`, `csharp.rs`,
   `ruby.rs`) exposes the three associated consts (`ID`, `EXTENSIONS`,
   `SHEBANGS`); the `extensions(&self) -> &[&str]` method is removed
   from the `LanguagePlugin` trait; every plugin compiles against the
   new context traits with zero direct filesystem access.
6. Compile-time enforcement proven:
   - Adding a duplicate extension to two plugins fails the build with a
     const-panic from `assert_no_extension_overlap`.
   - Adding a `LanguageId` variant without a matching arm in
     `plugin_for` fails the build via the exhaustive `match`.
   - Importing `FrameworkWorkspaceContext` from any plugin file outside
     `scope-core` fails the build (the trait is `pub(crate)` until R5).
7. `scope-core/src/parser.rs::detect_language` hardcoded match is
   removed; every caller routes through `scope-core::languages::dispatch`.

## Out of scope for this sprint

- The `LanguagePlugin` output type change (`RawCaptures`) — sprint 0003.
- The resolution typestate pipeline — sprint 0003.
- The `Extractor` layer — sprint 0003.
- Trait-shape audit, immutable-source audit, macro-shape audit, and the
  process-spawn denylist — sprint 0004.
- `FrameworkPlugin` trait body — sprint 0005 (Phase C). This sprint
  only lands the **context trait** the framework layer will consume.
