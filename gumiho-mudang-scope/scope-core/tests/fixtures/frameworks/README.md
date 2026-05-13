# Framework integration test fixtures

Sprint 0005 (R5) infrastructure fixtures.

## Layout

- `_pre_filter/{python,ruby}/` — cross-language pre-filter test
  corpus. Path resolved by sprint 0005 ambiguity #2 (see
  `ENFORCEMENT-MAP.md` § R5 → "Cross-language fixture
  location"). Underscore prefix denotes test-infrastructure-only.
- `synthetic/` — synthetic framework fixtures. Per-version subtrees
  appear here when version-pinned end-to-end tests are added.

## Current state

The R5 integration tests
(`scope-core/tests/framework_plugin_integration.rs`) construct
`Symbol` and `Edge` inputs inline rather than parsing real source
files. The dispatch layer (`scope_core::frameworks::dispatch`) is the
unit under test; language-plugin metadata population is sprint
0003's R2, already covered by per-language test corpora.

End-to-end fixtures will be added when a concrete framework adopts
post-refactor per `FRAMEWORK-PLAYBOOK.md`. Until then the directory
layout exists so future sprints have a documented place to land
fixtures.
