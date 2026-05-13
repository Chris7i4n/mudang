# Framework: <NAME>

> Replace `<NAME>` and every `<...>` placeholder. Delete this blockquote when done. Sections marked **required** must be filled before a framework plugin is considered shippable; sections marked **optional** are filled as content arises.

---

## Versions supported (required)

Framework version is **first-class** in this layer (asymmetric with language version, which is intentionally hidden from plugins per `LANGUAGE-PLAYBOOK.md` Step 4 C2). Patterns are version-gated via `semver::VersionReq` (full semver granularity, including patch — see `FRAMEWORK-PLAYBOOK.md` Step 3 "Granularity").

For each major version branch this plugin supports, state the detection rule and the version-coercion (if needed).

- `1.x` — detection: `<package>` in `<config-file>` (e.g., `package.json` + `package-lock.json`, `pyproject.toml` + `poetry.lock`, `Cargo.toml` + `Cargo.lock`, `Gemfile.lock`, `go.mod` + `go.sum`) resolves via `FrameworkWorkspaceContext::framework_versions()` to `DetectedVersion::Resolved(v)` satisfying `VersionReq` `">=1.0.0, <2.0.0"`.
- `2.x` — same path; satisfies `">=2.0.0, <3.0.0"`.

Each entry in "Patterns matched" below carries its own `available_in: VersionReq`; the plugin does not enumerate `(major, minor)` arms.

### Version coercion (required if the framework's versioning is not strict semver)

`semver::Version::parse` rejects strings like Rails `7.0.4.3` or Python `3.11.0a1`. State the coercion rule used by the framework's version reader (lives in `src/frameworks/<name>/mod.rs::detect`):

- **Rule**: `<one-line description>` — e.g., `"drop the 4th component: 7.0.4.3 → 7.0.4"` or `"strip pre-release identifiers: 3.11.0a1 → 3.11.0"`.
- **Affected versions**: `<list>` — concrete examples that exercise the rule, drawn from real maintainer projects.
- **Loss of precision**: one paragraph explaining what the coercion erases (e.g., security-patch granularity within a `7.0.4.x` line; pre-release-vs-stable distinction within a `3.11.0` window). The R8 audit cannot recover lost granularity; if a pattern depends on a coerced-away dimension, downgrade its confidence.

If the framework's versioning genuinely cannot be coerced (custom date-based, build-metadata-coupled, etc.), declare `DetectedVersion::NoVersionConcept` and justify; every `Pattern.available_in` then becomes `VersionReq::STAR`.

### Range-only manifests

When a workspace declares a range (`"^7.0"` in `package.json`, `"~> 7.0"` in `Gemfile`) without a lockfile resolving it, `Detection.version` is `DetectedVersion::Indeterminate`. The plugin's `unknown_version_policy()` (next section) decides what happens. The reader does **not** synthesize a concrete version inside the range, because doing so would silently lock in an answer the workspace has not committed to.

If the framework genuinely has no version concept (rare), state `version: DetectedVersion::NoVersionConcept` and justify in one paragraph.

---

## Unknown-version policy (required)

When the workspace config cannot resolve a clean semver (vendored fork, git dep with SHA, beta tag without parseable version, missing lockfile), `unknown_version_policy()` returns one of:

- **`Skip`** (recommended default) — emit zero domain edges; user re-runs lockfile installation to recover value
- **`StableOnlyLowConfidence`** — emit only patterns whose `available_in == VersionReq::STAR`, with `confidence=low` and `producer='framework:<name>:fallback'`
- **`AssumeLatest(<version>)`** — pretend a specific version is active; edges carry `producer='framework:<name>:assumed_<version>'`

**Adopted policy**: `Skip` | `StableOnlyLowConfidence` | `AssumeLatest(<version>)`
**Rationale**: <one paragraph; if not `Skip`, explain why the false-positive risk is acceptable>

---

## Code organization (required)

Per `ENFORCEMENT-MAP.md` R5, the plugin's source layout is fixed:

```
src/frameworks/<name>/
├── mod.rs              # FrameworkPlugin impl
├── patterns.rs         # ALL_PATTERNS: &[Pattern] — central catalog
├── predicates.rs       # the matching fns referenced from patterns.rs
└── fixtures/
    ├── v<X>_x/         # one fixture set per supported major (or per relevant minor)
    └── ...
```

Each `Pattern` in `patterns.rs` is defined once and carries:
- `id` — used in `producer.pattern_id` (R0) for audit telemetry
- `edge_kind` — the domain edge this pattern emits
- `available_in: VersionReq` — single source of truth for version applicability
- `predicate: fn(&[Symbol], &[Edge]) -> Vec<EdgeBuilder>` — the matcher fn from `predicates.rs`

---

## Applies to languages (required)

- **`applies_to_languages`**: list of languages this framework's predicate is allowed to match against, e.g., `[Ruby]` for Rails, `[TypeScript]` for React (JS files are handled via the TypeScript plugin's extension dispatch — there is no separate `JavaScript` `LanguageId` variant today), `[Python]` for Flask. The indexer pre-filters symbols/edges by this list before invoking the predicate (per [R5](../ENFORCEMENT-MAP.md#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata)); leaving it empty makes the plugin a no-op. Allowed values are the variants of `LanguageId` in `scope-core/src/languages/id.rs`.
- **Rationale**: one line on why these and not others. If a framework genuinely spans languages (e.g., NestJS supports TS and JS source), list both.

---

## Strategy (required)

- **Adopted strategy**: `A (latest only)` | `B (multi-version with detection)` | `C (declined — should not appear here if plugin exists)`
- **Rationale**: one paragraph explaining why this strategy was chosen given the maintainer's actual project usage.

If strategy ever changes, append an entry below:

```
- YYYY-MM-DD | strategy A → B | reason: project X moved to v2; project Y still on v1
```

---

## Patterns matched (required)

For every pattern this plugin emits edges for, fill out one block. Keep blocks small and concrete. Per `ENFORCEMENT-MAP.md` R5, framework plugins are predicates over `Symbol.metadata` and `Edge` rows — they do not own `.scm` queries.

### <pattern name 1>

- **Pattern ID**: `<framework>.<short_slug>` (used in `producer.pattern_id` per R0)
- **Edge kind emitted**: `<http_route | queue_handler | orm_relation | renders | hook_use | ...>`
- **`available_in`**: `VersionReq::parse("<expression>")`, e.g., `">=5.0.0"`, `">=1.0.0, <5.1.0"`, `">=7.0.0, <8.0.0"`
- **Predicate**: short prose describing the SQL/Rust matcher, e.g.,  
  `Symbol.metadata.decorators[].name matches '^app\.(get|post|put|delete|patch|route)$' AND args_text starts with a string literal`
- **Metadata keys consumed**: `decorators` | `annotations` | `template_calls` (one or more, or `none` if the predicate matches over `Symbol.name` and edges directly — e.g., naming-convention matchers like React's `^use[A-Z]` hook detection, which is matched at this layer rather than via a reserved metadata key)
- **Confidence**: `high | medium | low`
- **Example**:
  ```<lang>
  <minimal source code that triggers the match>
  ```
- **Why this confidence**: one to two sentences. If `medium` or `low`, state the failure mode that prevents `high`.

### <pattern name 2>

(same structure)

### <pattern name N>

(same structure)

---

## Patterns deliberately not matched (required if any exist)

For each pattern that exists in the framework but the plugin does not match, record the reason. This list prevents future-you from re-implementing what was previously rejected.

1. **<pattern>** — <why the plugin does not match>. Affected queries: <which Scope queries lose precision because of this gap>.
2. ...

---

## Common gotchas walkthrough (required)

Walk every category from `FRAMEWORK-PLAYBOOK.md` Step 4 (15 numbered categories). **Category titles and bodies live in the playbook, not here** — each row references the playbook's category by number; consult the playbook for the current canonical text. If the playbook is amended (categories renamed or reworded), the numeric references below remain valid.

Decisions that interact with language-plugin behavior must also respect the 18 rules in `LANGUAGE-PLAYBOOK.md` Step 4. The framework infrastructure is mechanically constrained by R5 in `ENFORCEMENT-MAP.md` (FrameworkPlugin operates on Symbols/Edges, not AST).

Decision values: `matched`, `not matched`, `matched with confidence downgrade`, or `N/A`. Use `N/A` only when the framework genuinely lacks any pattern in the category.

| Category # | Decision | Notes |
|---|---|---|
| 1 | <decision> | ... |
| 2 | <decision> | ... |
| 3 | <decision> | ... |
| 4 | <decision> | ... |
| 5 | <decision> | ... |
| 6 | <decision> | ... |
| 7 | <decision> | ... |
| 8 | <decision> | ... |
| 9 | <decision> | ... |
| 10 | <decision> | ... |
| 11 | <decision> | ... |
| 12 | <decision> | ... |
| 13 | <decision> | ... |
| 14 | <decision> | ... |
| 15 | <decision> | ... |

---

## Confidence rationale (required)

State the rules that map a pattern to a tier. This is the contract the plugin enforces.

- `<pattern>` + `<condition>` → `high`
- `<pattern>` + `<condition>` → `medium` because <reason>
- `<pattern>` + `<condition>` → `low` because <reason>

If audit later shows tier is wrong, downgrade and update this section.

---

## Test fixtures (required)

- **Real-world fixtures**: `tests/fixtures/frameworks/<name>/`
  - `v1/` — version 1.x fixtures
  - `v2/` — version 2.x fixtures
  - Minimum 5 fixtures per supported version, drawn from real maintainer projects (anonymized).
- **Integration test entry**: `tests/integration/test_framework_<name>.rs`
- **Snapshot tests** (insta): list paths to `.snap` files.

---

## Real-world precision (required)

After Step 5 of the framework playbook, record measured precision per pattern. This is the evidence that justifies the shipped tier.

| Pattern | Sample size | Correct edges | Precision | Tier shipped |
|---|---|---|---|---|
| `<pattern 1>` | 50 | 47 | 94% | high |
| `<pattern 2>` | 30 | 22 | 73% | medium (downgraded from high) |
| `<pattern 3>` | 20 | 8 | 40% | rejected (not shipped) |

Re-run the audit on every grammar or framework version bump. Append rather than overwrite.

---

## Maintenance log (required, append-only)

One entry per maintenance event.

```
- YYYY-MM-DD | <event, e.g., upstream framework released 2.4> | <action taken> | <outcome>
- YYYY-MM-DD | regression on fixture <foo> | <root cause> | <fix>
- YYYY-MM-DD | tempted to match <pattern X> | rejected because <reason> | recorded under "Patterns deliberately not matched"
```

---

## Rejected approaches (optional but recommended)

For tempting shortcuts considered and rejected during implementation:

```
### Rejected: <short name>
- Tempting because: ...
- Would violate: <CHARTER hard limit | LANGUAGE-PLAYBOOK rule ID | FRAMEWORK-PLAYBOOK Step 4 gotcha | ENFORCEMENT-MAP R-id if mechanically enforced>
- Non-negotiable because: ...
```

---

## SUNSET (filled in only when sunset; otherwise delete this section)

- **Date**: YYYY-MM-DD
- **Reason**: one paragraph
- **Last supported framework version**: `<version>`
- **Archive location**: `<path, e.g., src/frameworks/archived/<name>>`
- **Final maintenance entry**: <link to log entry that closed the plugin>
