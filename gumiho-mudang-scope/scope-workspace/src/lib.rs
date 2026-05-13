//! Scope's federated workspace facade.
//!
//! Owns the workspace-level query layer over multiple per-sub-root
//! graphs. R4 lands `LanguageWorkspaceContext` and
//! `FrameworkWorkspaceContext` here, per
//! `docs/todos/0006-split-scope-crate.md § Locked decisions § 4`.

pub mod workspace_graph;
