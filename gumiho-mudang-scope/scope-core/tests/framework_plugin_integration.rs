//! R5 acceptance — synthetic-framework integration tests (sprint 0005).
//!
//! Exercises every R5 acceptance bullet without committing scope to
//! any real-world framework:
//!
//! - Plugin emits edges when language metadata is populated.
//! - Removing the metadata produces zero edges (graph-only contract).
//! - Cross-language pre-filter excludes other-language symbols.
//! - Version pinned outside `available_in` produces zero edges.
//! - `unknown_version_policy()` variants produce the documented
//!   outcomes (Skip / StableOnlyLowConfidence / AssumeLatest).
//!
//! Bypasses `Indexer::index_full` deliberately. R5 ships the
//! `scope_core::frameworks::dispatch::run_frameworks` helper as the
//! cross-language pre-filter; the indexer wires the seam when real
//! frameworks adopt post-refactor (see comment block at the
//! framework-dispatch site in `scope-index::indexer`). Until then,
//! tests call the helper directly.

mod synthetic_framework;

use scope_core::edge::Producer;
use scope_core::frameworks::dispatch::run_frameworks;
use scope_core::frameworks::{DetectedVersion, FrameworkPlugin, UnknownVersionPolicy};
use scope_core::languages::LanguageId;
use scope_core::{Confidence, EdgeKind};
use semver::Version;

use synthetic_framework::{
    all_patterns, marked_symbol, plain_symbol, queue_symbol, SyntheticCtx, SyntheticPlugin, NAME,
};

fn boxed(p: SyntheticPlugin) -> Vec<Box<dyn FrameworkPlugin>> {
    vec![Box::new(p)]
}

// Acceptance bullet — pattern catalog shape is what the audit gate
// expects: every entry has a non-empty `id`, a `VersionReq`, and a
// referenced `predicate` fn (the fn-pointer slot is `Option`-free).
#[test]
fn synthetic_pattern_catalog_shape_is_audit_compatible() {
    let patterns = all_patterns();
    assert!(!patterns.is_empty(), "catalog must not be empty");
    for p in &patterns {
        assert!(!p.id.is_empty(), "pattern id must be non-empty");
        // VersionReq is constructed: tag is either STAR or a parsed range.
        // The Default impl rejects empty input so any value here is valid.
        let _ = &p.available_in;
        // predicate is a fn pointer, automatically non-null in Rust.
        let _ = p.predicate;
    }
}

// Acceptance bullet — plugin emits edges when metadata is populated.
#[test]
fn populated_metadata_produces_edges() {
    let plugins = boxed(SyntheticPlugin::detected_ruby_v1());
    let ctx = SyntheticCtx::default();
    let symbols = vec![marked_symbol(
        "ruby_marked",
        LanguageId::Ruby,
        "synthetic.render_v1",
    )];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);

    assert_eq!(edges.len(), 1, "one renders edge expected");
    let edge = &edges[0];
    assert_eq!(edge.kind(), EdgeKind::Renders);
    assert_eq!(edge.from_id(), "ruby_marked");
    assert_eq!(edge.pattern_id(), "synthetic.render_v1");
    assert!(matches!(edge.producer(), Producer::Framework(n) if n == NAME));
}

// Acceptance bullet — removing the metadata produces zero edges.
#[test]
fn unpopulated_metadata_produces_zero_edges() {
    let plugins = boxed(SyntheticPlugin::detected_ruby_v1());
    let ctx = SyntheticCtx::default();
    let symbols = vec![plain_symbol("ruby_plain", LanguageId::Ruby)];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert!(edges.is_empty(), "no metadata → no edges (graph-only)");
}

// Acceptance bullet — cross-language pre-filter blocks other-language
// matches.
#[test]
fn cross_language_pre_filter_blocks_other_language_match() {
    // Plugin declares Ruby; a Python symbol carries a matching marker.
    // Pre-filter must strip the Python symbol before the predicate.
    let plugins = boxed(SyntheticPlugin::detected_ruby_v1());
    let ctx = SyntheticCtx::default();
    let symbols = vec![marked_symbol(
        "python_decoy",
        LanguageId::Python,
        "synthetic.render_v1",
    )];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert!(
        edges.is_empty(),
        "cross-language pre-filter must strip Python symbol from Ruby framework"
    );
}

// Acceptance bullet — version pinned outside available_in → zero edges.
#[test]
fn version_outside_available_in_produces_zero_edges() {
    let mut plugin = SyntheticPlugin::detected_ruby_v1();
    // 3.0.0 is outside `>=1.0.0, <2.0.0` (the v1 pattern) and is also
    // matched by STAR (the fallback pattern) — but the fallback
    // pattern's marker is `synthetic.render_fallback`, which the
    // marked symbol below does not carry. So both patterns miss.
    plugin.version = DetectedVersion::Resolved(Version::new(3, 0, 0));
    let plugins: Vec<Box<dyn FrameworkPlugin>> = vec![Box::new(plugin)];
    let ctx = SyntheticCtx::default();
    let symbols = vec![marked_symbol(
        "ruby_marked",
        LanguageId::Ruby,
        "synthetic.render_v1",
    )];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert!(
        edges.is_empty(),
        "v3.0.0 outside `>=1.0.0, <2.0.0` and marker doesn't match fallback pattern"
    );
}

// Acceptance bullet — unknown_version_policy::Skip → zero edges.
#[test]
fn indeterminate_version_skip_policy_emits_zero_edges() {
    let plugin = SyntheticPlugin {
        detected: true,
        version: DetectedVersion::Indeterminate,
        languages: vec![LanguageId::Ruby],
        policy: UnknownVersionPolicy::Skip,
    };
    let plugins: Vec<Box<dyn FrameworkPlugin>> = vec![Box::new(plugin)];
    let ctx = SyntheticCtx::default();
    let symbols = vec![marked_symbol(
        "ruby_marked",
        LanguageId::Ruby,
        "synthetic.render_fallback",
    )];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert!(
        edges.is_empty(),
        "Skip policy short-circuits before match_edges"
    );
}

// Acceptance bullet — unknown_version_policy::StableOnlyLowConfidence →
// fallback-tagged edges only.
#[test]
fn indeterminate_version_fallback_policy_emits_low_confidence() {
    let plugin = SyntheticPlugin {
        detected: true,
        version: DetectedVersion::Indeterminate,
        languages: vec![LanguageId::Ruby],
        policy: UnknownVersionPolicy::StableOnlyLowConfidence,
    };
    let plugins: Vec<Box<dyn FrameworkPlugin>> = vec![Box::new(plugin)];
    let ctx = SyntheticCtx::default();
    let symbols = vec![
        marked_symbol("v1_marker", LanguageId::Ruby, "synthetic.render_v1"),
        marked_symbol(
            "fallback_marker",
            LanguageId::Ruby,
            "synthetic.render_fallback",
        ),
    ];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert_eq!(
        edges.len(),
        1,
        "fallback subset (STAR-tagged) only — v1 pattern is excluded"
    );
    assert_eq!(edges[0].pattern_id(), "synthetic.render_fallback");
    assert_eq!(edges[0].confidence(), Confidence::Low);
    assert!(
        matches!(edges[0].producer(), Producer::Framework(n) if n.ends_with(":fallback")),
        "producer must carry :fallback suffix"
    );
}

// Acceptance bullet — unknown_version_policy::AssumeLatest →
// assumed-tagged edges from the latest pattern set.
#[test]
fn indeterminate_version_assume_latest_emits_assumed() {
    let plugin = SyntheticPlugin {
        detected: true,
        version: DetectedVersion::Indeterminate,
        languages: vec![LanguageId::Ruby],
        policy: UnknownVersionPolicy::AssumeLatest(Version::new(1, 5, 0)),
    };
    let plugins: Vec<Box<dyn FrameworkPlugin>> = vec![Box::new(plugin)];
    let ctx = SyntheticCtx::default();
    let symbols = vec![marked_symbol(
        "assume_marker",
        LanguageId::Ruby,
        "synthetic.render_v1",
    )];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert_eq!(edges.len(), 1, "v1 pattern fires under assumed 1.5.0");
    assert!(
        matches!(edges[0].producer(), Producer::Framework(n) if n.contains(":assumed_")),
        "producer must carry :assumed_<v> suffix"
    );
}

// Acceptance bullet — detected: false plugin contributes no edges.
#[test]
fn undetected_plugin_skipped() {
    let plugin = SyntheticPlugin {
        detected: false,
        version: DetectedVersion::Resolved(Version::new(1, 0, 0)),
        languages: vec![LanguageId::Ruby],
        policy: UnknownVersionPolicy::Skip,
    };
    let plugins: Vec<Box<dyn FrameworkPlugin>> = vec![Box::new(plugin)];
    let ctx = SyntheticCtx::default();
    let symbols = vec![marked_symbol(
        "ruby_marked",
        LanguageId::Ruby,
        "synthetic.render_v1",
    )];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert!(edges.is_empty(), "undetected plugin must not emit");
}

// Acceptance bullet — cross-crate `queue_handler` edge.
// Two sender symbols and one receiver symbol live in different crates
// (different file path roots); the synthetic's cross-crate queue
// predicate must pair them by topic and emit `QueueHandler` edges
// from sender → receiver, validating that framework predicates see
// one polyglot pool of symbols regardless of crate boundary.
#[test]
fn cross_crate_queue_handler_edges_emitted() {
    let plugins = boxed(SyntheticPlugin::detected_ruby_v1());
    let ctx = SyntheticCtx::default();
    let symbols = vec![
        queue_symbol(
            "crate_a_sender",
            LanguageId::Ruby,
            "synthetic.queue_send",
            "orders",
            "crate_a",
        ),
        queue_symbol(
            "crate_b_receiver",
            LanguageId::Ruby,
            "synthetic.queue_recv",
            "orders",
            "crate_b",
        ),
        // Decoy: same crate as sender, different topic — must not pair.
        queue_symbol(
            "crate_a_decoy_recv",
            LanguageId::Ruby,
            "synthetic.queue_recv",
            "billing",
            "crate_a",
        ),
    ];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    let queue_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.kind() == EdgeKind::QueueHandler)
        .collect();
    assert_eq!(
        queue_edges.len(),
        1,
        "exactly one cross-crate queue edge (orders sender → orders receiver)"
    );
    assert_eq!(queue_edges[0].from_id(), "crate_a_sender");
    assert_eq!(queue_edges[0].to_id(), "crate_b_receiver");
    assert_eq!(queue_edges[0].pattern_id(), "synthetic.queue.cross_crate");
}

// Acceptance bullet — empty applies_to_languages with detected=true is
// rejected (runtime-skipped; audit gate is the build-time complement).
#[test]
fn empty_applies_to_languages_runtime_skipped() {
    let plugin = SyntheticPlugin {
        detected: true,
        version: DetectedVersion::Resolved(Version::new(1, 0, 0)),
        languages: vec![],
        policy: UnknownVersionPolicy::Skip,
    };
    let plugins: Vec<Box<dyn FrameworkPlugin>> = vec![Box::new(plugin)];
    let ctx = SyntheticCtx::default();
    let symbols = vec![marked_symbol(
        "ruby_marked",
        LanguageId::Ruby,
        "synthetic.render_v1",
    )];

    let edges = run_frameworks(&plugins, &ctx, &symbols, &[]);
    assert!(
        edges.is_empty(),
        "empty applies_to_languages contract violation must runtime-skip, not panic"
    );
}
