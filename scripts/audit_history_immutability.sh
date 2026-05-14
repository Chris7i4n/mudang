#!/usr/bin/env bash
# CI gate: edge_audit_history-source-immutability.
#
# Sprint 0004 (BACKLOG.md § Priority 1 sub-item (j)) carved a writable
# namespace for audit-derived rows (`edge_audit_history`) out of the
# auditor-immutability rule. Source-derived tables (`edges`, `symbols`,
# `file_hashes`) stay frozen during audit. This gate is the mechanical
# enforcement: the `--label` write path must touch `edge_audit_history`
# only.
#
# Two narrow-grep checks:
#
#   1. `gumiho-mudang-cli/src/commands/audit.rs` — the CLI surface that
#      orchestrates `scope audit confidence --label`. The file holds zero
#      SQL today (it delegates every mutation to `Graph` methods).
#      Re-introducing a SQL-write string literal targeting `edges`,
#      `symbols`, or `file_hashes` fails the gate.
#
#   2. `Graph::append_audit_history` body in
#      `gumiho-mudang-scope/scope-graph/src/graph.rs` — the sole mutator
#      reachable from the `--label` flow. The function body is extracted
#      and scanned; any SQL string targeting a source-derived table
#      fails the gate.
#
# Per ENFORCEMENT-MAP.md § R8 (auditor surface) and § R0 (schema
# closure). Recovery: revert the source-derived mutation; if the
# mutation is genuinely required, escalate via the ambiguity protocol
# (`sprints/README.md` § 3) so the writable-namespace carveout is
# amended on `main` first.
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

AUDIT_CMD="$ROOT/gumiho-mudang-cli/src/commands/audit.rs"
GRAPH_SRC="$ROOT/gumiho-mudang-scope/scope-graph/src/graph.rs"

for p in "$AUDIT_CMD" "$GRAPH_SRC"; do
    if [[ ! -e "$p" ]]; then
        echo "audit_history_immutability: scan path not found: $p" >&2
        exit 1
    fi
done

FAILED=0

# Forbidden: SQL-write keyword + source-derived table name on the same line.
#
# Word-boundary anchors on the table names exclude `edge_audit_history`,
# `edges_view`, `file_hashes_backup`, etc. The SQL keyword arm matches
# `INSERT INTO`, `INSERT OR REPLACE INTO`, `INSERT OR IGNORE INTO`,
# `UPDATE`, `DELETE FROM`, and `REPLACE INTO` shapes — every standard
# rusqlite mutator wording.
FORBIDDEN_SQL='(INSERT[[:space:]]+(OR[[:space:]]+(REPLACE|IGNORE)[[:space:]]+)?INTO|UPDATE|DELETE[[:space:]]+FROM|REPLACE[[:space:]]+INTO)[[:space:]]+\b(edges|symbols|file_hashes)\b'

# Check 1 — audit.rs.
hits=$(grep -nE "$FORBIDDEN_SQL" "$AUDIT_CMD" 2>/dev/null \
    | grep -vE '^[0-9]+:[[:space:]]*(//|///|//!|\*)' \
    || true)
if [[ -n "$hits" ]]; then
    echo "✗ edge_audit_history-source-immutability: source-derived SQL mutation in audit.rs" >&2
    echo "  rationale: the --label CLI surface must delegate every write to" >&2
    echo "    Graph::append_audit_history; source-derived tables (edges, symbols," >&2
    echo "    file_hashes) stay frozen during audit (sprint 0004 (j) writable-namespace" >&2
    echo "    carveout)." >&2
    echo "  hits:" >&2
    while IFS= read -r line; do
        echo "    gumiho-mudang-cli/src/commands/audit.rs:$line" >&2
    done <<< "$hits"
    echo >&2
    FAILED=1
fi

# Check 2 — Graph::append_audit_history body.
#
# Extract the function body between `pub fn append_audit_history(` and
# the matching closing brace via awk brace-balance tracker. The body
# starts at the line following the signature's opening `{`; the body
# ends at the matching `}` at brace depth 0.
body=$(awk '
    /pub fn append_audit_history\(/ { in_fn = 1 }
    in_fn {
        n_open = gsub(/\{/, "&")
        n_close = gsub(/\}/, "&")
        depth += n_open - n_close
        if (started == 0 && depth > 0) { started = 1; next }
        if (started == 1) {
            if (depth == 0) { exit }
            print NR ":" $0
        }
    }
' "$GRAPH_SRC")

if [[ -z "$body" ]]; then
    echo "✗ edge_audit_history-source-immutability: Graph::append_audit_history not found" >&2
    echo "  rationale: the gate scans the body of append_audit_history for source-derived" >&2
    echo "    table mutations. The function is the sole DB write reachable from the" >&2
    echo "    --label flow; removing or renaming it breaks the writable-namespace" >&2
    echo "    contract (sprint 0004 (j))." >&2
    echo >&2
    FAILED=1
else
    hits=$(echo "$body" | grep -nE "$FORBIDDEN_SQL" 2>/dev/null || true)
    if [[ -n "$hits" ]]; then
        echo "✗ edge_audit_history-source-immutability: source-derived SQL mutation in append_audit_history body" >&2
        echo "  rationale: append_audit_history is the writable-namespace contract — it" >&2
        echo "    writes to edge_audit_history exclusively. A mutation against edges /" >&2
        echo "    symbols / file_hashes inside this function breaks the carveout (sprint" >&2
        echo "    0004 (j) / AUDIT-LABEL-SCHEMA.md § Writable namespace for audit-derived" >&2
        echo "    rows)." >&2
        echo "  hits (line in extracted body):" >&2
        while IFS= read -r line; do
            echo "    gumiho-mudang-scope/scope-graph/src/graph.rs:$line" >&2
        done <<< "$hits"
        echo >&2
        FAILED=1
    fi
fi

if [[ "$FAILED" -eq 1 ]]; then
    echo "CI gate FAILED: edge_audit_history-source-immutability" >&2
    echo "" >&2
    echo "The --label write path must touch edge_audit_history only." >&2
    echo "Source-derived tables (edges, symbols, file_hashes) are frozen" >&2
    echo "during audit per AUDIT-LABEL-SCHEMA.md § Auditor immutability rule" >&2
    echo "and § Writable namespace for audit-derived rows. To amend the" >&2
    echo "carveout, follow the ambiguity protocol on main first" >&2
    echo "(sprints/README.md § 3) — never silently widen the writable" >&2
    echo "namespace." >&2
    exit 1
fi

echo "audit_history_immutability: OK (--label write path touches edge_audit_history only)"
