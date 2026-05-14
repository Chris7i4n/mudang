# Audit Label Schema

> **Source of truth**: [`ENFORCEMENT-MAP.md` § R8](ENFORCEMENT-MAP.md#r8--confidence-audit-subcommand).
> **Contract grade**: schema version is the wire contract for any external labeller (LLM, LSP cross-check, hybrid human-in-the-loop). Adding a field is a `schema_version` bump; removing one is charter-grade.

---

## Purpose

R8's `scope audit confidence` produces a **JSONL sample file** that any external labeller can fill in. Scope itself runs no labeller — the file format is the boundary. This document is the contract.

The same file is also the only mechanism by which a labelled sample re-enters the audit: `--label <path>` reads it back and produces the precision report.

This makes the audit loop **pluggable by design**:

```
scope audit confidence --emit-sample sample.jsonl   →   <external labeller>   →   scope audit confidence --label sample.jsonl
                                                         ↑ human, LLM, LSP, hybrid — Scope does not care
```

---

## File format

`*.jsonl` — one JSON object per line, no nesting at the file level. Empty lines and comment lines (lines starting with `#`) are ignored on read.

A header is **not** part of the format; `schema_version` lives inline on every record. Future readers detect schema drift per record, not per file.

---

## Record schema (`schema_version: "2"`)

Current emission and accepted version is `"2"`. Per single-operator posture ([`CHARTER.md` § 3 invariant 1](CHARTER.md#3-core-invariants--must-never-break)) there is exactly one accepted version on read — `--label` rejects any other value with the same "unknown schema_version" diagnostic that fires on a forward bump. The remediation is always wipe-and-reindex + re-emit; no dual-read shim ever ships.

Each line is a JSON object with the following fields:

| Field | Type | Required | Description |
|---|---|---|---|
| `schema_version` | `string` | yes | Always `"2"` on emit; readers must reject unknown values. |
| `edge_id` | `string` | yes | Stable surrogate edge id from the indexed graph (`edges.edge_id` per R0). The labeller is **not** required to interpret this; it is round-tripped verbatim. |
| `kind` | `string` | yes | EdgeKind from the R0 whitelist (e.g. `calls`, `imports`, `extends`, `implements`). |
| `confidence` | `string` | yes | One of `high` / `medium` / `low` (the producer's stamp; this is what R8 audits). |
| `producer` | `string` | yes | The producing plugin's identifier (e.g. `rust`, `python`, `typescript`, `framework:rails`). |
| `pattern_id` | `string` | yes | The pattern within the producer that emitted this edge (e.g. `rust.calls.method`, `python.imports.from`, `rails.routes.draw`). |
| `from` | `string` | yes | Source symbol id or name (the `edges.from_id` resolver input). |
| `to` | `string` | yes | Target symbol id or name (the `edges.to_id` resolver output; may be unresolved for Dangling edges). |
| `source_snippet` | `string` | yes | The relevant source text (typically the call site or definition site). Single-line preferred; multi-line allowed. Used by the labeller as the primary context. |
| `lang_version` | `string \| null` | yes | Per-project language version, populated by the `lang_version.rs` dispatcher when a per-language detector resolves it (sprint 0003). Labellers must accept `null` and may use the value as additional context when present. |
| `label` | `boolean \| null` | yes | `null` on emit, `true` (correct) or `false` (incorrect) on label. The labeller fills this. Any value other than `null` / `true` / `false` is rejected by `--label`. |
| `evidence` | `object \| null` | yes | Labeller-supplied structured evidence behind the verdict. Schema is labeller-defined; conventional keys: `{"resolver": "rust-analyzer", "target_uri": "...", "definition_range": [...]}` for LSP cross-check; `{"model": "claude-sonnet-4-6", "reasoning": "...", "prompt_hash": "..."}` for LLM. Records the *how*, not just the *what*. `null` on emit; capable labellers populate. |
| `target_proposed` | `string \| null` | yes | Labeller's correction for `to`. *"Scope said `to = foo::bar`; I see this call resolves to `foo::baz` instead."* Feeds the patch suggester to localise the extractor bug. `null` on emit; populated only when the labeller disagrees with `to`. |
| `kind_proposed` | `string \| null` | yes | Labeller's correction for `kind`. *"Scope said `references_type`; this is actually `calls`."* `null` on emit. |
| `confidence_proposed` | `string \| null` | yes | Labeller's correction for `confidence`. Distinct from a binary "wrong" verdict: the labeller may agree the edge is correct but say the confidence stamp is overstated (or understated). `null` on emit. |
| `reasoning_text` | `string \| null` | yes | Free-text human (or LLM) explanation. The post-hoc audit trail when a `false` verdict is reviewed months later. `null` on emit. |
| `lang_version_evidence` | `string \| null` | yes | Labeller's annotation distinguishing detected vs declared `lang_version` (e.g. `"detected:Cargo.toml#edition"` vs `"inferred:syntax-2021"`). `null` on emit. |
| `labeller_id` | `string \| null` | yes | Identifier of which labeller produced this verdict, for multi-labeller aggregation (Priority 1 sub-item (i)). Conventional values: `"lsp:rust-analyzer"`, `"llm:claude-sonnet-4-6"`, `"human:<initials>"`. `null` on emit; capable labellers populate. Matches the `MANIFEST.md` `labeller_id` column (see [§ Provenance record (`MANIFEST.md`)](#provenance-record-manifestmd)). |

### Partial-population semantics

The seven labeller-fillable fields (`evidence`, `target_proposed`, `kind_proposed`, `confidence_proposed`, `reasoning_text`, `lang_version_evidence`, `labeller_id`) are designed for **partial population**. A labeller that can only fill `target_proposed` leaves the rest `null`; `--label` tolerates this. Aggregators (sprint 0006) fuse partial verdicts from heterogeneous labellers into a single record. A future tightening that makes any of these required is a `schema_version` bump — never a silent contract change.

---

## Example — unlabelled (`--emit-sample` output)

```jsonl
{"schema_version":"2","edge_id":"e-9f2c","kind":"calls","confidence":"high","producer":"rust","pattern_id":"rust.calls.method","from":"crate::handlers::greet","to":"crate::utils::format_name","source_snippet":"format_name(&user.name)","lang_version":"2021","label":null,"evidence":null,"target_proposed":null,"kind_proposed":null,"confidence_proposed":null,"reasoning_text":null,"lang_version_evidence":null,"labeller_id":null}
{"schema_version":"2","edge_id":"e-3a17","kind":"extends","confidence":"high","producer":"typescript","pattern_id":"ts.extends.class","from":"components/Button.tsx::PrimaryButton","to":"components/Button.tsx::BaseButton","source_snippet":"class PrimaryButton extends BaseButton {","lang_version":"es2022","label":null,"evidence":null,"target_proposed":null,"kind_proposed":null,"confidence_proposed":null,"reasoning_text":null,"lang_version_evidence":null,"labeller_id":null}
```

## Example — labelled (`--label` input)

```jsonl
{"schema_version":"2","edge_id":"e-9f2c","kind":"calls","confidence":"high","producer":"rust","pattern_id":"rust.calls.method","from":"crate::handlers::greet","to":"crate::utils::format_name","source_snippet":"format_name(&user.name)","lang_version":"2021","label":true,"evidence":{"resolver":"rust-analyzer","target_uri":"file:///crate/src/utils.rs","definition_range":[12,4,18,5]},"target_proposed":null,"kind_proposed":null,"confidence_proposed":null,"reasoning_text":null,"lang_version_evidence":"detected:Cargo.toml#edition","labeller_id":"lsp:rust-analyzer"}
{"schema_version":"2","edge_id":"e-3a17","kind":"extends","confidence":"high","producer":"typescript","pattern_id":"ts.extends.class","from":"components/Button.tsx::PrimaryButton","to":"components/Button.tsx::BaseButton","source_snippet":"class PrimaryButton extends BaseButton {","lang_version":"es2022","label":false,"evidence":{"model":"claude-sonnet-4-6","prompt_hash":"sha256:abc..."},"target_proposed":"components/Button.tsx::ButtonBase","kind_proposed":null,"confidence_proposed":"medium","reasoning_text":"PrimaryButton extends ButtonBase via re-export; Scope traced the alias to BaseButton.","lang_version_evidence":null,"labeller_id":"llm:claude-sonnet-4-6"}
```

---

## External labeller examples

These are reference shapes only — none ship inside Scope. They live external to this repository in a future labeller crate ecosystem (see [`BACKLOG.md` § Priority 1 — Self-correction cycle](BACKLOG.md#priority-1--self-correction-cycle)).

### Human labeller

Open `sample.jsonl` in a text editor. For each line, read `source_snippet` and the `from` / `to` / `kind` triple. Decide if the edge is correct. Replace `"label":null` with `"label":true` or `"label":false`. Save.

### LLM labeller (pseudo)

```python
# pseudo-code — not part of Scope
import json, sys
from llm_client import classify  # any provider

for line in sys.stdin:
    rec = json.loads(line)
    prompt = f"Is this {rec['kind']} edge from `{rec['from']}` to `{rec['to']}` correct given this source?\n\n{rec['source_snippet']}"
    rec["label"] = classify(prompt)  # returns True | False
    print(json.dumps(rec))
```

Pipe: `cat sample.jsonl | python llm_label.py > sample.labelled.jsonl`.

### LSP cross-check labeller (pseudo)

```python
# pseudo-code — not part of Scope
import json, sys
from lsp_client import goto_definition  # any LSP transport

for line in sys.stdin:
    rec = json.loads(line)
    if rec["kind"] == "calls":
        actual_target = goto_definition(rec["source_snippet"], rec["from"])
        rec["label"] = (actual_target == rec["to"])
    else:
        rec["label"] = None  # leave undecided; --label tolerates partial coverage
    print(json.dumps(rec))
```

### Hybrid (LLM-first, human-reviews)

```bash
cat sample.jsonl | python llm_label.py > sample.llm.jsonl
diff <(jq -r '.label' sample.jsonl) <(jq -r '.label' sample.llm.jsonl) | review-tool
# human accepts / overrides into sample.final.jsonl
scope audit confidence --label sample.final.jsonl
```

The point: the labeller is replaceable. The contract is this schema.

---

## Versioning rules

Per single-operator posture ([`CHARTER.md` § 3 invariant 1](CHARTER.md#3-core-invariants--must-never-break)) every version change is hard-cutover. There is **no** dual-read path, **no** auto-upgrade of committed samples, **no** "old labeller still works on new file" allowance — exactly the named shim shapes the charter-sweep gate refuses re-introduction of ([`CI-GATES.md`](CI-GATES.md)). Changing the contract means:

1. Bump `schema_version` to the new value (any field add, remove, type change, or semantic change is enough — there is no "minor" tier).
2. Wipe the existing labelled corpus under `scope-core/tests/fixtures/reference/<db_slug>/audit-samples/`. The MANIFEST stays append-only for provenance but the `*.jsonl` files come back fresh.
3. Re-index (`scope index`) so any stored derived state matches the new contract.
4. Re-emit with `scope audit confidence --emit-sample <path>` and re-label.

`--label <path>` rejects every `schema_version` other than the current `SAMPLE_SCHEMA_VERSION` const with the same diagnostic that fires on a forward bump. Running an older `scope` binary against newer samples is the only path to "read the old shape" — and that is recovery, not a supported workflow.

The sprint 0004 cutover (v1 → v2) followed this procedure: the schema bump landed with the report-side coverage surface (BACKLOG (h)) and the DB audit-history namespace (BACKLOG (j)) in one sprint because all three are entry points for the same qualitative-signal surface; splitting them would have left the labeller-crate ecosystem (sprint 0005) targeting a half-upgraded contract.

---

## Auditor immutability rule

R8 measures **the extractor's accuracy at a snapshot in time**. The indexed graph and the source files together form the experimental subject. Editing source files between `scope audit confidence --emit-sample <path>` and `scope audit confidence --label <path>` — and during any reading or interpretation of the produced report — invalidates the measurement: the labeller would judge against source that no longer matches what the extractor saw.

The rule:

> **Between `--emit-sample` and `--label` (and during all downstream reading of the report) the maintainer / external labeller MUST NOT modify the source files Scope indexed.** Acceptable: re-index (which produces a fresh snapshot) then re-run `--emit-sample`. Not acceptable: edit-and-relabel.

The rule is **mechanically enforced**, not procedural. Both `--emit-sample` and `--label` re-compute the SHA-256 hash of every source file referenced by the sampled / labelled edges and compare against `file_hashes.hash` (the value the indexer stored at index time, per [`scope-graph/src/sql/schema.sql`](../scope-graph/src/sql/schema.sql) → `file_hashes` table). Any mismatch — different content, deleted file — produces a hard error and aborts the audit. There is **no** `--allow-drift` escape. The only remediation is `scope index` followed by re-running the audit.

Justification for the hard lock (rather than a soft warning + opt-out flag):

- **No legitimate case found** where comparing an indexed snapshot against drifted current source produces a meaningful audit signal. *"Want to see what the extractor said a month ago"* is archaeology, not an audit of accuracy; *"CI runs faster without re-index"* is the performance-over-honesty anti-pattern [`BACKLOG.md` § Priority 2 — Honesty audit](BACKLOG.md#priority-2--honesty-audit-eliminate-non-essential-approximations) explicitly forbids.
- **`mtime` drift without content drift** (file copied between machines, `git checkout` that touches mtime) is handled correctly by content-hash comparison — hash matches, audit proceeds. This is the only "looks like drift but isn't" case, and SHA-256 disambiguates it without a flag.

The SHA-256 check runs lazily: only the files referenced by the sample's edges are re-hashed, not the whole index (a typical N=30 sample touches ~10-30 distinct files). The cost is well under the time the labelling step itself takes.

### Writable namespace for audit-derived rows

The immutability rule binds **source-derived rows** — `edges`, `symbols`, `file_hashes` — produced by the indexer from the source tree. These rows model "what the extractor saw" and stay frozen for the audit's lifetime; the SHA-256 lock above protects them by content, the schema enforces it by ownership.

Sprint 0004 ([BACKLOG (j)](BACKLOG.md#priority-1--self-correction-cycle)) introduces a sibling namespace, `edge_audit_history`, that stores **audit-derived rows** — the labeller verdicts themselves. This table is append-only writable during `--label`:

- `--label` may **append** new rows to `edge_audit_history` keyed by `(audit_id, edge_id, labeller_id)`. Each labelling pass adds rows; existing rows are never updated or deleted.
- `--label` **never** touches `edges` / `symbols` / `file_hashes`. The SHA-256 check still runs first; the writable namespace does not relax it.
- A separate audit-script gate (`edge_audit_history-source-immutability`, [`CI-GATES.md`](CI-GATES.md)) verifies the namespace separation mechanically: any code path that writes to a source-derived table from the `--label` flow is a rule break.

The carveout preserves the invariant the immutability rule encodes — *"the experimental subject does not move under measurement"* — while admitting that **recording the measurement** is itself a write. Source-derived rows model the subject; audit-derived rows model the observations. Mixing them would re-introduce the edit-and-relabel anti-pattern this rule exists to forbid.

---

## Corpus accumulation policy

Owned by [`BACKLOG.md` § Priority 1 sub-item (e)](BACKLOG.md#priority-1--self-correction-cycle); shipped in sprint 0002 ([`SELF-CORRECTION-STATE.md`](SELF-CORRECTION-STATE.md)).

### Directory layout

Labelled `*.jsonl` files are committed under `gumiho-mudang-scope/scope-core/tests/fixtures/reference/<db_slug>/audit-samples/`, where `<db_slug>` is the `LanguageId::as_str()` slug ([`scope-core/src/languages/id.rs`](../scope-core/src/languages/id.rs)): `csharp`, `go`, `java`, `python`, `ruby`, `rust`, `typescript`. The layout is gated by the doc-sync gate ([`ENFORCEMENT-MAP.md` § R13](ENFORCEMENT-MAP.md)) — every supported `LanguageId` arm must own a directory; no extras, no missing.

### Why committed

Committed samples are **regression assets**. Re-running `--label` against a committed sample reproduces the precision baseline byte-for-byte (seed is pinned per `--seed`). A drift in the recomputed precision is a CI signal.

### Provenance record (`MANIFEST.md`)

Each `<db_slug>/audit-samples/` directory carries a `MANIFEST.md` that records provenance for every committed sample in that directory. The MANIFEST is append-only — a labelling pass that emits a new sample appends one row; old rows are never edited.

Minimum row fields:

| Field | Description |
|---|---|
| `sample_file` | Filename of the `*.jsonl` sample, relative to the MANIFEST directory. |
| `labeller_id` | Identifier of the labeller that produced the verdicts. Conventional values: `human:<initials>`, `llm:<model-id>`, `lsp:<server>`, `hybrid:<recipe>`. Matches the `labeller_id` field reserved for `schema_version: "2"` records (see [§ Reserved-for-future fields](#reserved-for-future-fields)). |
| `labelled_at` | ISO-8601 date the labelling pass was committed (`YYYY-MM-DD`). |
| `scope_commit` | Short SHA of the `scope` commit the sample was emitted against (`scope audit confidence --emit-sample` ties the sample to a specific extractor state). |
| `sample_count` | Number of labelled records in the file. |

The MANIFEST is a markdown table for git-diff readability; the row format is the contract, not the wrapping. A future sprint may add machine validation; today the doc-sync gate verifies only the directory's existence.

Sidecar `provenance.json` per sample file and JSONL front-matter were both considered and rejected: sidecars multiply files for no payoff; JSONL has no front-matter convention. A single per-directory append-only `MANIFEST.md` keeps the diff surface to one file per labelling pass per language.

### Retention rule

Old samples are not deleted unless the underlying fixture is removed. A 6-month-old sample with stable precision is a stronger signal than a freshly labelled one — see [§ Stable precision over time](#stable-precision-over-time) below.

Eviction policy (when sample volume eventually warrants one) is **explicitly deferred** to a future sprint per [`BACKLOG.md` § Priority 1 sub-item (j) — "future sprint adds eviction"](BACKLOG.md#priority-1--self-correction-cycle).

### Stable precision over time

The corpus is the longitudinal signal. Re-running `--label` against every committed sample in CI ([`BACKLOG.md` § Priority 1 sub-item (c) — continuous re-audit](BACKLOG.md#priority-1--self-correction-cycle), sprint 0007) yields a per-`(producer, pattern_id)` precision time-series. Stable precision over **months** under **unchanged extractor source** is itself the regression signal: a sudden drop at fixed source = the labelling drifted or a dependency shifted; a drop on an extractor edit = the patch regressed precision.

Therefore the *number* of samples is not the gate. The gate is **shape continuity** — same extractor source + same committed sample = same precision number. The sample corpus accumulates because each labelling pass adds a new vantage point on the same fixtures, not because more samples raise some threshold.

---

## Where this fits

- R8 is **the sensor**. This schema is **its serialisation contract**.
- The full self-correction loop — labellers, continuous CI re-audit, per-PR precision diff, ML-driven extractor patch suggestion — lives in [`BACKLOG.md` § Priority 1](BACKLOG.md#priority-1--self-correction-cycle). The schema documented here is the foundation that makes the loop pluggable.
