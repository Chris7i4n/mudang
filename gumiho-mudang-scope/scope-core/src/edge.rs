//! R1 typed-edge insertion API.
//!
//! Plugins construct a `RawEdge` via `Edge::builder()` (six required
//! setters, enforced by typestate at compile time). A resolver in
//! `scope-graph::resolve` converts the `RawEdge` to an
//! `InsertableEdge` by assigning `status`; the graph storage layer
//! accepts only `InsertableEdge` via the sealed `Insertable` trait.
//!
//! `InsertableEdge` and `Insertable` deliberately live in
//! `scope-graph::resolve`, not here — sprint 0003 chunk 6 moved them
//! out of `scope-core` so the resolver-only construction site is
//! module-private to `scope_graph::resolve`. See
//! `docs/ENFORCEMENT-MAP.md` § R3 ("Resolver location") and
//! `docs/CI-GATES.md` § Insertable typestate.
//!
//! See `docs/ENFORCEMENT-MAP.md` § R1 (typed edge insertion API)
//! for the contract and § R0 (schema closures) for the column layout.

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

const ARGS_TEXT_CAP_BYTES: usize = 2048;
const TRUNCATION_MARKER: &str = "[truncated]";

/// Strongly-typed edge kind. The whitelist is the R0 final set: 38
/// entries (8 universal + 30 domain). The wire/SQL representation is
/// the snake-case slug returned by [`EdgeKind::as_slug`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    // Universal (8)
    Calls,
    Imports,
    Extends,
    Implements,
    Instantiates,
    References,
    ReferencesType,
    Contains,
    // R0 baseline domain (13)
    HttpRoute,
    QueueHandler,
    OrmRelation,
    GreenThreadSpawn,
    Renders,
    HookUse,
    InheritsFrom,
    Migration,
    Cron,
    FeatureFlag,
    AwaitsOn,
    ChannelSend,
    ChannelRecv,
    // Tier 1 (5)
    Middleware,
    ValidatesWith,
    ErrorHandler,
    WebsocketHandler,
    ClientRoute,
    // Tier 2 (5)
    AuthGuard,
    CacheBinding,
    RuntimeTaskSpawn,
    RouteMount,
    StoreSelect,
    // Tier 3 (7)
    SseStream,
    SignalHandler,
    CancelToken,
    LazyLoad,
    QueryBinding,
    OsProcessSpawn,
    OsThreadSpawn,
}

impl EdgeKind {
    pub fn as_slug(self) -> &'static str {
        match self {
            EdgeKind::Calls => "calls",
            EdgeKind::Imports => "imports",
            EdgeKind::Extends => "extends",
            EdgeKind::Implements => "implements",
            EdgeKind::Instantiates => "instantiates",
            EdgeKind::References => "references",
            EdgeKind::ReferencesType => "references_type",
            EdgeKind::Contains => "contains",
            EdgeKind::HttpRoute => "http_route",
            EdgeKind::QueueHandler => "queue_handler",
            EdgeKind::OrmRelation => "orm_relation",
            EdgeKind::GreenThreadSpawn => "green_thread_spawn",
            EdgeKind::Renders => "renders",
            EdgeKind::HookUse => "hook_use",
            EdgeKind::InheritsFrom => "inherits_from",
            EdgeKind::Migration => "migration",
            EdgeKind::Cron => "cron",
            EdgeKind::FeatureFlag => "feature_flag",
            EdgeKind::AwaitsOn => "awaits_on",
            EdgeKind::ChannelSend => "channel_send",
            EdgeKind::ChannelRecv => "channel_recv",
            EdgeKind::Middleware => "middleware",
            EdgeKind::ValidatesWith => "validates_with",
            EdgeKind::ErrorHandler => "error_handler",
            EdgeKind::WebsocketHandler => "websocket_handler",
            EdgeKind::ClientRoute => "client_route",
            EdgeKind::AuthGuard => "auth_guard",
            EdgeKind::CacheBinding => "cache_binding",
            EdgeKind::RuntimeTaskSpawn => "runtime_task_spawn",
            EdgeKind::RouteMount => "route_mount",
            EdgeKind::StoreSelect => "store_select",
            EdgeKind::SseStream => "sse_stream",
            EdgeKind::SignalHandler => "signal_handler",
            EdgeKind::CancelToken => "cancel_token",
            EdgeKind::LazyLoad => "lazy_load",
            EdgeKind::QueryBinding => "query_binding",
            EdgeKind::OsProcessSpawn => "os_process_spawn",
            EdgeKind::OsThreadSpawn => "os_thread_spawn",
        }
    }

    pub fn from_slug(s: &str) -> Option<EdgeKind> {
        Some(match s {
            "calls" => EdgeKind::Calls,
            "imports" => EdgeKind::Imports,
            "extends" => EdgeKind::Extends,
            "implements" => EdgeKind::Implements,
            "instantiates" => EdgeKind::Instantiates,
            "references" => EdgeKind::References,
            "references_type" => EdgeKind::ReferencesType,
            "contains" => EdgeKind::Contains,
            "http_route" => EdgeKind::HttpRoute,
            "queue_handler" => EdgeKind::QueueHandler,
            "orm_relation" => EdgeKind::OrmRelation,
            "green_thread_spawn" => EdgeKind::GreenThreadSpawn,
            "renders" => EdgeKind::Renders,
            "hook_use" => EdgeKind::HookUse,
            "inherits_from" => EdgeKind::InheritsFrom,
            "migration" => EdgeKind::Migration,
            "cron" => EdgeKind::Cron,
            "feature_flag" => EdgeKind::FeatureFlag,
            "awaits_on" => EdgeKind::AwaitsOn,
            "channel_send" => EdgeKind::ChannelSend,
            "channel_recv" => EdgeKind::ChannelRecv,
            "middleware" => EdgeKind::Middleware,
            "validates_with" => EdgeKind::ValidatesWith,
            "error_handler" => EdgeKind::ErrorHandler,
            "websocket_handler" => EdgeKind::WebsocketHandler,
            "client_route" => EdgeKind::ClientRoute,
            "auth_guard" => EdgeKind::AuthGuard,
            "cache_binding" => EdgeKind::CacheBinding,
            "runtime_task_spawn" => EdgeKind::RuntimeTaskSpawn,
            "route_mount" => EdgeKind::RouteMount,
            "store_select" => EdgeKind::StoreSelect,
            "sse_stream" => EdgeKind::SseStream,
            "signal_handler" => EdgeKind::SignalHandler,
            "cancel_token" => EdgeKind::CancelToken,
            "lazy_load" => EdgeKind::LazyLoad,
            "query_binding" => EdgeKind::QueryBinding,
            "os_process_spawn" => EdgeKind::OsProcessSpawn,
            "os_thread_spawn" => EdgeKind::OsThreadSpawn,
            _ => return None,
        })
    }
}

/// Pattern-precision tier assigned by the extractor. Independent of
/// resolver outcome (see `LANGUAGE-PLAYBOOK.md` D2 orthogonality).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_slug(self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

/// Lookup-outcome tag assigned by the resolver. Construction-only via
/// resolver code paths. R3 ships the real resolver; Phase A ships a
/// trivial stub.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Resolved,
    Ambiguous,
    Dangling,
}

impl Status {
    pub fn as_slug(self) -> &'static str {
        match self {
            Status::Resolved => "resolved",
            Status::Ambiguous => "ambiguous",
            Status::Dangling => "dangling",
        }
    }
}

/// Identifier of the producing plugin or layer. Stored as a slug
/// string in `edges.producer`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "name")]
pub enum Producer {
    /// Language plugin (`"rust_lang"`, `"python"`, …).
    Lang(String),
    /// Framework plugin (`"flask"`, `"rails"`, …).
    Framework(String),
    /// Resolution layer (R3); not used at extraction.
    Resolution,
    /// Indexer-level synthesis (e.g. `contains` edges from file → module).
    Indexer,
}

impl Producer {
    pub fn as_slug(&self) -> String {
        match self {
            Producer::Lang(name) => name.clone(),
            Producer::Framework(name) => format!("framework:{name}"),
            Producer::Resolution => "resolution".to_string(),
            Producer::Indexer => "indexer".to_string(),
        }
    }
}

/// Output of the extractor. Carries the pattern-precision contract
/// (`confidence`, `producer`, `pattern_id`) but **no** `status` —
/// status is assigned exclusively by the resolver. Fields are
/// `pub(crate)`; construction goes through [`EdgeBuilder`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEdge {
    pub(crate) from_id: String,
    pub(crate) to_id: String,
    pub(crate) kind: EdgeKind,
    pub(crate) confidence: Confidence,
    pub(crate) producer: Producer,
    pub(crate) pattern_id: String,
    pub(crate) capture_id: Option<String>,
    pub(crate) framework: Option<String>,
    pub(crate) args_text: Option<String>,
    pub(crate) file_path: String,
    pub(crate) line: Option<u32>,
}

impl RawEdge {
    /// Entry point to the typed edge insertion API. Six required
    /// setters (`from`, `to`, `kind`, `confidence`, `producer`,
    /// `pattern_id`) must be called before `.build()`. The builder
    /// has no `.status(...)` method (R1 acceptance: status is the
    /// resolver's output, never the extractor's).
    pub fn builder() -> EdgeBuilder<No, No, No, No, No, No> {
        EdgeBuilder::new()
    }

    pub fn from_id(&self) -> &str {
        &self.from_id
    }
    pub fn to_id(&self) -> &str {
        &self.to_id
    }
    pub fn kind(&self) -> EdgeKind {
        self.kind
    }
    pub fn confidence(&self) -> Confidence {
        self.confidence
    }
    pub fn producer(&self) -> &Producer {
        &self.producer
    }
    pub fn pattern_id(&self) -> &str {
        &self.pattern_id
    }
    pub fn capture_id(&self) -> Option<&str> {
        self.capture_id.as_deref()
    }
    pub fn framework(&self) -> Option<&str> {
        self.framework.as_deref()
    }
    pub fn args_text(&self) -> Option<&str> {
        self.args_text.as_deref()
    }
    pub fn file_path(&self) -> &str {
        &self.file_path
    }
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Rebind `to_id` to a concrete candidate symbol id. Used by the
    /// resolver when an extractor-emitted bare name matches multiple
    /// rows in the symbols table: each Ambiguous candidate gets its own
    /// `InsertableEdge`, all sharing the original
    /// `(from_id, kind, confidence, producer, pattern_id)` but pointing
    /// at distinct candidate `to_id`s. Not for use outside resolver
    /// code paths (R3, sprint 0003).
    pub fn with_to_id(mut self, to_id: impl Into<String>) -> Self {
        self.to_id = to_id.into();
        self
    }
}

// `InsertableEdge` and `Insertable` live in `scope-graph::resolve`
// (R3, sprint 0003 chunk 6 migration). Constructor is module-private
// to the resolver — the compile-fail CI gate in
// `scope-graph/tests/compile_fail/typestate/` enforces it
// mechanically.

// ---------- Typestate builder ----------

/// Typestate marker — required field not yet set.
pub struct No;
/// Typestate marker — required field set.
pub struct Yes;

/// Edge construction site. The six required setters (`from`, `to`,
/// `kind`, `confidence`, `producer`, `pattern_id`) flip a phantom
/// marker; `.build()` is callable only when every marker is `Yes`.
/// `.status(...)` does **not** exist — status is the resolver's
/// output, never the extractor's (R1 acceptance bullet).
pub struct EdgeBuilder<F = No, T = No, K = No, C = No, P = No, I = No> {
    from_id: Option<String>,
    to_id: Option<String>,
    kind: Option<EdgeKind>,
    confidence: Option<Confidence>,
    producer: Option<Producer>,
    pattern_id: Option<String>,
    capture_id: Option<String>,
    framework: Option<String>,
    args_text: Option<String>,
    file_path: Option<String>,
    line: Option<u32>,
    _marker: PhantomData<(F, T, K, C, P, I)>,
}

impl Default for EdgeBuilder<No, No, No, No, No, No> {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeBuilder<No, No, No, No, No, No> {
    pub fn new() -> Self {
        Self {
            from_id: None,
            to_id: None,
            kind: None,
            confidence: None,
            producer: None,
            pattern_id: None,
            capture_id: None,
            framework: None,
            args_text: None,
            file_path: None,
            line: None,
            _marker: PhantomData,
        }
    }
}

// Generic state-rewrite helper to keep setter impls compact.
fn rewrap<F, T, K, C, P, I, F2, T2, K2, C2, P2, I2>(
    b: EdgeBuilder<F, T, K, C, P, I>,
) -> EdgeBuilder<F2, T2, K2, C2, P2, I2> {
    EdgeBuilder {
        from_id: b.from_id,
        to_id: b.to_id,
        kind: b.kind,
        confidence: b.confidence,
        producer: b.producer,
        pattern_id: b.pattern_id,
        capture_id: b.capture_id,
        framework: b.framework,
        args_text: b.args_text,
        file_path: b.file_path,
        line: b.line,
        _marker: PhantomData,
    }
}

impl<T, K, C, P, I> EdgeBuilder<No, T, K, C, P, I> {
    pub fn from(mut self, id: impl Into<String>) -> EdgeBuilder<Yes, T, K, C, P, I> {
        self.from_id = Some(id.into());
        rewrap(self)
    }
}

impl<F, K, C, P, I> EdgeBuilder<F, No, K, C, P, I> {
    pub fn to(mut self, id: impl Into<String>) -> EdgeBuilder<F, Yes, K, C, P, I> {
        self.to_id = Some(id.into());
        rewrap(self)
    }
}

impl<F, T, C, P, I> EdgeBuilder<F, T, No, C, P, I> {
    pub fn kind(mut self, k: EdgeKind) -> EdgeBuilder<F, T, Yes, C, P, I> {
        self.kind = Some(k);
        rewrap(self)
    }
}

impl<F, T, K, P, I> EdgeBuilder<F, T, K, No, P, I> {
    pub fn confidence(mut self, c: Confidence) -> EdgeBuilder<F, T, K, Yes, P, I> {
        self.confidence = Some(c);
        rewrap(self)
    }
}

impl<F, T, K, C, I> EdgeBuilder<F, T, K, C, No, I> {
    pub fn producer(mut self, p: Producer) -> EdgeBuilder<F, T, K, C, Yes, I> {
        self.producer = Some(p);
        rewrap(self)
    }
}

impl<F, T, K, C, P> EdgeBuilder<F, T, K, C, P, No> {
    pub fn pattern_id(mut self, id: impl Into<String>) -> EdgeBuilder<F, T, K, C, P, Yes> {
        self.pattern_id = Some(id.into());
        rewrap(self)
    }
}

// Optional setters available in any state.
impl<F, T, K, C, P, I> EdgeBuilder<F, T, K, C, P, I> {
    pub fn capture_id(mut self, id: impl Into<String>) -> Self {
        self.capture_id = Some(id.into());
        self
    }
    pub fn framework(mut self, name: impl Into<String>) -> Self {
        self.framework = Some(name.into());
        self
    }
    /// Truncates the literal at 2 KB and appends `[truncated]` if the
    /// input exceeds the cap. Mitigation 2 per
    /// `ENFORCEMENT-MAP.md` § R0 → edges.args_text.
    pub fn args_text(mut self, text: impl Into<String>) -> Self {
        let raw = text.into();
        let stored = if raw.len() > ARGS_TEXT_CAP_BYTES {
            let mut cut = ARGS_TEXT_CAP_BYTES;
            while !raw.is_char_boundary(cut) && cut > 0 {
                cut -= 1;
            }
            let mut s = String::with_capacity(cut + TRUNCATION_MARKER.len());
            s.push_str(&raw[..cut]);
            s.push_str(TRUNCATION_MARKER);
            s
        } else {
            raw
        };
        self.args_text = Some(stored);
        self
    }
    pub fn file_path(mut self, p: impl Into<String>) -> Self {
        self.file_path = Some(p.into());
        self
    }
    pub fn line(mut self, l: u32) -> Self {
        self.line = Some(l);
        self
    }
}

// `.build()` is callable only when every required marker is `Yes`.
impl EdgeBuilder<Yes, Yes, Yes, Yes, Yes, Yes> {
    pub fn build(self) -> RawEdge {
        RawEdge {
            from_id: self.from_id.expect("from set by typestate"),
            to_id: self.to_id.expect("to set by typestate"),
            kind: self.kind.expect("kind set by typestate"),
            confidence: self.confidence.expect("confidence set by typestate"),
            producer: self.producer.expect("producer set by typestate"),
            pattern_id: self.pattern_id.expect("pattern_id set by typestate"),
            capture_id: self.capture_id,
            framework: self.framework,
            args_text: self.args_text,
            file_path: self.file_path.unwrap_or_default(),
            line: self.line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_with_all_required_fields_succeeds() {
        let raw = EdgeBuilder::new()
            .from("a")
            .to("b")
            .kind(EdgeKind::Calls)
            .confidence(Confidence::High)
            .producer(Producer::Lang("rust_lang".into()))
            .pattern_id("calls.method")
            .file_path("src/lib.rs")
            .line(10)
            .build();

        assert_eq!(raw.from_id(), "a");
        assert_eq!(raw.to_id(), "b");
        assert_eq!(raw.kind(), EdgeKind::Calls);
        assert_eq!(raw.confidence(), Confidence::High);
    }

    #[test]
    fn args_text_under_cap_kept_verbatim() {
        let raw = EdgeBuilder::new()
            .from("a")
            .to("b")
            .kind(EdgeKind::HttpRoute)
            .confidence(Confidence::High)
            .producer(Producer::Framework("flask".into()))
            .pattern_id("http_route.decorator")
            .args_text("/users/<id>")
            .build();
        assert_eq!(raw.args_text(), Some("/users/<id>"));
    }

    #[test]
    fn args_text_over_cap_truncates_with_marker() {
        let big = "x".repeat(3000);
        let raw = EdgeBuilder::new()
            .from("a")
            .to("b")
            .kind(EdgeKind::HttpRoute)
            .confidence(Confidence::High)
            .producer(Producer::Framework("flask".into()))
            .pattern_id("http_route.decorator")
            .args_text(big)
            .build();
        let stored = raw.args_text().unwrap();
        assert!(stored.ends_with(TRUNCATION_MARKER));
        assert_eq!(stored.len(), ARGS_TEXT_CAP_BYTES + TRUNCATION_MARKER.len());
    }

    #[test]
    fn slug_roundtrip_all_38_edge_kinds() {
        for slug in [
            "calls",
            "imports",
            "extends",
            "implements",
            "instantiates",
            "references",
            "references_type",
            "contains",
            "http_route",
            "queue_handler",
            "orm_relation",
            "green_thread_spawn",
            "renders",
            "hook_use",
            "inherits_from",
            "migration",
            "cron",
            "feature_flag",
            "awaits_on",
            "channel_send",
            "channel_recv",
            "middleware",
            "validates_with",
            "error_handler",
            "websocket_handler",
            "client_route",
            "auth_guard",
            "cache_binding",
            "runtime_task_spawn",
            "route_mount",
            "store_select",
            "sse_stream",
            "signal_handler",
            "cancel_token",
            "lazy_load",
            "query_binding",
            "os_process_spawn",
            "os_thread_spawn",
        ] {
            let kind = EdgeKind::from_slug(slug).unwrap_or_else(|| panic!("unknown slug {slug}"));
            assert_eq!(kind.as_slug(), slug);
        }
    }
}
