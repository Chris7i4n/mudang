//! `EdgeSummary` — typed view over an edge row.
//!
//! Used by edge-emitting commands (`refs`, `deps`, `impact`). The
//! existing `gumiho-mudang-scope` types (`Reference`, `Dependency`,
//! `ImpactNode`) already derive `Serialize`; `EdgeSummary` is the
//! umbrella enum that future code paths consume when they want to
//! treat an edge uniformly regardless of its source query.
//!
//! Concrete per-command view structs (e.g. `RefsView`, `DepsView`,
//! `ImpactView`) embed `EdgeSummary` slices or owned `Reference` /
//! `Dependency` / `ImpactNode` collections.

use gumiho_mudang_scope::core::graph::{Dependency, ImpactNode, Reference};
use serde::Serialize;

/// Sum type over the edge views currently produced by Scope's
/// edge-emitting commands. Tagged-JSON shape so consumers can
/// discriminate on `edge_summary_kind`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "edge_summary_kind", rename_all = "snake_case")]
pub enum EdgeSummary {
    /// A reference row (`scope refs`).
    Reference(Reference),
    /// A dependency row (`scope deps`).
    Dependency(Dependency),
    /// An impact-analysis node (`scope impact`).
    Impact(ImpactNode),
}
