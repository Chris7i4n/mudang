# Scope Documentation

Entry point for the docs tree. If this is your first time, read in this order.

## Reading order (governing docs)

1. **`CHARTER.md`** — what Scope is, what it is not, hard limits, soft expansion zone, per-language IN/OUT. Permanent constraints; revisions require explicit charter-amendment commits per CHARTER §11.
2. **`ENFORCEMENT-MAP.md`** — rule→implementation map. R-entries (R0, R1, …) name the technique that enforces each charter / playbook rule. Mandatory end-of-sprint update gate keeps it live (see `sprints/README.md` §7.5).
3. **`LANGUAGE-PLAYBOOK.md`** — how to add and maintain a language plugin within the 18 universal boundaries.
4. **`FRAMEWORK-PLAYBOOK.md`** — how to add and maintain a framework plugin within the 15 gotcha categories. Includes version-strategy rules (A/B/C) and unknown-version policy.
5. **`POST-REFACTOR-PLAN.md`** — work queued against the current architecture, ordered by priority.
6. **`sprints/README.md`** — sprint methodology (linear order, atomic phase shipment, codex consultation, branch protocol, enforcement-map update gate). Durable for any initiative. The sprint skeleton lives in [`sprints/_TEMPLATE.md`](sprints/_TEMPLATE.md).

## Reference docs (read on demand)

- **`GLOSSARY.md`** — central term definitions across all docs (types, traits, processes, subcommands, classes of constraint).
- **`CI-GATES.md`** — single source of truth for every CI gate the architecture enforces. Each gate maps to an R-entry in `ENFORCEMENT-MAP.md`; the `justfile` recipes mirror this doc.

## Static reference

These documents govern decisions; revise via explicit commit, not silent edit.

| Document | Owns |
|---|---|
| `CHARTER.md` | Mission, hard limits, soft expansion, per-language IN/OUT, moats vs LSP, amendment rule |
| `ENFORCEMENT-MAP.md` | Rule→implementation map. R-entries (R0, R1, …) carry the durable contract for each enforcement technique |
| `sprints/README.md` | Sprint methodology — durable for any initiative |
| `sprints/_TEMPLATE.md` | Per-sprint doc skeleton |
| `LANGUAGE-PLAYBOOK.md` | 18 universal language-plugin rules, language-adoption flow, per-language doc structure |
| `FRAMEWORK-PLAYBOOK.md` | Framework adoption flow, version strategies, unknown-version policy, 15 gotcha categories |
| `POST-REFACTOR-PLAN.md` | Work queue against the current architecture |
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

The architecture is stable. Every CI gate listed in `CI-GATES.md` is `active`; `just gate-refactor` runs every gate green. Plugin authoring follows the shapes documented in `LANGUAGE-PLAYBOOK.md` Step 5. The rule→enforcement map lives in `ENFORCEMENT-MAP.md` and grows by mandatory end-of-sprint update (`sprints/README.md` §7.5).

## Where to put a new note

| Recording … | Goes in |
|---|---|
| Why a feature is permanent out-of-scope | `CHARTER.md` §5 (charter-amendment commit) |
| A new mechanically enforced rule | append a `### R<n>` to `ENFORCEMENT-MAP.md` in the same commit that ships the code (per `sprints/README.md` §7.5) |
| A new CI gate | row in `CI-GATES.md` (status `planned` until shipped) |
| Sprint sequencing / scope / ordering for a new initiative | new sprint doc copied from `sprints/_TEMPLATE.md`; methodology in `sprints/README.md` |
| A friction event for a candidate plugin | the matching trigger log |
| A verdict on a candidate plugin | the matching decision log |
| A per-plugin gotcha or rule-temptation rejection | the per-instance doc (`languages/<name>.md` or `frameworks/<name>.md`) |
| A work item against the current architecture | `POST-REFACTOR-PLAN.md` |
| An unfamiliar term used elsewhere | look up in `GLOSSARY.md`; if missing, add it |

## Outside this directory

- `README.md` (repo root) — install, CLI surface, quickstart.
- `Cargo.toml` — pinned tree-sitter grammars and crate dependencies.
- `scope-graph/src/sql/schema.sql` — current schema (R0 shape).
- `justfile` — CI-gate recipe wrappers; canonical paths live in `CI-GATES.md`.
