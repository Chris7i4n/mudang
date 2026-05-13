# Sprint 0003 — Priority 1: per-language `lang_version` detector matrix

> **Source of truth**: [`BACKLOG.md` § Priority 1 — Self-correction cycle](../BACKLOG.md#priority-1--self-correction-cycle), sub-item **(d) Per-language `lang_version` detector matrix**.
> **Phase**: A (single-sprint, atomic across all seven languages). Merges directly to `main`.
> **Ambiguity protocol**: [`README.md` § Ambiguity protocol](./README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc).

---

## Goal

Populate the `lang_version` JSONL slot atomically across all seven supported languages so the labelled corpus never splits into a "versioned" and "unversioned" era. Workspace-side detector wiring only; the plugin trait surface (R4 `LanguageWorkspaceContext`) is not widened.

## Scope owned this sprint

- **Priority 1 (d)** ([source link](../BACKLOG.md#priority-1--self-correction-cycle))

## Prerequisites

- Sprints 0001 + 0002 shipped — corpus on-disk policy + loop architecture doc landed; Priority 1 (a) and (e) rows `shipped`.

## Charter alignment

- **Hard limits** — none crossed; detectors are workspace-internal, not plugin trait additions.
- **Soft expansion zone** — `CHARTER.md` §6, version detection per LANGUAGE-PLAYBOOK Step 4.
- **Per-language IN/OUT** — touched: Rust, Go, Python, TypeScript, Java, C#, Ruby ([`CHARTER.md` § 7](../CHARTER.md#7-per-language-scope-and-non-scope)).
- **Invariants** — R4 `LanguageWorkspaceContext` shape preserved (detectors stay off plugin trait surface).

## Deliverables

### Priority 1 (d) acceptance ([source](../BACKLOG.md#priority-1--self-correction-cycle))

- [ ] Rust detector reads edition + `rust-version` from `Cargo.toml` (module exists; confirm coverage).
- [ ] Go detector reads `go` directive from `go.mod` (module exists; confirm coverage).
- [ ] Python detector reads `requires-python` from `pyproject.toml` and `setup.py`.
- [ ] TypeScript detector reads `target` from `tsconfig.json`.
- [ ] Java detector reads Maven `<source>` / `<target>` and Gradle source compatibility (two build systems).
- [ ] C# detector reads `<TargetFramework>` from `.csproj`.
- [ ] Ruby detector reads `.ruby-version` and Gemfile `ruby` directive.
- [ ] R8 audit emit writes a non-`null` `lang_version` for every supported language under realistic fixtures.

### Priority 1 (d) implementation deliverables

- [ ] Workspace-side detector module per language (or extend existing). Files land under `scope-workspace/src/...` or matching crate; **no addition** to the `LanguagePlugin` trait or `LanguageWorkspaceContext`.
- [ ] Unit tests per language verifying detection against representative `Cargo.toml` / `go.mod` / `pyproject.toml` / etc. fixtures.
- [ ] Integration test: emit JSONL samples across the seven-language fixture corpus; assert every record carries non-`null` `lang_version`.
- [ ] [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md) updated if R4 entry's "Where in the tree" or "Durable contract" lines shift; otherwise n/a.
- [ ] Per-language gotcha entries in `gumiho-mudang-scope/docs/languages/<name>.md` for any detector edge case (multi-package monorepos deferred per BACKLOG "deliberately deferred").

---

## Ambiguities resolved before this sprint opens

- Java's two build systems (Maven + Gradle) — both required per BACKLOG (d); halt only if a third build system surfaces in fixtures.
- Multi-package monorepos (npm workspaces, Python multi-package) — deferred per [`BACKLOG.md` § Items deliberately deferred beyond this plan](../BACKLOG.md#items-deliberately-deferred-beyond-this-plan).

---

## CI gates activated in this sprint

- [ ] Optionally: **lang_version coverage gate** — assert no `null` `lang_version` in the seven-language fixture audit emit. Status `planned → active` in the same commit if added; otherwise omit and queue in [`CI-GATES.md`](../CI-GATES.md) as `planned`.

## Glossary terms touched

- `DetectedVersion`, `ResolvedVersion`, `UnknownVersionPolicy` ([`GLOSSARY.md` § Versioning](../GLOSSARY.md#versioning)) — referenced; no new terms expected.

## Reporting

- **Branch**: `selfcorrect/sprint-0003-lang-version-detector-matrix`
- **Base**: `main`
- **Codex review**: canonical command per [`README.md` § 9 Role 1](./README.md#role-1--mandatory-sprint-review-checkpoint).

## Definition of done

All Deliverables bullets checked. **doc-sync gate green** (per-language detector modules referenced from per-language `docs/languages/<name>.md` gotcha entries; R4 contract surface unchanged or reflected in [`ENFORCEMENT-MAP.md`](../ENFORCEMENT-MAP.md)). R8 emit on the reference fixture corpus shows zero `null` `lang_version`. Enforcement-map: refinement only if R4 entry text shifts.

## Out of scope for this sprint

- Per-sub-root version detection for monorepos ([`BACKLOG.md` § Items deliberately deferred](../BACKLOG.md#items-deliberately-deferred-beyond-this-plan)).
- Any change to plugin trait surface (R4 shape preserved).
- Schema bump v1 → v2 — sprint 0004.
