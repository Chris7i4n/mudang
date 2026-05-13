# Reference fixture corpus — R8 confidence audit

> **Owner**: R8 — Confidence audit subcommand.
> **Source of truth**: [`ENFORCEMENT-MAP.md` § R8](../../../../docs/ENFORCEMENT-MAP.md#r8--confidence-audit-subcommand).
> **Sample schema**: [`AUDIT-LABEL-SCHEMA.md`](../../../../docs/AUDIT-LABEL-SCHEMA.md).

---

## Purpose

The reference corpus is the input that `scope audit confidence` indexes
when computing per-`(kind, tier, producer, pattern_id)` precision. It is
the only fixture set whose precision is gated by CI (see
[`CI-GATES.md`](../../../../docs/CI-GATES.md#gate-inventory) →
Confidence audit gate, activated in this sprint).

R8 measures **precision only**. Recall is caught by integration-test
snapshots and per-framework doc walkthroughs; do not optimize this
corpus for recall coverage.

---

## Layout

One directory per `LanguageId` `db_slug` (per
[`scope-core/src/languages/id.rs`](../../../src/languages/id.rs)):

```
reference/
  csharp/
    audit-samples/
  go/
    audit-samples/
  java/
    audit-samples/
  python/
    audit-samples/
  ruby/
    audit-samples/
  rust/
    audit-samples/
  typescript/
    audit-samples/
```

- `<lang>/` — source fixtures the indexer walks (real-shape code, not
  toy snippets; see [Anonymization rules](#anonymization-rules)).
- `<lang>/audit-samples/` — committed labelled JSONL samples per
  [`AUDIT-LABEL-SCHEMA.md` § Corpus accumulation policy](../../../../docs/AUDIT-LABEL-SCHEMA.md#corpus-accumulation-policy).
  Re-running `scope audit confidence --label` against a committed
  sample reproduces the precision baseline byte-for-byte (seed pinned
  per `--seed`). A drift is a CI signal.

Labelling passes accumulate per
[`AUDIT-LABEL-SCHEMA.md` § Corpus accumulation policy](../../../../docs/AUDIT-LABEL-SCHEMA.md#corpus-accumulation-policy).

---

## Anonymization rules

The corpus must look like the kind of code Scope is asked to index in
the field, **without** carrying provenance, secrets, or anything that
ties a fixture to a specific upstream project. Each fixture file is a
plausible-shape standalone snippet, not a verbatim copy of a
real-world file.

The rules are:

1. **No verbatim third-party code.** Even MIT-licensed snippets get
   restructured (rename symbols, rewrite control flow, change literal
   values) before they enter the corpus. A fixture is a Scope-authored
   exemplar of a real-world *pattern*, not a copy of a real-world
   *file*. If a Scope contributor cannot derive the fixture from
   public language/framework documentation, do not commit it.

2. **No customer code, no closed-source code.** Includes code from
   private repositories the contributor has access to, code from past
   employers, code surfaced by AI assistants whose training set is
   opaque, and code from coding-test submissions.

3. **No secrets, credentials, or PII.** API keys, tokens, hostnames,
   email addresses, and personal names are scrubbed. Use
   conventional fixture names: `acme`, `example.com`, `Alice` /
   `Bob`, `localhost`, `123 Main St`. Replace numeric secrets with
   `0` or `42`.

4. **No license-restricted assets.** No GPL/AGPL-derived snippets;
   no proprietary asset filenames (logos, brand names).

5. **No timestamps, build IDs, or environment fingerprints.** Fixtures
   are deterministic. If a fixture must mention a version (e.g. a
   `Cargo.toml` edition or a `go.mod` directive), use the lowest
   version that exercises the relevant AST shape.

6. **Pattern-coverage, not breadth.** Each fixture targets one
   `(producer, pattern_id)` cell. Adding a fixture means we want to
   exercise a specific extractor pattern; do not bulk-import "random
   real-shape code" to inflate the sample.

When a fixture is added, the commit message states (a) which
`(producer, pattern_id)` cell it covers, and (b) which of the rules
above the contributor checked.

---

## Provenance & sample policy

Sample files committed under `<lang>/audit-samples/` follow the
corpus accumulation policy in
[`AUDIT-LABEL-SCHEMA.md` § Corpus accumulation policy](../../../../docs/AUDIT-LABEL-SCHEMA.md#corpus-accumulation-policy).
That section is the source of truth; below is the operator-side
summary.

- Each committed file is the output of an
  `scope audit confidence --emit-sample --seed N` run plus a
  labelling pass.
- Per-sample provenance is recorded in the
  `<lang>/audit-samples/MANIFEST.md` row added in the same commit
  as the JSONL file (fields: `sample_file`, `labeller_id`,
  `labelled_at`, `scope_commit`, `sample_count`).
- The commit message also names the labeller used (human / LLM /
  LSP cross-check / hybrid) and the date — the MANIFEST is the
  machine-readable record; the commit message is the human-readable
  narrative.
- Old samples are not deleted unless the underlying fixture is
  removed — a 6-month-old sample with stable precision is a
  stronger signal than a freshly labelled one.

---

## What is *not* here

- **Synthetic framework fixtures** — see
  [`../frameworks/`](../frameworks/) (R5 infrastructure).
- **Cross-language pre-filter corpus** — see
  [`../frameworks/_pre_filter/`](../frameworks/_pre_filter/).
- **Per-language extractor unit-test corpora** — those live next to
  the extractor crates and stay the way they are; the reference
  corpus exists for the audit subcommand, not for extractor unit
  tests.
- **Per-language `lang_version` detector fixtures** — `lang_version`
  is reserved in the sample schema but always `null`.
  Detector fixtures land atomically across all seven languages as
  the
  [`BACKLOG.md` § Priority 1 sub-item (d)](../../../../docs/BACKLOG.md#priority-1--self-correction-cycle)
  delivery.
