//! `scope refs` view types.
//!
//! For class-like symbols, references render as kind-grouped buckets
//! (`calls`, `imports`, `extends`, `implements`, `instantiates`,
//! `references`). `RefsGrouped<'a>` is the typed envelope.

use gumiho_mudang_scope::graph::Reference;
use serde::Serialize;

/// One kind-grouped bucket inside `RefsGrouped`.
#[derive(Debug, Clone, Serialize)]
pub struct RefsGroup<'a> {
    pub kind: &'a str,
    pub refs: &'a [Reference],
}

/// Grouped refs output for a class-like symbol.
#[derive(Debug, Clone, Serialize)]
pub struct RefsGrouped<'a> {
    pub groups: Vec<RefsGroup<'a>>,
}
