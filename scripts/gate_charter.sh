#!/usr/bin/env bash
# Charter sweep gate — sprint 0009 (Phase E close).
#
# Refuses any re-introduction of the compatibility shim shapes that
# sprint 0009 chunks 1 and 2 retired. The chunk 2 grep pass was the
# manual baseline; this script is the durable mechanical successor.
#
# Rule (CHARTER.md § 2 + § 3 inv 8):
#   No third-party operator of scope. The repository owner deletes
#   `.scope/` / `.mudang/` directly. No backward-compatibility shim,
#   dual-read code path, or stored-format version detector is
#   permitted to survive a commit that lands on `main`.
#
# Reference: the architecture forbids compat shims (`CHARTER.md` § 2
# + § 3 inv 8). Any new compat shim requires charter amendment + an
# `ENFORCEMENT-MAP.md` R-entry update before landing — not a silent
# addition.
#
# Each check below targets a *specific shim shape* that sprint 0009
# retired. The patterns are narrow on purpose: catching loose
# substrings like `legacy` or `compat` would fire on charter-aligned
# prose (migration notes, language-feature documentation,
# directional call-graph terms). The chunk 2 grep pass already
# disposed those; this gate guards against regressions of the active
# shim shapes specifically.
#
# Exits non-zero on any match. Output identifies the failing check
# and the retiring commit so the operator can see why the shim shape
# was removed in the first place. The shim shapes below are forbidden
# outright; re-introducing one requires charter amendment + an
# `ENFORCEMENT-MAP.md` update, not a silent shim addition.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SCOPE_PATHS=(gumiho-mudang-scope gumiho-mudang-cli gumiho-mudang-lsp)
EXCLUDES=(--exclude-dir=target --exclude-dir=.scope --exclude-dir=tests --exclude-dir=node_modules --exclude-dir=fixtures)

FAILED=0

# Print a failure block. Reports the violating pattern + which shim
# the chunk 1 / chunk 2 retirement targeted, then appends the hit
# list verbatim so the operator can navigate to the offending line.
fail_block() {
    local pattern_name="$1"
    local retired_in="$2"
    local hits="$3"
    echo "✗ charter-sweep regression: $pattern_name" >&2
    echo "  retired in: $retired_in" >&2
    echo "  hits:" >&2
    while IFS= read -r line; do
        echo "    $line" >&2
    done <<< "$hits"
    echo >&2
    FAILED=1
}

# Check 1 — `gumiho_mudang_scope::core::` import path.
#
# This was the namespace-synth shim (`pub mod core { … }` in
# `gumiho-mudang-scope/src/lib.rs`) that re-exported every sub-crate
# under a single pre-split prefix. Retired in sprint 0009 chunk 2
# (commit e4bc323) — every consumer rewritten to the flat namespace.
hits=$(grep -RnE 'gumiho_mudang_scope::core::' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "namespace-synth import path \`gumiho_mudang_scope::core::*\`" \
               "sprint 0009 chunk 2 (commit e4bc323) — drop the \`::core::\` segment, use the flat namespace" \
               "$hits"
fi

# Check 2 — `pub mod core { … }` re-namespace declaration.
#
# Same shim as check 1, surfaced from the declaration side. The
# `pub mod core` block that synthesised the pre-split namespace was
# retired in chunk 2 (commit e4bc323).
hits=$(grep -RnE '^pub mod core[[:space:]]*\{' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "re-namespace shim \`pub mod core { … }\`" \
               "sprint 0009 chunk 2 (commit e4bc323)" \
               "$hits"
fi

# Check 3 — `pub use scope_X as scope_X_crate` dead façade aliases.
#
# Forward-looking façade aliases with no consumers — retired in
# sprint 0009 chunk 2 (commit 0924ac7).
hits=$(grep -RnE '^pub use scope_(core|graph|index|search|workspace) as scope_(core|graph|index|search|workspace)_crate' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "dead façade aliases \`pub use scope_X as scope_X_crate\`" \
               "sprint 0009 chunk 2 (commit 0924ac7)" \
               "$hits"
fi

# Check 4 — Transitional `Edge` alias re-introduction.
#
# Retired in sprint 0009 chunk 1 row 3 (commit 9184302). The R1
# split is complete; production code uses `RawEdge` (extractor
# output) or `InsertableEdge` (resolver output) directly.
#
# Catches both alias shapes:
#   - `pub type Edge = RawEdge;`
#   - `pub use scope_core::RawEdge as Edge;`
# `\bEdge\b` is the precise token boundary — `RawEdge`, `EdgeKind`,
# `EdgeBuilder`, `EdgeSummary`, `EdgeId`, and `InsertableEdge` all
# have a word-char immediately abutting the four letters, so the
# boundary anchor excludes them automatically.
hits=$(grep -RnE '^pub (type Edge\b|use [^;]*\bas Edge\b)' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "transitional \`Edge\` alias (\`pub type Edge = …\` or \`pub use … as Edge\`)" \
               "sprint 0009 chunk 1 row 3 (commit 9184302)" \
               "$hits"
fi

# Check 5 — Importing the retired `Edge` token in any `use` line.
#
# Companion to check 4: the alias is gone, so any import that names
# `Edge` directly is broken. Catches every import shape:
#   - direct:   `use scope_core::Edge;`
#   - qualified `use scope_core::types::Edge;` / `use crate::types::Edge;`
#   - braced:   `use scope_core::{Edge, Symbol};`
#   - braced+ws `use scope_core::{Confidence, Edge, RawEdge, ...};`
# `\bEdge\b` is the precise token boundary — see check 4 for the
# rationale that excludes `RawEdge` / `EdgeKind` / `EdgeBuilder` /
# `EdgeSummary` / `EdgeId` / `InsertableEdge` automatically. `[^;]*`
# crosses brace boundaries so braced imports surface alongside direct.
hits=$(grep -RnE '^use [^;]*\bEdge\b' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "import of retired \`Edge\` type alias" \
               "sprint 0009 chunk 1 row 3 (commit 9184302) — switch to \`RawEdge\`" \
               "$hits"
fi

# Check 6 — `INSERT OR IGNORE` in production SQL.
#
# R0 (sprint 0001) replaced the legacy `INSERT OR IGNORE` edge insert
# with a plain INSERT against the surrogate `edge_id` primary key —
# `INSERT OR IGNORE` hides shape drift by silently swallowing edges
# whose (from_id, to_id, kind) tuple already exists. The only SQL
# shape allowed in `scope-graph/src/sql/` and `INSERT … VALUES` sites
# in `scope-graph/src/` is plain INSERT.
hits=$(grep -RnE 'INSERT[[:space:]]+OR[[:space:]]+IGNORE' --include='*.rs' --include='*.sql' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(--|//|\*|//!|///)' || true)
if [[ -n "$hits" ]]; then
    fail_block "\`INSERT OR IGNORE\` in production SQL" \
               "sprint 0001 R0 — plain \`INSERT\` against surrogate PK is the only allowed shape" \
               "$hits"
fi

# Check 7 — Pre-R0 schema detector function names.
#
# `Graph::has_legacy_edges_table` + companion bail were retired in
# sprint 0009 chunk 1 row 1 (commit b41be2d). Wipe-and-reindex is
# the canonical migration path per CHARTER § 3 inv 8 — no in-place
# detector is allowed.
hits=$(grep -RnE 'fn has_legacy_|fn pre_r0_|fn pre_r[1-9]_|fn detect_pre_r' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "stored-format version-detector fn name (\`has_legacy_*\` / \`pre_r[0-9]+_*\`)" \
               "sprint 0009 chunk 1 row 1 (commit b41be2d)" \
               "$hits"
fi

# Check 8 — `command_label` deprecation-alias parameter.
#
# `run_callers_transitive` accepted `command_label: &'static str` so
# the deprecated `scope impact` command could identify itself in the
# JSON envelope as `command: "impact"`. Retired in sprint 0009 chunk
# 1 rows 5+6 (commit 0374a24) — the JSON envelope hard-codes
# `command: "callers"` now.
hits=$(grep -RnE 'command_label[[:space:]]*:[[:space:]]*&.static[[:space:]]+str' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "\`command_label: &'static str\` deprecation-alias parameter" \
               "sprint 0009 chunk 1 rows 5+6 (commit 0374a24)" \
               "$hits"
fi

# Check 9 — `scope impact` CLI subcommand wiring.
#
# Deleted entirely in sprint 0009 chunk 1 row 5 (commit 0374a24).
# Re-introducing the variant or the module would re-introduce the
# user-facing deprecation shim that CHARTER § 2 forbids.
hits=$(grep -RnE 'pub mod impact[[:space:]]*;|Impact\(commands::impact::ImpactArgs\)|commands::impact::run' --include='*.rs' "${EXCLUDES[@]}" gumiho-mudang-cli/src 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "\`scope impact\` CLI subcommand wiring" \
               "sprint 0009 chunk 1 row 5 (commit 0374a24) — \`scope callers <symbol> --depth N\` is the canonical form" \
               "$hits"
fi

# Check 10 — `__module__` synthetic-ID fallback pattern in graph
# query helpers.
#
# `find_deps` / `get_class_relationships` no longer unconditionally
# tack on `<file>::__module__::class` or `<file>::__module__::function`
# to source-ID lists. `find_deps` threads it through as an
# `imports_only_id` filter; `get_class_relationships` doesn't surface
# module-level edges at all (class extends/implements are never
# module-attributed). Retired in sprint 0009 chunk 1 row 2
# (commit d8d4574).
#
# The pattern below catches the typical `format!("{}/__module__::class"…)`
# string-template that constructs the fallback. The `find_file_deps`
# call site is allowlisted because file deps legitimately surface
# module-level edges (imports + top-level expressions).
hits=$(grep -RnE 'format!\("[^"]*__module__::class' --include='*.rs' "${EXCLUDES[@]}" gumiho-mudang-scope/scope-graph 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "\`__module__::class\` synthetic-ID fallback in graph query layer" \
               "sprint 0009 chunk 1 row 2 (commit d8d4574)" \
               "$hits"
fi

if [[ "$FAILED" -eq 1 ]]; then
    echo >&2
    echo "Charter sweep gate FAILED." >&2
    echo >&2
    echo "Every active shim must either be reverted in this commit or" >&2
    echo "escalated via sprints/README.md § 3 ambiguity protocol so the" >&2
    echo "charter / refactor-closure doc is amended first. Compat shims" >&2
    echo "are charter-violating at close (CHARTER.md § 3 invariant 8);" >&2
    echo "re-introducing one requires an amendment, not a silent addition." >&2
    exit 1
fi

echo "gate-charter: OK (no charter-violating shim shapes detected)"
