# Language: <NAME>

> Replace `<NAME>` and every `<...>` placeholder. Delete this blockquote when done. Sections marked **required** must be filled before a language plugin is considered shippable; sections marked **optional** are filled as content arises.

---

## Tree-sitter grammar (required)

- **Crate / package**: `tree-sitter-<name>` (version `<X.Y.Z>`)
- **Source**: `<repo URL or crates.io link>`
- **License**: `<MIT | Apache-2.0 | ...>`
- **Maturity assessment**: `<stable | maturing | experimental>` — one paragraph describing release cadence, upstream activity, breaking-change history.
- **Known grammar gaps**: list AST shapes that are absent, wrong, or unstable. Each gap = one bullet describing the impact on Scope's queries.

---

## Depth target (required)

- **Level**: `surface` | `depth`
- **Post-refactor depth queue**: `yes` | `no`. Depth feature work resumes only after `ARCHITECTURAL-REFACTOR.md` ships; until then, mark `yes` if the language is on the depth track and queue the items here.
- **Queued depth items** (only fill if depth queue is `yes`):
  ```
  - <one-line description of a depth item>, e.g., "pub use chain following via static text"
  - ...
  ```
- **Promotion / demotion history**: append-only. Empty initially.
  ```
  - YYYY-MM-DD | adopted as <surface | depth> | reasoning: ...
  ```

---

## Symbol kinds emitted (required)

List every kind this plugin can produce, with the `.scm` capture group that drives it.

| Kind | Capture | Notes |
|---|---|---|
| function | `@function` | ... |
| class | `@class` | ... |
| method | `@method` | ... |
| struct | `@struct` | ... |
| enum | `@enum` | ... |
| variant | `@variant` | enum variants |
| interface | `@interface` | ... |
| type | `@type` | type aliases (`type Foo = ...`); kind name matches `src/sql/schema.sql` (R0 keeps `type`; the rename to `type_alias` is deferred to a follow-up migration) |
| const | `@const` | constants (`const FOO = 1`); kind name matches `src/sql/schema.sql` (R0 keeps `const`; the rename to `constant` is deferred to a follow-up migration) |
| property | `@property` | ... |
| trait | `@trait` | post-R0 only — until R0 ships, Rust traits are coerced to `interface` per `src/languages/rust_lang.rs:44` |
| module | `@module` | post-R0 only — until R0 ships, Ruby modules are coerced to `interface` |
| macro | `@macro` | post-R0 only; definition only — no expansion (see C1) |

Remove rows that do not apply. Add rows for language-specific kinds, but do not invent kinds outside the `symbols.kind` whitelist (see `ARCHITECTURAL-REFACTOR.md` R0 for the post-refactor whitelist; the schema source of truth is `src/sql/schema.sql`). Pre-R0 the whitelist is 10 kinds (`function`, `class`, `method`, `interface`, `struct`, `enum`, `const`, `type`, `property`, `variant`); R0 adds `trait`, `module`, `macro`.

---

## Edge kinds emitted (required)

For each edge kind, state the confidence rationale. Do not list a kind that this plugin never emits.

| Kind | Confidence | Rationale |
|---|---|---|
| calls | high | Direct call expression with resolvable callee in scope. |
| imports | high | Explicit import statement. |
| contains | high | Lexical containment (universal — every language plugin must emit this for nested definitions). |
| references_type | high \| medium | Declared type annotation. Forward refs and string-based types: medium. |
| extends | high | Explicit class/interface inheritance syntax. |
| implements | high \| medium | Direct `impl Trait for Type` (Rust): high. Method-set comparison (Go): medium. Document case-by-case. |
| inherits_from | high | Type embedding (Go), mixin (where supported). |

Language-specific edges (e.g., `goroutine_spawn`, `channel_send`, `hook_use`) are added here when applicable; each must already exist in the `edges.kind` whitelist (see `ARCHITECTURAL-REFACTOR.md` R0 for the post-refactor whitelist; the schema source of truth is `src/sql/schema.sql`).

---

## Metadata schema (required)

The plugin populates `Symbol.metadata` with the three reserved framework-primitive keys per `LANGUAGE-PLAYBOOK.md` Step 5. Mark each key:
- `populated` — plugin emits this key when AST contains a matching instance
- `omitted (N/A in language)` — language has no AST shape for this concept
- `NEEDS REVIEW` — placeholder; ship blocker

| Key | Status | Notes (which AST node populates this) |
|---|---|---|
| `decorators` | `populated` \| `omitted (N/A)` \| `NEEDS REVIEW` | e.g., `decorator` node in Python, `@decorator` in TS |
| `annotations` | ... | e.g., `attribute_item` in Rust, `annotation` in Java |
| `template_calls` | ... | template/component invocation nodes — e.g., JSX in TS/TSX, ERB partial calls in Ruby, Jinja `{% include %}` / `{% extends %}` in Python, HEEx function components in Elixir; `omitted (N/A)` for languages whose grammar exposes no template/component-call shape |

A plugin with any `NEEDS REVIEW` entry is not shippable. Every `populated` key must have at least one fixture exercising the field. The earlier `hooks` key was removed — naming-convention regexes (e.g., React `^use[A-Z]`) are framework-plugin territory per E2; do not pre-compute them in the language plugin.

---

## Universal boundaries — compliance log (required)

For each of the 18 rules in `LANGUAGE-PLAYBOOK.md` Step 4, record compliance status. **Rule bodies live in the playbook, not here.** The IDs below match the playbook's section; cross-reference for the current canonical text. The corresponding mechanical-enforcement move (R0–R12) lives in `ARCHITECTURAL-REFACTOR.md`; cite it when explaining how compliance is achieved.

This section deliberately does not restate the rule text. If the playbook is amended, the IDs below remain valid and the cross-reference always resolves to the current wording — no per-language doc needs to be touched.

Status values:
- `trivially compliant` — the plugin does not even attempt the forbidden behavior.
- `compliant by design` — the plugin had a tempting shortcut but explicitly rejected it.
- `mechanically enforced` — compliance is ensured by the architecture (R-id from `ARCHITECTURAL-REFACTOR.md`); cite the move.
- `NEEDS REVIEW` — placeholder; ship blocker.

One to three lines per rule.

### Category A — Type system

- **A1**: `<status>` — <reasoning>
- **A2**: `<status>` — <reasoning>
- **A3**: `<status>` — <reasoning>

### Category B — Runtime semantics

- **B1**: `<status>` — <reasoning>
- **B2**: `<status>` — <reasoning>
- **B3**: `<status>` — <reasoning>

### Category C — Macros, templates, version semantics

- **C1**: `<status>` — <reasoning>
- **C2**: `<status>` — <reasoning>

### Category D — Resolution discipline

- **D1**: `<status>` — <reasoning>
- **D2**: `<status>` — <reasoning>
- **D3**: `<status>` — <reasoning>

### Category E — Output discipline

- **E1**: `<status>` — <reasoning>
- **E2**: `<status>` — <reasoning>
- **E3**: `<status>` — <reasoning>

### Category F — Architecture discipline

- **F1**: `<status>` — <reasoning>
- **F2**: `<status>` — <reasoning>
- **F3**: `<status>` — <reasoning>
- **F4**: `<status>` — <reasoning>

A plugin with any `NEEDS REVIEW` entry is not shippable.

---

## Known gotchas (required if any exist)

Numbered list. Each entry: title, description, decision, reasoning. Keep concise.

1. **<gotcha title>** — <one-line description of the problem>.
   - Decision: matched | not matched | matched with confidence downgrade.
   - Reasoning: <one to two sentences>.

2. ...

---

## Rejected approaches (optional but recommended)

For each tempting shortcut considered and rejected during implementation, record:

- What looked tempting (the shortcut).
- Which Step 4 rule it would have violated.
- Why the rejection is non-negotiable.

This list exists so future-you does not re-debate the same shortcut.

```
### Rejected: <short name>
- Tempting because: ...
- Would violate: <rule ID, e.g., A3 from LANGUAGE-PLAYBOOK Step 4, or hard limit from CHARTER section 5; cite ARCHITECTURAL-REFACTOR R-id if the rule is mechanically enforced>
- Non-negotiable because: ...
```

---

## Test fixtures (required)

- **Real-world fixtures**: `tests/fixtures/languages/<name>/`
  - List each fixture file and what it exercises.
  - Minimum 5 fixtures derived from real maintainer projects (anonymized).
- **Integration test entry**: `tests/integration/test_<name>.rs`
- **Snapshot tests** (insta): list paths to `.snap` files.

---

## Maintenance log (required, append-only)

One entry per maintenance event. Include grammar bumps, regressions, and rule-temptation rejections.

```
- YYYY-MM-DD | grammar bump tree-sitter-<name> X.Y.Z → X.Y.Z+1 | status: clean | notes: ...
- YYYY-MM-DD | regression on fixture <foo> | root cause: ... | fix: ...
- YYYY-MM-DD | rejected shortcut for <feature> | rule violated: <ID> | recorded above
```

---

## Real-world precision (optional, recommended)

If you ran a precision audit, record results so future audits can detect drift.

| Edge kind | Sample size | Correct | Precision | Confidence shipped |
|---|---|---|---|---|
| calls | 50 | 48 | 96% | high |
| references_type | 40 | 36 | 90% | high |
| ... | ... | ... | ... | ... |

---

## SUNSET (filled in only when sunset; otherwise delete this section)

- **Date**: YYYY-MM-DD
- **Reason**: <one paragraph>
- **Last supported grammar version**: `tree-sitter-<name> X.Y.Z`
- **Archive location**: `<path>`
- **Final maintenance entry**: <link to maintenance log entry that closed the plugin>
