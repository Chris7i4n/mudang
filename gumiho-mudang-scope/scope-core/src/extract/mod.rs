//! Extractor layer (R2 target state).
//!
//! Sits between the language plugin and the resolver in the
//! `extract → resolve → write` pipeline encoded by R3:
//!
//! ```text
//! plugin (.scm)  ──►  RawCaptures  ──►  Extractor  ──►  RawEdge  ──►  Resolver  ──►  InsertableEdge  ──►  Graph
//!                                       └──── this module ────┘    └──── scope-graph::resolve ────┘
//! ```
//!
//! ## Responsibilities
//!
//! - Read `RawCaptures` (typed tree-sitter capture results + metadata
//!   + plugin-driven skipped ranges) from a language plugin.
//! - Apply per-`EdgeKind` decisions: which capture name (or capture
//!   combination) maps to which `EdgeKind`, what confidence tier the
//!   pattern earns, which `pattern_id` to stamp.
//! - Emit `RawEdge` values via `EdgeBuilder`. No `status` field is
//!   set; that is the resolver's job (R3).
//!
//! ## Non-responsibilities
//!
//! - **No workspace I/O.** The extractor does not consult
//!   `LanguageWorkspaceContext`; resolution against the symbol table
//!   happens in `scope-graph::resolve`.
//! - **No `EdgeKind` outside this module.** Per R2 target state, the
//!   extractor is the **only place that knows about `EdgeKind`** —
//!   plugin code emits typed captures, never edges.
//! - **No status assignment.** `RawEdge` carries no `status`; the
//!   builder has no `.status(...)` setter (R1).
//! - **No filesystem, no tree-sitter parse, no LanguageWorkspaceContext.**
//!
//! ## Charter alignment
//!
//! - Charter §3 invariant 5 (tree-sitter resilience): the extractor
//!   forwards `RawCaptures.skipped_ranges` verbatim; it never
//!   re-orders, merges, or filters them. Indexer concatenates these
//!   with tree-sitter-error skips into `file_hashes.skipped_ranges`
//!   (R&).
//! - `LANGUAGE-PLAYBOOK.md` C2 (no version-coupled config in language
//!   layer): the extractor has no parameter that could carry such
//!   config — `RawCaptures` is the only input, and its types are
//!   pinned by `Capture` / `MetadataField` / `SkippedRange` below.
//!
//! ## Sprint 0003 status
//!
//! Chunks 1-3: types + module skeleton + per-language extractor
//! relocation + metadata reserved-keys + skipped_ranges plumbing.
//!
//! Chunk 7: RawCaptures plugin-output migration. The parser pipeline
//! builds `RawCaptures` per file (with per-match `CapturedMatch`
//! grouping carrying `pattern_index` + pre-resolved
//! `enclosing_scope_id`) and dispatches into the per-language
//! extractor through `extract_edges`. The legacy
//! `HashMap<String, (text, line)>` plumbing and the
//! `pattern_id = "legacy.<kind>"` placeholder are deleted in this
//! same chunk; `make_edge` now takes an explicit `pattern_id`.
//!
//! Phase B ships R12 (trait-shape audit). The CI gate
//! at that point validates that this module exposes no method whose
//! signature implies inference, expansion, resolution, evaluation,
//! narrowing, or overload resolution.

use serde::{Deserialize, Serialize};

use crate::edge::RawEdge;
use crate::languages::LanguageId;

pub mod csharp;
pub mod error_scan;
pub mod go_lang;
pub mod java;
pub mod python;
pub mod ruby;
pub mod rust_lang;
pub mod typescript;

pub use error_scan::scan_tree_sitter_errors;

/// Typed output of a language plugin's tree-sitter capture pass.
///
/// Replaces the historical "plugin returns edges directly" shape with
/// the R2 contract: plugin reports **what it saw**, not **what edges
/// it implies**. The extractor (this module) translates capture
/// results into `RawEdge` values; the resolver
/// (`scope-graph::resolve`) translates `RawEdge` into `InsertableEdge`
/// by consulting `LanguageWorkspaceContext`.
///
/// All three fields are owned: captured text is materialised eagerly
/// so the structure can outlive the tree-sitter `Tree` it came from.
/// This keeps the extractor / resolver boundary cleanly cloneable
/// and serialisable (handy for snapshot tests in the R6 malformed-source
/// harness).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawCaptures {
    /// One entry per query match. Each `CapturedMatch` groups the
    /// captures that fired together for a single tree-sitter query
    /// match (along with its `pattern_index` and the pre-resolved
    /// `enclosing_scope_id`). Per-match grouping is required because
    /// the extractor dispatches per-match: a single match defining
    /// `@imported_name` + `@source` needs both captures together.
    pub matches: Vec<CapturedMatch>,

    /// Declared metadata for symbols the plugin saw — decorator args,
    /// annotation text, template render call lists, etc. The three
    /// reserved keys per R0 schema and `LANGUAGE-PLAYBOOK.md` Step 5
    /// are `decorators`, `annotations`, `template_calls`. Plugins
    /// **omit** a key entirely if the language has no AST surface for
    /// it; presence of the key with an empty array means "I looked
    /// and found none" (lands the rule mechanically).
    pub metadata: Vec<MetadataField>,

    /// Plugin-driven skipped ranges. Each entry is a region the plugin
    /// chose not to analyse (e.g., a macro body it cannot interpret).
    /// The indexer merges these with tree-sitter-error skips into
    /// `file_hashes.skipped_ranges` (R6). The extractor
    /// passes them through unchanged; charter invariant 5
    /// (tree-sitter resilience) forbids any reorder / merge / filter
    /// at this layer.
    pub skipped_ranges: Vec<SkippedRange>,
}

/// A single tree-sitter query match: a pattern fired and produced one
/// or more captures.
///
/// `pattern_index` is the `usize` returned by tree-sitter's
/// `QueryMatch::pattern_index` and identifies which pattern in
/// `queries/<lang>/edges.scm` fired.
/// `enclosing_scope_id` is the resolved-from-the-AST scope the parser
/// computed (smallest captured node → walked up to nearest
/// function-like / class-like ancestor); `None` for module-level
/// matches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapturedMatch {
    /// Index of the pattern that fired, as returned by tree-sitter's
    /// `QueryMatch::pattern_index`. Matches the source-file order of
    /// patterns in `queries/<lang>/edges.scm`.
    pub pattern_index: u32,

    /// Captures that fired in this match, in pattern declaration
    /// order. Each `Capture` is one bound capture name — a pattern
    /// declaring `@imported_name` + `@source` emits two `Capture`
    /// rows in one `CapturedMatch`.
    pub captures: Vec<Capture>,

    /// Pre-resolved enclosing scope id (function-like / class-like
    /// ancestor of the match's representative node, computed by the
    /// parser). `None` for module-level matches; per-extractor logic
    /// substitutes `{file_path}::__module__::{kind}` via
    /// [`resolve_scope_id`].
    pub enclosing_scope_id: Option<String>,
}

/// One captured node from a tree-sitter `.scm` query.
///
/// Eagerly materialised: `text` owns its bytes, position fields are
/// concrete `u32`s. The `RawCaptures` value can therefore travel
/// across the extractor / resolver / storage boundary without
/// carrying a `Tree` lifetime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capture {
    /// Capture name as declared in the `.scm` query, including any
    /// dotted prefix (e.g., `function.name`, `class.parent.qualified`).
    pub name: String,

    /// Tree-sitter node kind (e.g., `identifier`, `string_literal`,
    /// `class_declaration`). Used by the extractor to disambiguate
    /// when one capture name fires on multiple node kinds.
    pub node_kind: String,

    /// Captured text, owned.
    pub text: String,

    /// Byte offset of the capture's first byte in the source file.
    pub start_byte: u32,
    /// Byte offset of the capture's one-past-end byte.
    pub end_byte: u32,
    /// 1-based line of the capture's first byte.
    pub start_line: u32,
    /// 1-based line of the capture's last byte.
    pub end_line: u32,
    /// 0-based column of the capture's first byte (UTF-8 byte column).
    pub start_column: u32,
    /// 0-based column of the capture's one-past-end byte.
    pub end_column: u32,
}

/// One entry inside a reserved-key metadata array — `decorators`,
/// `annotations`, or `template_calls`.
///
/// Per `LANGUAGE-PLAYBOOK.md` § Step 5 / Metadata schema for framework
/// primitives, every entry in those three reserved arrays carries:
/// - `name`: the decorator / annotation / template / component name as
///   written in source (no `@`, no `#[]`, no `<>` brackets).
/// - `args_text`: optional raw argument list as written, including
///   delimiters. `None` when the form has no args
///   (e.g., `@staticmethod`, `#[derive]` with no body); the JSON shape
///   then omits the key entirely.
///
/// **Distinct from `Edge.args_text`** (R0 column on `edges`) — the two
/// share the field name because both record verbatim argument literals
/// captured by the language plugin, but they live on different rows.
/// Same E2 rule applies: the language plugin captures the literal
/// verbatim; framework plugins (R5) interpret.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MetadataEntry {
    /// Decorator / annotation / template name as written in source.
    pub name: String,
    /// Raw argument list, including delimiters. `None` when the form
    /// has no args; the JSON shape then omits the key entirely so the
    /// `args_text` field is present only when meaningful.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub args_text: Option<String>,
}

/// One declared metadata field attached to a symbol the plugin saw.
///
/// Reserved keys per `ENFORCEMENT-MAP.md` § R0 schema and
/// `LANGUAGE-PLAYBOOK.md` Step 5: `decorators`, `annotations`,
/// `template_calls`. Plugins may emit other keys for their own
/// reserved domain, but framework-shaped derivations (`hooks`, route
/// metadata, queue handlers) belong to the framework layer (R5,
/// the architecture) — language plugins must not pre-compute them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetadataField {
    /// The symbol this metadata is attached to. Carries the same
    /// `"{file_path}::{name}::{kind}"` shape as `Symbol.id`.
    pub symbol_id: String,

    /// Metadata key (e.g., `decorators`, `annotations`, `template_calls`).
    pub key: String,

    /// Metadata value as JSON. The exact shape per key is fixed by
    /// `LANGUAGE-PLAYBOOK.md` Step 5; the extractor passes the value
    /// through verbatim to the symbol's `metadata` JSON blob.
    pub value: serde_json::Value,
}

/// A region of source the plugin chose to skip.
///
/// Distinct from tree-sitter-error skips: those come from the
/// indexer's parser-level error recovery and are merged with the
/// plugin's `skipped_ranges` into `file_hashes.skipped_ranges` only
/// at the indexer layer (R6). The plugin records
/// **intentional** skips here — e.g., a macro body whose syntactic
/// content the plugin cannot interpret meaningfully.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkippedRange {
    /// 1-based first line of the skipped region.
    pub start_line: u32,
    /// 1-based last line of the skipped region (inclusive).
    pub end_line: u32,
    /// Free-form reason. Convention: `plugin_skip:<plugin>:<rationale>`,
    /// e.g., `plugin_skip:rust:unparseable_macro_body`. Sprint 0007's
    /// R6 harness greps this prefix for the malformed-source gate.
    pub reason: String,
}

/// Convert a `RawCaptures` snapshot for the given language into a
/// flat list of `RawEdge` values.
///
/// Iterates every `CapturedMatch` in input order, dispatches to the
/// per-language extractor by `pattern_index`, and concatenates the
/// resulting edges. The dispatch is an exhaustive match on
/// `LanguageId` inside [`extract_edges_for_match`] — there is no
/// trait method, no registry lookup.
///
/// `file_path` is contextual (not on `RawCaptures` because captures
/// are a structural record of what the plugin saw, not where it saw
/// it); the indexer threads it in.
pub fn extract_edges(lang: LanguageId, file_path: &str, captures: &RawCaptures) -> Vec<RawEdge> {
    captures
        .matches
        .iter()
        .flat_map(|m| {
            extract_edges_for_match(
                lang,
                m.pattern_index as usize,
                &m.captures,
                file_path,
                m.enclosing_scope_id.as_deref(),
            )
        })
        .collect()
}

/// Find the last `Capture` in `captures` whose `name` matches `name`.
///
/// Uses `rfind` for `HashMap`-style last-write-wins semantics: when
/// a pattern has multiple captures sharing one name (rare but
/// possible for repeated children), the final occurrence in
/// source/pattern order wins. Edge-equivalence with the legacy
/// `HashMap` path depends on this.
pub fn find_capture<'a>(captures: &'a [Capture], name: &str) -> Option<&'a Capture> {
    captures.iter().rfind(|c| c.name == name)
}

/// Resolve the `from_id` for an outgoing edge.
///
/// If `enclosing_scope_id` is present (i.e. the edge originates inside a
/// function or method), that ID is used directly. Otherwise a synthetic
/// module-level ID of the form `"{file_path}::__module__::{kind}"` is
/// returned, where `kind` is either `"function"` or `"class"`.
///
/// Lived in `crate::languages` before R2. Now owned by the extractor since
/// it is the only EdgeKind-aware site (R2 target state).
pub fn resolve_scope_id(enclosing_scope_id: Option<&str>, file_path: &str, kind: &str) -> String {
    enclosing_scope_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{file_path}::__module__::{kind}"))
}

/// Build a `RawEdge` with the given fields, routed through `EdgeBuilder`
/// per R1.
///
/// `confidence=medium` and `producer=Indexer` are defaulted because
/// every current extractor pattern is the language indexer producing
/// a clear-syntactic-form edge. Patterns that warrant `high` or `low`
/// will get dedicated constructors when they appear; charter §3
/// invariant D2 forbids "best-guess" tier downgrades inside the
/// extractor.
///
/// `pattern_id` is supplied by the caller. Convention:
/// `"<kind>.<pattern>"` where `<kind>` matches the `EdgeKind` slug
/// (e.g., `calls.method`, `imports.named`, `extends.class`,
/// `references_type.annotation`). The pattern_id is the durable
/// audit key for R8's confidence audit subcommand.
///
/// Returns a `RawEdge`; the resolver assigns `status` downstream (R3).
pub fn make_edge(
    from_id: impl Into<String>,
    to_id: impl Into<String>,
    kind: &str,
    pattern_id: &str,
    file_path: &str,
    line: u32,
) -> RawEdge {
    let kind_enum = crate::edge::EdgeKind::from_slug(kind)
        .unwrap_or_else(|| panic!("make_edge called with unknown edge kind: {kind}"));

    RawEdge::builder()
        .from(from_id)
        .to(to_id)
        .kind(kind_enum)
        .confidence(crate::edge::Confidence::Medium)
        .producer(crate::edge::Producer::Indexer)
        .pattern_id(pattern_id)
        .file_path(file_path)
        .line(line)
        .build()
}

/// Per-match edge extraction entry point.
///
/// Dispatches into the per-language extractor by `LanguageId`. Each
/// per-language `extract_*_edge` consumes a `&[Capture]` slice for one
/// tree-sitter query match (the `pattern_index` identifies which
/// pattern in `queries/<lang>/edges.scm` fired) plus the parser's
/// pre-resolved `enclosing_scope_id`.
///
pub fn extract_edges_for_match(
    lang: LanguageId,
    pattern_index: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    match lang {
        LanguageId::TypeScript => {
            typescript::extract_ts_edge(pattern_index, captures, file_path, enclosing_scope_id)
        }
        LanguageId::CSharp => {
            csharp::extract_cs_edge(pattern_index, captures, file_path, enclosing_scope_id)
        }
        LanguageId::Python => {
            python::extract_py_edge(pattern_index, captures, file_path, enclosing_scope_id)
        }
        LanguageId::Go => {
            go_lang::extract_go_edge(pattern_index, captures, file_path, enclosing_scope_id)
        }
        LanguageId::Java => {
            java::extract_java_edge(pattern_index, captures, file_path, enclosing_scope_id)
        }
        LanguageId::Rust => {
            rust_lang::extract_rust_edge(pattern_index, captures, file_path, enclosing_scope_id)
        }
        LanguageId::Ruby => {
            ruby::extract_ruby_edge(pattern_index, captures, file_path, enclosing_scope_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_captures_default_is_empty() {
        let captures = RawCaptures::default();
        assert!(captures.matches.is_empty());
        assert!(captures.metadata.is_empty());
        assert!(captures.skipped_ranges.is_empty());
    }

    #[test]
    fn capture_round_trips_through_json() {
        let capture = Capture {
            name: "function.name".to_string(),
            node_kind: "identifier".to_string(),
            text: "process_payment".to_string(),
            start_byte: 100,
            end_byte: 115,
            start_line: 12,
            end_line: 12,
            start_column: 4,
            end_column: 19,
        };
        let json = serde_json::to_string(&capture).expect("serialise");
        let back: Capture = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(capture, back);
    }

    #[test]
    fn metadata_field_carries_json_value() {
        let field = MetadataField {
            symbol_id: "src/users.py::User::class".to_string(),
            key: "decorators".to_string(),
            value: serde_json::json!([
                { "name": "dataclass", "args": [] },
                { "name": "frozen", "args": [] },
            ]),
        };
        assert_eq!(field.key, "decorators");
        let arr = field.value.as_array().expect("array");
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn skipped_range_carries_reason_convention() {
        let range = SkippedRange {
            start_line: 42,
            end_line: 84,
            reason: "plugin_skip:rust:unparseable_macro_body".to_string(),
        };
        assert!(range.reason.starts_with("plugin_skip:"));
    }

    #[test]
    fn extract_edges_returns_empty_for_empty_captures() {
        use crate::languages::dispatch::REGISTERED;
        let captures = RawCaptures::default();
        for &lang in REGISTERED {
            let edges = extract_edges(lang, "src/x", &captures);
            assert!(
                edges.is_empty(),
                "empty RawCaptures must yield no edges for {lang}; got {} edges",
                edges.len()
            );
        }
    }

    #[test]
    fn find_capture_returns_last_occurrence() {
        let caps = vec![
            Capture {
                name: "x".to_string(),
                node_kind: "identifier".to_string(),
                text: "first".to_string(),
                start_byte: 0,
                end_byte: 5,
                start_line: 1,
                end_line: 1,
                start_column: 0,
                end_column: 5,
            },
            Capture {
                name: "x".to_string(),
                node_kind: "identifier".to_string(),
                text: "second".to_string(),
                start_byte: 10,
                end_byte: 16,
                start_line: 1,
                end_line: 1,
                start_column: 10,
                end_column: 16,
            },
        ];
        let hit = find_capture(&caps, "x").expect("matches");
        assert_eq!(hit.text, "second", "HashMap-equivalent last-write-wins");
        assert!(find_capture(&caps, "missing").is_none());
    }

    #[test]
    fn raw_captures_round_trip_with_all_fields_populated() {
        let captures = RawCaptures {
            matches: vec![CapturedMatch {
                pattern_index: 5,
                enclosing_scope_id: Some("src/svc.ts::PaymentService::class".to_string()),
                captures: vec![Capture {
                    name: "class.parent".to_string(),
                    node_kind: "type_identifier".to_string(),
                    text: "BaseService".to_string(),
                    start_byte: 50,
                    end_byte: 61,
                    start_line: 5,
                    end_line: 5,
                    start_column: 20,
                    end_column: 31,
                }],
            }],
            metadata: vec![MetadataField {
                symbol_id: "src/svc.ts::PaymentService::class".to_string(),
                key: "annotations".to_string(),
                value: serde_json::json!([{ "name": "Component" }]),
            }],
            skipped_ranges: vec![SkippedRange {
                start_line: 100,
                end_line: 110,
                reason: "plugin_skip:typescript:unparseable_template_literal".to_string(),
            }],
        };
        let json = serde_json::to_string(&captures).expect("serialise");
        let back: RawCaptures = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(captures, back);
    }
}
