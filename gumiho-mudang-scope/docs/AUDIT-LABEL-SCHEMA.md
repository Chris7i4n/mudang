# Audit Label Schema

> **Source of truth**: [`ARCHITECTURAL-REFACTOR.md` § R8](ARCHITECTURAL-REFACTOR.md#r8--confidence-audit-subcommand).
> **Ships**: sprint 0007 (R8 — Confidence audit subcommand).
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

## Record schema (`schema_version: "1"`)

Each line is a JSON object with the following fields:

| Field | Type | Required | Description |
|---|---|---|---|
| `schema_version` | `string` | yes | Always `"1"` on emit; readers must reject unknown values. |
| `edge_id` | `string` | yes | Stable surrogate edge id from the indexed graph (`edges.edge_id` per R0). The labeller is **not** required to interpret this; it is round-tripped verbatim. |
| `kind` | `string` | yes | EdgeKind from the R0 whitelist (e.g. `calls`, `imports`, `extends`, `implements`). |
| `confidence` | `string` | yes | One of `high` / `medium` / `low` (the producer's stamp; this is what R8 audits). |
| `producer` | `string` | yes | The producing plugin's identifier (e.g. `rust`, `python`, `typescript`, `framework:rails`). |
| `pattern_id` | `string` | yes | The pattern within the producer that emitted this edge (e.g. `rust.calls.method`, `python.imports.from`, `rails.routes.draw`). |
| `from` | `string` | yes | Source symbol id or name (the `edges.from_id` resolver input). |
| `to` | `string` | yes | Target symbol id or name (the `edges.to_id` resolver output; may be unresolved for Dangling edges). |
| `source_snippet` | `string` | yes | The relevant source text (typically the call site or definition site). Single-line preferred; multi-line allowed. Used by the labeller as the primary context. |
| `lang_version` | `string \| null` | yes | Reserved slot for per-project language version (e.g. Rust edition `"2021"`, Go directive `"1.21"`, Python `"3.11"`, TypeScript `"es2022"`, Java `"17"`, C# `"net8.0"`, Ruby `"3.2"`). Sprint 0007 always emits `null`; populated by a future sprint when all seven per-language detectors land atomically. Labellers must accept `null` and may use the value as additional context when present. |
| `label` | `boolean \| null` | yes | `null` on emit, `true` (correct) or `false` (incorrect) on label. The labeller fills this. Any value other than `null` / `true` / `false` is rejected by `--label`. |

### Reserved-for-future fields

Sprint 0007 does **not** define additional fields. Future schema bumps may add (with corresponding `schema_version` bump):

- `evidence` (`object | null`) — for LSP labellers to carry the cross-check result (e.g. `{"resolver": "rust-analyzer", "target_uri": "..."}`).
- `confidence_proposed` (`string | null`) — for labellers that suggest a corrected tier rather than a binary verdict.
- `lang_version_evidence` (`string | null`) — for distinguishing detected vs declared version.

None of these ship in `schema_version: "1"`. Adding any of them is a `schema_version` bump to `"2"` with a migration note here.

---

## Example — unlabelled (`--emit-sample` output)

```jsonl
{"schema_version":"1","edge_id":"e-9f2c","kind":"calls","confidence":"high","producer":"rust","pattern_id":"rust.calls.method","from":"crate::handlers::greet","to":"crate::utils::format_name","source_snippet":"format_name(&user.name)","lang_version":null,"label":null}
{"schema_version":"1","edge_id":"e-3a17","kind":"extends","confidence":"high","producer":"typescript","pattern_id":"ts.extends.class","from":"components/Button.tsx::PrimaryButton","to":"components/Button.tsx::BaseButton","source_snippet":"class PrimaryButton extends BaseButton {","lang_version":null,"label":null}
```

## Example — labelled (`--label` input)

```jsonl
{"schema_version":"1","edge_id":"e-9f2c","kind":"calls","confidence":"high","producer":"rust","pattern_id":"rust.calls.method","from":"crate::handlers::greet","to":"crate::utils::format_name","source_snippet":"format_name(&user.name)","lang_version":null,"label":true}
{"schema_version":"1","edge_id":"e-3a17","kind":"extends","confidence":"high","producer":"typescript","pattern_id":"ts.extends.class","from":"components/Button.tsx::PrimaryButton","to":"components/Button.tsx::BaseButton","source_snippet":"class PrimaryButton extends BaseButton {","lang_version":null,"label":true}
```

---

## External labeller examples

These are reference shapes only — none ship inside Scope. They live external to this repository in a future labeller crate ecosystem (see [`POST-REFACTOR-PLAN.md` § Priority 1 — Self-correction cycle](POST-REFACTOR-PLAN.md#priority-1-immediately-post-refactor--self-correction-cycle)).

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

- **Adding a field**: bump `schema_version` to `"2"`. Old labellers reading old samples continue to work. New labellers reading old samples treat new fields as `null` / default. Old labellers reading new samples ignore unknown fields.
- **Removing a field**: charter-grade change. Bumps `schema_version` and breaks the contract. Requires re-labelling existing committed samples.
- **Changing the type of an existing field**: charter-grade change. Same as removal.
- **Changing the semantics of an existing field without changing its type**: charter-grade change. Same as removal.

`--label <path>` rejects records with an unknown `schema_version`. The maintainer either re-emits a fresh sample at the current version or runs the older `scope` binary that emitted the file.

---

## Committed sample policy

Labelled `*.jsonl` files committed under `gumiho-mudang-scope/scope-core/tests/fixtures/reference/<lang>/audit-samples/` are **regression assets**. Re-running `--label` against a committed sample reproduces the precision baseline byte-for-byte (seed is pinned per `--seed`). A drift in the recomputed precision is a CI signal.

Sample provenance: each committed file is the output of an `--emit-sample --seed N` run plus a labelling pass by the maintainer (or by a reviewed external labeller). The commit message names the labeller used and the date of labelling.

The sample corpus grows over time. Old samples are not deleted unless the underlying fixture is removed — a 6-month-old sample with stable precision is a stronger signal than a freshly labelled one.

---

## Where this fits

- R8 is **the sensor**. This schema is **its serialisation contract**.
- The full self-correction loop — labellers, continuous CI re-audit, per-PR precision diff, ML-driven extractor patch suggestion — lives in [`POST-REFACTOR-PLAN.md` § Priority 1](POST-REFACTOR-PLAN.md#priority-1-immediately-post-refactor--self-correction-cycle). The schema documented here is the foundation that makes the loop pluggable.
