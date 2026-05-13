//! R5 — framework infrastructure.
//!
//! Defines the `FrameworkPlugin` trait and supporting types.
//! `FrameworkPlugin` consumes already-extracted `&[Symbol]` and
//! `&[RawEdge]` plus a resolved framework version, and emits `RawEdge`s
//! that the R3 resolver promotes to `InsertableEdge`. **It never sees
//! tree-sitter nodes, source text, or filesystem paths.** This is the
//! mechanical safeguard for `LANGUAGE-PLAYBOOK.md` E2 — frameworks
//! match over graph rows produced by the language layer.
//!
//! # Negative trait shape (R11 + R12 — mechanically enforced)
//!
//! `FrameworkPlugin` and its companion types must not contain methods
//! whose names imply type-system work or runtime evaluation:
//!
//! - `infer_*` / `evaluate_*` / `solve_*` / `narrow_*` /
//!   `resolve_overload_*` (R12) — `scripts/audit_trait_shape.sh`
//!   greps `scope-core/src/frameworks/` alongside `languages/` and
//!   `extract/`.
//! - `expand_*` (R11) — macros are indexed as `Symbol{kind: macro}`;
//!   framework plugins resolve invocations via the existing
//!   `calls.macro` edges, never via expansion.
//!
//! # Why graph-only (R5 rejected variants)
//!
//! - **Variant A (eager primitive edges)**: emitting `decorator_call`
//!   for every `@something` would inflate the edge graph by 10–50% in
//!   projects that use no framework. Rejected.
//! - **Variant C (one `.scm` per framework per language)**: violates
//!   E2 (framework would parse AST), forces O(framework × language)
//!   `.scm` files, duplicates B3 tolerance, makes cross-cutting
//!   queries infeasible. The `scripts/audit_no_framework_scm.sh` gate
//!   refuses any `queries/<lang>/frameworks/` directory.
//!
//! Variant B (eager metadata) is the active design: language plugins
//! populate the three reserved metadata keys (`decorators`,
//! `annotations`, `template_calls`) on `Symbol.metadata` (R0 schema);
//! framework plugins read that metadata + `&[RawEdge]` and emit derived
//! edges only when their predicate fires.
//!
//! See `docs/ENFORCEMENT-MAP.md` § R5 for the durable contract.

use crate::edge::{EdgeKind, RawEdge};
use crate::languages::LanguageId;
use crate::types::Symbol;
use crate::workspace_context::FrameworkWorkspaceContext;
use semver::{Version, VersionReq};

/// A framework plugin emits derived edges by predicate-matching over
/// already-parsed `Symbol` and `Edge` rows. It is opaque to source
/// text, tree-sitter nodes, and filesystem paths — the trait surface
/// is the mechanical safeguard.
///
/// Implementors live at `src/frameworks/<name>/mod.rs` when concrete
/// frameworks are adopted per `FRAMEWORK-PLAYBOOK.md`. No concrete
/// framework is in-tree yet; the synthetic test framework at
/// `scope-core/tests/synthetic_framework/mod.rs` exercises the trait
/// surface.
pub trait FrameworkPlugin: Send + Sync {
    /// Stable lowercase identifier, e.g., `"rails"`, `"react"`. Used
    /// in `Producer::Framework(name)` and integration-test fixtures.
    fn name(&self) -> &str;

    /// Runs once per workspace at index time. Reads framework version
    /// and lockfile via `FrameworkWorkspaceContext` (the R4 split —
    /// frameworks see framework version and lockfile, language plugins
    /// do not).
    fn detect(&self, ctx: &dyn FrameworkWorkspaceContext) -> Detection;

    /// Behaviour when `Detection.version == DetectedVersion::Indeterminate`.
    /// One of `Skip` / `StableOnlyLowConfidence` / `AssumeLatest(v)`.
    fn unknown_version_policy(&self) -> UnknownVersionPolicy;

    /// Match the framework's patterns against the pre-filtered symbol
    /// and edge slices. The indexer (via `dispatch::run_frameworks`)
    /// applies the cross-language pre-filter from
    /// `Detection.applies_to_languages` before invoking this method,
    /// so implementors can assume every input symbol or edge belongs
    /// to a language the framework declared.
    ///
    /// Returned `RawEdge`s flow through the R3 resolver; the framework
    /// plugin never inserts directly.
    fn match_edges(
        &self,
        symbols: &[Symbol],
        edges: &[RawEdge],
        version: ResolvedVersion,
    ) -> Vec<RawEdge>;
}

/// Output of `FrameworkPlugin::detect`. `applies_to_languages` MUST be
/// non-empty when `detected == true` — `dispatch::run_frameworks`
/// rejects empty-vec detections at runtime; the
/// `scripts/audit_patterns.sh` CI gate is the build-time complement.
#[derive(Debug, Clone)]
pub struct Detection {
    pub detected: bool,
    pub version: DetectedVersion,
    pub applies_to_languages: Vec<LanguageId>,
}

/// Outcome of reading the workspace's framework version. The three
/// variants exist because `Option<semver::Version>` overloaded the
/// `None` case across three policy-distinct situations; see
/// `ENFORCEMENT-MAP.md` § R5 → "Version source semantics".
#[derive(Debug, Clone)]
pub enum DetectedVersion {
    /// Lockfile (or equivalent pinned manifest) resolved to a single
    /// semver. The `Version` may have been coerced from a
    /// non-strict-semver string (Rails `7.0.4.3` → `7.0.4`; Python
    /// `3.11.0a1` → `3.11.0`); the per-framework doc records the rule.
    Resolved(Version),
    /// Manifest declares a range, no lockfile resolved it. Or beta tag
    /// without parseable version. Or unparseable manifest. Routed to
    /// `unknown_version_policy()`.
    Indeterminate,
    /// Framework genuinely lacks versioned releases (rare). Documented
    /// in the per-framework doc with rationale; every pattern's
    /// `available_in` is treated as `VersionReq::STAR`.
    NoVersionConcept,
}

/// What `match_edges` actually receives. The indexer collapses
/// `(DetectedVersion, UnknownVersionPolicy)` into one of these.
#[derive(Debug, Clone)]
pub enum ResolvedVersion {
    /// `DetectedVersion::Resolved(v)`.
    Detected(Version),
    /// `DetectedVersion::Indeterminate + Policy::StableOnlyLowConfidence`.
    /// Predicates emit only their fallback subset
    /// (`available_in: VersionReq::STAR`) with `Confidence::Low`.
    Fallback,
    /// `DetectedVersion::Indeterminate + Policy::AssumeLatest(v)`.
    /// Predicates run as if `v` were resolved; `producer` carries
    /// `framework:<name>:assumed_<v>`.
    Assumed(Version),
    /// `DetectedVersion::NoVersionConcept`. Every pattern's
    /// `available_in` is treated as `VersionReq::STAR`.
    Versionless,
    // `Indeterminate + Skip` never reaches `match_edges` — the
    // indexer short-circuits before invoking it.
}

/// Three policies for `DetectedVersion::Indeterminate`.
#[derive(Debug, Clone)]
pub enum UnknownVersionPolicy {
    /// `match_edges` is not called; zero edges emitted. Recommended
    /// default.
    Skip,
    /// `match_edges` is called with `ResolvedVersion::Fallback`. The
    /// predicate is responsible for emitting only its fallback subset
    /// with `Confidence::Low`.
    StableOnlyLowConfidence,
    /// `match_edges` is called with `ResolvedVersion::Assumed(v)`.
    AssumeLatest(Version),
}

/// A single match rule in a framework's pattern catalog.
///
/// Pattern catalogs live at `src/frameworks/<name>/patterns.rs` as
/// `pub static ALL_PATTERNS: &[Pattern]` (or `LazyLock<Vec<Pattern>>`
/// when `VersionReq::parse` is needed). `match_edges` filters them
/// by `available_in.matches(version)` and dispatches to `predicate`.
///
/// The `scripts/audit_patterns.sh` CI gate refuses patterns with
/// empty `id` or unreferenced `predicate` symbols.
#[derive(Clone)]
pub struct Pattern {
    /// Stable identifier surfaced in `Producer.pattern_id` (R0). For
    /// example: `"rails.belongs_to"`, `"flask.route.decorator"`.
    /// Must be non-empty (audit-enforced).
    pub id: &'static str,
    /// The edge kind this pattern produces.
    pub edge_kind: EdgeKind,
    /// Framework versions where this pattern applies. Use
    /// `VersionReq::STAR` for "all versions" / fallback subset.
    pub available_in: VersionReq,
    /// Predicate function: receives pre-filtered symbols/edges,
    /// returns `RawEdge`s. Each `RawEdge` MUST have
    /// `producer = Producer::Framework(plugin.name())`.
    pub predicate: fn(&[Symbol], &[RawEdge]) -> Vec<RawEdge>,
}

pub mod dispatch;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_with_star_versionreq_matches_any_version() {
        let req = VersionReq::STAR;
        for v in ["1.0.0", "7.0.4", "0.0.1"] {
            assert!(
                req.matches(&Version::parse(v).unwrap()),
                "STAR should match {v}"
            );
        }
    }

    #[test]
    fn detection_carries_required_fields() {
        let d = Detection {
            detected: true,
            version: DetectedVersion::Resolved(Version::new(7, 0, 4)),
            applies_to_languages: vec![LanguageId::Ruby],
        };
        assert!(d.detected);
        assert_eq!(d.applies_to_languages, vec![LanguageId::Ruby]);
        matches!(d.version, DetectedVersion::Resolved(_));
    }
}
