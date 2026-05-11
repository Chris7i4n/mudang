//! Scope's SQLite-backed dependency graph.
//!
//! Owns the schema and the recursive graph queries (`find_refs`,
//! `find_impact`, `find_deps`, `find_call_paths`, `find_flow_paths`).

pub mod graph;
pub mod resolver;
