# Priority 1 — Self-correction cycle state

State-tracking doc for the Priority 1 initiative ([`BACKLOG.md` § Priority 1 — Self-correction cycle](./BACKLOG.md#priority-1--self-correction-cycle)). Reporting hook target for sprints 0001–0009 per [`sprints/README.md` § 4](./sprints/README.md#4-reporting-hooks).

Status values: `unstarted` · `in-progress` · `shipped`.

Initiative prefix: `selfcorrect/`. Merge mode: rebase-merge (sprint → `main`; direct, no phase integration branches — every sprint in Priority 1 is single-sprint per its own plan doc).

---

## Snapshot

| Sub-item | Owning sprint | Status | Branch | Commit | Notes |
|---|---|---|---|---|---|
| Priority 1 (a) — Loop architecture document | 0001 | shipped | `selfcorrect/sprint-0001-loop-architecture` | `c06a23d` | 2026-05-13 |
| Priority 1 (b) — Reference labeller crates | 0005 | unstarted | — | — | — |
| Priority 1 (c) — Continuous re-audit in CI | 0007 | unstarted | — | — | — |
| Priority 1 (d) — `lang_version` detector matrix | 0003 | unstarted | — | — | — |
| Priority 1 (e) — Labelled corpus accumulation policy | 0002 | shipped | `selfcorrect/sprint-0002-corpus-accumulation-policy` | `467c356` | 2026-05-13 |
| Priority 1 (f) — ML-driven patch suggester | 0008 | unstarted | — | — | corpus-size-gated (≥1000 samples × ≥4 langs) |
| Priority 1 (g) — Richer auditor verdict types | 0004 | unstarted | — | — | bundled with (h) + (j) |
| Priority 1 (h) — Per-group coverage on report | 0004 | unstarted | — | — | bundled with (g) + (j) |
| Priority 1 (i) — Multi-labeller verdict aggregation | 0006 | unstarted | — | — | — |
| Priority 1 (j) — Audit-history DB persistence | 0004 | unstarted | — | — | bundled with (g) + (h) |
| Priority 1 (k) — Confidence re-stamping policy | 0009 | unstarted | — | — | ships last; riskiest |
| Doc-sync gate scaffolding (cross-cutting) | 0001 | shipped | `selfcorrect/sprint-0001-loop-architecture` | `c06a23d` | new R-entry R13; 4 checks active |

| Sprint | Status | Branch | Merged-on commit |
|---|---|---|---|
| 0001 — loop architecture + doc-sync gate | shipped | `selfcorrect/sprint-0001-loop-architecture` | `c06a23d` |
| 0002 — corpus accumulation policy | shipped | `selfcorrect/sprint-0002-corpus-accumulation-policy` | `467c356` |
| 0003 — `lang_version` detector matrix | unstarted | — | — |
| 0004 — schema-v2 bundle (g+h+j) | unstarted | — | — |
| 0005 — reference labeller crates | unstarted | — | — |
| 0006 — multi-labeller aggregation | unstarted | — | — |
| 0007 — continuous re-audit in CI | unstarted | — | — |
| 0008 — ML patch suggester | unstarted | — | — |
| 0009 — confidence re-stamping policy | unstarted | — | — |

---

## Log

Append-only. Newest entry at the bottom.

- 2026-05-13 | initiative open | — | commit pending | notes: state-tracking doc created on `main` ahead of sprint 0001 open
- 2026-05-13 | Priority 1 (a) | unstarted → in-progress | branch `selfcorrect/sprint-0001-loop-architecture` | notes: sprint 0001 opened
- 2026-05-13 | Doc-sync gate | unstarted → in-progress | branch `selfcorrect/sprint-0001-loop-architecture` | notes: sprint 0001 ships scaffolding
- 2026-05-13 | Priority 1 (a) | in-progress → shipped | commit `c06a23d` | notes: sprint 0001 closed; SELF-CORRECTION-CYCLE.md landed; cross-links from docs/README.md; codex review converged round 3 (2 findings addressed at `62c1c7a` + `c06a23d`)
- 2026-05-13 | Doc-sync gate | in-progress → shipped | commit `c06a23d` | notes: R13 entry + 4 checks active (enforcement-map-paths, ci-gates-recipes, doc-relative-links w/ anchor validation, cycle-docs-indexed); extension protocol documented in SELF-CORRECTION-CYCLE.md § Extending the doc-sync gate
- 2026-05-13 | sprint 0001 | in-progress → shipped | commit `c06a23d` | notes: branch ready for PR + merge to main
- 2026-05-13 | Priority 1 (e) | unstarted → in-progress | branch `selfcorrect/sprint-0002-corpus-accumulation-policy` | notes: sprint 0002 opened
- 2026-05-13 | Priority 1 (e) | in-progress → shipped | commit `467c356` | notes: sprint 0002 closed; `AUDIT-LABEL-SCHEMA.md § Corpus accumulation policy` landed; per-`<db_slug>/audit-samples/MANIFEST.md` provenance scaffold; doc-sync gate extended with `check_audit_samples_layout`; general-purpose subagent review converged after one fix-round (3 findings addressed in the same commit; codex paused this sprint)
- 2026-05-13 | sprint 0002 | in-progress → shipped | commit `467c356` | notes: branch ready for FF merge to main
