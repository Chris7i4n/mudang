# Malformed-source fixture corpus — R6 resilience harness

> **Ships**: sprint 0008 (R6 — Malformed-source test harness).
> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R6](../../../../docs/ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness).
> **Sprint plan**: [`sprints/0008-phase-e-malformed-source-harness.md`](../../../../docs/sprints/0008-phase-e-malformed-source-harness.md).

---

## Purpose

This corpus is the input that the malformed-source integration test
(`scope-core/tests/malformed_sources.rs`, lands in chunk 4 of sprint
0008) walks to assert the [`CHARTER.md` §3 invariant 5](../../../../docs/CHARTER.md#3-core-invariants--must-never-break)
mechanical gate:

> Tree-sitter resilience. The index updates correctly even when source
> code does not compile. Mid-refactor, broken branches, generated code
> with gaps — all must produce a useful (if incomplete) index.

R6 is the mechanical gate for that invariant. For every fixture below,
the harness checks four things:

1. **No panic** — the parser must complete on the broken source.
2. **Parseable prefix produces ≥ 1 symbol** — the index is not silently
   empty just because a tail is malformed.
3. **`file_hashes.skipped_ranges` is non-empty** — the indexer must
   honestly record which line range was skipped, with reason from one
   of the three families the implementation emits:
   `tree_sitter_error:syntax_error` (tree-sitter ERROR node),
   `tree_sitter_error:missing_node` (tree-sitter MISSING node), or
   `plugin_skip:<plugin>:<rationale>` (plugin-driven skip).
   Silent drops are no longer acceptable.
4. **Snapshot test pins the recorded reason + range** — regressions
   surface as `insta` snapshot diffs. The pinned range covers the lines
   that each fixture's `expected.md` says should be skipped.

This corpus is **distinct** from the R8 reference corpus
([`../reference/`](../reference/README.md)) and from the framework
fixture corpus ([`../frameworks/`](../frameworks/)) — see
[What is *not* here](#what-is-not-here) below.

---

## Layout

One directory per `LanguageId` `db_slug` (per
[`scope-core/src/languages/id.rs`](../../../src/languages/id.rs#L80-L90)).
Inside each language directory, one directory per fixture **case**,
named after the failure category:

```
malformed/
  csharp/
    <case>/
      <broken-source>.cs
      expected.md
  go/
    <case>/
      <broken-source>.go
      expected.md
  java/
    <case>/
      <broken-source>.java
      expected.md
  python/
    <case>/
      <broken-source>.py
      expected.md
  ruby/
    <case>/
      <broken-source>.rb
      expected.md
  rust/
    <case>/
      <broken-source>.rs
      expected.md
  typescript/
    <case>/
      <broken-source>.ts   # or .tsx for JSX-flavoured cases
      expected.md
```

Each fixture directory contains exactly:

- **One broken-source file**, with the language's natural extension.
  The harness reads this file as the parser input.
- **`expected.md`** — the human-readable expectation. Pins:
  - The failure **category** name (one of the per-language categories
    below).
  - The **rationale** the contributor reached when authoring the
    fixture (one sentence — "missing closing brace at line 17 makes
    the trailing function body unparseable").
  - The **line range** the harness's snapshot should record as
    `skipped_ranges` (e.g. `lines 17–24` — inclusive, 1-indexed).
  - The **reason tag** expected in `skipped_ranges`. `expected.md`
    typically writes the family prefix (e.g. `tree_sitter_error`); the
    `insta` snapshot test in chunk 4 pins the precise subkind emitted
    by `error_scan.rs` (one of `tree_sitter_error:syntax_error`,
    `tree_sitter_error:missing_node`, or
    `plugin_skip:<plugin>:<rationale>`).

Sprint 0008 chunk 1 lands the directory skeleton (this file + per-lang
`.gitkeep` markers). Chunk 2 populates the 35-fixture floor (5 per
language × 7 languages); chunks 3–4 wire the harness against them.

---

## Fixture provenance — hand-crafted synthetic

The fixtures here are **hand-crafted synthetic**, not anonymized
real-world code. This is the inverse of the
[reference corpus](../reference/README.md#anonymization-rules)
discipline because the source signal is different:

- The reference corpus exists to measure **precision on real-shape
  code**. Real-world code is the only source signal that matters there.
- This corpus exists to **exercise the parser's recovery surface**.
  Real-world broken code is rare, inconsistent, and uncontrolled —
  hand-crafted synthetic fixtures expose every recovery path the
  grammar surfaces, with the failure mode pinned to a known line range
  per `expected.md`.

The contract:

1. **Hand-crafted, not copied.** Every fixture is authored from
   scratch to target one specific grammar failure mode. Do not
   "scrape broken code from a real branch and check it in" — provenance
   is opaque and the failure pattern shifts under us.
2. **One failure mode per fixture.** Compose the broken source so the
   parser's recovery diagnostic is unambiguous: one truncation, one
   unbalanced brace, one EOF-in-string. A fixture with two interacting
   failures hides which recovery path the snapshot is pinning.
3. **Parseable prefix.** The portion of the source preceding the
   failure must be valid for its language, exercise at least one
   symbol-producing node (function / class / module / etc.), and be
   long enough that the harness's "≥ 1 symbol" assertion is a real
   signal, not a near-empty-tree fluke.
4. **No secrets, credentials, or PII.** Same convention as the
   reference corpus — use `acme`, `example.com`, `Alice` / `Bob`,
   `localhost`, `42`.

When a fixture is added, the commit message states (a) the category
name, (b) the expected recovery-reason family
(`tree_sitter_error` / `plugin_skip:…` — the family prefix; the snapshot
pins the precise subkind), and (c) which of rules 1–4 above the
contributor checked.

---

## Per-language category coverage (5-fixture floor)

The 5-fixture floor per language is **mechanical** (CI counts).
**Category selection is editorial** — driven by each language's
grammar. The shared base set from
[`ARCHITECTURAL-REFACTOR.md` § R6](../../../../docs/ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness)
is the starting point; each language records its actual five
categories in the per-language section below as fixtures land.

Shared base categories (when the language admits them):

- **truncated mid-function** — function body abruptly EOFs.
- **unbalanced delimiters** — missing closing `}` / `)` / `]` /
  `end` / etc. for languages that have such tokens.
- **eof in string** — unterminated string / heredoc / raw string.
- **mixed indent collapse** — Python-style indentation rule violated
  (Python only).
- **missing close tag in JSX** — `<div>` opened, never closed
  (TS/TSX only).

Per-language category index (filled in as chunk 2 lands fixtures):

| Language | 5 categories selected | Notes |
|---|---|---|
| C# | `truncated-mid-method`, `unbalanced-brace`, `eof-in-string`, `eof-in-verbatim-string`, `eof-in-attribute-list` | Brace-balanced grammar; verbatim string exercises C#-specific multi-line `@"..."` recovery, attribute list exercises bracketed-call recovery in attribute position. |
| Go | `truncated-mid-function`, `unbalanced-brace`, `eof-in-raw-string`, `missing-closing-paren-on-call`, `eof-in-import-block` | Brace-balanced; raw-string variant covers backtick recovery, import-block variant covers grouped-import recovery, paren-on-call covers Sprintf-style multi-argument recovery. |
| Java | `truncated-mid-method`, `unbalanced-brace`, `eof-in-string`, `eof-in-annotation`, `eof-in-generics-angle` | Brace-balanced; annotation variant covers `@Route(...)` element-value-pair recovery, generics variant covers the LL(1)-ambiguous `<` `>` recovery stress point. |
| Python | `truncated-mid-function`, `mixed-indent-collapse`, `eof-in-triple-string`, `eof-in-bracketed-call`, `eof-in-decorator` | No braces — indentation is the structural delimiter. Mixed-indent fills the "unbalanced delimiters" slot; bracketed-call + decorator variants exercise the implicit-line-continuation recovery surface. |
| Ruby | `truncated-mid-method`, `unbalanced-end`, `eof-in-heredoc`, `eof-in-regex`, `eof-in-string-interpolation` | `end`-balanced; heredoc variant covers `<<~HTML ... HTML` recovery, regex variant covers `/.../` delimiter ambiguity recovery, interpolation variant covers cascading `"#{...}"` recovery. |
| Rust | `truncated-mid-function`, `unbalanced-brace`, `eof-in-raw-string`, `eof-in-macro-body`, `eof-in-generics-angle` | Brace-balanced; raw-string variant covers `r#"..."#` recovery, macro-body variant covers permissive token-tree recovery, generics variant covers the LL(1)-ambiguous `<`/`>` recovery stress point. |
| TypeScript | `truncated-mid-function`, `unbalanced-brace`, `eof-in-template-literal`, `missing-close-tag-jsx`, `eof-in-type-annotation` | Brace-balanced + JSX (TSX-only). Template-literal variant covers backtick + `${...}` interpolation recovery; JSX variant covers element-tag recovery (TSX-only fixture, `source.tsx`); generics variant covers the LL(1)-ambiguous `<`/`>` recovery stress point. |

The table is editorial; chunk 2 finalises the picks and rewrites the
`_TBD chunk 2_` cells with the actual fixture-directory names.

---

## What is *not* here

- **Anonymized real-world fixtures for precision audit** — those live
  in [`../reference/`](../reference/README.md) and are the R8 input
  (sprint 0007).
- **Framework adoption fixtures** — synthetic per-framework fixtures
  live in [`../frameworks/`](../frameworks/) (R5 infrastructure,
  sprint 0005). Framework fixtures exercise call-pattern walkthroughs;
  this corpus exercises parser recovery.
- **Adoption-time per-language real-shape fixtures** — covered by
  [`LANGUAGE-PLAYBOOK.md` Step 5 item 6](../../../../docs/LANGUAGE-PLAYBOOK.md#step-5--implementation-procedure-within-bounds)
  ("Build 5+ real-world fixtures"). Those land per-language at
  adoption time, not in this sprint.
- **Cross-language pre-filter corpus** — see
  [`../frameworks/_pre_filter/`](../frameworks/_pre_filter/).
- **Byte-level invalid-UTF-8 fixtures** — explicitly out of scope per
  [`ARCHITECTURAL-REFACTOR.md` § R6](../../../../docs/ARCHITECTURAL-REFACTOR.md#r6--malformed-source-test-harness)
  ("invalid UTF-8 at the byte level"). Trigger-deferred to a
  post-refactor effort
  ([`POST-REFACTOR-PLAN.md` § Items deliberately deferred](../../../../docs/POST-REFACTOR-PLAN.md#items-deliberately-deferred-beyond-this-plan)).
