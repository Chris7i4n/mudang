//! SQLite-backed dependency graph storage.
//!
//! Stores symbols, edges, and file hashes. Provides query methods
//! for refs, deps, rdeps, and impact analysis.
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
// Re-export so pre-split callers reaching for `scope_graph::graph::Symbol`
// (and via the façade, `gumiho_mudang_scope::core::graph::Symbol`) keep
// resolving. The structs themselves live in scope-core; this preserves
// the 1:1 public-surface promise in TODO 0006 § Sprint 0000 ambiguity
// resolutions § 2.
pub use crate::resolve::InsertableEdge;
use scope_core::extract::SkippedRange;
pub use scope_core::{Edge, RawEdge, Symbol};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// One row destined for the `file_hashes` table (R6 — sprint 0003 chunk 3a).
///
/// Wraps the historical `(file_path, hash)` pair with `skipped_ranges` so
/// the indexer can record both tree-sitter parser-recovery skips (R6) and
/// plugin-driven skips (R2) in a single transactional write. The
/// `file_hashes.skipped_ranges` column defaults to `'[]'`; passing an
/// empty `Vec` here yields the same JSON literal.
#[derive(Debug, Clone, Default)]
pub struct FileHashRow {
    /// SHA-256 hex digest of file contents — the staleness key.
    pub hash: String,
    /// Concatenation of plugin-driven skips and tree-sitter-error skips
    /// for this file. Per Charter §3 invariant 5, the indexer must
    /// forward both in source order; the storage layer serialises the
    /// slice verbatim.
    pub skipped_ranges: Vec<SkippedRange>,
}

/// The dependency graph backed by SQLite.
pub struct Graph {
    conn: Connection,
}

/// Result of comparing current file hashes against the stored index.
#[derive(Debug, Default)]
pub struct ChangedFiles {
    /// Files that are new (not previously indexed).
    pub added: Vec<String>,
    /// Files whose content hash has changed.
    pub modified: Vec<String>,
    /// Files that were previously indexed but no longer exist.
    pub deleted: Vec<String>,
}

impl ChangedFiles {
    /// Returns true if there are no changes.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }
}

/// Relationships of a class symbol: inheritance, interfaces, and dependencies.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClassRelationships {
    /// Classes this class extends.
    pub extends: Vec<String>,
    /// Interfaces this class implements.
    pub implements: Vec<String>,
    /// Distinct symbol names from outgoing edges (imports, calls, etc.).
    pub dependencies: Vec<String>,
}

/// Information about a caller of a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerInfo {
    /// Display name of the caller (e.g. `OrderController.checkout`).
    pub name: String,
    /// Number of call sites from this caller.
    pub count: usize,
}

/// A reference to a symbol from elsewhere in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// The ID of the symbol making the reference.
    pub from_id: String,
    /// The human-readable name of the referencing symbol.
    pub from_name: String,
    /// The kind of reference (calls, imports, extends, etc.).
    pub kind: String,
    /// File path where the reference occurs.
    pub file_path: String,
    /// Line number of the reference, if known.
    pub line: Option<i64>,
    /// Context string (caller name or description).
    pub context: String,
    /// The actual source line at the reference location (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_line: Option<String>,
    /// Multi-line context around the reference (if --context N was used).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<Vec<String>>,
}

/// A node in an impact analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactNode {
    /// Symbol ID.
    pub id: String,
    /// Symbol name.
    pub name: String,
    /// File path where this symbol is defined.
    pub file_path: String,
    /// Symbol kind (function, class, method, etc.).
    pub kind: String,
    /// Depth in the impact graph (1 = direct caller).
    pub depth: usize,
}

/// Result of an impact analysis query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResult {
    /// Nodes grouped by depth level: `(depth, nodes_at_that_depth)`.
    pub nodes_by_depth: Vec<(usize, Vec<ImpactNode>)>,
    /// Test files that are affected (separated from main results).
    pub test_files: Vec<ImpactNode>,
    /// Total number of distinct affected symbols (excluding test files).
    pub total_affected: usize,
}

/// A dependency of a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// The name of the dependency.
    pub name: String,
    /// File path of the dependency, if it exists in the index.
    pub file_path: Option<String>,
    /// Kind of dependency relationship (imports, calls, extends, etc.).
    pub kind: String,
    /// True if the dependency is not in the index (external package).
    pub is_external: bool,
    /// Depth in the dependency tree (1 = direct).
    pub depth: usize,
}

/// A single step in a call path from entry point to target.
#[derive(Debug, Clone, Serialize)]
pub struct CallPathStep {
    /// Display name of the symbol at this step.
    pub symbol_name: String,
    /// Full symbol ID.
    pub symbol_id: String,
    /// File path where this symbol is defined.
    pub file_path: String,
    /// Line number of the symbol definition.
    pub line: u32,
    /// Symbol kind (function, class, method, etc.).
    pub kind: String,
}

/// A complete call path from an entry point to the target symbol.
#[derive(Debug, Clone, Serialize)]
pub struct CallPath {
    /// Ordered steps from entry point (first) to target (last).
    pub steps: Vec<CallPathStep>,
}

/// Result of a trace query — all call paths reaching a target symbol.
#[derive(Debug, Serialize)]
pub struct TraceResult {
    /// The target symbol name.
    pub target: String,
    /// All discovered call paths from entry points to the target.
    pub paths: Vec<CallPath>,
}

/// Build a `Symbol` from a rusqlite row. `Symbol` lives in `scope-core`
/// (no rusqlite dep); this helper bridges the row → struct in
/// scope-graph where `rusqlite` is already a dependency.
fn symbol_from_row(row: &rusqlite::Row) -> rusqlite::Result<Symbol> {
    Ok(Symbol {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        file_path: row.get("file_path")?,
        line_start: row.get("line_start")?,
        line_end: row.get("line_end")?,
        signature: row.get("signature")?,
        docstring: row.get("docstring")?,
        parent_id: row.get("parent_id")?,
        language: row.get("language")?,
        metadata: row.get("metadata")?,
    })
}

/// SQL `CASE` fragment that ranks file paths so canonical source dirs win
/// over test/fixture/dummy duplicates when multiple symbols share a name.
/// Lower number = higher priority.
fn path_priority_case_sql(column: &str) -> String {
    format!(
        "CASE \
            WHEN {col} LIKE '%/test/dummy/%' OR {col} LIKE 'test/dummy/%' THEN 40 \
            WHEN {col} LIKE '%/fixtures/%' OR {col} LIKE 'fixtures/%' THEN 35 \
            WHEN {col} LIKE '%/spec/%' OR {col} LIKE 'spec/%' THEN 30 \
            WHEN {col} LIKE '%/test/%' OR {col} LIKE 'test/%' THEN 30 \
            WHEN {col} LIKE '%/tests/%' OR {col} LIKE 'tests/%' THEN 30 \
            WHEN {col} LIKE '%/lib/%' OR {col} LIKE 'lib/%' THEN 0 \
            WHEN {col} LIKE '%/src/%' OR {col} LIKE 'src/%' THEN 0 \
            WHEN {col} LIKE '%/app/%' OR {col} LIKE 'app/%' THEN 5 \
            ELSE 10 \
         END",
        col = column,
    )
}

impl Graph {
    /// Open or create a graph database at the given path.
    ///
    /// Applies performance pragmas and ensures the schema is up to date.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open graph database at {}", path.display()))?;

        // Busy timeout for concurrent read/write safety (watch mode)
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        // Performance pragmas — safe for single-writer use
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            PRAGMA foreign_keys = ON;
            PRAGMA case_sensitive_like = ON;
        ",
        )?;

        Self::ensure_schema(&conn)?;

        Ok(Self { conn })
    }

    /// Create the schema tables and indexes if they do not exist.
    ///
    /// R0 has no in-place migration (pre-1.0 wipe policy). When a
    /// pre-R0 index database is opened, the `edges` table exists but
    /// has the legacy 5-column shape — the new CREATE INDEX statements
    /// would then fail with a cryptic `no such column: confidence`.
    /// Detect that case first and surface the wipe instruction to the
    /// user before any DDL runs.
    fn ensure_schema(conn: &Connection) -> Result<()> {
        if Self::has_legacy_edges_table(conn)? {
            anyhow::bail!(
                "scope index database has a pre-R0 schema (`edges` table lacks \
                 the `confidence` column). R0 ships no in-place migration; \
                 wipe the index and rebuild:\n    rm -rf .scope/ && scope index\n\
                 See ARCHITECTURAL-REFACTOR.md § R0 → Migration."
            );
        }
        conn.execute_batch(include_str!("sql/schema.sql"))?;
        Ok(())
    }

    /// Returns true when an `edges` table exists but lacks the R0
    /// `confidence` column — i.e., a pre-R0 schema is on disk.
    fn has_legacy_edges_table(conn: &Connection) -> Result<bool> {
        let table_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='edges'",
            [],
            |row| row.get(0),
        )?;
        if table_exists == 0 {
            return Ok(false);
        }
        // PRAGMA table_info returns a row per column; we look for a
        // `confidence` row.
        let mut stmt = conn.prepare("PRAGMA table_info(edges)")?;
        let has_confidence = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .any(|col| col == "confidence");
        Ok(!has_confidence)
    }

    /// Find a symbol by exact name match, or by qualified name (Class.method).
    ///
    /// Lookup order:
    /// 1. Exact match on `symbols.name`. If multiple matches, prefer the one
    ///    with no `parent_id` (top-level symbol).
    /// 2. If not found and `name` contains `.`, split on `.` and try qualified
    ///    lookup: `parent.name = class_part AND s.name = method_part`.
    /// 3. Returns `None` for unknown symbols.
    pub fn find_symbol(&self, name: &str) -> Result<Option<Symbol>> {
        // Try exact match first, preferring top-level symbols (parent_id IS NULL first)
        // and library/source paths over test/fixture paths so canonical
        // production definitions outrank duplicates buried in test trees.
        let result = self
            .conn
            .query_row(
                &format!(
                    "SELECT * FROM symbols WHERE name = ?1
                     ORDER BY (CASE WHEN parent_id IS NULL THEN 0 ELSE 1 END),
                              ({path_priority_sql})
                     LIMIT 1",
                    path_priority_sql = path_priority_case_sql("file_path"),
                ),
                params![name],
                symbol_from_row,
            )
            .optional()?;

        if result.is_some() {
            return Ok(result);
        }

        // Try qualified name (ClassName.methodName)
        if let Some((class, method)) = name.split_once('.') {
            return self
                .conn
                .query_row(
                    "SELECT s.* FROM symbols s
                     JOIN symbols parent ON s.parent_id = parent.id
                     WHERE parent.name = ?1 AND s.name = ?2",
                    params![class, method],
                    symbol_from_row,
                )
                .optional()
                .map_err(Into::into);
        }

        Ok(None)
    }

    /// Find a symbol by ID prefix (e.g. `"src/core/graph.rs::find_symbol"`).
    ///
    /// Matches any symbol whose `id` starts with the given prefix.
    /// Returns the first match, or `None` if no symbol matches.
    pub fn find_symbol_by_id_prefix(&self, prefix: &str) -> Result<Option<Symbol>> {
        self.conn
            .query_row(
                // Require prefix matches at a :: boundary to avoid
                // "find_symbol" matching "find_symbol_by_id_prefix".
                "SELECT * FROM symbols WHERE (id = ?1 OR substr(id, 1, length(?1) + 2) = ?1 || '::') LIMIT 1",
                params![prefix],
                symbol_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Find all symbols matching a name (for disambiguation).
    ///
    /// Unlike `find_symbol` which returns at most one, this returns all
    /// symbols with the given name so the caller can present a choice.
    pub fn find_all_matching_symbols(&self, name: &str) -> Result<Vec<Symbol>> {
        // Order by path priority so library/source paths surface before
        // test/fixture/dummy duplicates when callers pick the first result.
        let sql = format!(
            "SELECT * FROM symbols WHERE name = ?1
             ORDER BY ({path_priority_sql}), file_path, line_start",
            path_priority_sql = path_priority_case_sql("file_path"),
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![name], symbol_from_row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Get all child symbols (methods, properties) of a class/interface.
    ///
    /// Returns symbols where `parent_id = class_id`, ordered by `line_start`.
    pub fn get_methods(&self, class_id: &str) -> Result<Vec<Symbol>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM symbols WHERE parent_id = ?1 ORDER BY line_start")?;
        let rows = stmt.query_map(params![class_id], symbol_from_row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Count incoming call edges for a symbol (how many callers it has).
    pub fn get_caller_count(&self, symbol_id: &str) -> Result<usize> {
        // Extract bare name from the ID for matching member-call edges (e.g. svc.processPayment)
        let bare_name = self.symbol_name_from_id(symbol_id);
        let like_member = format!("%.{bare_name}");
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE (to_id = ?1 OR to_id = ?2 OR to_id LIKE ?3) AND kind = 'calls'",
            params![symbol_id, bare_name, like_member],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Batch version of `get_caller_count` — returns a map of symbol_id to caller count.
    ///
    /// Fetches all caller counts in a single aggregate query, then resolves
    /// each symbol in O(1) using pre-computed HashMaps. Much faster than
    /// calling `get_caller_count` per symbol (which would be N individual queries).
    pub fn get_caller_counts(&self, symbol_ids: &[&str]) -> Result<HashMap<String, usize>> {
        let mut result = HashMap::new();
        if symbol_ids.is_empty() {
            return Ok(result);
        }

        // Single aggregate query over all call edges, then O(1) lookups per symbol.
        let maps = self.get_all_caller_counts()?;

        for &sym_id in symbol_ids {
            let bare_name = self.symbol_name_from_id(sym_id);
            let count = resolve_caller_count(&maps, sym_id, &bare_name);
            if count > 0 {
                result.insert(sym_id.to_string(), count);
            }
        }

        Ok(result)
    }

    /// Get class relationships: extends, implements, and dependencies.
    pub fn get_class_relationships(&self, class_id: &str) -> Result<ClassRelationships> {
        let mut rels = ClassRelationships::default();

        // Build the set of source IDs to check: the class itself and the
        // __module__::class synthetic ID. Also check __module__ synthetic ID
        // for backward compatibility with pre-fix indexes and for import edges
        // which intentionally use module-level from_id.
        let file_path = class_id.split("::").next().unwrap_or("");
        let module_class_id = format!("{file_path}::__module__::class");

        // Get 'extends' edges from this class
        let mut stmt = self
            .conn
            .prepare("SELECT to_id FROM edges WHERE from_id IN (?1, ?2) AND kind = 'extends'")?;
        let rows = stmt.query_map(params![class_id, module_class_id], |row| {
            row.get::<_, String>(0)
        })?;
        for row in rows {
            let to_id = row?;
            let name = self.symbol_name_from_id(&to_id);
            if !rels.extends.contains(&name) {
                rels.extends.push(name);
            }
        }

        // Get 'implements' edges from this class
        let mut stmt = self
            .conn
            .prepare("SELECT to_id FROM edges WHERE from_id IN (?1, ?2) AND kind = 'implements'")?;
        let rows = stmt.query_map(params![class_id, module_class_id], |row| {
            row.get::<_, String>(0)
        })?;
        for row in rows {
            let to_id = row?;
            let name = self.symbol_name_from_id(&to_id);
            if !rels.implements.contains(&name) {
                rels.implements.push(name);
            }
        }

        // Get dependencies: distinct symbol names from outgoing edges of the class
        // and its methods (excluding extends/implements, which are already captured)
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT e.to_id FROM edges e
             WHERE (e.from_id = ?1 OR e.from_id IN (
                 SELECT id FROM symbols WHERE parent_id = ?1
             ))
             AND e.kind NOT IN ('extends', 'implements')
             AND e.to_id != ?1",
        )?;
        let rows = stmt.query_map(params![class_id], |row| row.get::<_, String>(0))?;
        for row in rows {
            let to_id = row?;
            let name = self.symbol_name_from_id(&to_id);
            if !rels.dependencies.contains(&name) {
                rels.dependencies.push(name);
            }
        }

        Ok(rels)
    }

    /// Get outgoing call edges from a symbol.
    ///
    /// Returns the names of symbols that this symbol calls.
    /// Note: edges may use `__module__` synthetic IDs for `from_id`, so results
    /// from a specific method may be incomplete.
    pub fn get_outgoing_calls(&self, symbol_id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT e.to_id FROM edges e
             WHERE e.from_id = ?1 AND e.kind = 'calls'",
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for row in rows {
            let to_id = row?;
            result.push(self.symbol_name_from_id(&to_id));
        }
        Ok(result)
    }

    /// Get incoming callers for a symbol, grouped by caller with count.
    ///
    /// Uses broad matching (exact ID, bare name, and member-call pattern)
    /// to find all call edges targeting this symbol, consistent with
    /// `get_caller_count()`.
    pub fn get_incoming_callers(&self, symbol_id: &str) -> Result<Vec<CallerInfo>> {
        let bare_name = self.symbol_name_from_id(symbol_id);
        let like_member = format!("%.{bare_name}");
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, COUNT(*) as cnt FROM edges e
             WHERE (e.to_id = ?1 OR e.to_id = ?2 OR e.to_id LIKE ?3) AND e.kind = 'calls'
             GROUP BY e.from_id
             ORDER BY cnt DESC",
        )?;
        let rows = stmt.query_map(params![symbol_id, bare_name, like_member], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (from_id, count) = row?;
            let name = self.caller_display_name(&from_id);
            result.push(CallerInfo {
                name,
                count: count as usize,
            });
        }
        Ok(result)
    }

    /// Get all caller names grouped by target symbol ID.
    ///
    /// Returns a map: `target_id -> vec of caller names`. Used during indexing
    /// to enrich FTS text with relationship context. Caller lists are deduped
    /// and truncated to 10 entries per symbol to avoid bloating the FTS text.
    pub fn get_all_caller_names(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.to_id, s.name
             FROM edges e
             JOIN symbols s ON s.id = e.from_id
             WHERE e.kind = 'calls'
             ORDER BY e.to_id",
        )?;

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (target_id, caller_name) = row?;
            let entry = result.entry(target_id).or_default();
            // Dedup: only add if not already present
            if !entry.contains(&caller_name) {
                entry.push(caller_name);
            }
        }

        // Truncate long lists to avoid bloating FTS text
        for names in result.values_mut() {
            names.truncate(10);
        }

        Ok(result)
    }

    /// Get all callee names grouped by source symbol ID.
    ///
    /// Returns a map: `source_id -> vec of callee names`. Used during indexing
    /// to enrich FTS text with relationship context. Callee lists are deduped
    /// and truncated to 10 entries per symbol to avoid bloating the FTS text.
    pub fn get_all_callee_names(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, s.name
             FROM edges e
             JOIN symbols s ON s.id = e.to_id
             WHERE e.kind = 'calls'
             ORDER BY e.from_id",
        )?;

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (source_id, callee_name) = row?;
            let entry = result.entry(source_id).or_default();
            // Dedup: only add if not already present
            if !entry.contains(&callee_name) {
                entry.push(callee_name);
            }
        }

        // Truncate long lists to avoid bloating FTS text
        for names in result.values_mut() {
            names.truncate(10);
        }

        Ok(result)
    }

    /// Compute normalized importance scores for all symbols.
    ///
    /// Score = incoming_call_count / max_incoming_call_count (0.0-1.0).
    /// Symbols with no incoming calls get 0.0.
    pub fn compute_importance_scores(&self) -> Result<HashMap<String, f64>> {
        let mut stmt = self.conn.prepare(
            "SELECT to_id, COUNT(*) as cnt FROM edges WHERE kind = 'calls' GROUP BY to_id",
        )?;

        let rows: Vec<(String, usize)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let max_count = rows.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;

        let mut scores = HashMap::new();
        for (id, count) in rows {
            scores.insert(id, count as f64 / max_count);
        }
        Ok(scores)
    }

    /// Get symbols that implement a given interface.
    pub fn get_implementors(&self, interface_id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id FROM edges e
             WHERE e.to_id = ?1 AND e.kind = 'implements'",
        )?;
        let rows = stmt.query_map(params![interface_id], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for row in rows {
            let from_id = row?;
            result.push(self.symbol_name_from_id(&from_id));
        }
        Ok(result)
    }

    /// Get all symbols in a file, ordered by line_start.
    pub fn get_file_symbols(&self, file_path: &str) -> Result<Vec<Symbol>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM symbols WHERE file_path = ?1 ORDER BY line_start")?;
        let rows = stmt.query_map(params![file_path], symbol_from_row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Find symbols that are entry points — symbols with zero incoming call edges.
    ///
    /// Returns each symbol paired with its outgoing call count (fan-out).
    /// Only considers functions, methods, and classes. Filters out test files
    /// (paths containing `test` or `spec`).
    pub fn get_entrypoints(&self) -> Result<Vec<(Symbol, usize)>> {
        // Step 1: Get all candidate symbols (functions, methods, classes), excluding test files.
        let mut stmt = self.conn.prepare(
            "SELECT * FROM symbols
             WHERE kind IN ('function', 'method', 'class')
             ORDER BY file_path, line_start",
        )?;
        let all_symbols: Vec<Symbol> = stmt
            .query_map([], symbol_from_row)?
            .filter_map(|r| r.ok())
            .collect();

        // Step 2: Get all symbol IDs/names that ARE called (targets of call edges).
        let mut called_stmt = self
            .conn
            .prepare("SELECT DISTINCT to_id FROM edges WHERE kind = 'calls'")?;
        let called_set: std::collections::HashSet<String> = called_stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Step 3: Build a HashSet of bare names from called targets for O(1) suffix matching.
        let bare_called_names: std::collections::HashSet<&str> = called_set
            .iter()
            .filter_map(|to_id| to_id.rsplit('.').next())
            .collect();

        // Step 4: Pre-compute outgoing call counts in a single aggregate query.
        let mut outgoing_stmt = self
            .conn
            .prepare("SELECT from_id, COUNT(*) FROM edges WHERE kind = 'calls' GROUP BY from_id")?;
        let outgoing_counts: HashMap<String, usize> = outgoing_stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, count)| (id, count as usize))
            .collect();

        // Step 5: Filter — keep symbols not in the called set.
        let mut results = Vec::new();
        for sym in &all_symbols {
            // Skip test files
            if is_test_file(&sym.file_path) {
                continue;
            }

            // Check if this symbol is called by any edge (3 patterns, all O(1))
            let is_called = called_set.contains(&sym.id)
                || called_set.contains(&sym.name)
                || bare_called_names.contains(sym.name.as_str());

            if !is_called {
                let outgoing = outgoing_counts.get(&sym.id).copied().unwrap_or(0);
                results.push((sym.clone(), outgoing));
            }
        }

        Ok(results)
    }

    /// Display-name lookup for an edge endpoint id.
    ///
    /// Returns the symbol's name when the id is present in the
    /// symbols table (Resolved / Ambiguous endpoints). For unresolved
    /// (Dangling) endpoints the column stores the extractor's
    /// original `to_id` text verbatim, so passing through `id` as-is
    /// preserves the unresolved reference without parsing — R3
    /// acceptance bullet 5 deletes the synthetic-id text-parse
    /// fallback that previously hid the resolved-vs-dangling
    /// distinction behind a `file::name::kind` split.
    fn symbol_name_from_id(&self, id: &str) -> String {
        self.conn
            .query_row(
                "SELECT name FROM symbols WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or_else(|| id.to_string())
    }

    /// Build a display name for a caller, including parent class if available.
    ///
    /// For `__module__` synthetic IDs, extracts the file stem.
    fn caller_display_name(&self, from_id: &str) -> String {
        // Check if this is a real symbol
        let sym = self
            .conn
            .query_row(
                "SELECT s.name, p.name FROM symbols s
                 LEFT JOIN symbols p ON s.parent_id = p.id
                 WHERE s.id = ?1",
                params![from_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional();

        match sym {
            Ok(Some((name, Some(parent)))) => format!("{parent}.{name}"),
            Ok(Some((name, None))) => name,
            _ => {
                // Synthetic ID — extract something meaningful
                if from_id.contains("__module__") {
                    // Format: "file_path::__module__::module"
                    if let Some(file_part) = from_id.split("::").next() {
                        if let Some(filename) = file_part.rsplit('/').next() {
                            return filename
                                .rsplit_once('.')
                                .map_or(filename, |(name, _)| name)
                                .to_string();
                        }
                    }
                }
                self.symbol_name_from_id(from_id)
            }
        }
    }

    /// Find all references to a symbol, with optional kind filtering and limit.
    ///
    /// Returns `(references, total_count)` where `total_count` is the untruncated
    /// count used for displaying "N more" in truncated output.
    ///
    /// Matches edges where `to_id` is either:
    /// - The exact symbol ID (e.g. `src/payments/service.ts::PaymentService::class`)
    /// - The bare symbol name (e.g. `PaymentService`)
    /// - A relative-path qualified name ending with `::SymbolName`
    pub fn find_refs(
        &self,
        symbol_name: &str,
        kinds: Option<&[&str]>,
        limit: usize,
    ) -> Result<(Vec<Reference>, usize)> {
        let symbol = self.find_symbol(symbol_name)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Symbol '{}' not found in index.\n\
                 Tip: Check spelling, or use 'scope find \"{}\"' for semantic search.",
                symbol_name,
                symbol_name
            )
        })?;

        // Collect all names to match against to_id
        let mut match_names = vec![symbol.name.clone(), symbol.id.clone()];

        // For classes, also include child method names
        if symbol.kind == "class" || symbol.kind == "struct" || symbol.kind == "interface" {
            let methods = self.get_methods(&symbol.id)?;
            for m in &methods {
                match_names.push(m.name.clone());
                match_names.push(m.id.clone());
            }
        }

        // Build the to_id matching clause:
        // Match exact name, exact ID, to_id ending with ::Name, or to_id ending with .Name
        let match_conditions = self.build_to_id_match_clause(&match_names, 1);
        let next_param = match_names.len() * 3 + 1; // each name uses 3 params (exact + %::name + %.name)

        let (kind_clause, kind_values): (String, Vec<String>) = if let Some(k) = kinds {
            let placeholders: Vec<String> = (next_param..next_param + k.len())
                .map(|i| format!("?{i}"))
                .collect();
            (
                format!("AND e.kind IN ({})", placeholders.join(", ")),
                k.iter().map(|s| s.to_string()).collect(),
            )
        } else {
            (String::new(), Vec::new())
        };

        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for name in &match_names {
            param_values.push(Box::new(name.clone()));
            param_values.push(Box::new(format!("%::{name}")));
            param_values.push(Box::new(format!("%.{name}")));
        }
        for kv in &kind_values {
            param_values.push(Box::new(kv.clone()));
        }

        // Count total
        let count_sql =
            format!("SELECT COUNT(*) FROM edges e WHERE ({match_conditions}) {kind_clause}");
        let mut count_stmt = self.conn.prepare(&count_sql)?;
        let params_ref: Vec<&dyn rusqlite::ToSql> = param_values
            .iter()
            .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
            .collect();
        let total: i64 = count_stmt.query_row(params_ref.as_slice(), |row| row.get(0))?;
        let total = total as usize;

        // Fetch refs with limit
        let limit_idx = param_values.len() + 1;
        let fetch_sql = format!(
            "SELECT e.from_id, e.kind, e.file_path, e.line
             FROM edges e
             WHERE ({match_conditions}) {kind_clause}
             ORDER BY e.kind, e.file_path, e.line
             LIMIT ?{limit_idx}"
        );
        let mut stmt = self.conn.prepare(&fetch_sql)?;

        let mut fetch_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for name in &match_names {
            fetch_params.push(Box::new(name.clone()));
            fetch_params.push(Box::new(format!("%::{name}")));
            fetch_params.push(Box::new(format!("%.{name}")));
        }
        for kv in &kind_values {
            fetch_params.push(Box::new(kv.clone()));
        }
        fetch_params.push(Box::new(limit as i64));
        let fetch_ref: Vec<&dyn rusqlite::ToSql> = fetch_params
            .iter()
            .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt.query_map(fetch_ref.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        })?;

        let mut refs = Vec::new();
        for row in rows {
            let (from_id, kind, file_path, line) = row?;
            let context = self.caller_display_name(&from_id);
            let from_name = self.symbol_name_from_id(&from_id);
            refs.push(Reference {
                from_id,
                from_name,
                kind,
                file_path,
                line,
                context,
                snippet_line: None,
                snippet: None,
            });
        }

        Ok((refs, total))
    }

    /// Build a SQL clause matching `to_id` against a set of symbol names.
    ///
    /// For each name, matches: `e.to_id = ?N OR e.to_id LIKE ?N` (pattern `%::Name`).
    /// `start_param` is the 1-based parameter index to begin with.
    /// Build a SQL WHERE clause matching edges by `to_id`.
    ///
    /// Matches exact name, fully-qualified ID suffix (`%::Name`), and
    /// dot-separated member calls (`%.Name`) so that `svc.processPayment`
    /// matches when searching for `processPayment`.
    fn build_to_id_match_clause(&self, names: &[String], start_param: usize) -> String {
        let mut conditions = Vec::new();
        let mut idx = start_param;
        for _name in names {
            // ?idx = exact match, ?idx+1 = LIKE %::name, ?idx+2 = LIKE %.name
            conditions.push(format!(
                "e.to_id = ?{idx} OR e.to_id LIKE ?{} OR e.to_id LIKE ?{}",
                idx + 1,
                idx + 2
            ));
            idx += 3;
        }
        conditions.join(" OR ")
    }

    /// Find references to a symbol, grouped by kind.
    ///
    /// Used for class symbols where refs should be displayed in groups
    /// (instantiated, extended, used as type, imported).
    #[allow(clippy::type_complexity)]
    pub fn find_refs_grouped(
        &self,
        symbol_name: &str,
        limit: usize,
    ) -> Result<(Vec<(String, Vec<Reference>)>, usize)> {
        let (refs, total) = self.find_refs(symbol_name, None, limit)?;

        // Group by kind, preserving insertion order
        let mut groups: Vec<(String, Vec<Reference>)> = Vec::new();
        for r in refs {
            if let Some(group) = groups.iter_mut().find(|(k, _)| *k == r.kind) {
                group.1.push(r);
            } else {
                let kind = r.kind.clone();
                groups.push((kind, vec![r]));
            }
        }

        Ok((groups, total))
    }

    /// Find all references to symbols in a file.
    ///
    /// Aggregates refs to every symbol defined in the given file path.
    pub fn find_file_refs(
        &self,
        file_path: &str,
        kinds: Option<&[&str]>,
        limit: usize,
    ) -> Result<(Vec<Reference>, usize)> {
        let symbols = self.get_file_symbols(file_path)?;
        if symbols.is_empty() {
            anyhow::bail!(
                "No symbols found for file '{}'.\n\
                 Tip: Check the path is relative to the project root. Run 'scope index' if the file is new.",
                file_path
            );
        }

        // Collect all names and IDs to match against to_id
        let mut match_names: Vec<String> = Vec::new();
        for sym in &symbols {
            match_names.push(sym.name.clone());
            match_names.push(sym.id.clone());
        }

        let match_conditions = self.build_to_id_match_clause(&match_names, 1);
        let next_param = match_names.len() * 3 + 1; // each name uses 3 params (exact + %::name + %.name)

        let (kind_clause, kind_values): (String, Vec<String>) = if let Some(k) = kinds {
            let placeholders: Vec<String> = (next_param..next_param + k.len())
                .map(|i| format!("?{i}"))
                .collect();
            (
                format!("AND e.kind IN ({})", placeholders.join(", ")),
                k.iter().map(|s| s.to_string()).collect(),
            )
        } else {
            (String::new(), Vec::new())
        };

        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for name in &match_names {
            param_values.push(Box::new(name.clone()));
            param_values.push(Box::new(format!("%::{name}")));
            param_values.push(Box::new(format!("%.{name}")));
        }
        for kv in &kind_values {
            param_values.push(Box::new(kv.clone()));
        }

        // Count
        let count_sql =
            format!("SELECT COUNT(*) FROM edges e WHERE ({match_conditions}) {kind_clause}");
        let mut count_stmt = self.conn.prepare(&count_sql)?;
        let params_ref: Vec<&dyn rusqlite::ToSql> = param_values
            .iter()
            .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
            .collect();
        let total: i64 = count_stmt.query_row(params_ref.as_slice(), |row| row.get(0))?;
        let total = total as usize;

        // Fetch
        let limit_idx = param_values.len() + 1;
        let fetch_sql = format!(
            "SELECT e.from_id, e.kind, e.file_path, e.line
             FROM edges e
             WHERE ({match_conditions}) {kind_clause}
             ORDER BY e.kind, e.file_path, e.line
             LIMIT ?{limit_idx}"
        );
        let mut stmt = self.conn.prepare(&fetch_sql)?;
        let mut fetch_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for name in &match_names {
            fetch_params.push(Box::new(name.clone()));
            fetch_params.push(Box::new(format!("%::{name}")));
            fetch_params.push(Box::new(format!("%.{name}")));
        }
        for kv in &kind_values {
            fetch_params.push(Box::new(kv.clone()));
        }
        fetch_params.push(Box::new(limit as i64));
        let fetch_ref: Vec<&dyn rusqlite::ToSql> = fetch_params
            .iter()
            .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt.query_map(fetch_ref.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        })?;

        let mut refs = Vec::new();
        for row in rows {
            let (from_id, kind, file_path, line) = row?;
            let context = self.caller_display_name(&from_id);
            let from_name = self.symbol_name_from_id(&from_id);
            refs.push(Reference {
                from_id,
                from_name,
                kind,
                file_path,
                line,
                context,
                snippet_line: None,
                snippet: None,
            });
        }

        Ok((refs, total))
    }

    /// Find dependencies of a symbol (outgoing edges).
    ///
    /// For depth 1: returns direct dependencies.
    /// For depth > 1: uses a recursive CTE to traverse transitive dependencies.
    /// For classes: includes dependencies from all child methods.
    ///
    /// Also includes edges from the `__module__` synthetic node for the symbol's
    /// file, since tree-sitter extractors often attribute edges to the module level.
    pub fn find_deps(&self, symbol_name: &str, max_depth: usize) -> Result<Vec<Dependency>> {
        let symbol = self.find_symbol(symbol_name)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Symbol '{}' not found in index.\n\
                 Tip: Check spelling, or use 'scope find \"{}\"' for semantic search.",
                symbol_name,
                symbol_name
            )
        })?;

        // Collect source IDs: symbol itself, child methods, and __module__ synthetic IDs
        let mut source_ids = vec![symbol.id.clone()];
        if symbol.kind == "class" || symbol.kind == "struct" || symbol.kind == "interface" {
            let methods = self.get_methods(&symbol.id)?;
            for m in &methods {
                source_ids.push(m.id.clone());
            }
        }

        // Also check __module__ synthetic ID for backward compatibility with
        // pre-fix indexes and for import edges which intentionally use module-level from_id.
        let module_id = format!("{}::__module__::function", symbol.file_path);
        if !source_ids.contains(&module_id) {
            source_ids.push(module_id);
        }

        if max_depth <= 1 {
            self.find_direct_deps(&source_ids)
        } else {
            self.find_transitive_deps(&source_ids, max_depth)
        }
    }

    /// Find dependencies of all symbols in a file.
    pub fn find_file_deps(&self, file_path: &str, max_depth: usize) -> Result<Vec<Dependency>> {
        let symbols = self.get_file_symbols(file_path)?;
        if symbols.is_empty() {
            anyhow::bail!(
                "No symbols found for file '{}'.\n\
                 Tip: Check the path is relative to the project root. Run 'scope index' if the file is new.",
                file_path
            );
        }

        let mut source_ids: Vec<String> = symbols.iter().map(|s| s.id.clone()).collect();

        // Also check __module__ synthetic ID for backward compatibility with
        // pre-fix indexes and for import edges which intentionally use module-level from_id.
        let module_id = format!("{file_path}::__module__::function");
        if !source_ids.contains(&module_id) {
            source_ids.push(module_id);
        }

        if max_depth <= 1 {
            self.find_direct_deps(&source_ids)
        } else {
            self.find_transitive_deps(&source_ids, max_depth)
        }
    }

    /// Get direct (depth-1) dependencies from a set of source symbol IDs.
    fn find_direct_deps(&self, source_ids: &[String]) -> Result<Vec<Dependency>> {
        let placeholders: Vec<String> = (1..=source_ids.len()).map(|i| format!("?{i}")).collect();
        let id_clause = placeholders.join(", ");

        let sql = format!(
            "SELECT DISTINCT e.to_id, e.kind
             FROM edges e
             WHERE e.from_id IN ({id_clause})
             ORDER BY e.kind, e.to_id"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = source_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut deps = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for row in rows {
            let (to_id, kind) = row?;

            // Skip self-references
            if source_ids.contains(&to_id) {
                continue;
            }

            // Dedup by (name, kind) to avoid listing same dep multiple times
            let name = self.symbol_name_from_id(&to_id);
            let key = format!("{name}::{kind}");
            if !seen.insert(key) {
                continue;
            }

            // Check if the dep exists in the index — try by ID first, then by name
            let sym_info = self.resolve_dep_symbol(&to_id, &name)?;

            let (dep_name, file_path, is_external) = match sym_info {
                Some((n, fp)) => (n, Some(fp), false),
                None => (name, None, true),
            };

            deps.push(Dependency {
                name: dep_name,
                file_path,
                kind,
                is_external,
                depth: 1,
            });
        }

        Ok(deps)
    }

    /// Resolve a dependency target to a symbol in the index.
    ///
    /// Tries: exact ID match, then name match (for relative-path style to_ids).
    fn resolve_dep_symbol(
        &self,
        to_id: &str,
        extracted_name: &str,
    ) -> Result<Option<(String, String)>> {
        // Try exact ID match
        let by_id: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT name, file_path FROM symbols WHERE id = ?1",
                params![to_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        if by_id.is_some() {
            return Ok(by_id);
        }

        // Try by name — prefer top-level symbols (no parent)
        let by_name: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT name, file_path FROM symbols WHERE name = ?1
                 ORDER BY (CASE WHEN parent_id IS NULL THEN 0 ELSE 1 END)
                 LIMIT 1",
                params![extracted_name],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        Ok(by_name)
    }

    /// Get transitive dependencies using a recursive CTE.
    fn find_transitive_deps(
        &self,
        source_ids: &[String],
        max_depth: usize,
    ) -> Result<Vec<Dependency>> {
        // We need a temp table approach since CTEs can't easily take dynamic IN clauses
        // for recursive seeds. Instead, build the seed UNION for all source IDs.
        let seed_conditions: Vec<String> = (1..=source_ids.len())
            .map(|i| format!("SELECT e.to_id, e.kind, 1 FROM edges e WHERE e.from_id = ?{i}"))
            .collect();
        let seed_union = seed_conditions.join(" UNION ALL ");

        let depth_param_idx = source_ids.len() + 1;
        let sql = format!(
            "WITH RECURSIVE deps(id, kind, depth) AS (
                {seed_union}
                UNION
                SELECT e.to_id, e.kind, d.depth + 1
                FROM edges e
                JOIN deps d ON e.from_id = d.id
                WHERE d.depth < ?{depth_param_idx}
            )
            SELECT DISTINCT d.id, d.kind, MIN(d.depth) as min_depth
            FROM deps d
            GROUP BY d.id, d.kind
            ORDER BY min_depth, d.kind, d.id"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for id in source_ids {
            param_values.push(Box::new(id.clone()));
        }
        param_values.push(Box::new(max_depth as i64));
        let params_ref: Vec<&dyn rusqlite::ToSql> = param_values
            .iter()
            .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;

        let mut deps = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for row in rows {
            let (to_id, kind, depth) = row?;

            // Skip self-references
            if source_ids.contains(&to_id) {
                continue;
            }

            let name = self.symbol_name_from_id(&to_id);
            let key = format!("{name}::{kind}");
            if !seen.insert(key) {
                continue;
            }

            let sym_info = self.resolve_dep_symbol(&to_id, &name)?;

            let (dep_name, file_path, is_external) = match sym_info {
                Some((n, fp)) => (n, Some(fp), false),
                None => (name, None, true),
            };

            deps.push(Dependency {
                name: dep_name,
                file_path,
                kind,
                is_external,
                depth: depth as usize,
            });
        }

        Ok(deps)
    }

    /// Check if a symbol is a class (or struct/interface — types that get grouped refs).
    pub fn is_class_like(&self, symbol_name: &str) -> Result<bool> {
        let symbol = self.find_symbol(symbol_name)?;
        Ok(symbol
            .map(|s| s.kind == "class" || s.kind == "struct" || s.kind == "interface")
            .unwrap_or(false))
    }

    /// Find the transitive impact (blast radius) of changing a symbol.
    ///
    /// Performs a recursive reverse dependency traversal: finds all symbols
    /// that directly or transitively depend on the given symbol. Results are
    /// grouped by depth and test files are separated.
    ///
    /// Uses the same name-matching pattern as `find_refs` (exact name, exact
    /// ID, or `LIKE '%::Name'`) to match `to_id` in the edges table.
    pub fn find_impact(&self, symbol_name: &str, max_depth: usize) -> Result<ImpactResult> {
        let symbol = self.find_symbol(symbol_name)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Symbol '{}' not found in index.\n\
                 Tip: Check spelling, or use 'scope find \"{}\"' for semantic search.",
                symbol_name,
                symbol_name
            )
        })?;

        // Collect all IDs to seed the impact traversal
        let mut seed_ids = vec![symbol.id.clone()];

        // For classes, also include child methods as seeds
        if symbol.kind == "class" || symbol.kind == "struct" || symbol.kind == "interface" {
            let methods = self.get_methods(&symbol.id)?;
            for m in &methods {
                seed_ids.push(m.id.clone());
            }
        }

        self.run_impact_query(&seed_ids, max_depth)
    }

    /// Find the impact of changing any symbol in a file.
    ///
    /// Collects all symbols in the file and runs impact analysis for each,
    /// deduplicating results.
    pub fn find_file_impact(&self, file_path: &str, max_depth: usize) -> Result<ImpactResult> {
        let symbols = self.get_file_symbols(file_path)?;
        if symbols.is_empty() {
            anyhow::bail!(
                "No symbols found for file '{}'.\n\
                 Tip: Check the path is relative to the project root. Run 'scope index' if the file is new.",
                file_path
            );
        }

        let seed_ids: Vec<String> = symbols.iter().map(|s| s.id.clone()).collect();
        self.run_impact_query(&seed_ids, max_depth)
    }

    /// Execute the recursive CTE impact query for a set of seed symbol IDs.
    fn run_impact_query(&self, seed_ids: &[String], max_depth: usize) -> Result<ImpactResult> {
        // Build seed conditions: for each seed ID, match edges where
        // to_id equals the ID exactly, matches the name, or ends with ::Name
        let mut seed_unions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1usize;

        for seed_id in seed_ids {
            // Extract the bare name from the ID
            let bare_name = self.symbol_name_from_id(seed_id);
            let like_qualified = format!("%::{bare_name}");
            let like_member = format!("%.{bare_name}");

            seed_unions.push(format!(
                "SELECT e.from_id, 1, CAST(e.from_id AS TEXT) \
                 FROM edges e WHERE (e.to_id = ?{idx} OR e.to_id = ?{} OR e.to_id LIKE ?{} OR e.to_id LIKE ?{})",
                idx + 1,
                idx + 2,
                idx + 3
            ));
            param_values.push(Box::new(seed_id.clone()));
            param_values.push(Box::new(bare_name));
            param_values.push(Box::new(like_qualified));
            param_values.push(Box::new(like_member));
            idx += 4;
        }

        let seed_sql = seed_unions.join(" UNION ALL ");
        let depth_param = idx;
        param_values.push(Box::new(max_depth as i64));

        let sql = format!(
            "WITH RECURSIVE impact(id, depth, path) AS (
                {seed_sql}
                UNION ALL
                SELECT e.from_id, i.depth + 1, i.path || ',' || e.from_id
                FROM edges e
                JOIN impact i ON e.to_id = i.id
                WHERE i.depth < ?{depth_param}
                  AND INSTR(',' || i.path || ',', ',' || e.from_id || ',') = 0
            )
            SELECT DISTINCT i.id, MIN(i.depth) as min_depth, s.name, s.file_path, s.kind
            FROM impact i
            JOIN symbols s ON s.id = i.id
            GROUP BY i.id
            ORDER BY min_depth, s.file_path"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::ToSql> = param_values
            .iter()
            .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(ImpactNode {
                id: row.get(0)?,
                depth: row.get::<_, i64>(1)? as usize,
                name: row.get(2)?,
                file_path: row.get(3)?,
                kind: row.get(4)?,
            })
        })?;

        let mut all_nodes: Vec<ImpactNode> = Vec::new();
        for row in rows {
            let node = row?;
            // Skip seed IDs from appearing in the results
            if seed_ids.contains(&node.id) {
                continue;
            }
            all_nodes.push(node);
        }

        // Separate test files from non-test files
        let mut test_files: Vec<ImpactNode> = Vec::new();
        let mut non_test_nodes: Vec<ImpactNode> = Vec::new();

        for node in all_nodes {
            if is_test_file(&node.file_path) {
                test_files.push(node);
            } else {
                non_test_nodes.push(node);
            }
        }

        let total_affected = non_test_nodes.len();

        // Group non-test nodes by depth
        let mut depth_map: std::collections::BTreeMap<usize, Vec<ImpactNode>> =
            std::collections::BTreeMap::new();
        for node in non_test_nodes {
            depth_map.entry(node.depth).or_default().push(node);
        }

        let nodes_by_depth: Vec<(usize, Vec<ImpactNode>)> = depth_map.into_iter().collect();

        Ok(ImpactResult {
            nodes_by_depth,
            test_files,
            total_affected,
        })
    }

    /// Find call paths from `start_id` to `end_id` through the call graph.
    ///
    /// Uses a forward BFS via recursive CTE. Returns up to `max_paths`
    /// shortest paths, each as a `Vec<CallPathStep>`.
    pub fn find_flow_paths(
        &self,
        start_id: &str,
        end_id: &str,
        max_depth: usize,
        max_paths: usize,
    ) -> Result<Vec<Vec<CallPathStep>>> {
        let sql = "
            WITH RECURSIVE flow(current_id, path, depth) AS (
                -- Seed: the start symbol
                SELECT ?1, ?1, 0

                UNION ALL

                -- Walk forward via 'calls' edges
                SELECT e.to_id,
                       flow.path || '>' || e.to_id,
                       flow.depth + 1
                FROM edges e
                JOIN flow ON e.from_id = flow.current_id
                WHERE e.kind = 'calls'
                  AND flow.depth < ?3
                  AND INSTR('>' || flow.path || '>', '>' || e.to_id || '>') = 0
            )
            SELECT path FROM flow
            WHERE current_id = ?2
            ORDER BY depth
            LIMIT ?4
        ";

        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(
            params![start_id, end_id, max_depth as i64, max_paths as i64],
            |row| {
                let path: String = row.get(0)?;
                Ok(path)
            },
        )?;

        let mut result: Vec<Vec<CallPathStep>> = Vec::new();
        for row in rows {
            let raw_path = row?;
            let ids: Vec<&str> = raw_path.split('>').collect();
            let mut steps = Vec::new();
            for id in &ids {
                steps.push(self.resolve_call_path_step(id)?);
            }
            if !steps.is_empty() {
                result.push(steps);
            }
        }

        Ok(result)
    }

    /// Find all call paths from entry points to a target symbol.
    ///
    /// Walks the call graph backward from the target to discover entry points
    /// (symbols with no incoming `calls` edges). Returns all distinct paths
    /// from each entry point through intermediate callers to the target.
    pub fn find_call_paths(
        &self,
        target_id: &str,
        target_name: &str,
        max_depth: usize,
        max_paths: usize,
    ) -> Result<TraceResult> {
        // Extract bare name for flexible matching
        let bare_name = self.symbol_name_from_id(target_id);
        let like_qualified = format!("%::{bare_name}");
        let like_member = format!("%.{bare_name}");

        // Recursive CTE: walk backward from target, keeping the full path.
        // The path is built as `from_id>from_id>...>target_id` (entry-point first after reversal).
        // We filter leaf nodes = those whose `id` has no incoming `calls` edges (entry points).
        let sql = "
            WITH RECURSIVE trace(id, depth, path) AS (
                -- Seed: direct callers of the target
                SELECT e.from_id, 1, e.from_id || '>' || ?1
                FROM edges e
                WHERE (e.to_id = ?1 OR e.to_id = ?2 OR e.to_id LIKE ?3 OR e.to_id LIKE ?4)
                  AND e.kind = 'calls'

                UNION ALL

                -- Walk backward: find who calls the current head of the path.
                -- Use fuzzy matching on e.to_id since edges may store bare names,
                -- qualified names (::Name), or member names (.Name) instead of full IDs.
                SELECT e.from_id, t.depth + 1, e.from_id || '>' || t.path
                FROM edges e
                JOIN trace t ON (
                    e.to_id = t.id
                    OR e.to_id = REPLACE(REPLACE(t.id, RTRIM(t.id, REPLACE(t.id, '::', '')), ''), '::', '')
                    OR t.id LIKE '%::' || e.to_id || '::%'
                    OR t.id LIKE '%.' || e.to_id
                )
                WHERE e.kind = 'calls'
                  AND t.depth < ?5
                  AND INSTR('>' || t.path || '>', '>' || e.from_id || '>') = 0
            )
            -- Return paths that terminate at entry points (no incoming calls).
            -- Check both exact and bare-name matches for incoming edges.
            SELECT t.path, t.depth
            FROM trace t
            WHERE NOT EXISTS (
                SELECT 1 FROM edges e2
                WHERE (e2.to_id = t.id
                    OR e2.to_id = REPLACE(REPLACE(t.id, RTRIM(t.id, REPLACE(t.id, '::', '')), ''), '::', '')
                    OR t.id LIKE '%::' || e2.to_id || '::%'
                    OR t.id LIKE '%.' || e2.to_id)
                  AND e2.kind = 'calls'
            )
            ORDER BY t.depth, t.path
            LIMIT ?6
        ";

        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(
            params![
                target_id,
                bare_name,
                like_qualified,
                like_member,
                max_depth as i64,
                max_paths as i64,
            ],
            |row| {
                let path: String = row.get(0)?;
                Ok(path)
            },
        )?;

        let mut raw_paths: Vec<String> = Vec::new();
        for row in rows {
            raw_paths.push(row?);
        }

        // Deduplicate paths (same sequence of symbol IDs)
        let mut seen = std::collections::HashSet::new();
        raw_paths.retain(|p| seen.insert(p.clone()));

        // Resolve each path: split on '>' and look up symbol info for each step
        let mut paths: Vec<CallPath> = Vec::new();
        for raw_path in &raw_paths {
            let ids: Vec<&str> = raw_path.split('>').collect();
            let mut steps: Vec<CallPathStep> = Vec::new();

            for id in &ids {
                let step = self.resolve_call_path_step(id)?;
                steps.push(step);
            }

            if !steps.is_empty() {
                paths.push(CallPath { steps });
            }
        }

        // Sort: shortest paths first, then alphabetically by first step name
        paths.sort_by(|a, b| {
            a.steps.len().cmp(&b.steps.len()).then_with(|| {
                let a_name = a
                    .steps
                    .first()
                    .map(|s| s.symbol_name.as_str())
                    .unwrap_or("");
                let b_name = b
                    .steps
                    .first()
                    .map(|s| s.symbol_name.as_str())
                    .unwrap_or("");
                a_name.cmp(b_name)
            })
        });

        Ok(TraceResult {
            target: target_name.to_string(),
            paths,
        })
    }

    /// Resolve a symbol ID to a `CallPathStep`.
    ///
    /// On miss (Dangling endpoint), returns a `CallPathStep` with the
    /// id preserved verbatim as both `symbol_id` and `symbol_name`, no
    /// file path, no line. R3 acceptance bullet 5 deletes the
    /// synthetic-id text-parse fallback that previously fabricated a
    /// `(file_path, name, kind)` triple by splitting `id` on `::` —
    /// the parse silently turned Dangling into something that *looked*
    /// like Resolved, hiding the cleanest-signal distinction from
    /// downstream consumers.
    fn resolve_call_path_step(&self, id: &str) -> Result<CallPathStep> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, name, kind, file_path, line_start FROM symbols WHERE id = ?1",
                params![id],
                |row| {
                    Ok(CallPathStep {
                        symbol_id: row.get(0)?,
                        symbol_name: row.get(1)?,
                        kind: row.get(2)?,
                        file_path: row.get(3)?,
                        line: row.get(4)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_else(|| CallPathStep {
                symbol_id: id.to_string(),
                symbol_name: id.to_string(),
                file_path: String::new(),
                line: 0,
                kind: "unknown".to_string(),
            }))
    }

    /// Route a single `RawEdge` through the R3 resolver
    /// ([`crate::resolve::Resolver`]) and return every `InsertableEdge`
    /// the resolution produced. The return shape is `Vec` because
    /// multi-row Ambiguous expansion can turn one `RawEdge` into
    /// `N` `InsertableEdge`s, one per matched candidate symbol
    /// (R3 multiplicity commitment).
    pub fn resolve(&self, raw: RawEdge) -> Result<Vec<InsertableEdge>> {
        Ok(crate::resolve::Resolver::new(&self.conn)
            .resolve(crate::resolve::Captured::new(raw))?
            .into_insertable())
    }

    /// Batch variant of [`Graph::resolve`]. Total output length is
    /// `>=` input length: each Ambiguous input contributes one row per
    /// candidate target; Resolved + Dangling contribute one row each.
    pub fn resolve_batch(&self, raws: Vec<RawEdge>) -> Result<Vec<InsertableEdge>> {
        crate::resolve::Resolver::new(&self.conn).resolve_batch(raws)
    }

    /// Insert (or replace) the symbols for a single file in their own
    /// transaction. The two-pass indexer uses this to write every
    /// file's symbols before the resolver runs over any file's edges,
    /// so same-file and cross-file targets are visible to the Phase A
    /// resolver stub.
    pub fn insert_symbols_for_file(&mut self, file_path: &str, symbols: &[Symbol]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM symbols WHERE file_path = ?1",
            params![file_path],
        )?;
        // Order so that parents (parent_id = NULL) are inserted before
        // children. SQLite enforces the parent_id FK on each statement.
        let mut ordered: Vec<&Symbol> = symbols.iter().collect();
        ordered.sort_by_key(|s| s.parent_id.is_some());
        {
            let mut stmt = tx.prepare(
                "INSERT INTO symbols
                 (id, name, kind, file_path, line_start, line_end, signature, docstring, parent_id, language, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for symbol in &ordered {
                stmt.execute(params![
                    symbol.id,
                    symbol.name,
                    symbol.kind,
                    symbol.file_path,
                    symbol.line_start,
                    symbol.line_end,
                    symbol.signature,
                    symbol.docstring,
                    symbol.parent_id,
                    symbol.language,
                    symbol.metadata,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Insert (or replace) the edges for a single file in their own
    /// transaction. Pairs with [`Graph::insert_symbols_for_file`].
    pub fn insert_edges_for_file(
        &mut self,
        file_path: &str,
        edges: &[InsertableEdge],
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM edges WHERE file_path = ?1", params![file_path])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO edges
                 (from_id, to_id, kind, confidence, status, producer, pattern_id,
                  capture_id, framework, args_text, file_path, line)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;
            for edge in edges {
                stmt.execute(params![
                    edge.from_id(),
                    edge.to_id(),
                    edge.kind().as_slug(),
                    edge.confidence().as_slug(),
                    edge.status().as_slug(),
                    edge.producer().as_slug(),
                    edge.pattern_id(),
                    edge.capture_id(),
                    edge.framework(),
                    edge.args_text(),
                    edge.file_path(),
                    edge.line(),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// R1: storage layer accepts only `InsertableEdge` (output of the
    /// resolver). `RawEdge` does not implement `Insertable` and is
    /// rejected at the type level. Callers must route through
    /// [`Graph::resolve`] (Phase A trivial stub; R3 replaces it).
    ///
    /// This single-shot path writes a file's symbols **then** its
    /// edges atomically. For full-index runs use the two-pass
    /// [`Graph::insert_symbols_for_file`] / [`Graph::insert_edges_for_file`]
    /// pair so cross-file targets are visible to the resolver.
    pub fn insert_file_data(
        &mut self,
        file_path: &str,
        symbols: &[Symbol],
        edges: &[InsertableEdge],
    ) -> Result<()> {
        let tx = self.conn.transaction()?;

        // Delete existing data for this file
        tx.execute("DELETE FROM edges WHERE file_path = ?1", params![file_path])?;
        tx.execute(
            "DELETE FROM symbols WHERE file_path = ?1",
            params![file_path],
        )?;

        // Insert symbols. Order so that parents (parent_id = NULL) are inserted
        // before children. SQLite enforces the parent_id FK on each statement, not
        // at commit time, so a child whose parent has not been inserted yet would
        // fail. This happens in Rust when `impl Foo { ... }` appears earlier in
        // the file than `struct Foo;` — extraction order yields methods before the
        // struct, but the methods' parent_id is rewritten to point at the struct
        // by `associate_rust_impl_methods`.
        let mut ordered: Vec<&Symbol> = symbols.iter().collect();
        ordered.sort_by_key(|s| s.parent_id.is_some());
        {
            let mut stmt = tx.prepare(
                "INSERT INTO symbols
                 (id, name, kind, file_path, line_start, line_end, signature, docstring, parent_id, language, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;

            for symbol in &ordered {
                stmt.execute(params![
                    symbol.id,
                    symbol.name,
                    symbol.kind,
                    symbol.file_path,
                    symbol.line_start,
                    symbol.line_end,
                    symbol.signature,
                    symbol.docstring,
                    symbol.parent_id,
                    symbol.language,
                    symbol.metadata,
                ])?;
            }
        }

        // R0 surrogate edge_id PK + the new NOT NULL columns. The legacy
        // INSERT OR IGNORE is replaced by plain INSERT — the surrogate PK
        // never collides; multiplicity (one row per candidate target on
        // ambiguous resolution, one row per call site) is preserved.
        {
            let mut stmt = tx.prepare(
                "INSERT INTO edges
                 (from_id, to_id, kind, confidence, status, producer, pattern_id,
                  capture_id, framework, args_text, file_path, line)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;

            for edge in edges {
                stmt.execute(params![
                    edge.from_id(),
                    edge.to_id(),
                    edge.kind().as_slug(),
                    edge.confidence().as_slug(),
                    edge.status().as_slug(),
                    edge.producer().as_slug(),
                    edge.pattern_id(),
                    edge.capture_id(),
                    edge.framework(),
                    edge.args_text(),
                    edge.file_path(),
                    edge.line(),
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Delete all symbols, edges, and file hash data for a given file path.
    pub fn delete_file_data(&mut self, file_path: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM edges WHERE file_path = ?1", params![file_path])?;
        tx.execute(
            "DELETE FROM symbols WHERE file_path = ?1",
            params![file_path],
        )?;
        tx.execute(
            "DELETE FROM file_hashes WHERE file_path = ?1",
            params![file_path],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Clear all data from the graph (used before a full re-index).
    pub fn clear_all(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM edges", [])?;
        tx.execute("DELETE FROM symbols", [])?;
        tx.execute("DELETE FROM file_hashes", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Get the total number of symbols in the index.
    pub fn symbol_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get the total number of indexed files.
    pub fn file_count(&self) -> Result<usize> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(DISTINCT file_path) FROM symbols", [], |row| {
                    row.get(0)
                })?;
        Ok(count as usize)
    }

    /// Get the total number of edges in the index.
    pub fn edge_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get top N symbols by incoming call count (importance).
    ///
    /// Returns `(Symbol, caller_count)` pairs sorted by caller count descending.
    /// Only considers functions and methods with at least one incoming call edge.
    /// Uses the same matching logic as `get_caller_count`: exact ID, bare name,
    /// and member-call patterns (e.g. `svc.processPayment`).
    pub fn get_symbols_by_importance(&self, limit: usize) -> Result<Vec<(Symbol, usize)>> {
        // Pre-compute all caller counts in a single aggregate query to avoid N+1.
        let caller_counts = self.get_all_caller_counts()?;

        // Fetch all function/method symbols.
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM symbols WHERE kind IN ('function', 'method')")?;
        let all_symbols: Vec<Symbol> = stmt
            .query_map([], symbol_from_row)?
            .filter_map(|r| r.ok())
            .collect();

        let mut scored: Vec<(Symbol, usize)> = Vec::new();
        for sym in &all_symbols {
            let count = resolve_caller_count(&caller_counts, &sym.id, &sym.name);
            if count > 0 {
                scored.push((sym.clone(), count));
            }
        }

        // Sort by caller count descending, then by name for deterministic output.
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.name.cmp(&b.0.name)));
        scored.truncate(limit);
        Ok(scored)
    }

    /// Fetch all call-edge target counts in a single aggregate query.
    ///
    /// Returns two maps for O(1) lookup by all three matching patterns:
    /// - `by_id`: exact `to_id` → count (covers patterns 1 and 2: exact ID and bare name)
    /// - `by_suffix`: bare name (part after last `.`) → count (covers pattern 3: member-call)
    fn get_all_caller_counts(&self) -> Result<CallerCountMaps> {
        let mut stmt = self
            .conn
            .prepare("SELECT to_id, COUNT(*) FROM edges WHERE kind = 'calls' GROUP BY to_id")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut by_id = HashMap::new();
        let mut by_suffix: HashMap<String, usize> = HashMap::new();
        for row in rows {
            let (to_id, count) = row?;
            let count = count as usize;
            // Build suffix map: extract bare name after last '.'
            if let Some(bare) = to_id.rsplit('.').next() {
                if bare != to_id {
                    *by_suffix.entry(bare.to_string()).or_insert(0) += count;
                }
            }
            by_id.insert(to_id, count);
        }
        Ok(CallerCountMaps { by_id, by_suffix })
    }

    /// Get directory-level statistics.
    ///
    /// Returns `(directory_path, file_count, symbol_count)` tuples grouped by
    /// the top-level directory component (after stripping a leading `src/`).
    pub fn get_directory_stats(&self) -> Result<Vec<(String, usize, usize)>> {
        let mut stmt = self.conn.prepare("SELECT file_path FROM symbols")?;
        let paths: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Group by top-level directory.
        let mut dir_files: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        let mut dir_symbols: HashMap<String, usize> = HashMap::new();

        for path in &paths {
            let normalized = path.replace('\\', "/");
            // Strip leading "src/" if present.
            let stripped = normalized.strip_prefix("src/").unwrap_or(&normalized);

            // Extract top-level directory component.
            let dir = if let Some(slash_pos) = stripped.find('/') {
                &stripped[..slash_pos]
            } else {
                // File is directly in src/ or root — use "(root)".
                "(root)"
            };

            let dir_key = format!("{dir}/");
            dir_files
                .entry(dir_key.clone())
                .or_default()
                .insert(normalized);
            *dir_symbols.entry(dir_key).or_insert(0) += 1;
        }

        let mut results: Vec<(String, usize, usize)> = dir_files
            .iter()
            .map(|(dir, files)| {
                let sym_count = dir_symbols.get(dir).copied().unwrap_or(0);
                (dir.clone(), files.len(), sym_count)
            })
            .collect();

        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }

    /// Get distinct languages present in the index.
    pub fn get_languages(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT language FROM symbols ORDER BY language")?;
        let langs: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(langs)
    }

    // -- File hash operations --

    /// Compare current file hashes against the stored index to find changes.
    pub fn get_changed_files(
        &self,
        current_hashes: &HashMap<String, String>,
    ) -> Result<ChangedFiles> {
        let mut changed = ChangedFiles::default();

        // Load stored hashes
        let stored: HashMap<String, String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT file_path, hash FROM file_hashes")?;
            let rows: Vec<(String, String)> = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .filter_map(|r| r.ok())
                .collect();
            rows.into_iter().collect()
        };

        for (path, hash) in current_hashes {
            match stored.get(path) {
                None => changed.added.push(path.clone()),
                Some(old_hash) if old_hash != hash => changed.modified.push(path.clone()),
                _ => {} // unchanged
            }
        }

        for path in stored.keys() {
            if !current_hashes.contains_key(path) {
                changed.deleted.push(path.clone());
            }
        }

        Ok(changed)
    }

    /// Update the stored file hashes after indexing.
    ///
    /// The `skipped_ranges` slice on each `FileHashRow` is serialised to JSON
    /// and written to `file_hashes.skipped_ranges`. An empty slice yields
    /// `'[]'`, matching the column default. R6 (sprint 0007) reads this column
    /// during the malformed-source harness to distinguish indexed-but-degraded
    /// files from clean parses.
    pub fn update_file_hashes(&mut self, hashes: &HashMap<String, FileHashRow>) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO file_hashes (file_path, hash, indexed_at, skipped_ranges)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;

            for (path, row) in hashes {
                let skipped_json = serde_json::to_string(&row.skipped_ranges)?;
                stmt.execute(params![path, row.hash, now, skipped_json])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Get the most recent `indexed_at` timestamp from file_hashes.
    ///
    /// Returns `None` if no files have been indexed yet.
    pub fn last_indexed_at(&self) -> Result<Option<i64>> {
        let ts: Option<i64> =
            self.conn
                .query_row("SELECT MAX(indexed_at) FROM file_hashes", [], |row| {
                    row.get(0)
                })?;
        Ok(ts)
    }

    /// Quick staleness check: returns `true` if any indexed file has been
    /// modified or deleted since its `indexed_at` timestamp. Short-circuits
    /// on first stale file found — O(1) best case for large repos.
    pub fn has_stale_files(&self, project_root: &Path) -> Result<bool> {
        // Quick check: compare the most-recently-indexed file's mtime first.
        // This catches the common case (active development on recent files)
        // with a single stat() call instead of scanning every row.
        let newest: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT file_path, indexed_at FROM file_hashes ORDER BY indexed_at DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        if let Some((file_path, indexed_at)) = newest {
            let full_path = project_root.join(&file_path);
            match std::fs::metadata(&full_path) {
                Ok(meta) => {
                    if let Ok(mtime) = meta.modified() {
                        let mtime_secs = mtime
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        if mtime_secs > indexed_at {
                            return Ok(true);
                        }
                    }
                }
                Err(_) => return Ok(true),
            }
        }

        // Full scan: iterate remaining rows but return on first stale hit.
        let mut stmt = self
            .conn
            .prepare("SELECT file_path, indexed_at FROM file_hashes")?;
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let file_path: String = row.get(0)?;
            let indexed_at: i64 = row.get(1)?;
            let full_path = project_root.join(&file_path);
            match std::fs::metadata(&full_path) {
                Ok(meta) => {
                    if let Ok(mtime) = meta.modified() {
                        let mtime_secs = mtime
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        if mtime_secs > indexed_at {
                            return Ok(true);
                        }
                    }
                }
                Err(_) => return Ok(true),
            }
        }

        Ok(false)
    }
}

/// Check if a file path belongs to a test file.
///
/// Heuristic: returns `true` if the lowercase path contains common test path
/// segments or test file naming patterns.
pub fn is_test_file(file_path: &str) -> bool {
    let lower = file_path.to_lowercase().replace('\\', "/");
    lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains("_test.")
        || lower.contains("_spec.")
        || lower.starts_with("test/")
        || lower.starts_with("tests/")
}

/// Pre-computed caller count maps for O(1) symbol resolution.
struct CallerCountMaps {
    /// Exact `to_id` → count (for pattern 1: exact ID, pattern 2: bare name)
    by_id: HashMap<String, usize>,
    /// Bare name (after last `.`) → count (for pattern 3: member-call suffix)
    by_suffix: HashMap<String, usize>,
}

/// Resolve a symbol's caller count from pre-computed maps in O(1).
///
/// Matches using the same three patterns as `get_caller_count`:
/// 1. Exact ID match (O(1) HashMap lookup)
/// 2. Bare name match (O(1) HashMap lookup)
/// 3. Member-call suffix match via pre-computed suffix map (O(1) lookup)
fn resolve_caller_count(maps: &CallerCountMaps, symbol_id: &str, symbol_name: &str) -> usize {
    let mut total = 0usize;
    // Pattern 1: exact ID match
    if let Some(&c) = maps.by_id.get(symbol_id) {
        total += c;
    }
    // Pattern 2: bare name match (only if different from ID)
    if symbol_name != symbol_id {
        if let Some(&c) = maps.by_id.get(symbol_name) {
            total += c;
        }
    }
    // Pattern 3: member-call suffix — use pre-computed suffix map
    if let Some(&c) = maps.by_suffix.get(symbol_name) {
        total += c;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_caller_count_exact_id() {
        let maps = CallerCountMaps {
            by_id: HashMap::from([("src/foo.ts::processPayment::function::10".to_string(), 3)]),
            by_suffix: HashMap::new(),
        };
        assert_eq!(
            resolve_caller_count(
                &maps,
                "src/foo.ts::processPayment::function::10",
                "processPayment"
            ),
            3
        );
    }

    #[test]
    fn test_resolve_caller_count_bare_name() {
        let maps = CallerCountMaps {
            by_id: HashMap::from([("processPayment".to_string(), 2)]),
            by_suffix: HashMap::new(),
        };
        assert_eq!(
            resolve_caller_count(
                &maps,
                "src/foo.ts::processPayment::function::10",
                "processPayment"
            ),
            2
        );
    }

    #[test]
    fn test_resolve_caller_count_suffix_match() {
        let maps = CallerCountMaps {
            by_id: HashMap::new(),
            by_suffix: HashMap::from([("processPayment".to_string(), 5)]),
        };
        assert_eq!(
            resolve_caller_count(
                &maps,
                "src/foo.ts::processPayment::function::10",
                "processPayment"
            ),
            5
        );
    }

    #[test]
    fn test_resolve_caller_count_all_three_patterns() {
        let maps = CallerCountMaps {
            by_id: HashMap::from([
                ("src/foo.ts::processPayment::function::10".to_string(), 1),
                ("processPayment".to_string(), 2),
            ]),
            by_suffix: HashMap::from([("processPayment".to_string(), 3)]),
        };
        // All three patterns match: 1 + 2 + 3 = 6
        assert_eq!(
            resolve_caller_count(
                &maps,
                "src/foo.ts::processPayment::function::10",
                "processPayment"
            ),
            6
        );
    }

    #[test]
    fn test_resolve_caller_count_no_match() {
        let maps = CallerCountMaps {
            by_id: HashMap::from([("other".to_string(), 10)]),
            by_suffix: HashMap::from([("other".to_string(), 5)]),
        };
        assert_eq!(
            resolve_caller_count(
                &maps,
                "src/foo.ts::processPayment::function::10",
                "processPayment"
            ),
            0
        );
    }

    #[test]
    fn test_is_test_file() {
        assert!(is_test_file("tests/unit/payment.test.ts"));
        assert!(is_test_file("src/payments/PaymentService.spec.ts"));
        assert!(is_test_file("Tests/Unit/PaymentTests.cs"));
        assert!(!is_test_file("src/payments/PaymentService.ts"));
        assert!(!is_test_file("src/controllers/OrderController.cs"));
    }

    #[test]
    fn test_incoming_callers_finds_bare_name_edges() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("graph.db");
        let mut graph = Graph::open(&db_path).unwrap();

        // Target symbol with full ID
        let target = Symbol {
            id: "src/payment.ts::processPayment::function::10".to_string(),
            name: "processPayment".to_string(),
            kind: "function".to_string(),
            file_path: "src/payment.ts".to_string(),
            line_start: 10,
            line_end: 20,
            signature: None,
            docstring: None,
            parent_id: None,
            language: "typescript".to_string(),
            metadata: "{}".to_string(),
        };

        // Caller symbol
        let caller = Symbol {
            id: "src/order.ts::checkout::function::5".to_string(),
            name: "checkout".to_string(),
            kind: "function".to_string(),
            file_path: "src/order.ts".to_string(),
            line_start: 5,
            line_end: 15,
            signature: None,
            docstring: None,
            parent_id: None,
            language: "typescript".to_string(),
            metadata: "{}".to_string(),
        };

        graph
            .insert_file_data("src/payment.ts", &[target], &[])
            .unwrap();

        use scope_core::{Confidence, EdgeKind, Producer};

        // Edge uses bare name as to_id (the bug scenario)
        let raw_bare = Edge::builder()
            .from("src/order.ts::checkout::function::5")
            .to("processPayment")
            .kind(EdgeKind::Calls)
            .confidence(Confidence::Medium)
            .producer(Producer::Lang("typescript".into()))
            .pattern_id("test.calls.bare")
            .file_path("src/order.ts")
            .line(8)
            .build();

        // Edge uses member-call pattern as to_id
        let raw_member = Edge::builder()
            .from("src/order.ts::checkout::function::5")
            .to("svc.processPayment")
            .kind(EdgeKind::Calls)
            .confidence(Confidence::Medium)
            .producer(Producer::Lang("typescript".into()))
            .pattern_id("test.calls.member")
            .file_path("src/order.ts")
            .line(9)
            .build();

        let mut edges = graph.resolve(raw_bare).unwrap();
        edges.extend(graph.resolve(raw_member).unwrap());

        graph
            .insert_file_data("src/order.ts", &[caller], &edges)
            .unwrap();

        let callers = graph
            .get_incoming_callers("src/payment.ts::processPayment::function::10")
            .unwrap();

        // Should find the caller via bare-name and member-pattern matching
        assert_eq!(callers.len(), 1, "expected 1 caller, got {:?}", callers);
        assert_eq!(callers[0].count, 2, "expected 2 call sites from checkout");
    }

    #[test]
    fn test_update_file_hashes_persists_skipped_ranges() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("graph.db");
        let mut graph = Graph::open(&db_path).unwrap();

        let mut hashes = HashMap::new();
        hashes.insert(
            "src/messy.rs".to_string(),
            FileHashRow {
                hash: "abc123".to_string(),
                skipped_ranges: vec![
                    SkippedRange {
                        start_line: 5,
                        end_line: 10,
                        reason: "tree_sitter_error:syntax_error".to_string(),
                    },
                    SkippedRange {
                        start_line: 42,
                        end_line: 84,
                        reason: "plugin_skip:rust:unparseable_macro_body".to_string(),
                    },
                ],
            },
        );

        graph.update_file_hashes(&hashes).unwrap();

        let json: String = graph
            .conn
            .query_row(
                "SELECT skipped_ranges FROM file_hashes WHERE file_path = ?1",
                ["src/messy.rs"],
                |row| row.get(0),
            )
            .unwrap();

        let parsed: Vec<SkippedRange> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2, "expected 2 ranges, got {parsed:?}");
        assert_eq!(parsed[0].start_line, 5);
        assert!(parsed[0].reason.starts_with("tree_sitter_error:"));
        assert_eq!(parsed[1].start_line, 42);
        assert!(parsed[1].reason.starts_with("plugin_skip:"));
    }

    #[test]
    fn test_update_file_hashes_empty_skipped_ranges_writes_empty_array() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("graph.db");
        let mut graph = Graph::open(&db_path).unwrap();

        let mut hashes = HashMap::new();
        hashes.insert(
            "src/clean.rs".to_string(),
            FileHashRow {
                hash: "deadbeef".to_string(),
                skipped_ranges: Vec::new(),
            },
        );

        graph.update_file_hashes(&hashes).unwrap();

        let json: String = graph
            .conn
            .query_row(
                "SELECT skipped_ranges FROM file_hashes WHERE file_path = ?1",
                ["src/clean.rs"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            json, "[]",
            "clean parse must serialise to literal '[]' (matches column default)"
        );
    }
}
