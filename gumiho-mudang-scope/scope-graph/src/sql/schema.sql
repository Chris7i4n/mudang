-- Scope graph schema
-- Single source of truth for the SQLite schema; loaded via include_str! in src/graph.rs.
--
-- R0 closure: surrogate edge_id PK, confidence/status/producer/pattern_id
-- (+ optional capture_id/framework/args_text), 38-kind edge whitelist (8 universal + 30
-- domain), 13-kind symbol whitelist, file_hashes.skipped_ranges. Migration is
-- wipe-and-reindex per CHARTER §2 (rm -rf .scope/ && mudang index).

-- symbols: every named code construct.
-- kind whitelist = 13; see ENFORCEMENT-MAP.md § R0 (Symbol kind whitelist).
CREATE TABLE IF NOT EXISTS symbols (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK(kind IN (
                    'function','class','method','interface',
                    'struct','enum','const','type','property','variant',
                    'macro','module','trait'
                )),
    file_path   TEXT NOT NULL,
    line_start  INTEGER NOT NULL,
    line_end    INTEGER NOT NULL,
    signature   TEXT,
    docstring   TEXT,
    parent_id   TEXT REFERENCES symbols(id) ON DELETE CASCADE,
    language    TEXT NOT NULL,
    metadata    TEXT NOT NULL DEFAULT '{}'
);

-- edges: relationships between symbols.
-- from_id and to_id are intentionally NOT foreign keys — edges may reference
-- synthetic IDs (e.g. __module__), external library symbols, or cross-file
-- symbols that are indexed separately. Deletion is handled in insert_file_data
-- by deleting all edges WHERE file_path = ? before re-inserting.
--
-- edge_id is a surrogate primary key (R0): the old composite PK
-- (from_id, to_id, kind) collapsed multiple call sites between the same pair
-- into one row. The surrogate PK preserves multiplicity (one row per
-- candidate target on resolution status='ambiguous'; one row per call site).
-- A non-unique covering index on (from_id, to_id, kind) keeps the previous
-- query patterns fast.
--
-- kind whitelist = 38 (8 universal + 30 domain). See
-- ARCHITECTURAL-REFACTOR.md § R0 → Edge kind whitelist additions for the
-- full list and tier rationale.
--
-- confidence: pattern-precision tier assigned by the extractor.
-- status: lookup-outcome tag assigned by the resolver (R3) /
--         Phase A trivial stub (R1, retired by R3).
-- producer: identifier of the producing plugin or layer.
-- pattern_id: short slug naming the pattern that produced the edge.
-- capture_id: tree-sitter capture name when applicable.
-- framework: populated only for framework-derived edges.
-- args_text: call-site / declaration-site argument literal as written in
--            source, capped at 2 KB. Mitigation 1: NULL when target is a
--            fully-qualified import (resolver decision). Mitigation 2:
--            truncation marker `[truncated]` appended when literal exceeds
--            the 2 KB cap. See ARCHITECTURAL-REFACTOR.md § R0 →
--            edges.args_text mitigations.
CREATE TABLE IF NOT EXISTS edges (
    edge_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id     TEXT NOT NULL,
    to_id       TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK(kind IN (
                    -- Universal (8)
                    'calls','imports','extends','implements',
                    'instantiates','references','references_type','contains',
                    -- R0 baseline domain (13)
                    'http_route','queue_handler','orm_relation',
                    'green_thread_spawn','renders','hook_use',
                    'inherits_from','migration','cron','feature_flag',
                    'awaits_on','channel_send','channel_recv',
                    -- Tier 1 domain (5)
                    'middleware','validates_with','error_handler',
                    'websocket_handler','client_route',
                    -- Tier 2 domain (5)
                    'auth_guard','cache_binding','runtime_task_spawn',
                    'route_mount','store_select',
                    -- Tier 3 domain (7)
                    'sse_stream','signal_handler','cancel_token',
                    'lazy_load','query_binding','os_process_spawn',
                    'os_thread_spawn'
                )),
    confidence  TEXT NOT NULL CHECK(confidence IN ('high','medium','low')),
    status      TEXT NOT NULL CHECK(status IN ('resolved','ambiguous','dangling')),
    producer    TEXT NOT NULL,
    pattern_id  TEXT NOT NULL,
    capture_id  TEXT,
    framework   TEXT,
    args_text   TEXT,
    file_path   TEXT NOT NULL,
    line        INTEGER
);

-- file_hashes: tracks which files are indexed and whether they have changed.
-- skipped_ranges (R0): JSON array `[{start_line, end_line, reason}]`,
-- populated when tree-sitter recovery skipped a region (R6) or when a
-- plugin deliberately skipped a sub-tree (R2). Defaults to '[]'.
CREATE TABLE IF NOT EXISTS file_hashes (
    file_path       TEXT PRIMARY KEY,
    hash            TEXT NOT NULL,
    indexed_at      INTEGER NOT NULL,
    skipped_ranges  TEXT NOT NULL DEFAULT '[]'
);

-- edge_audit_history: append-only audit-derived namespace introduced by
-- sprint 0004 (BACKLOG.md § Priority 1 sub-item (j); also see
-- docs/AUDIT-LABEL-SCHEMA.md § Auditor immutability rule § Writable
-- namespace for audit-derived rows).
--
-- Append-only by writer contract; the `--label` flow only INSERTs
-- (never UPDATE / DELETE), and the sibling auditor-immutability rule
-- forbids `--label` from touching source-derived tables (`edges`,
-- `symbols`, `file_hashes`). The CI gate
-- `edge_audit_history-source-immutability` (sprint 0004 CP6) is the
-- mechanical enforcement; this comment is the structural one.
--
-- audit_id groups all rows from one `--label` invocation. Generated at
-- write time by Graph::append_audit_history as
-- `COALESCE(MAX(audit_id), 0) + 1` inside the writing transaction.
-- Single-operator posture (CHARTER.md § 3 invariant 1) — no concurrent
-- writers, so the read-then-insert is race-free.
--
-- edge_id intentionally has no FK to edges(edge_id). History outlives
-- source: a `scope index` between audits may delete an edge (edge_id
-- is INTEGER PRIMARY KEY AUTOINCREMENT and is not stable across
-- wipe-and-reindex), but the historical verdicts against it remain.
--
-- pattern_id is denormalised onto the audit row so the read-side
-- pattern drill (`scope audit history pattern <id>`) is robust under
-- re-index. `edges.pattern_id` is the source-derived value at audit
-- time; copying it here lets `audit_history_pattern` query directly,
-- without a JOIN to the mutable `edges` table that may have been
-- wiped between audits (CP6.5 — addresses codex review on sprint 0004
-- regarding "history outlives source"). For the `currently_incorrect`
-- drivers query the JOIN is preserved because it is scoped to
-- MAX(audit_id) where the edges still exist.
--
-- label is the verdict as stored: `correct` (label=true), `incorrect`
-- (label=false), `skipped` (label=null). Storing the trichotomy
-- explicitly lets future flapping / disagreement queries match on the
-- string without re-deriving from a nullable boolean.
--
-- evidence_json is the SampleRecord.evidence field as TEXT JSON (the
-- column type is TEXT; readers parse it with json_extract or
-- serde_json). NULL when the labeller supplied no evidence.
--
-- No primary key: the table is the audit log, duplicates are
-- structurally permitted (a labeller can be re-run by mistake; the
-- history shows that as two rows, not a silent overwrite). The two
-- BACKLOG-mandated indices cover the CP5 query patterns.
CREATE TABLE IF NOT EXISTS edge_audit_history (
    audit_id            INTEGER NOT NULL,
    edge_id             INTEGER NOT NULL,
    pattern_id          TEXT NOT NULL,
    labelled_at         INTEGER NOT NULL,
    labeller_id         TEXT,
    label               TEXT NOT NULL CHECK(label IN ('correct','incorrect','skipped')),
    target_proposed     TEXT,
    kind_proposed       TEXT,
    confidence_proposed TEXT,
    evidence_json       TEXT
);

-- FTS5 virtual table for semantic-like search (`scope find`).
-- Stores a rich text representation of each symbol for full-text search.
-- The content is kept in sync with the symbols table via the searcher module.
-- Using content="" (contentless) to avoid data duplication — the actual
-- symbol data lives in the symbols table. We use rowid mapping via symbol_id.
CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
    symbol_id UNINDEXED,
    name,
    kind UNINDEXED,
    file_path UNINDEXED,
    body,
    tokenize = 'porter unicode61'
);

-- Covering indices for common query patterns.
CREATE INDEX IF NOT EXISTS idx_symbols_name        ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_file        ON symbols(file_path);
CREATE INDEX IF NOT EXISTS idx_symbols_kind        ON symbols(kind);
CREATE INDEX IF NOT EXISTS idx_symbols_parent      ON symbols(parent_id);
-- Covering index replaces the old composite PK on edges.
CREATE INDEX IF NOT EXISTS idx_edges_triple        ON edges(from_id, to_id, kind);
CREATE INDEX IF NOT EXISTS idx_edges_from          ON edges(from_id, kind);
CREATE INDEX IF NOT EXISTS idx_edges_to            ON edges(to_id, kind);
CREATE INDEX IF NOT EXISTS idx_edges_file          ON edges(file_path);
-- R0: audit-targeted indices for R8 (Phase D) precision queries.
CREATE INDEX IF NOT EXISTS idx_edges_confidence    ON edges(confidence);
CREATE INDEX IF NOT EXISTS idx_edges_status        ON edges(status);
CREATE INDEX IF NOT EXISTS idx_edges_producer      ON edges(producer);
CREATE INDEX IF NOT EXISTS idx_edges_pattern       ON edges(pattern_id);
-- Sprint 0004 (BACKLOG.md § Priority 1 sub-item (j)) — audit-history
-- lookup indices. (edge_id, audit_id) covers the `scope audit history
-- edge <edge_id>` drill; (labeller_id, audit_id) covers the deferred
-- sprint 0006 `scope audit history labeller <id>` drill.
CREATE INDEX IF NOT EXISTS idx_edge_audit_history_edge_audit      ON edge_audit_history(edge_id, audit_id);
CREATE INDEX IF NOT EXISTS idx_edge_audit_history_labeller_audit  ON edge_audit_history(labeller_id, audit_id);
-- Pattern drill `scope audit history pattern <id>` is the canonical
-- consumer of this index: scopes the audit-history → pattern-precision
-- timeline lookup without the JOIN to `edges` that was the codex-review
-- finding for sprint 0004 (CP6.5).
CREATE INDEX IF NOT EXISTS idx_edge_audit_history_pattern_audit   ON edge_audit_history(pattern_id, audit_id);
