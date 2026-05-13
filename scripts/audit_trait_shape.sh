#!/usr/bin/env bash
# CI gate: Trait-shape audit (R12) + Macro definition-only (R11 subset).
#
# Rule (CI-GATES.md):
#   No function in scope-core/src/languages/, scope-core/src/extract/,
#   or scope-core/src/frameworks/ has a name implying type-system
#   inference, evaluation, narrowing, overload resolution, or macro
#   expansion. R5 extends the gate to `frameworks/` so
#   future `FrameworkPlugin` impls cannot regress the negative trait
#   shape.
#
# Forbidden prefixes (R12 + R11):
#   infer_*           — implies type inference
#   evaluate_*        — implies expression evaluation
#   solve_*           — implies constraint solving
#   narrow_*          — implies type narrowing
#   resolve_overload_*— implies overload resolution
#   expand_*          — implies macro / template expansion (R11)
#
# Mechanism: source-text grep over the post-A.4 plugin surface — the
# `impl LanguageId` block in scope-core/src/languages/id.rs (which
# replaced the historical `trait LanguagePlugin`), the per-language
# modules under scope-core/src/languages/, and the per-language
# extractors under scope-core/src/extract/. Lines that are comments,
# doc comments, or string literals do not count (`// infer_*` in prose
# is fine).
#
# Per ENFORCEMENT-MAP.md § R11 + § R12, and
# LANGUAGE-PLAYBOOK.md A1/A2/A3/B2/C1.
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SCAN_PATHS=(
    "$ROOT/gumiho-mudang-scope/scope-core/src/languages"
    "$ROOT/gumiho-mudang-scope/scope-core/src/extract"
    "$ROOT/gumiho-mudang-scope/scope-core/src/frameworks"
)

for p in "${SCAN_PATHS[@]}"; do
    if [[ ! -d "$p" ]]; then
        echo "audit_trait_shape: scan dir not found: $p" >&2
        exit 1
    fi
done

# Match function definitions only; the `fn ` keyword anchors the pattern
# so renamed-call sites and prose mentioning the forbidden prefixes do
# not trip the gate.
PATTERN='\bfn[[:space:]]+(infer_|evaluate_|solve_|narrow_|resolve_overload_|expand_)\w+'

# Strip comment-only lines (`//` at any indent) before matching.
hits=$(grep -RnE "$PATTERN" --include='*.rs' "${SCAN_PATHS[@]}" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
    || true)

if [[ -z "$hits" ]]; then
    echo "trait-shape: OK (no infer_/evaluate_/solve_/narrow_/resolve_overload_/expand_ functions in languages/ or extract/)"
    exit 0
fi

echo "CI gate FAILED: Trait-shape audit (R12) + Macro definition-only (R11)" >&2
echo "" >&2
echo "Functions with names implying type inference / evaluation /" >&2
echo "narrowing / overload resolution / macro expansion:" >&2
echo "" >&2
printf '%s\n' "$hits" >&2
echo "" >&2
echo "Per R12: the plugin / extractor surface must not have functions" >&2
echo "whose names imply type-system work or runtime evaluation." >&2
echo "Per R11: no expand_* function may exist; macros are indexed as" >&2
echo "Symbol{kind: macro}, and invocations are 'calls.macro' edges to" >&2
echo "the macro symbol — never expanded." >&2
echo "" >&2
echo "If the function does NOT actually do inference / evaluation /" >&2
echo "expansion, rename it. There is no allowlist tag for this gate —" >&2
echo "the name itself is the contract." >&2
exit 1
