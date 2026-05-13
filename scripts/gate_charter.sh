#!/usr/bin/env bash
# Charter sweep gate.
#
# Refuses every compatibility-shim shape the charter forbids
# (`CHARTER.md` § 2 + § 3 invariant 8):
#   No third-party operator of scope. The repository owner deletes
#   `.scope/` / `.mudang/` directly. No backward-compatibility shim,
#   dual-read code path, or stored-format version detector is
#   permitted to survive a commit that lands on `main`.
#
# Any new compat shim requires charter amendment + an
# `ENFORCEMENT-MAP.md` R-entry update before landing — never a silent
# addition.
#
# Each check below targets a *specific shim shape*. The patterns are
# narrow on purpose: catching loose substrings like `legacy` or
# `compat` would fire on charter-aligned prose (migration notes,
# language-feature documentation, directional call-graph terms).
# This gate guards against regressions of the named shim shapes
# specifically.
#
# Exits non-zero on any match. Output identifies the failing check
# and the canonical replacement so the operator can see how to
# resolve the regression.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SCOPE_PATHS=(gumiho-mudang-scope gumiho-mudang-cli gumiho-mudang-lsp)
EXCLUDES=(--exclude-dir=target --exclude-dir=.scope --exclude-dir=tests --exclude-dir=node_modules --exclude-dir=fixtures)

FAILED=0

# Print a failure block. Reports the violating pattern + canonical
# replacement, then appends the hit list verbatim so the operator can
# navigate to the offending line.
fail_block() {
    local pattern_name="$1"
    local rationale="$2"
    local hits="$3"
    echo "✗ charter-sweep regression: $pattern_name" >&2
    echo "  rationale: $rationale" >&2
    echo "  hits:" >&2
    while IFS= read -r line; do
        echo "    $line" >&2
    done <<< "$hits"
    echo >&2
    FAILED=1
}

# Check 1 — `gumiho_mudang_scope::core::` import path.
#
# Namespace-synth shim (`pub mod core { … }` in
# `gumiho-mudang-scope/src/lib.rs`) that re-exported every sub-crate
# under a single pre-split prefix. The canonical namespace is flat.
hits=$(grep -RnE 'gumiho_mudang_scope::core::' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "namespace-synth import path \`gumiho_mudang_scope::core::*\`" \
               "drop the \`::core::\` segment, use the flat namespace" \
               "$hits"
fi

# Check 2 — `pub mod core { … }` re-namespace declaration.
#
# Same shim as check 1, surfaced from the declaration side. The
# `pub mod core` block that synthesised the pre-split namespace is
# forbidden.
hits=$(grep -RnE '^pub mod core[[:space:]]*\{' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "re-namespace shim \`pub mod core { … }\`" \
               "remove the synthesised namespace; sub-crates are the flat namespace" \
               "$hits"
fi

# Check 3 — `pub use scope_X as scope_X_crate` dead façade aliases.
#
# Forward-looking façade aliases with no consumers.
hits=$(grep -RnE '^pub use scope_(core|graph|index|search|workspace) as scope_(core|graph|index|search|workspace)_crate' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "dead façade aliases \`pub use scope_X as scope_X_crate\`" \
               "drop the alias; consumers depend on the canonical sub-crate name" \
               "$hits"
fi

# Check 4 — Transitional `Edge` alias re-introduction.
#
# The R1 split is complete; production code uses `RawEdge` (extractor
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
               "use \`RawEdge\` (extractor output) or \`InsertableEdge\` (resolver output) directly" \
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
    fail_block "import of the retired \`Edge\` type alias" \
               "switch to \`RawEdge\`" \
               "$hits"
fi

# Check 6 — `INSERT OR IGNORE` in production SQL.
#
# R0 mandates plain INSERT against the surrogate `edge_id` primary
# key. `INSERT OR IGNORE` hides shape drift by silently swallowing
# edges whose (from_id, to_id, kind) tuple already exists.
hits=$(grep -RnE 'INSERT[[:space:]]+OR[[:space:]]+IGNORE' --include='*.rs' --include='*.sql' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(--|//|\*|//!|///)' || true)
if [[ -n "$hits" ]]; then
    fail_block "\`INSERT OR IGNORE\` in production SQL" \
               "R0 — plain \`INSERT\` against the surrogate PK is the only allowed shape" \
               "$hits"
fi

# Check 7 — Schema-detector function names.
#
# Wipe-and-reindex is the canonical migration path per CHARTER § 3
# inv 8 — no in-place detector is allowed.
hits=$(grep -RnE 'fn has_legacy_|fn pre_r0_|fn pre_r[1-9]_|fn detect_pre_r' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "stored-format version-detector fn name (\`has_legacy_*\` / \`pre_r[0-9]+_*\`)" \
               "drop the detector; wipe-and-reindex is the canonical migration path (CHARTER § 3 inv 8)" \
               "$hits"
fi

# Check 8 — `command_label` deprecation-alias parameter.
#
# A deprecation-alias parameter that lets a function identify itself
# in the JSON envelope under a different name violates the
# single-canonical-name rule.
hits=$(grep -RnE 'command_label[[:space:]]*:[[:space:]]*&.static[[:space:]]+str' --include='*.rs' "${EXCLUDES[@]}" "${SCOPE_PATHS[@]}" 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "\`command_label: &'static str\` deprecation-alias parameter" \
               "hard-code the JSON envelope's \`command\` field; no per-caller relabeling" \
               "$hits"
fi

# Check 9 — `scope impact` CLI subcommand wiring.
#
# Re-introducing the variant or the module would re-introduce the
# user-facing deprecation shim that CHARTER § 2 forbids.
hits=$(grep -RnE 'pub mod impact[[:space:]]*;|Impact\(commands::impact::ImpactArgs\)|commands::impact::run' --include='*.rs' "${EXCLUDES[@]}" gumiho-mudang-cli/src 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "\`scope impact\` CLI subcommand wiring" \
               "\`scope callers <symbol> --depth N\` is the canonical form" \
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
# module-attributed).
#
# The pattern below catches the typical `format!("{}/__module__::class"…)`
# string-template that constructs the fallback. The `find_file_deps`
# call site is allowlisted because file deps legitimately surface
# module-level edges (imports + top-level expressions).
hits=$(grep -RnE 'format!\("[^"]*__module__::class' --include='*.rs' "${EXCLUDES[@]}" gumiho-mudang-scope/scope-graph 2>/dev/null || true)
if [[ -n "$hits" ]]; then
    fail_block "\`__module__::class\` synthetic-ID fallback in graph query layer" \
               "thread the synthetic ID through \`imports_only_id\`; do not append unconditionally" \
               "$hits"
fi

if [[ "$FAILED" -eq 1 ]]; then
    echo >&2
    echo "Charter sweep gate FAILED." >&2
    echo >&2
    echo "Every active shim must either be reverted in this commit or" >&2
    echo "escalated via sprints/README.md § 3 ambiguity protocol so the" >&2
    echo "charter / ENFORCEMENT-MAP doc is amended first. Compat shims" >&2
    echo "are charter-violating (CHARTER.md § 3 invariant 8); re-introducing" >&2
    echo "one requires an amendment, not a silent addition." >&2
    exit 1
fi

echo "gate-charter: OK (no charter-violating shim shapes detected)"
