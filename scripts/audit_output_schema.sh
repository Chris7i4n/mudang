#!/usr/bin/env bash
# CI gate: Output schema audit (R10).
#
# Rule (CI-GATES.md):
#   No `pub struct` or `struct` definition under
#   `gumiho-mudang-cli/src/output/` or `gumiho-mudang-cli/src/commands/`
#   may declare a field whose name is one of:
#     error, warning, diagnostic, is_valid, lint, correctness
#
#   These names imply diagnostic / correctness-assertion content,
#   which E1 (CHARTER.md §5 — "no semantic correctness assertions")
#   forbids. The output struct surface is the mechanical enforcement
#   point for E1.
#
# Per ARCHITECTURAL-REFACTOR.md § R10 → "Sprint 0006 scope decision".
#
# Mechanism: awk state machine that tracks `struct <Name> {` ... `}`
# blocks and emits any field declaration matching the banned set.
# Field declaration form: `[pub( ... )?] <field_name> : <type>` at the
# start of a line (after stripping leading whitespace).
#
# Path filter:
#   - gumiho-mudang-cli/src/output/
#   - gumiho-mudang-cli/src/commands/
#   (Shared output crate, if extracted post-refactor, is added here.)
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SCAN_PATHS=(
    "$ROOT/gumiho-mudang-cli/src/output"
    "$ROOT/gumiho-mudang-cli/src/commands"
)

for p in "${SCAN_PATHS[@]}"; do
    if [[ ! -d "$p" ]]; then
        echo "audit_output_schema: scan dir not found: $p" >&2
        exit 1
    fi
done

# Discover .rs files.
rs_files=()
for p in "${SCAN_PATHS[@]}"; do
    while IFS= read -r f; do
        rs_files+=("$f")
    done < <(find "$p" -type f -name '*.rs')
done

# Awk: tracks struct-block depth; emits any field whose name is in
# the banned set, with file:line for diagnostics.
#
# State:
#   inside_struct = 1 when between `struct X {` and its matching `}`.
#   depth         = brace depth within the struct block.
#
# Field-line shape: leading whitespace, optional `pub` (possibly
# `pub(crate)`/`pub(super)`/`pub(in path)`), then `<identifier> :`.
# We do not parse the type — the field name is what we audit.
banned_re='^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?(error|warning|diagnostic|is_valid|lint|correctness)[[:space:]]*:'

failures=""

for f in "${rs_files[@]}"; do
    while IFS= read -r hit; do
        failures+="$f:$hit"$'\n'
    done < <(awk -v banned_re="$banned_re" '
        function count(s, ch,   n) { n = gsub(ch, ch, s); return n }
        BEGIN { inside = 0; depth = 0 }
        {
            ln = NR
            line = $0

            # Strip line and block comments inside this scan to avoid
            # tripping on commented-out struct fields.
            sub(/\/\/.*$/, "", line)

            if (inside == 0) {
                # Look for `struct <Name> {` or `pub struct <Name> {` —
                # also tuple/unit structs but those have no field names
                # so they will not match the banned regex.
                if (match(line, /(^|[^A-Za-z0-9_])struct[[:space:]]+[A-Z][A-Za-z0-9_]*[^;]*\{/)) {
                    inside = 1
                    depth = count(line, "{") - count(line, "}")
                    if (depth <= 0) inside = 0
                }
                next
            }

            # Inside a struct body — emit any banned-named field.
            if (line ~ banned_re) {
                print ln ":" $0
            }

            depth += count(line, "{") - count(line, "}")
            if (depth <= 0) {
                inside = 0
                depth = 0
            }
        }
    ' "$f")
done

if [[ -z "${failures// /}" ]]; then
    echo "output-schema: OK (no banned diagnostic-shaped field names in output struct surface)"
    exit 0
fi

echo "CI gate FAILED: Output schema audit (R10)" >&2
echo "" >&2
echo "Struct fields named error / warning / diagnostic / is_valid /" >&2
echo "lint / correctness in the CLI output surface:" >&2
echo "" >&2
printf '%s' "$failures" >&2
echo "" >&2
echo "Per ARCHITECTURAL-REFACTOR.md § R10 + CHARTER.md §5 (E1 — no" >&2
echo "semantic correctness assertions): output structs cannot carry" >&2
echo "diagnostic content. Rename the field to its observable shape" >&2
echo "(e.g., 'skipped_ranges' instead of 'errors', 'status' instead of" >&2
echo "'is_valid'), or move the field off the output surface entirely." >&2
exit 1
