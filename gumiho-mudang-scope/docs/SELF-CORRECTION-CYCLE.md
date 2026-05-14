# Self-correction cycle

The closed loop that converts R8 audit signal into automated extractor improvement. Names every contract surface, the mandatory human review gate, and the rollback path when an analyser-suggested patch regresses precision elsewhere.

Source of truth for **the loop shape**. Every Priority 1 sprint (0001–0009, see [`BACKLOG.md` § Priority 1 — Self-correction cycle](./BACKLOG.md#priority-1--self-correction-cycle)) links into this doc instead of restating the pipeline.

State-tracking lives in [`SELF-CORRECTION-STATE.md`](./SELF-CORRECTION-STATE.md). Doc-↔-code drift is mechanically prevented by the **doc-sync gate** (see [Extending the doc-sync gate](#extending-the-doc-sync-gate) below).

---

## Purpose

R8 ships the **sensor**: `scope audit confidence` measures per-`(producer, pattern_id)` precision against a labelled fixture corpus and fails the build when any tier falls below target ([`ENFORCEMENT-MAP.md` § R8](./ENFORCEMENT-MAP.md#r8--confidence-audit-subcommand)). R8 alone does not close the loop: when a tier drops, a human still has to read the labelled samples, find the failing pattern, and patch the extractor by hand.

This doc names the **actuator** — the surfaces, transitions, and gates that turn R8 signal into extractor improvement without losing the auditor-immutability invariant ([`CHARTER.md` § 3 Core invariants](./CHARTER.md#3-core-invariants--must-never-break)) or the single-operator posture ([`CHARTER.md` § Single-operator posture](./CHARTER.md#single-operator-posture)).

---

## Pipeline

```
                    ┌────────────────────────────┐
                    │  source code                │
                    │  (operator working tree)    │
                    └──────────┬─────────────────┘
                               │
                               │ scope index
                               ▼
                    ┌────────────────────────────┐
                    │  graph.db                   │
                    │  (edges, symbols, …;       │
                    │   source-derived; immutable │
                    │   during audit)             │
                    └──────────┬─────────────────┘
                               │
                               │ scope audit confidence
                               │   --emit-sample
                               ▼
                    ┌────────────────────────────┐
                    │  sample.jsonl (v2)          │
                    │  schema_version: "2"        │
                    │  one row per audited edge   │
                    └──────────┬─────────────────┘
                               │
                               │ labeller(s) — LLM / LSP /
                               │   hybrid / human
                               │   (sprint 0005 (b), sprint 0006 (i))
                               ▼
                    ┌────────────────────────────┐
                    │  labelled.jsonl (v2,        │
                    │  optionally aggregated)     │
                    │  carries: label,            │
                    │  target_proposed,           │
                    │  kind_proposed,             │
                    │  confidence_proposed,       │
                    │  evidence, reasoning_text,  │
                    │  labeller_id                │
                    └──────────┬─────────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            │                  │                  │
            ▼                  ▼                  ▼
   scope audit         edge_audit_history   precision report
   confidence            (audit-derived       (--format json/tsv;
   --label                writable;             coverage_summary;
   (precision check)      sprint 0004 (j))      sprint 0004 (h))
                               │
                               │ scope audit history
                               ▼
                    ┌────────────────────────────┐
                    │  patch suggester            │
                    │  (sprint 0008 (f);          │
                    │  reads history; proposes    │
                    │  extractor diff)            │
                    └──────────┬─────────────────┘
                               │
                               │ proposal artefact
                               ▼
                    ┌────────────────────────────┐
                    │  HUMAN REVIEW GATE          │ ◀── non-bypassable
                    │  (mandatory; not optional)  │
                    └──────────┬─────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
              ▼                ▼                ▼
        accept patch      reject patch    restamp policy
        (edit extractor   (record in PR    (sprint 0009 (k);
        source; commit)    body; tracked    audit-trail file;
                           in BACKLOG)      indexer reads at
                                            next index run)
                               │
                               ▼
                    ┌────────────────────────────┐
                    │  next `scope index` run     │
                    │  emits corrected edges      │
                    │  (existing rows: wipe-and-  │
                    │  reindex per CHARTER §2)    │
                    └─────────────────────────────┘
```

---

## Contract surfaces

Each surface has a single owner sprint. Other sprints reference it; never duplicate.

| Surface | Owner | Shape |
|---|---|---|
| Sensor (`scope audit confidence`) | R8 (shipped) | Per-tier precision check; fails build on tier-target miss |
| Sample JSONL emitter | R8 (shipped) | One row per audited edge; `schema_version: "2"` from sprint 0004 |
| Sample schema | sprint 0004 (g) | `SampleRecord` struct ↔ [`AUDIT-LABEL-SCHEMA.md`](./AUDIT-LABEL-SCHEMA.md); gated by doc-sync |
| Labelled corpus on disk | sprint 0002 (e) | `scope-core/tests/fixtures/reference/<lang>/audit-samples/*.jsonl` + per-directory `MANIFEST.md` provenance; policy in [`AUDIT-LABEL-SCHEMA.md` § Corpus accumulation policy](./AUDIT-LABEL-SCHEMA.md#corpus-accumulation-policy) |
| Labellers | sprint 0005 (b₁) — scaffolding + reference noop; sprints 0010 (b₂) / 0011 (b₃) / 0012 (b₄) — concrete LLM / LSP / hybrid | Sibling cargo workspace `gumiho-mudang-labeller/` excluded from the root Scope workspace; shared `scope-audit-labeller-core` defines the `Labeller` trait + v2 JSONL helpers; concrete labellers depend only on core, never on Scope crates ([§ Labeller workspace](#labeller-workspace) below) |
| Aggregator | sprint 0006 (i) | Runner-side; merges multi-labeller verdicts; emits single aggregated JSONL |
| Coverage report | sprint 0004 (h) | `coverage_summary` top-level + per-row `skipped_count` / `coverage_ratio` |
| Audit-history table | sprint 0004 (j) | `edge_audit_history` — append-only; sibling auditor-immutability rule |
| `scope audit history` | sprint 0004 (j) | Read-side surface: per-edge timeline, per-`pattern_id` trend, per-labeller agreement |
| Continuous re-audit in CI | sprint 0007 (c) | Per-PR precision diff + nightly full audit |
| Patch suggester | sprint 0008 (f) | Reads `edge_audit_history`; proposes extractor diff; **never** opens PR autonomously |
| Restamp policy | sprint 0009 (k) | Audit-trail file → indexer reads at next index run |
| Doc-sync gate | sprint 0001 (this doc) | `scripts/gate_doc_sync.sh` — narrow-grep gate against doc-↔-code drift |

---

## Labeller workspace

Concrete labellers (LLM / LSP / hybrid, plus the reference noop) live in the **sibling cargo workspace** `gumiho-mudang-labeller/` at the repo root. The workspace is listed under the root `Cargo.toml`'s `[workspace] exclude = [...]`; cargo builds at the repo root never see it.

The boundary is the build-system fact that turns two CHARTER lines into mechanical guarantees:

- **CHARTER §3 invariant 6** — *"Deterministic, read-only at query time. No network calls."* LLM labellers call provider APIs (network) and LSP labellers spawn language servers (toolchain). The exclusion ensures both can exist as Scope-adjacent tooling without their dependencies entering the Scope binary's `Cargo.lock`.
- **CHARTER §5 hard limits** — *"Network calls during query"*, *"No toolchain required"*, *"Invoking the language's compiler or interpreter"*. Labellers may legitimately do all three because they are not Scope; the workspace boundary keeps that division enforceable in review (a cargo edit that adds the wrong dependency fails the [`CI-GATES.md` R14 gate](./CI-GATES.md), it is not a code-review judgment call).

### Surface

| Crate | Sprint | Role |
|---|---|---|
| `scope-audit-labeller-core` | 0005 (b₁) | `Labeller` trait, `SampleRecord` v2 wire types, JSONL read/write helpers. Consumes only the published schema doc; zero dependency edges to Scope crates. |
| `scope-audit-labeller-noop` | 0005 (b₁) | Reference impl. Stamps `labeller_id = "noop:reference-v0"` and passes every other field through. Proves the trait + IO loop end-to-end before concrete labellers ship. |
| `scope-audit-labeller-llm` | 0010 (b₂) | Provider-agnostic LLM wrapper. First provider: DeepSeek (`deepseek` cargo feature, default; OpenAI-compatible chat-completions endpoint). Additional providers (Anthropic / OpenAI / Gemini / local) land as separate cargo features in follow-up sprints. |
| `scope-audit-labeller-lsp` | 0011 (b₃) | Per-language LSP cross-check via `tower-lsp` clients. |
| `scope-audit-labeller-hybrid` | 0012 (b₄) | LLM-first composition over an inner labeller plus a human-reviews-diffs surface. |

The trait shape is **frozen at sprint 0005's close** — concrete labellers in 0010-0012 inherit it. A future bump to the trait is `schema_version`-grade discipline: charter-amendment commit on `main`, all four concrete labellers updated in lockstep, never a silent shape change.

### Contract direction

The labeller side imports nothing from Scope. The v2 record types in `scope-audit-labeller-core::record` are a **duplicate** of what `gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md` § Record schema documents. The duplicate is intentional: the schema doc is the contract, the code on each side is an implementation of it. Drift surfaces as integration-test failure long before any silent breakage could escape.

The wiring is intentionally one-way:

```
gumiho-mudang-scope (cargo workspace at repo root)
    │
    │  scope audit confidence --emit-sample sample.jsonl
    │     writes v2 records per AUDIT-LABEL-SCHEMA.md
    ▼
sample.jsonl (the contract)
    ▲
    │  read_records / write_record from scope-audit-labeller-core
    │
gumiho-mudang-labeller (excluded sibling workspace)
    └── scope-audit-labeller-{core, noop, llm, lsp, hybrid}
```

No cargo `path` dependency runs between the two workspaces in either direction. R14's narrow-grep gate verifies this on every CI run (sprint 0005 ships the gate alongside the workspace).

### First concrete labeller — `scope-audit-labeller-llm`

Sprint 0010 (b₂) ships the first concrete impl on top of the sprint-0005 scaffolding. The crate splits provider-agnostic logic from transport:

- `Provider` trait — one method (`complete(&Prompt)`) plus stable `provider_id` / `model_id` strings. Retry / rate-limit handling is the provider's responsibility; by the time `complete` returns the bounded policy has already run.
- `LlmLabeller<P: Provider>` — implements `Labeller`. Renders the prompt, calls the provider, parses the verdict, copies verdict fields onto the record, stamps `labeller_id` as `llm:<provider_id>:<model_id>` (three-segment shape; the two-segment form from earlier docs is retired).
- Concrete providers live behind individual cargo features (`deepseek` ships in this sprint; Anthropic / OpenAI / Gemini / local follow). Adding a provider does not touch `LlmLabeller`, the prompt template, or the verdict parser.

The `Provider` seam is also the test seam: `MockProvider` (in the crate's public surface, not `cfg(test)`-only) substitutes canned responses without any HTTP fake. The live DeepSeek transport is exercised by a separate test gated behind the `live-deepseek-tests` cargo feature **and** a `DEEPSEEK_API_KEY` env-var presence check; default `cargo test --workspace` never reaches the network.

Per-record failure mode is **abstain, not corrupt**: a transport error after the retry policy, or an unparseable model response, writes a stderr diagnostic line and emits a record with the seven labeller-fillable fields untouched plus `labeller_id` stamped. The downstream `scope audit confidence --label` already tolerates `label: null` rows — an abstain is signal, not pipeline failure.

## Mandatory human review gate

**Non-bypassable.** Every analyser-suggested patch reaches the operator as a proposal artefact, not as an applied change. The operator:

1. Reviews the suggested diff against the extractor source.
2. Validates against fresh fixtures the suggester did not see.
3. Either applies the patch (a regular `feat`/`fix` commit on `main`), records the rejection (PR body + [`BACKLOG.md`](./BACKLOG.md) entry if the failure mode is worth tracking), or escalates to a [`sprints/README.md` § 3 ambiguity protocol](./sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc) consultation if the proposal exposes a rule gap.

The suggester **never**:

- Opens a PR autonomously.
- Edits extractor source files directly.
- Mutates `graph.db` source-derived rows (`edges`, `symbols`, `file_hashes`).

The suggester **may**:

- Append to the audit-trail file (sprint 0009 (k)) when the policy is the automatic-downgrade variant — and only within the bounds that policy commits permit.
- Write proposal artefacts to a scratch location for human pickup.

---

## Rollback path

When an analyser-suggested patch is merged and then proves to regress precision elsewhere:

1. The continuous re-audit gate (sprint 0007 (c)) catches the regression on the next PR / nightly run.
2. The audit-trail file (sprint 0009 (k)) records the original triggering signal and the merged commit SHA.
3. Rollback options, in order of cheapness:
   - **Revert the extractor commit** — standard `git revert`; next `scope index` run re-emits the previous edges. Wipe-and-reindex if persisted rows are now wrong.
   - **Downgrade the affected `pattern_id`** via the audit-trail file (sprint 0009 (k) hybrid policy variant) — leaves the patch in place but lowers the confidence stamp until further signal arrives.
   - **Escalate as `pattern_id` quarantine** — temporarily stop emitting the edge until a fresh patch lands. Recorded in the audit-trail file with explicit rationale; never silent.
4. Rollback decision is recorded in the PR body of the reverting / patching commit. The rollback path itself is never silent — every step leaves a trace in the audit-trail file or in `BACKLOG.md`.

---

## Auditor-immutability invariant (extended for the loop)

The closed loop introduces **two distinct namespaces** in `graph.db`:

- **Source-derived** — `edges`, `symbols`, `file_hashes`, every other table populated by `scope index`. **Immutable during audit.** Wipe-and-reindex per [`CHARTER.md` § Single-operator posture](./CHARTER.md#single-operator-posture) is the only migration path.
- **Audit-derived** — `edge_audit_history` (sprint 0004 (j)) plus any future tables sprint 0009 (k) adds. **Writable by `scope audit confidence --label`.** Never mutates source-derived rows.

Two namespaces, two distinct mechanical enforcement gates. The CI gate added in sprint 0004 verifies `--label` writes touch only `edge_audit_history`. The source-derived auditor-immutability rule ([`AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule](./AUDIT-LABEL-SCHEMA.md#auditor-immutability-rule)) gains the writable-namespace paragraph in sprint 0004's commit.

The indexer (sprint 0009 (k)) reads the audit-trail file at index time to apply confidence re-stamps. The extractor source stays canonical for **what the stamp means**; the audit-trail file captures **the cycle-driven correction** on top.

---

## Extending the doc-sync gate

The doc-sync gate (`scripts/gate_doc_sync.sh`, recipe `just gate-doc-sync`, [`ENFORCEMENT-MAP.md` § R13](./ENFORCEMENT-MAP.md)) is the **mechanical** half of preventing doc-↔-code drift across the self-correction loop. Every later sprint (0002–0009) ships code and docs that move in lockstep; the gate enforces it.

The gate is modelled on [`scripts/gate_charter.sh`](../../scripts/gate_charter.sh): a single shell script with **named, narrow check functions**, each targeting one specific drift shape. Adding a new check is cheap.

### How to add a check (per-sprint recipe)

When a later sprint introduces a new code-↔-doc pair that must stay in sync, the sprint's implementation commit edits `scripts/gate_doc_sync.sh` to add **one** new check function. Pattern:

```bash
# Check N — <one-line drift shape this catches>
#
# Rationale: <which doc, which code surface, why drift would matter>.
check_<short_name>() {
    local doc_value code_value
    doc_value=$(grep -oE '<doc pattern>' "$DOC_PATH" | head -1)
    code_value=$(grep -oE '<code pattern>' "$CODE_PATH" | head -1)
    if [[ "$doc_value" != "$code_value" ]]; then
        fail_block "<short_name>" \
                   "doc says '$doc_value'; code says '$code_value'" \
                   "$DOC_PATH ↔ $CODE_PATH"
    fi
}
```

Then invoke it from `main()` alongside the other checks. The check stays narrow: it asserts **one** drift shape, never a loose substring scan that could fire on charter-aligned prose.

### Sprint-by-sprint additions expected

| Sprint | Sub-item | New check(s) the sprint commit adds |
|---|---|---|
| 0002 | (e) | Corpus directory layout exists for every supported `LanguageId` arm (no extras, no missing) |
| 0003 | (d) | Per-language detector module presence matches the list in [`CHARTER.md` § 7](./CHARTER.md#7-per-language-scope-and-non-scope) |
| 0004 | (g) | `SCHEMA_VERSION` const value ↔ `schema_version` in [`AUDIT-LABEL-SCHEMA.md`](./AUDIT-LABEL-SCHEMA.md); `SampleRecord` field set ⊆ documented v2 fields |
| 0004 | (h) | `coverage_summary` struct field set ↔ documented coverage fields |
| 0004 | (j) | `edge_audit_history` SQL columns ↔ documented columns |
| 0006 | (i) | Documented default aggregation policy ↔ aggregator's hard-coded default |
| 0007 | (c) | Every `audit-ci` / `audit-nightly` recipe in [`justfile`](../../justfile) referenced in [`CI-GATES.md`](./CI-GATES.md) |
| 0009 | (k) | Audit-trail file path documented ↔ path the indexer reads |

Each addition is **one commit on the owning sprint's branch**, in the same commit that ships the code-↔-doc pair. The gate flip from `planned → active` for the new check happens in the same commit per [`sprints/README.md` § 7](./sprints/README.md#7-ci-gate-activation).

### When the gate is the wrong tool

The gate catches **drift between named code surfaces and named doc passages**. It does not:

- Catch semantic drift ("the doc says X is fast; the code is slow"). That is acceptance-test territory.
- Catch missing documentation for a code surface. That is reviewer / `ENFORCEMENT-MAP.md` § 7.5 territory.
- Catch out-of-band rule amendments. That is the [`sprints/README.md` § 3](./sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc) protocol.

If a sprint's drift shape does not fit "one named code value ↔ one named doc value" or "one named code surface ↔ one named directory layout", the gate is the wrong place. Surface it in the sprint plan and escalate per § 3 before opening the branch.

---

## See also

- [`BACKLOG.md` § Priority 1 — Self-correction cycle](./BACKLOG.md#priority-1--self-correction-cycle) — sub-item (a) through (k) catalogue.
- [`SELF-CORRECTION-STATE.md`](./SELF-CORRECTION-STATE.md) — sprint state tracking.
- [`AUDIT-LABEL-SCHEMA.md`](./AUDIT-LABEL-SCHEMA.md) — JSONL contract surface (current `schema_version: "2"`, shipped sprint 0004).
- [`ENFORCEMENT-MAP.md` § R8](./ENFORCEMENT-MAP.md) — confidence-audit sensor; [`§ R13`](./ENFORCEMENT-MAP.md) — doc-sync gate.
- [`CI-GATES.md`](./CI-GATES.md) — gate inventory including doc-sync.
- [`CHARTER.md` § 6 Soft expansion zone](./CHARTER.md#6-soft-expansion-zone--scope-expands-freely-here) — the surface this initiative expands.
