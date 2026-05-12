//! Typed output schemas (R10).
//!
//! Every CLI command renders its output through a `#[derive(Serialize)]`
//! struct or enum defined in this module. Procedural `println!`
//! / `format!` and `serde_json::json!()` ad-hoc trees are forbidden by
//! the strict-reading scope decision recorded in
//! [`gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md` § R10](../../../../gumiho-mudang-scope/docs/ARCHITECTURAL-REFACTOR.md#r10--typed-output-schema).
//!
//! Module layout:
//!
//! - `compact_symbol` — borrowed `CompactSymbol<'a>`, the trimmed
//!   `Symbol` projection used by `--compact` JSON output.
//! - `symbol_sketch` — the `SymbolSketch<'a>` sum type used by
//!   `scope sketch` (variants: Class, Method, Interface, Enum, Generic,
//!   File).
//! - `edge_summary` — the `EdgeSummary` view used by edge-emitting
//!   commands (`refs`, `deps`, `impact`).
//!
//! Display impls live next to the type definition. JSON serialization
//! is via `serde_json::to_string_pretty(&JsonOutput { data: <T>, ... })`
//! where `T` is one of these types (or a per-command view wrapping
//! them).

pub mod compact_symbol;
pub mod edge_summary;
pub mod symbol_sketch;

pub use compact_symbol::CompactSymbol;
pub use edge_summary::EdgeSummary;
pub use symbol_sketch::{
    ClassSketch, EnumSketch, EnumVariantView, FieldView, FileSketch, GenericSketch,
    InterfaceSketch, MethodSketch, SketchSymbol, SymbolSketch,
};
