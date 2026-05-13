#!/usr/bin/env bash
# CI gate: Edge sealed (R1).
#
# Rule (CI-GATES.md):
#   "`Edge {` literal outside core::graph compiles" fails the gate.
#
# Post-sprint-0000 layout: the edge types (`RawEdge`, `InsertableEdge`,
# and the historical `Edge = RawEdge` alias) live in scope-core. Their
# fields are `pub(crate)`, so external struct-literal construction is
# already a compile error; this script is the belt-and-suspenders grep
# defence that catches accidental within-scope-core construction outside
# the owning modules (`scope-core/src/edge.rs`, `scope-core/src/types.rs`)
# and refuses test fixtures that paper over the seal.
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Allowed construction sites (owning modules — exact relative paths,
# not basename excludes, so accidental `types.rs` elsewhere in the
# workspace is not silently allowlisted).
#
# `scope-graph/src/resolve/mod.rs` owns `InsertableEdge` (R3, sprint
# 0003 chunk 6 migration); the compile-fail fixture
# `insertable_fields_private.rs` deliberately exercises the
# struct-literal-construction failure mode and is allowlisted because
# it never compiles — it is the gate itself.
ALLOW_PATHS=(
    "gumiho-mudang-scope/scope-core/src/edge.rs"
    "gumiho-mudang-scope/scope-core/src/types.rs"
    "gumiho-mudang-scope/scope-graph/src/resolve/mod.rs"
    "gumiho-mudang-scope/scope-graph/tests/compile_fail/typestate/insertable_fields_private.rs"
)

# Patterns that indicate struct-literal construction of the sealed types.
# Match identifier boundary so we don't catch `EdgeKind {` or comments.
PATTERN='\b(Edge|RawEdge|InsertableEdge)[[:space:]]*\{'

cd "$ROOT"

# Build an awk filter that strips lines whose file path matches one
# of the exact allowlisted paths (grep -n emits `path:lineno:content`,
# split on the first colon).
ALLOW_AWK=$(printf '%s\n' "${ALLOW_PATHS[@]}" | awk 'BEGIN{ORS="|"}{print}' | sed 's/|$//')

hits=$(grep -RnE "$PATTERN" \
    --include='*.rs' \
    gumiho-mudang-scope/ gumiho-mudang-cli/ gumiho-mudang-lsp/ 2>/dev/null \
    | awk -F: -v allow="$ALLOW_AWK" '
        BEGIN {
            n = split(allow, a, "|")
            for (i = 1; i <= n; i++) blocked[a[i]] = 1
        }
        { if (!($1 in blocked)) print }
      ' \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
    | grep -vE -- '->[[:space:]]*(Vec<)?(&\[)?(Edge|RawEdge|InsertableEdge)' \
    | grep -vE -- '(impl|for|trait|where|<|where[[:space:]]+[A-Za-z_]+:)[[:space:]]+(Edge|RawEdge|InsertableEdge)' \
    | grep -vE -- '\b(&|&mut)[[:space:]]*(Edge|RawEdge|InsertableEdge)' \
    || true)

if [[ -n "$hits" ]]; then
    echo "CI gate FAILED: Edge sealed (R1)" >&2
    echo "" >&2
    echo "Found struct-literal construction of Edge / RawEdge / InsertableEdge" >&2
    echo "outside the owning modules (${ALLOW_PATHS[*]}):" >&2
    echo "" >&2
    echo "$hits" >&2
    echo "" >&2
    echo "Construction must go through Edge::builder() per R1." >&2
    echo "See ENFORCEMENT-MAP.md § R1 — typed edge insertion API." >&2
    exit 1
fi

echo "edge-sealed: OK (no struct-literal construction outside ${ALLOW_PATHS[*]})"
