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
| Priority 1 (d) — `lang_version` detector matrix | 0003 | shipped | `selfcorrect/sprint-0003-lang-version-detector-matrix` | `cc61a5b` | 2026-05-13 |
| Priority 1 (e) — Labelled corpus accumulation policy | 0002 | shipped | `selfcorrect/sprint-0002-corpus-accumulation-policy` | `467c356` | 2026-05-13 |
| Priority 1 (f) — ML-driven patch suggester | 0008 | unstarted | — | — | corpus-size-gated (≥1000 samples × ≥4 langs) |
| Priority 1 (g) — Richer auditor verdict types | 0004 | in-progress | `selfcorrect/sprint-0004-schema-v2-bundle` | — | bundled with (h) + (j) |
| Priority 1 (h) — Per-group coverage on report | 0004 | in-progress | `selfcorrect/sprint-0004-schema-v2-bundle` | — | bundled with (g) + (j) |
| Priority 1 (i) — Multi-labeller verdict aggregation | 0006 | unstarted | — | — | absorbs deferred `audit history labeller <id>` + `agreement-matrix` views from sprint 0004 (commit `db4c3ac`) |
| Priority 1 (j) — Audit-history DB persistence | 0004 | in-progress | `selfcorrect/sprint-0004-schema-v2-bundle` | — | bundled with (g) + (h); `audit history labeller` + `agreement-matrix` views deferred to 0006 |
| Priority 1 (k) — Confidence re-stamping policy | 0009 | unstarted | — | — | ships last; riskiest |
| Doc-sync gate scaffolding (cross-cutting) | 0001 | shipped | `selfcorrect/sprint-0001-loop-architecture` | `c06a23d` | new R-entry R13; 4 checks active |

| Sprint | Status | Branch | Merged-on commit |
|---|---|---|---|
| 0001 — loop architecture + doc-sync gate | shipped | `selfcorrect/sprint-0001-loop-architecture` | `c06a23d` |
| 0002 — corpus accumulation policy | shipped | `selfcorrect/sprint-0002-corpus-accumulation-policy` | `467c356` |
| 0003 — `lang_version` detector matrix | shipped | `selfcorrect/sprint-0003-lang-version-detector-matrix` | `cc61a5b` |
| 0004 — schema-v2 bundle (g+h+j) | in-progress | `selfcorrect/sprint-0004-schema-v2-bundle` | — |
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
- 2026-05-13 | Priority 1 (d) | unstarted → in-progress | branch `selfcorrect/sprint-0003-lang-version-detector-matrix` | notes: sprint 0003 opened; user authorised checkpoint-style execution (one branch, multi-commit; review per checkpoint) given conceptual surface touched (C2 boundary clarification before code work)
- 2026-05-13 | Priority 1 (d) | in-progress → shipped | commit `cc61a5b` | notes: sprint 0003 closed; 9 workspace readers (4 extended + 5 new) expose version-extraction free functions under the R4 indexer-side carveout; `lang_version.rs` dispatcher walks file → manifest with per-lang priority chains; R8 audit emit (`SampleRecord.from_row`) populates `lang_version` from the detector and `label_pass` drift check recomputes-and-compares; doc-sync gate `check_lang_version_detector_modules` added (bidirectional: variant ↔ CHARTER §7 subsection); 7 per-language docs gained a `lang_version` gotcha entry; **lang_version coverage gate active** — `test_audit_confidence_lang_version_coverage_all_seven_languages` runs full `mudang init + index + audit emit` against one canonical fixture per supported language, asserts every emitted record carries the manifest-declared `lang_version` (closes sprint plan acceptance bullet "R8 audit emit writes a non-`null` `lang_version` for every supported language under realistic fixtures" and DoD "R8 emit on the reference fixture corpus shows zero `null` `lang_version`"); independent reviewer subagent verdict MERGE-READY WITH NON-BLOCKERS, all 6 non-blockers addressed in one fix-round; gates green (`gate-doc-sync` 6 checks, `gate-charter`, `ci-context-shape`, `audit-confidence` 24 integration tests, 173 per-language tests unchanged); codex paused this sprint
- 2026-05-13 | sprint 0003 | in-progress → shipped | commit `cc61a5b` | notes: branch ready for FF merge to main
- 2026-05-13 | sprint 0003 | merged | commit `6e6b9d5` | notes: FF merge to `main` complete; branch `selfcorrect/sprint-0003-lang-version-detector-matrix` deleted post-merge per single-operator hygiene
- 2026-05-13 | sprint 0004 prep | doc amendment | commit `db4c3ac` | notes: `scope audit history` flag-surface decision recorded on `main` ahead of sprint open per ambiguity protocol; three-form layout (default dashboard + `edge <id>` + `pattern <id>`) committed for sprint 0004; `labeller <id>` solo timeline + `agreement-matrix` views deferred to sprint 0006 (i) under single-operator-posture reasoning (CHARTER §3 invariant 1); BACKLOG (j) + BACKLOG (i) + sprint-0004 plan + sprint-0006 plan + GLOSSARY (new entries: `labeller_id`, `edge_audit_history`, `coverage_ratio`, `coverage_summary`, `agreement matrix`, `scope audit history`) all amended in one commit; doc-sync gate green
- 2026-05-13 | Priority 1 (g) | unstarted → in-progress | branch `selfcorrect/sprint-0004-schema-v2-bundle` | notes: sprint 0004 opened — schema-v2 bundle (g + h + j) per BACKLOG mandate; codex review resumes this sprint after the 0002/0003 pause
- 2026-05-13 | Priority 1 (h) | unstarted → in-progress | branch `selfcorrect/sprint-0004-schema-v2-bundle` | notes: bundled with (g) + (j); per-group coverage on the precision report
- 2026-05-13 | Priority 1 (j) | unstarted → in-progress | branch `selfcorrect/sprint-0004-schema-v2-bundle` | notes: bundled with (g) + (h); `edge_audit_history` writable namespace + `scope audit history` three-form subcommand
- 2026-05-13 | sprint 0004 CP2 fix | charter realignment | branch `selfcorrect/sprint-0004-schema-v2-bundle` | notes: stripped dual-version shim introduced in CP2 (`ACCEPTED_SAMPLE_SCHEMA_VERSIONS`, `REQUIRED_FIELDS_V1`, version-branching in `label_pass`, `#[serde(default)]` on v2 fields) — direct charter §3 invariant 1 violation. `--label` now accepts exactly `SAMPLE_SCHEMA_VERSION`; v1 records get the same "unknown schema_version" reject as a forward bump. BACKLOG (g) clause "labellers continue to work — accepts both versions" reverted in same commit; AUDIT-LABEL-SCHEMA.md § Versioning rules rewritten as hard-cutover (wipe corpus + reindex + re-emit + re-label); sprint plan acceptance bullet reverted; sprint plan § Single-operator-posture rewritten. Integration test `test_audit_confidence_label_accepts_v1_records_backward_compatible` → `test_audit_confidence_label_rejects_v1_records_single_operator_posture`. Tests: 34 unit + 26 integration pass; doc-sync gate green.
