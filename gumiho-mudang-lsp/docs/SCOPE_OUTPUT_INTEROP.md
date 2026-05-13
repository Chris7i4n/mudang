# Scope ↔ LSP typed-output interop (forward-looking)

> **Status**: forward-looking note. No binding cross-crate API exists today; this document records a possibility that R10 (typed output schema, shipped 2026-05-12) unlocks for a future composition sprint.

## What R10 changed

R10 (Phase D of the gumiho-mudang-scope architectural refactor) shipped on 2026-05-12 — every CLI output path in `gumiho-mudang-cli` is a `#[derive(Serialize)]` Rust struct or enum at the boundary, with the strict-reading decision recorded in [`gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` § R10](../../gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema).

After R10, the JSON envelope emitted by `mudang ... --json` is the on-the-wire encoding of a concrete Rust type. Field names, shape, nullability, and enum variants are owned by the type definition, not by `serde_json::json!()` macro call sites.

## Why this matters for the LSP adapter

`gumiho-mudang-lsp` is the raw LSP-protocol surface (per [`README.md` § Surface Boundary](README.md)). Higher composition layers — including any future "Scope + LSP" composer that combines Scope's graph rows with LSP enrichments (call hierarchy, references, definitions) — must read Scope's output without string parsing if they want to be reliable.

Two concrete things the typed boundary unlocks:

1. **Schema export to TypeScript / other LSP-client languages.** Tools like `ts-rs` or `cargo-typeshare` walk a `#[derive(TS)]` / `#[typeshare]` annotation on each Scope output struct and emit a `.d.ts` (or equivalent) binding. A composition layer written in TypeScript can `import type { SymbolSketch, EdgeSummary, CompactView } from '@gumiho-mudang/scope-output'` and the compiler enforces shape parity with Rust.
2. **Native consumption from another Rust crate.** A future `gumiho-mudang-composer` crate can depend on the same Rust types directly, deserializing Scope's JSON output via `serde_json::from_str::<SymbolSketch>(...)` instead of walking a `serde_json::Value`. The composer's type system then reflects R3's `Resolved` / `Ambiguous` / `Dangling` edge status as enum variants — invalid compositions (e.g., reading a `position` off an `Ambiguous` edge that has multiple targets) fail at compile time.

## What is not in scope today

- No crate currently exports Scope's output structs as a public API. R10 landed them as `pub(crate)` (or workspace-internal) inside `gumiho-mudang-cli`. A later sprint can lift them into a shared crate (`gumiho-mudang-scope-output`?) if a real consumer materialises.
- No `ts-rs` / `typeshare` derive is wired. The Rust-side types are the contract; binding generation is opt-in by the consumer when a real composition use case appears.
- The composition layer itself is queued in `gumiho-mudang-scope/docs/POST-REFACTOR-PLAN.md`; `gumiho-mudang-scope/docs/SCOPE-LSP-COMPOSITION.md` § 5.4 owns the design.

## When the door opens

A consumer triggers this work when:

- A composition tool (CLI, IDE plugin, agent) needs to consume Scope's `--json` output programmatically and the maintainer wants the consumer to fail at build time on Scope output-shape drift, not at runtime.
- The LSP composer described in `SCOPE-LSP-COMPOSITION.md` lands and the join key (`scope.edges.edge_id` → `edge_enrichments.edge_id`) needs to be exchanged via a typed envelope rather than a free-form JSON.

Until then, `--json` is read as untyped JSON by current consumers; the typed Rust structs are an implementation detail of `gumiho-mudang-cli` but the JSON wire format they produce is the durable contract.

## See also

- [`gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` § R10](../../gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema) — refactor move that landed the typed structs.
- [`gumiho-mudang-scope/docs/SCOPE-LSP-COMPOSITION.md`](../../gumiho-mudang-scope/docs/SCOPE-LSP-COMPOSITION.md) § 5.4 — composition layer design that consumes Scope output + LSP enrichments.
