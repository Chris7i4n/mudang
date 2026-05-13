# Scope Documentation

Entry point for the docs tree. If this is your first time, read in this order.

## Reading order (governing docs)

1. **`CHARTER.md`** — what Scope is, what it is not, hard limits, soft expansion zone, per-language IN/OUT. Permanent constraints; revisions require explicit charter-amendment commits per CHARTER §11.
2. **`ARCHITECTURAL-REFACTOR.md`** — closure record of the structural refactor (shipped 2026-05-12). R0–R12 across phases A–E. Maps each charter / playbook rule to the R-move and CI audit that enforces it.
3. **`LANGUAGE-PLAYBOOK.md`** — how to add and maintain a language plugin within the 18 universal boundaries.
4. **`FRAMEWORK-PLAYBOOK.md`** — how to add and maintain a framework plugin within the 15 gotcha categories. Includes version-strategy rules (A/B/C) and unknown-version policy.
5. **`POST-REFACTOR-PLAN.md`** — work queued against the closed architecture, ordered by priority. Eligibility holds.
6. **`sprints/README.md`** — sprint methodology (linear order, atomic phase shipment, codex consultation, branch protocol). Used by past sprints (architectural refactor) and durable for future initiatives. The sprint skeleton lives in [`sprints/_TEMPLATE.md`](sprints/_TEMPLATE.md).

## Reference docs (read on demand)

- **`GLOSSARY.md`** — central term definitions across all docs (types, traits, processes, subcommands, classes of constraint).
- **`CI-GATES.md`** — single source of truth for every CI gate the architecture enforces. Owned by R-moves; the `justfile` recipes mirror this doc.

## Static reference

These documents govern decisions; revise via explicit commit, not silent edit.

| Document | Owns |
|---|---|
| `CHARTER.md` | Mission, hard limits, soft expansion, per-language IN/OUT, moats vs LSP, amendment rule |
| `ARCHITECTURAL-REFACTOR.md` | R0–R12 closure record; rule-to-enforcement map; phase-order historical record |
| `sprints/README.md` | Sprint methodology — durable for any future initiative |
| `sprints/_TEMPLATE.md` | Per-sprint doc skeleton |
| `LANGUAGE-PLAYBOOK.md` | 18 universal language-plugin rules, language-adoption flow, per-language doc structure |
| `FRAMEWORK-PLAYBOOK.md` | Framework adoption flow, version strategies, unknown-version policy, 15 gotcha categories |
| `POST-REFACTOR-PLAN.md` | Work queue eligible against the closed architecture |
| `GLOSSARY.md` | Central term definitions |
| `CI-GATES.md` | CI gate inventory; canonical script paths; allowlist convention |

## Runtime artifacts (created on demand, append-only)

| Artifact | Purpose | Format owner |
|---|---|---|
| `LANGUAGE-TRIGGERS.md` | Friction events per language candidate | `LANGUAGE-PLAYBOOK.md` Step 1 |
| `LANGUAGE-DECISIONS.md` | Verdicts (BUILD / DEFER / REJECT) per language | `LANGUAGE-PLAYBOOK.md` Step 2 |
| `FRAMEWORK-TRIGGERS.md` | Friction events per framework candidate | `FRAMEWORK-PLAYBOOK.md` Step 1 |
| `FRAMEWORK-DECISIONS.md` | Verdicts per framework | `FRAMEWORK-PLAYBOOK.md` Step 2 |
| `languages/<name>.md` | Per-language gotcha + 18-rule compliance log | `languages/_TEMPLATE.md` |
| `frameworks/<name>.md` | Per-framework gotcha + version + walkthrough table | `frameworks/_TEMPLATE.md` |

## Templates

- `sprints/_TEMPLATE.md` — start a new sprint doc by copying this file.
- `languages/_TEMPLATE.md` — start a new per-language doc by copying this file.
- `frameworks/_TEMPLATE.md` — same for a new framework doc.

## Current state

The architectural refactor (R0–R12 across phases A–E) closed on **2026-05-12**. Every CI gate listed in `CI-GATES.md` is `active`; `just gate-refactor` runs every gate green. Plugin authoring follows the closed shapes (see `LANGUAGE-PLAYBOOK.md` Step 5).

The historical sprint plans (`sprints/0000…0009-*.md`) and the append-only refactor-status log (`REFACTOR-STATUS.md`) were retired at refactor close. Their history is preserved in git — search for `refactor/sprint-*` merge commits and `chore(refactor-status): …` commits on `main`.

## Where to put a new note

| Recording … | Goes in |
|---|---|
| Why a feature is permanent out-of-scope | `CHARTER.md` §5 (charter-amendment commit) |
| A new mechanically enforced rule | propose a new R-move via amendment to `ARCHITECTURAL-REFACTOR.md` |
| A new CI gate | row in `CI-GATES.md` (status `planned` until shipped) |
| Sprint sequencing / scope / ordering for a new initiative | new sprint doc copied from `sprints/_TEMPLATE.md`; methodology in `sprints/README.md` |
| A friction event for a candidate plugin | the matching trigger log |
| A verdict on a candidate plugin | the matching decision log |
| A per-plugin gotcha or rule-temptation rejection | the per-instance doc (`languages/<name>.md` or `frameworks/<name>.md`) |
| A post-refactor work item | `POST-REFACTOR-PLAN.md` |
| An unfamiliar term used elsewhere | look up in `GLOSSARY.md`; if missing, add it |

## Outside this directory

- `README.md` (repo root) — install, CLI surface, quickstart.
- `Cargo.toml` — pinned tree-sitter grammars and crate dependencies.
- `scope-graph/src/sql/schema.sql` — current schema (R0 closed shape).
- `justfile` — CI-gate recipe wrappers; canonical paths live in `CI-GATES.md`.
