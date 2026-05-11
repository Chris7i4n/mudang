//! Scope's federated workspace facade.
//!
//! Owns the workspace-level query layer over multiple per-sub-root
//! graphs. R4 (sprint 0002) lands `LanguageWorkspaceContext` and
//! `FrameworkWorkspaceContext` here, per
//! `docs/todos/0006-split-scope-crate.md § Sprint 0000 ambiguity
//! resolutions § 4`.

pub mod workspace_graph;
