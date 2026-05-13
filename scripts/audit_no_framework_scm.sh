#!/usr/bin/env bash
# CI gate: No `.scm` per framework (R5).
#
# Rule (CI-GATES.md):
#   `queries/<lang>/frameworks/` directories must not exist anywhere in
#   the scope sub-crate tree. Framework predicates live in Rust code at
#   `scope-core/src/frameworks/<name>/` and consume graph rows; they
#   never compile to tree-sitter queries.
#
# Per ENFORCEMENT-MAP.md § R5 → "Why not `.scm` per framework
# (variant C)", and CHARTER.md § E2 (frameworks operate on graph rows,
# not AST).
#
# Mechanism: `find` for any path component named `frameworks` directly
# under any directory named `queries`. Both per-language `queries/`
# trees in `scope-core/src/languages/` and any future top-level
# `queries/` directories are scanned.
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SCAN_ROOT="$ROOT/gumiho-mudang-scope"

if [[ ! -d "$SCAN_ROOT" ]]; then
    echo "audit_no_framework_scm: scan root not found: $SCAN_ROOT" >&2
    exit 1
fi

# Find any `queries/*/frameworks` or `queries/frameworks` directory.
# `-type d` so files coincidentally named `frameworks` do not trip.
hits=$(find "$SCAN_ROOT" -type d -path '*/queries/*/frameworks' -o -type d -path '*/queries/frameworks' 2>/dev/null || true)

if [[ -z "$hits" ]]; then
    echo "no-framework-scm: OK (no queries/<lang>/frameworks/ directories)"
    exit 0
fi

echo "CI gate FAILED: No `.scm` per framework (R5)" >&2
echo "" >&2
echo "Found queries/*/frameworks/ directories:" >&2
echo "" >&2
printf '%s\n' "$hits" >&2
echo "" >&2
echo "Per ENFORCEMENT-MAP.md § R5: framework predicates live in" >&2
echo "Rust at scope-core/src/frameworks/<name>/ and consume graph rows" >&2
echo "(symbols + edges + metadata). They never compile to" >&2
echo "tree-sitter queries because that would violate E2 (frameworks" >&2
echo "operating on AST) and force O(framework × language) .scm files." >&2
exit 1
