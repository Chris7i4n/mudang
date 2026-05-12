//! Indexer-side framework dispatcher (R5 acceptance: indexer applies
//! the cross-language pre-filter before invoking `match_edges`).
//!
//! `run_frameworks` is the sole call site that invokes
//! `FrameworkPlugin::match_edges`. It performs three jobs:
//!
//! 1. Skip the plugin entirely when `detect()` returns `detected:
//!    false`, or when the (detected, policy) pair collapses to "no
//!    edges" (`Indeterminate + Skip`).
//! 2. Apply the cross-language pre-filter from
//!    `Detection.applies_to_languages` to `symbols` and `edges`
//!    before the plugin sees them. A plugin that declared `[Ruby]`
//!    cannot see Python symbols even if a Python decorator shares a
//!    name with a Ruby callback.
//! 3. Collapse `DetectedVersion + UnknownVersionPolicy` into a single
//!    `ResolvedVersion` passed to the plugin.
//!
//! The indexer (`scope-index::indexer::Indexer`) calls this function
//! between symbol writes and edge writes; the synthetic-framework
//! integration tests at `scope-core/tests/` also call it directly.

use std::collections::HashSet;

use super::{Detection, DetectedVersion, FrameworkPlugin, ResolvedVersion, UnknownVersionPolicy};
use crate::edge::RawEdge;
use crate::types::{Edge, Symbol};
use crate::workspace_context::FrameworkWorkspaceContext;

/// Run every framework plugin's predicate over `symbols` + `edges`,
/// applying the cross-language pre-filter declared in each plugin's
/// `Detection.applies_to_languages` first.
///
/// Edge keep-rule: an edge is kept for a plugin iff its **source
/// endpoint** (`from_id`) belongs to a symbol whose language is in
/// the plugin's `applies_to_languages`. The source endpoint is the
/// language that emitted the edge — keeping only those edges guarantees
/// no cross-language leakage (a Python `extends` edge whose `to_id`
/// happens to match a Ruby symbol id will not reach a Ruby-only
/// framework). Bare-name `to_id` targets stay accessible because the
/// rule does not require `to_id` membership; the resolver later
/// promotes unresolved bare names to `Dangling`. Sprint 0005 codex
/// review (round 1) tightened this from the looser
/// `from ∈ kept ∨ to ∈ kept` form that the original draft used.
///
/// A plugin whose `Detection.detected` is `false`, or whose
/// `applies_to_languages` is empty when `detected: true` (a R5
/// acceptance violation — never panic; runtime-skip and let the
/// `audit_patterns.sh` CI gate catch it at build time), is skipped
/// entirely.
pub fn run_frameworks(
    plugins: &[Box<dyn FrameworkPlugin>],
    ctx: &dyn FrameworkWorkspaceContext,
    symbols: &[Symbol],
    edges: &[Edge],
) -> Vec<RawEdge> {
    let mut out = Vec::new();
    for plugin in plugins {
        let detection = plugin.detect(ctx);
        if !detection.detected || detection.applies_to_languages.is_empty() {
            continue;
        }
        let Some(version) = resolve_version(&detection.version, &plugin.unknown_version_policy())
        else {
            continue;
        };

        let (filtered_symbols, filtered_edges) = apply_pre_filter(&detection, symbols, edges);
        out.extend(plugin.match_edges(&filtered_symbols, &filtered_edges, version));
    }
    out
}

/// Cross-language pre-filter exposed separately so integration tests
/// can assert it independently from `run_frameworks`.
pub fn apply_pre_filter(
    detection: &Detection,
    symbols: &[Symbol],
    edges: &[Edge],
) -> (Vec<Symbol>, Vec<Edge>) {
    let lang_slugs: HashSet<&'static str> = detection
        .applies_to_languages
        .iter()
        .map(|l| l.as_str())
        .collect();

    let filtered_symbols: Vec<Symbol> = symbols
        .iter()
        .filter(|s| lang_slugs.contains(s.language.as_str()))
        .cloned()
        .collect();

    let kept_ids: HashSet<&str> = filtered_symbols.iter().map(|s| s.id.as_str()).collect();
    let filtered_edges: Vec<Edge> = edges
        .iter()
        .filter(|e| kept_ids.contains(e.from_id()))
        .cloned()
        .collect();

    (filtered_symbols, filtered_edges)
}

/// Collapse `(DetectedVersion, UnknownVersionPolicy)` into the version
/// `match_edges` receives. Returns `None` for the `Indeterminate +
/// Skip` short-circuit (zero edges, plugin not invoked).
pub fn resolve_version(
    detected: &DetectedVersion,
    policy: &UnknownVersionPolicy,
) -> Option<ResolvedVersion> {
    match (detected, policy) {
        (DetectedVersion::Resolved(v), _) => Some(ResolvedVersion::Detected(v.clone())),
        (DetectedVersion::NoVersionConcept, _) => Some(ResolvedVersion::Versionless),
        (DetectedVersion::Indeterminate, UnknownVersionPolicy::Skip) => None,
        (DetectedVersion::Indeterminate, UnknownVersionPolicy::StableOnlyLowConfidence) => {
            Some(ResolvedVersion::Fallback)
        }
        (DetectedVersion::Indeterminate, UnknownVersionPolicy::AssumeLatest(v)) => {
            Some(ResolvedVersion::Assumed(v.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::LanguageId;
    use semver::Version;

    fn sym(id: &str, lang: &str) -> Symbol {
        Symbol {
            id: id.into(),
            name: id.into(),
            kind: "function".into(),
            file_path: "x".into(),
            line_start: 1,
            line_end: 1,
            signature: None,
            docstring: None,
            parent_id: None,
            language: lang.into(),
            metadata: "{}".into(),
        }
    }

    #[test]
    fn resolve_version_indeterminate_skip_short_circuits() {
        assert!(
            resolve_version(&DetectedVersion::Indeterminate, &UnknownVersionPolicy::Skip).is_none()
        );
    }

    #[test]
    fn resolve_version_indeterminate_fallback_returns_fallback() {
        let v = resolve_version(
            &DetectedVersion::Indeterminate,
            &UnknownVersionPolicy::StableOnlyLowConfidence,
        );
        assert!(matches!(v, Some(ResolvedVersion::Fallback)));
    }

    #[test]
    fn resolve_version_indeterminate_assume_latest_returns_assumed() {
        let v = resolve_version(
            &DetectedVersion::Indeterminate,
            &UnknownVersionPolicy::AssumeLatest(Version::new(7, 0, 0)),
        );
        assert!(matches!(v, Some(ResolvedVersion::Assumed(_))));
    }

    #[test]
    fn resolve_version_resolved_passes_through() {
        let v = resolve_version(
            &DetectedVersion::Resolved(Version::new(5, 2, 0)),
            &UnknownVersionPolicy::Skip,
        );
        assert!(matches!(v, Some(ResolvedVersion::Detected(_))));
    }

    #[test]
    fn pre_filter_drops_other_language_symbols() {
        let detection = Detection {
            detected: true,
            version: DetectedVersion::NoVersionConcept,
            applies_to_languages: vec![LanguageId::Ruby],
        };
        let symbols = vec![sym("ruby_one", "ruby"), sym("py_one", "python")];
        let (kept, _) = apply_pre_filter(&detection, &symbols, &[]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "ruby_one");
    }

    // Codex sprint 0005 round-1 regression: edge keep-rule must reject
    // edges emitted by a non-target language even when the to-endpoint
    // happens to point at a target-language symbol. The old OR form
    // (`from ∈ kept ∨ to ∈ kept`) leaked a Python `extends` edge to a
    // Ruby-only framework whenever the edge pointed at a Ruby symbol.
    #[test]
    fn pre_filter_excludes_other_language_edge_pointing_at_kept_symbol() {
        let detection = Detection {
            detected: true,
            version: DetectedVersion::NoVersionConcept,
            applies_to_languages: vec![LanguageId::Ruby],
        };
        let symbols = vec![sym("ruby_target", "ruby"), sym("py_source", "python")];
        // Python-emitted edge whose to_id is a Ruby symbol. The old OR
        // rule kept this (`to_id ∈ kept`); the tightened rule drops it
        // (`from_id ∉ kept`).
        let edge = crate::edge::RawEdge::builder()
            .from("py_source")
            .to("ruby_target")
            .kind(crate::edge::EdgeKind::Extends)
            .confidence(crate::edge::Confidence::High)
            .producer(crate::edge::Producer::Lang("python".into()))
            .pattern_id("extends.class")
            .file_path("a.py")
            .build();
        let (_, kept_edges) = apply_pre_filter(&detection, &symbols, &[edge]);
        assert!(
            kept_edges.is_empty(),
            "edges emitted by an excluded language must be dropped"
        );
    }

    // Complement: a target-language edge with a bare-name to_id (no
    // corresponding kept symbol) must still pass through, because the
    // resolver later promotes such bare names to Dangling.
    #[test]
    fn pre_filter_keeps_target_language_edge_with_bare_name_to() {
        let detection = Detection {
            detected: true,
            version: DetectedVersion::NoVersionConcept,
            applies_to_languages: vec![LanguageId::Ruby],
        };
        let symbols = vec![sym("ruby_source", "ruby")];
        let edge = crate::edge::RawEdge::builder()
            .from("ruby_source")
            .to("some_bare_name_with_no_symbol_row")
            .kind(crate::edge::EdgeKind::Calls)
            .confidence(crate::edge::Confidence::High)
            .producer(crate::edge::Producer::Lang("ruby".into()))
            .pattern_id("calls.method")
            .file_path("a.rb")
            .build();
        let (_, kept_edges) = apply_pre_filter(&detection, &symbols, &[edge]);
        assert_eq!(
            kept_edges.len(),
            1,
            "bare-name to_id should not block a target-language edge"
        );
    }
}
