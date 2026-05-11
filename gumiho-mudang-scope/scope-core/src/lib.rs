//! Scope's foundation crate.
//!
//! Owns the tree-sitter `Parser`, per-language plugins, `Symbol` /
//! `Edge` types, and the project / workspace configuration loader.
//!
//! Downstream sub-crates (`scope-index`, `scope-graph`, `scope-search`,
//! `scope-workspace`) depend on this crate; this crate depends on no
//! sibling sub-crate.

pub mod config;
pub mod edge;
pub mod languages;
pub mod parser;
pub mod types;

pub use edge::{
    Confidence, EdgeBuilder, EdgeKind, InsertableEdge, Insertable, Producer, RawEdge, Status,
};
pub use types::{Edge, Symbol};
