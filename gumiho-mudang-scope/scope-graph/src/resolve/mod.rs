//! R3 resolver — typestate `Captured` → `Resolution`.
//!
//! Sole resolver in `scope-graph`. The Phase A `resolver:phase-a` stub
//! was retired wholesale in sprint 0003 chunk 5; `Graph::resolve` /
//! `Graph::resolve_batch` route directly through this module's
//! `Resolver`.
//!
//! Sprint 0003 chunk 6 migrated `InsertableEdge` + the sealed
//! `Insertable` trait into this module. The constructor
//! [`InsertableEdge::new`] is module-private (no `pub`) and the
//! struct fields are private, so no caller outside
//! `scope_graph::resolve` can produce an `InsertableEdge`. The
//! compile-fail CI gate at
//! `scope-graph/tests/compile_fail/typestate/` proves this
//! mechanically. See `docs/ARCHITECTURAL-REFACTOR.md` § R3
//! ("Resolver location") and `docs/CI-GATES.md` § Insertable
//! typestate.
//!
//! ## Pipeline ordering
//!
//! ```text
//!   extractor                 resolver                  storage
//!  ──────────►  Captured  ──►  Resolution  ──►  Vec<InsertableEdge>  ──►  Graph
//!               (RawEdge)
//! ```
//!
//! - `Captured` wraps an extractor-output `RawEdge` and is the sole
//!   resolver input. `RawEdge` does **not** implement `Insertable`, so
//!   the storage layer's `Graph::insert_edges` signature refuses to
//!   accept a `RawEdge` directly. The typestate Captured → Resolution
//!   is what mechanically encodes R3's ordering.
//! - `Resolution` is the resolver's output: exactly one of
//!   `Resolved` / `Ambiguous` / `Dangling`. `Ambiguous` carries one
//!   `InsertableEdge` per matched candidate symbol (multi-row
//!   expansion per § R3 — multiplicity is representable via R0's
//!   surrogate `edge_id`).
//!
//! ## What this scaffold owns
//!
//! - Multi-row Ambiguous expansion: when the symbols-table lookup
//!   returns N > 1 ids, the resolver clones the `RawEdge` once per
//!   candidate, rebinding `to_id` via `RawEdge::with_to_id`. Each
//!   clone receives `Status::Ambiguous`.
//! - `Confidence` and `Producer` and `pattern_id` pass through
//!   verbatim — the resolver never downgrades a high-precision pattern
//!   on the basis of a lookup hit count (LANGUAGE-PLAYBOOK D2
//!   orthogonality).
//!
//! ## Workspace-aware resolution
//!
//! Consulting `LanguageWorkspaceContext` (workspace-internal-vs-external
//! import resolution, package-scoped name lookup) is **out of scope for
//! sprint 0003**: it requires `LanguageWorkspaceContext` to thread
//! through every indexer call site, which depends on R4's full
//! `WorkspaceContext` plumbing. The current resolver lookup is
//! `symbols.id = ?1 OR symbols.name = ?1`. The R3 mechanically
//! enforced contracts are the typestate + multi-row Ambiguous
//! expansion + the `Captured`→`Resolution` pipeline; lookup-quality
//! upgrades are queued post-refactor.

use anyhow::Result;
use rusqlite::Connection;
use scope_core::{Confidence, EdgeKind, Producer, RawEdge, Status};
use serde::{Deserialize, Serialize};

// ---------- Insertable typestate (R3, sprint 0003 chunk 6) ----------
//
// `InsertableEdge` and `Insertable` live here, not in `scope-core`,
// because the resolver is the sole legitimate construction site and
// Rust's module-level visibility is the mechanical safeguard
// (`ARCHITECTURAL-REFACTOR.md` § R3 — "Resolver location"). The
// constructor `InsertableEdge::new` is module-private (no `pub`) and
// the struct fields are private — only code inside this module can
// produce an `InsertableEdge`. The compile-fail CI gate proves both
// the `pub fn new` and the field-literal forms fail to compile from
// callers outside `scope_graph::resolve`.

/// Output of the resolver. Sole type accepted by the graph storage
/// layer via the sealed [`Insertable`] trait. Fields are private and
/// the constructor [`InsertableEdge::new`] is module-private to
/// `scope_graph::resolve`; the compile-fail CI gate enforces both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertableEdge {
    raw: RawEdge,
    status: Status,
}

impl InsertableEdge {
    /// Resolver-only constructor — module-private (no `pub`). Reachable
    /// solely from code inside `scope_graph::resolve`. Construction
    /// outside this module is a compile error, enforced by
    /// `scope-graph/tests/compile_fail/typestate/`.
    fn new(raw: RawEdge, status: Status) -> Self {
        Self { raw, status }
    }

    pub fn raw(&self) -> &RawEdge {
        &self.raw
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn from_id(&self) -> &str {
        self.raw.from_id()
    }
    pub fn to_id(&self) -> &str {
        self.raw.to_id()
    }
    pub fn kind(&self) -> EdgeKind {
        self.raw.kind()
    }
    pub fn confidence(&self) -> Confidence {
        self.raw.confidence()
    }
    pub fn producer(&self) -> &Producer {
        self.raw.producer()
    }
    pub fn pattern_id(&self) -> &str {
        self.raw.pattern_id()
    }
    pub fn capture_id(&self) -> Option<&str> {
        self.raw.capture_id()
    }
    pub fn framework(&self) -> Option<&str> {
        self.raw.framework()
    }
    pub fn args_text(&self) -> Option<&str> {
        self.raw.args_text()
    }
    pub fn file_path(&self) -> &str {
        self.raw.file_path()
    }
    pub fn line(&self) -> Option<u32> {
        self.raw.line()
    }
}

/// Sealed marker trait: the graph storage layer accepts only types
/// implementing `Insertable`. Only [`InsertableEdge`] implements it.
/// `RawEdge` deliberately does not — the type system forbids
/// inserting an edge that has not been through the resolver.
pub trait Insertable: insertable_sealed::Sealed {}

mod insertable_sealed {
    pub trait Sealed {}
}

impl insertable_sealed::Sealed for InsertableEdge {}
impl Insertable for InsertableEdge {}

/// Sole resolver input. Wraps a `RawEdge` to make the typestate
/// transition explicit at call sites: `Captured::new(raw)` → resolver
/// → `Resolution`. The wrapper is zero-cost (it owns the `RawEdge`)
/// and intentionally minimal — Captured carries no resolver state and
/// has no methods beyond construct / unwrap.
#[derive(Debug, Clone)]
pub struct Captured {
    raw: RawEdge,
}

impl Captured {
    /// Wrap an extractor-output `RawEdge` for resolution. The
    /// extractor is the only legitimate construction site.
    pub fn new(raw: RawEdge) -> Self {
        Self { raw }
    }

    /// Borrow the underlying `RawEdge` (read-only). Useful for
    /// resolver internals that need to inspect kind / confidence /
    /// pattern before deciding on a lookup strategy.
    pub fn raw(&self) -> &RawEdge {
        &self.raw
    }

    /// Consume the wrapper and return the inner `RawEdge`. Called by
    /// the resolver when assembling `InsertableEdge`s.
    pub fn into_raw(self) -> RawEdge {
        self.raw
    }
}

/// Sole resolver output. Exactly one variant is constructed per
/// `Captured`; downstream call sites flatten via
/// [`Resolution::into_insertable`].
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Exactly one candidate symbol matched. Carries the single
    /// `InsertableEdge` bound to that candidate's id, with
    /// `Status::Resolved`.
    Resolved(InsertableEdge),
    /// More than one candidate matched. Carries one `InsertableEdge`
    /// per candidate, each pointing at a distinct candidate id, all
    /// stamped `Status::Ambiguous`. Multiplicity is representable
    /// because R0 gave `edges` a surrogate `edge_id` PK.
    Ambiguous(Vec<InsertableEdge>),
    /// No candidate matched. Carries the single `InsertableEdge`
    /// pointing at the original (unresolved) `to_id`, stamped
    /// `Status::Dangling`. The unresolved target text is preserved so
    /// downstream queries can surface the dangling reference verbatim.
    Dangling(InsertableEdge),
}

impl Resolution {
    /// Flatten a `Resolution` into the vector form consumed by the
    /// storage layer. `Resolved` and `Dangling` collapse to a single-
    /// element vector; `Ambiguous` passes through unchanged.
    pub fn into_insertable(self) -> Vec<InsertableEdge> {
        match self {
            Self::Resolved(e) | Self::Dangling(e) => vec![e],
            Self::Ambiguous(v) => v,
        }
    }

    /// Number of rows that will reach storage. Useful for assertions
    /// and metrics.
    pub fn row_count(&self) -> usize {
        match self {
            Self::Resolved(_) | Self::Dangling(_) => 1,
            Self::Ambiguous(v) => v.len(),
        }
    }
}

/// R3 resolver. Holds a borrow of the connection used for symbol-table
/// lookups; constructed per resolve pass.
pub struct Resolver<'a> {
    conn: &'a Connection,
}

impl<'a> Resolver<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Resolve a single `Captured` edge.
    ///
    /// Lookup contract: a row matches when `symbols.id = ?1` (full
    /// synthetic id) or `symbols.name = ?1` (bare-name shorthand from
    /// extractors).
    ///
    /// `to_id` rebinding policy:
    /// - **Resolved (exactly one match)**: `to_id` is preserved
    ///   verbatim. Status is the new signal; downstream queries that
    ///   need to follow the candidate's full id JOIN against `symbols`
    ///   on `id = to_id OR name = to_id`. Pre-R3 query patterns thus
    ///   keep working — the only behavioural change is `status` being
    ///   set instead of implicit.
    /// - **Ambiguous (N > 1 matches)**: one `InsertableEdge` per
    ///   candidate, each with `to_id` set to the candidate's full
    ///   symbol id (`ARCHITECTURAL-REFACTOR.md § R3` line "to_id set
    ///   to the candidate"). Multi-row spread is the only place
    ///   rebinding is structurally required, because each row needs a
    ///   distinct target identity.
    /// - **Dangling (zero matches)**: `to_id` preserved verbatim.
    pub fn resolve(&self, captured: Captured) -> Result<Resolution> {
        let raw = captured.into_raw();
        let lookup_key = raw.to_id().to_string();

        let mut stmt = self
            .conn
            .prepare("SELECT id FROM symbols WHERE id = ?1 OR name = ?1")?;
        let ids: Vec<String> = stmt
            .query_map(rusqlite::params![&lookup_key], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(match ids.len() {
            0 => Resolution::Dangling(InsertableEdge::new(raw, Status::Dangling)),
            1 => Resolution::Resolved(InsertableEdge::new(raw, Status::Resolved)),
            _ => {
                let candidates = ids
                    .into_iter()
                    .map(|id| InsertableEdge::new(raw.clone().with_to_id(id), Status::Ambiguous))
                    .collect();
                Resolution::Ambiguous(candidates)
            }
        })
    }

    /// Batch helper. Flattens every `Resolution` into the storage-layer
    /// vector. Caller must accept that `raws.len() != out.len()`
    /// when any input resolves Ambiguous.
    pub fn resolve_batch(&self, raws: Vec<RawEdge>) -> Result<Vec<InsertableEdge>> {
        let mut out = Vec::with_capacity(raws.len());
        for raw in raws {
            out.extend(self.resolve(Captured::new(raw))?.into_insertable());
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scope_core::{Confidence, EdgeKind, Producer};

    fn open_in_memory() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE symbols (
                id   TEXT PRIMARY KEY,
                name TEXT NOT NULL
            );",
        )
        .expect("symbols schema");
        conn
    }

    fn insert_symbol(conn: &Connection, id: &str, name: &str) {
        conn.execute(
            "INSERT INTO symbols (id, name) VALUES (?1, ?2)",
            rusqlite::params![id, name],
        )
        .expect("insert symbol");
    }

    fn raw_edge(to: &str) -> RawEdge {
        RawEdge::builder()
            .from("src/caller.rs::call_site::function::1")
            .to(to)
            .kind(EdgeKind::Calls)
            .confidence(Confidence::High)
            .producer(Producer::Lang("rust_lang".into()))
            .pattern_id("calls.method")
            .file_path("src/caller.rs")
            .line(1)
            .build()
    }

    #[test]
    fn zero_matches_yields_dangling_with_original_to_id() {
        let conn = open_in_memory();
        let resolver = Resolver::new(&conn);

        let raw = raw_edge("nonexistent_target");
        let resolution = resolver
            .resolve(Captured::new(raw))
            .expect("resolve should succeed");

        match resolution {
            Resolution::Dangling(edge) => {
                assert_eq!(edge.to_id(), "nonexistent_target");
                assert_eq!(edge.status(), Status::Dangling);
            }
            other => panic!("expected Dangling, got {other:?}"),
        }
    }

    #[test]
    fn exactly_one_match_yields_resolved_preserving_to_id() {
        let conn = open_in_memory();
        insert_symbol(
            &conn,
            "src/payment.rs::process_payment::function::10",
            "process_payment",
        );
        let resolver = Resolver::new(&conn);

        let raw = raw_edge("process_payment");
        let resolution = resolver
            .resolve(Captured::new(raw))
            .expect("resolve should succeed");

        match resolution {
            Resolution::Resolved(edge) => {
                assert_eq!(
                    edge.to_id(),
                    "process_payment",
                    "to_id preserved verbatim on single-match Resolved — status is the new signal, downstream JOINs follow id-or-name"
                );
                assert_eq!(edge.status(), Status::Resolved);
            }
            other => panic!("expected Resolved, got {other:?}"),
        }
    }

    #[test]
    fn multiple_matches_yield_ambiguous_one_row_per_candidate() {
        let conn = open_in_memory();
        insert_symbol(&conn, "src/a.rs::handler::function::1", "handler");
        insert_symbol(&conn, "src/b.rs::handler::function::2", "handler");
        insert_symbol(&conn, "src/c.rs::handler::function::3", "handler");
        let resolver = Resolver::new(&conn);

        let raw = raw_edge("handler");
        let resolution = resolver
            .resolve(Captured::new(raw))
            .expect("resolve should succeed");

        let edges = match resolution {
            Resolution::Ambiguous(v) => v,
            other => panic!("expected Ambiguous, got {other:?}"),
        };

        assert_eq!(edges.len(), 3, "one row per candidate");
        for edge in &edges {
            assert_eq!(edge.status(), Status::Ambiguous);
            assert_eq!(
                edge.confidence(),
                Confidence::High,
                "D2: resolver must not downgrade confidence on ambiguity"
            );
            assert_eq!(
                edge.pattern_id(),
                "calls.method",
                "pattern_id passes through verbatim"
            );
        }

        let mut targets: Vec<&str> = edges.iter().map(|e| e.to_id()).collect();
        targets.sort();
        assert_eq!(
            targets,
            vec![
                "src/a.rs::handler::function::1",
                "src/b.rs::handler::function::2",
                "src/c.rs::handler::function::3",
            ]
        );
    }

    #[test]
    fn id_match_takes_precedence_over_name_match_when_both_exist() {
        let conn = open_in_memory();
        insert_symbol(&conn, "src/x.rs::frobnicate::function::1", "frobnicate");
        let resolver = Resolver::new(&conn);

        let raw = raw_edge("src/x.rs::frobnicate::function::1");
        let resolution = resolver
            .resolve(Captured::new(raw))
            .expect("resolve should succeed");

        match resolution {
            Resolution::Resolved(edge) => {
                assert_eq!(edge.to_id(), "src/x.rs::frobnicate::function::1");
                assert_eq!(edge.status(), Status::Resolved);
            }
            other => panic!("expected Resolved on full-id lookup, got {other:?}"),
        }
    }

    #[test]
    fn resolution_into_insertable_flattens_correctly() {
        let conn = open_in_memory();
        insert_symbol(&conn, "src/a.rs::h::function::1", "h");
        insert_symbol(&conn, "src/b.rs::h::function::2", "h");
        let resolver = Resolver::new(&conn);

        let resolved = resolver
            .resolve(Captured::new(raw_edge("src/a.rs::h::function::1")))
            .unwrap();
        assert_eq!(resolved.row_count(), 1);
        assert_eq!(resolved.into_insertable().len(), 1);

        let ambiguous = resolver.resolve(Captured::new(raw_edge("h"))).unwrap();
        assert_eq!(ambiguous.row_count(), 2);
        assert_eq!(ambiguous.into_insertable().len(), 2);

        let dangling = resolver.resolve(Captured::new(raw_edge("zzz"))).unwrap();
        assert_eq!(dangling.row_count(), 1);
        let v = dangling.into_insertable();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].status(), Status::Dangling);
    }

    #[test]
    fn resolve_batch_preserves_total_multiplicity() {
        let conn = open_in_memory();
        insert_symbol(&conn, "src/a.rs::h::function::1", "h");
        insert_symbol(&conn, "src/b.rs::h::function::2", "h");
        insert_symbol(&conn, "src/c.rs::only::function::3", "only");
        let resolver = Resolver::new(&conn);

        let inputs = vec![
            raw_edge("h"),    // Ambiguous → 2 rows
            raw_edge("only"), // Resolved  → 1 row
            raw_edge("none"), // Dangling  → 1 row
        ];
        let out = resolver.resolve_batch(inputs).expect("batch ok");

        assert_eq!(out.len(), 4, "2 + 1 + 1 = 4 rows reach storage");
    }
}
