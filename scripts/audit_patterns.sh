#!/usr/bin/env bash
# CI gate: Pattern catalog audit (R5).
#
# Rule (CI-GATES.md):
#   Every `Pattern { ... }` struct literal under
#   `scope-core/src/frameworks/` and `scope-core/tests/synthetic_framework/`
#   must have a non-empty `id`, an `available_in`, and a `predicate`
#   slot. Empty `id` is the bypass reviewers flagged;
#   missing `available_in` defaults to a build-time error (the field is
#   non-Option), so the gate only checks for empty literals.
#
# Mechanism: source-text grep + awk that pulls each `Pattern { ... }`
# block out of the file and inspects its `id:` slot. A pattern whose
# `id:` value is the empty-string literal `""` fails; a pattern with no
# `id:` field at all fails. Heuristic; the real safety net is the test
# at `framework_plugin_integration::synthetic_pattern_catalog_shape_is_audit_compatible`
# and any future framework-doc walkthrough.
#
# Path filter:
#   - gumiho-mudang-scope/scope-core/src/frameworks/
#   - gumiho-mudang-scope/scope-core/tests/synthetic_framework/
#   (Adoption of a real framework adds its `patterns.rs` under
#   `src/frameworks/<name>/`, automatically in scope.)
#
# Per ENFORCEMENT-MAP.md § R5 → Pattern catalog organization.
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SCAN_PATHS=(
    "$ROOT/gumiho-mudang-scope/scope-core/src/frameworks"
    "$ROOT/gumiho-mudang-scope/scope-core/tests/synthetic_framework"
)

# It is legitimate for one scan path to not exist (e.g., synthetic dir
# absent in a future restructure). Only abort if NO path exists.
existing=()
for p in "${SCAN_PATHS[@]}"; do
    [[ -d "$p" ]] && existing+=("$p")
done

if (( ${#existing[@]} == 0 )); then
    echo "audit_patterns: no scan paths exist; nothing to audit" >&2
    exit 0
fi

# Discover .rs files.
rs_files=()
for p in "${existing[@]}"; do
    while IFS= read -r f; do
        rs_files+=("$f")
    done < <(find "$p" -type f -name '*.rs')
done

failures=""

# Pull each `Pattern { ... }` block (single-line or multi-line) and
# inspect its `id:` slot. Awk state machine: opens on `Pattern {`,
# accumulates until brace depth returns to zero, then emits the block
# and resets.
extract_patterns() {
    local f="$1"
    awk -v f="$f" '
        function count(s, ch,   n) { n = gsub(ch, ch, s); return n }
        BEGIN { ln = 0; depth = 0; buf = ""; bln = 0 }
        {
            ln++
            if (depth > 0) {
                buf = buf " " $0
                depth += count($0, "{") - count($0, "}")
                if (depth <= 0) {
                    print f ":" bln ":" buf
                    buf = ""; depth = 0
                }
                next
            }
            if ($0 ~ /Pattern[[:space:]]*\{/) {
                d = count($0, "{") - count($0, "}")
                if (d > 0) {
                    buf = $0
                    bln = ln
                    depth = d
                } else if (d == 0) {
                    # Single-line `Pattern { ... }`.
                    print f ":" ln ":" $0
                }
            }
        }
        END {
            if (depth > 0) { print f ":" bln ":" buf }
        }
    ' "$f"
}

# Patterns whose `id:` is an empty literal are bypass-grade. Patterns
# without any `id:` field at all fail to compile (struct literal must
# include all fields), so we do not separately re-check that.
for f in "${rs_files[@]}"; do
    while IFS= read -r block; do
        # block is "file:lineno:text"
        text="${block#*:}"
        text="${text#*:}"
        # Match `id : ""` or `id: ""` after `Pattern {` — strip everything
        # before `Pattern` to keep the inspection bounded to the literal.
        literal="${text#*Pattern}"
        if [[ "$literal" =~ id[[:space:]]*:[[:space:]]*\"\" ]]; then
            failures+="$block"$'\n'
        fi
    done < <(extract_patterns "$f")
done

if [[ -z "${failures// /}" ]]; then
    echo "patterns: OK (every Pattern literal has a non-empty id)"
    exit 0
fi

echo "CI gate FAILED: Pattern catalog audit (R5)" >&2
echo "" >&2
echo "Pattern { ... } literals with empty id:" >&2
echo "" >&2
printf '%s' "$failures" >&2
echo "" >&2
echo "Per ENFORCEMENT-MAP.md § R5 → Pattern catalog organization:" >&2
echo "every Pattern in ALL_PATTERNS must have a non-empty id (used in" >&2
echo 'edges.pattern_id and the R8 confidence audit). Rename "" to a' >&2
echo "stable identifier in the form <framework>.<descriptor>." >&2
exit 1
