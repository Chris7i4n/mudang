-- Scope graph schema
-- Single source of truth for the SQLite schema; loaded via include_str! in src/graph.rs.
--
-- R0 (sprint 0001) landed: surrogate edge_id PK, confidence/status/producer/pattern_id
-- (+ optional capture_id/framework/args_text), 38-kind edge whitelist (8 universal + 30
-- domain), 13-kind symbol whitelist, file_hashes.skipped_ranges. No in-place migration:
-- pre-1.0 single-user wipe policy (rm -rf .scope/ && scope index).

-- symbols: every named code construct.
-- kind whitelist = 13 (10 legacy + macro + module + trait); see
-- ARCHITECTURAL-REFACTOR.md § R0 → Symbol kind whitelist additions.
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
