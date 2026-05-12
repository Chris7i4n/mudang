# Scope Documentation

Entry point for the docs tree. If this is your first time, read in this order.

## Reading order (governing docs)

1. **`CHARTER.md`** — what Scope is, what it is not, hard limits, soft expansion zone, per-language IN/OUT. Permanent constraints; revisions require explicit charter-amendment commits per CHARTER §11.
2. **`ARCHITECTURAL-REFACTOR.md`** — the structural closure that mechanically enforces charter rules and the playbook boundaries (R0–R12 across phases A–E). **Active work item.** Until Phase E ships, feature work is paused.
3. **`sprints/README.md`** — linear, sequential sprint plan that decomposes the refactor into shippable units. Source of truth for **ordering**; the R-moves' acceptance criteria still live in `ARCHITECTURAL-REFACTOR.md`.
4. **`REFACTOR-STATUS.md`** — current state of each refactor move and phase. Append-only progress log. Sprints report into this document.
5. **`LANGUAGE-PLAYBOOK.md`** — how to add and maintain a language plugin within the 18 universal boundaries.
6. **`FRAMEWORK-PLAYBOOK.md`** — how to add and maintain a framework plugin within the 15 gotcha categories. Includes version-strategy rules (A/B/C) and unknown-version policy.
7. **`POST-REFACTOR-PLAN.md`** — work queued for after Phase E acceptance. No item starts before then.

## Reference docs (read on demand)

- **`GLOSSARY.md`** — central term definitions across all docs (types, traits, processes, subcommands, classes of constraint).
- **`CI-GATES.md`** — single source of truth for every CI gate the refactor turns on. Owned by R-moves; the `justfile` recipes mirror this doc.

## Static reference

These documents govern decisions; revise via explicit commit, not silent edit.

| Document | Owns |
|---|---|
| `CHARTER.md` | Mission, hard limits, soft expansion, per-language IN/OUT, moats vs LSP, amendment rule |
| `ARCHITECTURAL-REFACTOR.md` | R0–R12 mechanical/detectable closure, type-state pipeline, schema design, phase order |
| `sprints/` | Sequential delivery plan; one file per sprint; links into the docs above; never restates rules |
| `LANGUAGE-PLAYBOOK.md` | 18 universal language-plugin rules, language-adoption flow, per-language doc structure |
| `FRAMEWORK-PLAYBOOK.md` | Framework adoption flow, version strategies, unknown-version policy, 15 gotcha categories |
| `POST-REFACTOR-PLAN.md` | Future work queued behind Phase E; does not authorize starting before |
| `GLOSSARY.md` | Central term definitions |
| `CI-GATES.md` | CI gate inventory; canonical script paths; allowlist convention |

## Runtime artifacts (created on demand, append-only)

| Artifact | Purpose | Format owner |
|---|---|---|
| `REFACTOR-STATUS.md` | Refactor progress per move and per phase | this README + `ARCHITECTURAL-REFACTOR.md` |
| `LANGUAGE-TRIGGERS.md` | Friction events per language candidate | `LANGUAGE-PLAYBOOK.md` Step 1 |
| `LANGUAGE-DECISIONS.md` | Verdicts (BUILD / DEFER / REJECT) per language | `LANGUAGE-PLAYBOOK.md` Step 2 |
| `FRAMEWORK-TRIGGERS.md` | Friction events per framework candidate | `FRAMEWORK-PLAYBOOK.md` Step 1 |
| `FRAMEWORK-DECISIONS.md` | Verdicts per framework | `FRAMEWORK-PLAYBOOK.md` Step 2 |
| `languages/<name>.md` | Per-language gotcha + 18-rule compliance log | `languages/_TEMPLATE.md` |
| `frameworks/<name>.md` | Per-framework gotcha + version + walkthrough table | `frameworks/_TEMPLATE.md` |

## Templates

- `languages/_TEMPLATE.md` — start a new per-language doc by copying this file.
- `frameworks/_TEMPLATE.md` — same for a new framework doc.

## Current state

Refactor progress snapshot (live state lives in
`REFACTOR-STATUS.md`):

- **Sprint 0000** — crate decomposition (structural prerequisite inside
  mudang's Phase A). **Shipped 2026-05-11.** Five sub-crates
  (`scope-core`, `scope-index`, `scope-graph`, `scope-search`,
  `scope-workspace`) live under `gumiho-mudang-scope/`; the legacy
  crate name is a façade re-exporting them 1:1.
- **Sprint 0001** — R0 + R1 schema and storage. **Shipped 2026-05-11.**
  Confidence / status / producer / pattern_id / args_text columns,
  surrogate `edge_id` PK, 38 edge kinds, sealed `Edge` / `EdgeBuilder`
  typestate, Phase A resolver stub.
- **Sprint 0002** — R7 + R4 dispatch and workspace context. **Closed on
  `refactor/phase-b`** (not yet on `main`). `LanguageId` enum + const-
  fn dispatch + `register_languages!` macro; `LanguagePlugin` trait and
  7 `*Plugin` structs removed; `LanguageWorkspaceContext` typed split;
  3 CI gates activated (context-shape / no-fs / dispatch).
- **Sprint 0003** — R2 + R3 typed plugin output and resolution pipeline.
  **In-progress on `refactor/sprint-0003-typed-plugin-resolution`**.
  Chunks 1, 2, 3a, 3b + codex review round-trip shipped on the sprint
  branch (extract module scaffolding, per-language extractor migration,
  `skipped_ranges` plumbing, metadata reserved-key spec compliance,
  charter amend + shim audit + sprint 0009 added — the sprint number
  reflects the post-Phase-D-split renumbering on 2026-05-12). Chunks
  4–7 (R3 resolver scaffold, stub retirement, typestate CI gate,
  sprint-close) pending.
- **Sprints 0002–0005** — shipped (Phase B atomic close on 2026-05-12
  for 0002/0003/0004; Phase C close on 2026-05-12 for 0005).
- **Sprints 0006–0009** — Phase D + Phase E. **Sprint 0006**
  (R10 typed output schema) in-progress on
  `refactor/sprint-0006-typed-output-schema` (off the
  `refactor/phase-d` integration branch). **Sprint 0007**
  (R8 confidence audit) unstarted — cuts from `refactor/phase-d` after
  sprint 0006 merges. **Sprint 0008** (R6 malformed-source harness)
  and **sprint 0009** (charter sweep + shim retirement; closes the
  refactor) follow after Phase D atomic close.

Until the refactor closes, plugin authoring follows the pre-refactor
trait shapes (see `LANGUAGE-PLAYBOOK.md` Step 5 pre-R2 vs post-R2
sections).

## Where to put a new note

| Recording … | Goes in |
|---|---|
| Why a feature is permanent out-of-scope | `CHARTER.md` §5 (charter-amendment commit) |
| A new mechanically enforced rule | propose a new R-move via amendment to `ARCHITECTURAL-REFACTOR.md` |
| A new CI gate | row in `CI-GATES.md` (status `planned` until shipped) |
| Sprint sequencing / scope / ordering | `sprints/` (one file per sprint; never duplicate rules) |
| A friction event for a candidate plugin | the matching trigger log |
| A verdict on a candidate plugin | the matching decision log |
| A per-plugin gotcha or rule-temptation rejection | the per-instance doc (`languages/<name>.md` or `frameworks/<name>.md`) |
| A post-refactor work item | `POST-REFACTOR-PLAN.md` |
| A refactor-move status transition | `REFACTOR-STATUS.md` (sprints report into this) |
| An unfamiliar term used elsewhere | look up in `GLOSSARY.md`; if missing, add it |

## Outside this directory

- `README.md` (repo root) — install, CLI surface, quickstart.
- `Cargo.toml` — pinned tree-sitter grammars and crate dependencies.
- `src/sql/schema.sql` — current schema (will diverge from the R0 target until R0 ships; cross-check with `REFACTOR-STATUS.md` for the live schema-version state).
- `justfile` — CI-gate recipe wrappers; canonical paths live in `CI-GATES.md`.
