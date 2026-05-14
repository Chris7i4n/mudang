# gumiho-mudang-labeller

Sibling cargo workspace housing reference labellers for the Scope audit loop. **Excluded** from the root Scope workspace's `[workspace] exclude = [...]`; labeller dependencies never enter the Scope binary's `Cargo.lock`.

The contract this workspace consumes is the JSONL sample format documented in [`gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md`](../gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md) at `schema_version: "2"`. The labeller side reads the contract from the doc, not from a Scope crate; no cargo `path` dependency runs in either direction.

## Why a separate workspace

Two CHARTER lines turn into build-system facts at this boundary:

- [`CHARTER.md` §3 invariant 6](../gumiho-mudang-scope/docs/CHARTER.md#3-core-invariants--must-never-break) — *"Deterministic, read-only at query time. No network calls."* LLM labellers call provider APIs; LSP labellers spawn language servers. Excluding this workspace from the root prevents those dependencies from entering the Scope binary.
- [`CHARTER.md` §5 hard limits](../gumiho-mudang-scope/docs/CHARTER.md#5-hard-limits--scope-will-never-cross-these) — *"Network calls during query"*, *"No toolchain required"*, *"Invoking the language's compiler or interpreter"*. Labellers may legitimately do all three. The workspace boundary preserves the rule that Scope itself never does.

The full rationale lives in [`BACKLOG.md` § Priority 1 (b)](../gumiho-mudang-scope/docs/BACKLOG.md#priority-1--self-correction-cycle) and the elaborated pipeline in [`SELF-CORRECTION-CYCLE.md` § Labeller workspace](../gumiho-mudang-scope/docs/SELF-CORRECTION-CYCLE.md#labeller-workspace).

## Crates

| Crate | Status | Sprint | Role |
|---|---|---|---|
| `scope-audit-labeller-core` | shipped | 0005 (b₁) | `Labeller` trait, v2 `SampleRecord` types, JSONL read/write helpers. |
| `scope-audit-labeller-noop` | shipped | 0005 (b₁) | Reference impl: stamps `labeller_id` and passes every other field through. |
| `scope-audit-labeller-llm` | unstarted | 0010 (b₂) | Provider-agnostic LLM wrapper. |
| `scope-audit-labeller-lsp` | unstarted | 0011 (b₃) | Per-language LSP cross-check via `tower-lsp` clients. |
| `scope-audit-labeller-hybrid` | unstarted | 0012 (b₄) | LLM-first composition + human-reviews-diffs surface. |

State tracking lives in [`SELF-CORRECTION-STATE.md`](../gumiho-mudang-scope/docs/SELF-CORRECTION-STATE.md).

## Pipeline

The labellers sit between `--emit-sample` and `--label`:

```
scope audit confidence --emit-sample sample.jsonl
    →  <one or more labellers from this workspace>
    →  sample.labelled.jsonl
    →  scope audit confidence --label sample.labelled.jsonl
```

Each labeller reads a v2 JSONL stream and writes a v2 JSONL stream. Labellers compose: the hybrid composer (sprint 0012) holds an inner labeller and forwards records; aggregators (sprint 0006 (i)) merge multi-labeller outputs into a single stream.

## Trait shape

Every labeller implements [`scope_audit_labeller_core::Labeller`](scope-audit-labeller-core/src/runner.rs):

```rust
pub trait Labeller {
    type Error: std::error::Error + Send + Sync + 'static;
    fn label_one(&mut self, record: SampleRecord) -> Result<SampleRecord, Self::Error>;
    fn labeller_id(&self) -> &str;
}
```

The trait is **frozen at sprint 0005's close**. Concrete labellers in sprints 0010-0012 inherit it as-is. A future trait change is charter-amendment grade: commit on `main`, update every concrete labeller in lockstep, never a silent shape change.

## Field-population convention

The seven labeller-fillable columns of a v2 record (`evidence`, `target_proposed`, `kind_proposed`, `confidence_proposed`, `reasoning_text`, `lang_version_evidence`, plus the verdict `label`) are designed for **partial population** per [`AUDIT-LABEL-SCHEMA.md` § Partial-population semantics](../gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md#partial-population-semantics). A labeller writes only the fields it has an opinion about; the others stay `null`. Aggregators fuse partial verdicts from heterogeneous labellers into a single record.

Every labeller **must** stamp its identifier into the `labeller_id` column. Convention: `<kind>:<recipe>` (`noop:reference-v0`, `llm:anthropic:claude-sonnet-4-6`, `lsp:rust-analyzer:2025.5.12`, `hybrid:llm-first-human-review:anthropic-claude-sonnet-4-6`).

## Building

This workspace builds independently:

```sh
cd gumiho-mudang-labeller
cargo build
cargo test
```

The root Scope workspace remains untouched. `cargo metadata --no-deps` at the repo root lists no labeller crates; root `cargo build` does not compile any labeller-side code. The R14 gate (`just gate-labeller-isolation` from the repo root, planned for sprint 0005 CP5) verifies the boundary on every CI run.

## Adding a labeller

1. Add the crate name under `gumiho-mudang-labeller/Cargo.toml`'s `[workspace] members` list.
2. Take `path` dependencies on `scope-audit-labeller-core` only. **No `path = "../gumiho-mudang-scope/..."` deps**; the R14 gate refuses them.
3. Implement `Labeller`. Stamp `labeller_id`; populate any subset of the labeller-fillable fields the labeller has an opinion about.
4. Ship an integration test that pipes a v2 fixture through the binary and asserts the output stream is v2-conformant.
5. Cross-link from [`SELF-CORRECTION-CYCLE.md` § Labeller workspace](../gumiho-mudang-scope/docs/SELF-CORRECTION-CYCLE.md#labeller-workspace) — extend the surface table.
