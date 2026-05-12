//! Synthetic framework for R5 integration tests (sprint 0005).
//!
//! Exercises the `FrameworkPlugin` trait surface without committing
//! scope to maintain any real-world framework. Sprint 0005 ships only
//! infrastructure; concrete framework adoption follows
//! `FRAMEWORK-PLAYBOOK.md` post-refactor.
//!
//! The synthetic recognises symbols with a specific decorator marker
//! in their metadata (the R0 reserved key `decorators`) and emits
//! `Renders` edges. It treats Ruby as its target language by default,
//! so the cross-language pre-filter is exercised when fed Python
//! symbols. Several version-pinned patterns prove the `available_in`
//! gate; an `unknown_version_policy()` variant is selected by the
//! plugin's constructor argument.
//!
//! See sprint 0005 ambiguity #1 resolution in
//! `ARCHITECTURAL-REFACTOR.md` § R5 → "Synthetic test framework
//! location" for why this lives under `tests/` rather than
//! `src/frameworks/_test/`.

use std::collections::BTreeMap;

use scope_core::edge::Producer;
use scope_core::frameworks::{
    Detection, DetectedVersion, FrameworkPlugin, Pattern, ResolvedVersion, UnknownVersionPolicy,
};
use scope_core::languages::LanguageId;
use scope_core::types::{Edge, Symbol};
use scope_core::workspace_context::{
    Dependency, FileId, FrameworkVersions, FrameworkWorkspaceContext, LanguageWorkspaceContext,
    Lockfile, ModuleLayout, Package,
};
use scope_core::{Confidence, EdgeKind, RawEdge};
use semver::{Version, VersionReq};

/// Synthetic framework name — used in `Producer::Framework(name)`.
pub const NAME: &str = "synthetic";

/// Plugin builder. Each test constructs one with explicit fields so the
/// detection / version-policy permutations are obvious at the call
/// site.
pub struct SyntheticPlugin {
    pub detected: bool,
    pub version: DetectedVersion,
    pub languages: Vec<LanguageId>,
    pub policy: UnknownVersionPolicy,
}

impl SyntheticPlugin {
    /// Default: detected, Ruby-only, version `1.0.0`, policy `Skip`.
    pub fn detected_ruby_v1() -> Self {
        Self {
            detected: true,
            version: DetectedVersion::Resolved(Version::new(1, 0, 0)),
            languages: vec![LanguageId::Ruby],
            policy: UnknownVersionPolicy::Skip,
        }
    }
}

impl FrameworkPlugin for SyntheticPlugin {
    fn name(&self) -> &str {
        NAME
    }

    fn detect(&self, _ctx: &dyn FrameworkWorkspaceContext) -> Detection {
        Detection {
            detected: self.detected,
            version: self.version.clone(),
            applies_to_languages: self.languages.clone(),
        }
    }

    fn unknown_version_policy(&self) -> UnknownVersionPolicy {
        self.policy.clone()
    }

    fn match_edges(
        &self,
        symbols: &[Symbol],
        _edges: &[Edge],
        version: ResolvedVersion,
    ) -> Vec<RawEdge> {
        let mut out = Vec::new();
        let (confidence, producer_suffix) = match &version {
            ResolvedVersion::Fallback => (Confidence::Low, ":fallback".to_string()),
            ResolvedVersion::Assumed(v) => (Confidence::High, format!(":assumed_{v}")),
            _ => (Confidence::High, String::new()),
        };
        let producer_name = format!("{NAME}{producer_suffix}");

        // Per-symbol patterns (Renders).
        for pattern in patterns_per_symbol() {
            if !pattern_active(&pattern, &version) {
                continue;
            }
            for s in symbols {
                if !symbol_has_marker(s, pattern.id) {
                    continue;
                }
                out.push(
                    RawEdge::builder()
                        .from(s.id.clone())
                        .to(format!("{}::view", s.id))
                        .kind(pattern.edge_kind)
                        .confidence(confidence)
                        .producer(Producer::Framework(producer_name.clone()))
                        .pattern_id(pattern.id)
                        .file_path(s.file_path.clone())
                        .line(s.line_start)
                        .build(),
                );
            }
        }

        // Cross-crate queue pattern: connect every send-marked symbol
        // to every recv-marked symbol with a matching topic field.
        // Validates that framework predicates see one polyglot pool
        // (charter §3 invariant 4); the indexer's cross-language
        // pre-filter already restricted to the plugin's
        // applies_to_languages.
        let queue_pattern = pattern_queue_cross_crate();
        if pattern_active(&queue_pattern, &version) {
            let senders: Vec<&Symbol> = symbols
                .iter()
                .filter(|s| symbol_has_marker(s, "synthetic.queue_send"))
                .collect();
            let receivers: Vec<&Symbol> = symbols
                .iter()
                .filter(|s| symbol_has_marker(s, "synthetic.queue_recv"))
                .collect();
            for send in &senders {
                let Some(send_topic) = symbol_topic(send) else {
                    continue;
                };
                for recv in &receivers {
                    if symbol_topic(recv).as_deref() != Some(send_topic.as_str()) {
                        continue;
                    }
                    out.push(
                        RawEdge::builder()
                            .from(send.id.clone())
                            .to(recv.id.clone())
                            .kind(queue_pattern.edge_kind)
                            .confidence(confidence)
                            .producer(Producer::Framework(producer_name.clone()))
                            .pattern_id(queue_pattern.id)
                            .file_path(send.file_path.clone())
                            .line(send.line_start)
                            .build(),
                    );
                }
            }
        }
        out
    }
}

/// Whether a pattern's `available_in` matches the resolved version. A
/// `Fallback` resolution restricts to `VersionReq::STAR`-tagged
/// patterns; `Versionless` and `Assumed`/`Detected` use `matches` on
/// the carried `Version` (or accept STAR for `Versionless`).
fn pattern_active(pattern: &Pattern, version: &ResolvedVersion) -> bool {
    match version {
        ResolvedVersion::Detected(v) | ResolvedVersion::Assumed(v) => {
            pattern.available_in.matches(v)
        }
        ResolvedVersion::Fallback => pattern.available_in == VersionReq::STAR,
        ResolvedVersion::Versionless => true,
    }
}

/// Synthetic markers live in the symbol's metadata blob — the R5
/// model: language plugin populates `Symbol.metadata`, framework
/// plugin reads it. The test corpus uses a JSON object with key
/// `synthetic_marker`.
fn symbol_has_marker(symbol: &Symbol, marker: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&symbol.metadata) else {
        return false;
    };
    value
        .get("synthetic_marker")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s == marker)
}

/// Synthetic per-symbol pattern catalog. Two patterns — one v1-only,
/// one fallback-eligible (`VersionReq::STAR`).
pub fn patterns_per_symbol() -> Vec<Pattern> {
    vec![
        Pattern {
            id: "synthetic.render_v1",
            edge_kind: EdgeKind::Renders,
            available_in: VersionReq::parse(">=1.0.0, <2.0.0").unwrap(),
            predicate: per_symbol_predicate_stub,
        },
        Pattern {
            id: "synthetic.render_fallback",
            edge_kind: EdgeKind::Renders,
            available_in: VersionReq::STAR,
            predicate: per_symbol_predicate_stub,
        },
    ]
}

/// Synthetic cross-crate queue pattern. Validates that framework
/// predicates see one polyglot pool of symbols regardless of crate /
/// file boundary.
pub fn pattern_queue_cross_crate() -> Pattern {
    Pattern {
        id: "synthetic.queue.cross_crate",
        edge_kind: EdgeKind::QueueHandler,
        available_in: VersionReq::STAR,
        predicate: cross_crate_predicate_stub,
    }
}

/// Full catalog (per-symbol + cross-crate). Used by
/// `audit_patterns.sh` reachability checks.
pub fn all_patterns() -> Vec<Pattern> {
    let mut p = patterns_per_symbol();
    p.push(pattern_queue_cross_crate());
    p
}

// Predicate fn-pointer stubs. The synthetic plugin's `match_edges`
// constructs edges directly because predicate signatures cannot carry
// `ResolvedVersion` (R5 doc fixes the signature); the predicate slot
// on `Pattern` is therefore a catalog marker for `audit_patterns.sh`
// rather than a live dispatcher.
fn per_symbol_predicate_stub(_symbols: &[Symbol], _edges: &[Edge]) -> Vec<RawEdge> {
    Vec::new()
}

fn cross_crate_predicate_stub(_symbols: &[Symbol], _edges: &[Edge]) -> Vec<RawEdge> {
    Vec::new()
}

/// Extract the `topic` field from the JSON-encoded `Symbol.metadata`.
/// Used by the cross-crate queue predicate to pair senders with
/// receivers.
fn symbol_topic(symbol: &Symbol) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(&symbol.metadata).ok()?;
    value
        .get("topic")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Minimal `FrameworkWorkspaceContext` implementation for integration
/// tests. Carries an empty package set and an empty
/// `FrameworkVersions`; tests that need a version pass it through the
/// plugin's `version` field directly (since the synthetic plugin
/// doesn't actually read `ctx` — its `detect()` returns
/// pre-constructed state).
pub struct SyntheticCtx {
    pub frameworks: FrameworkVersions,
    pub layout: ModuleLayout,
}

impl Default for SyntheticCtx {
    fn default() -> Self {
        Self {
            frameworks: FrameworkVersions {
                versions: BTreeMap::new(),
            },
            layout: ModuleLayout::default(),
        }
    }
}

impl LanguageWorkspaceContext for SyntheticCtx {
    fn package_for_file(&self, _file: FileId) -> Option<&Package> {
        None
    }
    fn dependencies(&self, _package: &Package) -> &[Dependency] {
        &[]
    }
    fn is_workspace_internal(&self, _import: &str, _from: FileId) -> bool {
        false
    }
    fn module_layout(&self, _package: &Package) -> &ModuleLayout {
        &self.layout
    }
}

impl FrameworkWorkspaceContext for SyntheticCtx {
    fn framework_versions(&self) -> &FrameworkVersions {
        &self.frameworks
    }
    fn lockfile(&self) -> Option<&Lockfile> {
        None
    }
}

/// Builds a symbol carrying the given synthetic marker. Tests use this
/// to construct minimal corpora without going through tree-sitter.
pub fn marked_symbol(id: &str, language: LanguageId, marker: &str) -> Symbol {
    Symbol {
        id: id.into(),
        name: id.into(),
        kind: "function".into(),
        file_path: format!("app/{id}.rb"),
        line_start: 1,
        line_end: 1,
        signature: None,
        docstring: None,
        parent_id: None,
        language: language.as_str().into(),
        metadata: format!(r#"{{"synthetic_marker":"{marker}"}}"#),
    }
}

/// Builds a symbol with no synthetic marker (negative-control fixture).
pub fn plain_symbol(id: &str, language: LanguageId) -> Symbol {
    Symbol {
        id: id.into(),
        name: id.into(),
        kind: "function".into(),
        file_path: format!("app/{id}.rb"),
        line_start: 1,
        line_end: 1,
        signature: None,
        docstring: None,
        parent_id: None,
        language: language.as_str().into(),
        metadata: "{}".into(),
    }
}

/// Builds a symbol carrying both a marker and a queue topic — for
/// cross-crate queue tests. `crate_root` is the crate-relative file
/// prefix (e.g., `"crate_a"`), so the resulting `file_path` is e.g.
/// `crate_a/src/sender.rs`.
pub fn queue_symbol(
    id: &str,
    language: LanguageId,
    marker: &str,
    topic: &str,
    crate_root: &str,
) -> Symbol {
    Symbol {
        id: id.into(),
        name: id.into(),
        kind: "function".into(),
        file_path: format!("{crate_root}/src/{id}.rs"),
        line_start: 1,
        line_end: 1,
        signature: None,
        docstring: None,
        parent_id: None,
        language: language.as_str().into(),
        metadata: format!(r#"{{"synthetic_marker":"{marker}","topic":"{topic}"}}"#),
    }
}
