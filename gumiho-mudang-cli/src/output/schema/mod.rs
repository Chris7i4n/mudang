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
//! - `summary` — `Summary<'a>` (sum type: Symbol / File) for
//!   `scope summary`.
//! - `refs` — `RefsGrouped<'a>` envelope used when `scope refs`
//!   renders kind-grouped output for a class-like symbol. The flat /
//!   workspace / file paths feed `Reference` / `WorkspaceRef` directly
//!   through the JSON envelope, since both already derive `Serialize`
//!   in `scope-graph`.
//!
//! Display impls live next to the type definition. JSON serialization
//! is via `serde_json::to_string_pretty(&JsonOutput { data: <T>, ... })`
//! where `T` is one of these types (or a per-command view wrapping
//! them).

pub mod compact_symbol;
pub mod refs;
pub mod summary;
pub mod symbol_sketch;

pub use compact_symbol::CompactSymbol;
pub use refs::{RefsGroup, RefsGrouped};
pub use summary::{FileSummary, Summary, SymbolSummary};
pub use symbol_sketch::{
    ClassSketch, EnumSketch, EnumVariantView, FieldView, FileSketch, GenericSketch,
    InterfaceSketch, MethodSketch, SketchSymbol, SymbolSketch,
};
