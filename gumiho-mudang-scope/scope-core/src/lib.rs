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
pub mod extract;
pub mod frameworks;
pub mod languages;
pub mod parser;
pub mod types;
pub mod workspace;
pub mod workspace_context;

pub use edge::{Confidence, EdgeBuilder, EdgeKind, Producer, RawEdge, Status};
pub use extract::{Capture, MetadataEntry, MetadataField, RawCaptures, SkippedRange};
pub use frameworks::{
    Detection, DetectedVersion, FrameworkPlugin, Pattern, ResolvedVersion, UnknownVersionPolicy,
};
pub use types::Symbol;
pub use workspace_context::{
    Dependency, FileId, FrameworkVersions, FrameworkWorkspaceContext, LanguageWorkspaceContext,
    Lockfile, ModuleLayout, NoopWorkspaceContext, Package,
};
