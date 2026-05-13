# Framework integration test fixtures

R5 infrastructure fixtures.

## Layout

- `_pre_filter/{python,ruby}/` — cross-language pre-filter test
  corpus. Path resolved by ambiguity resolution (see
  `ENFORCEMENT-MAP.md` § R5 → "Cross-language fixture
  location"). Underscore prefix denotes test-infrastructure-only.
- `synthetic/` — synthetic framework fixtures. Per-version subtrees
  appear here when version-pinned end-to-end tests are added.

## Current state

The R5 integration tests
(`scope-core/tests/framework_plugin_integration.rs`) construct
`Symbol` and `Edge` inputs inline rather than parsing real source
files. The dispatch layer (`scope_core::frameworks::dispatch`) is the
unit under test; language-plugin metadata population belongs to R2
and is already covered by per-language test corpora.

End-to-end fixtures will be added when a concrete framework adopts
per `FRAMEWORK-PLAYBOOK.md`. Until then the directory layout exists
so future work has a documented place to land fixtures.
