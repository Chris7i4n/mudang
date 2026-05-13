#!/usr/bin/env bash
# CI gate: Process-spawn denylist (R12).
#
# Rule (CI-GATES.md):
#   `Command::new(` / `process::Command` / `std::process::Command`
#   must not appear in the plugin / extractor / resolver / query paths.
#   These would let the language layer invoke an external compiler or
#   interpreter, violating CHARTER.md § 5 "No compiler/interpreter
#   invocation" and R12's hard-limit detection layer.
#
# Path filter (sub-crate layout):
#   - gumiho-mudang-scope/scope-core/src/languages/
#   - gumiho-mudang-scope/scope-core/src/frameworks/
#   - gumiho-mudang-scope/scope-core/src/parser.rs
#   - gumiho-mudang-scope/scope-core/src/extract/
#   - gumiho-mudang-scope/scope-graph/src/resolve/
#
# Mechanism: source-text grep, line-by-line, after multi-line
# `use ... { ... };` blocks are collapsed to a single logical line by
# `awk` preprocessing. Allowlist tag: `// scope:audit-allow
# process-spawn — <rationale>` on the line immediately preceding the
# call. Wrapping a Command::new in a helper inside an audited path and
# tagging the helper is forbidden — the tag follows the construct, not
# the abstraction (per CI-GATES.md § Allowlist convention).
#
# Alias-import detection: `use std::process::Command as <Alias>;` is
# matched directly so introducing an alias does not bypass the gate.
# The codex sprint-0004 review surfaced this as a P2 bypass.
#
# Per ENFORCEMENT-MAP.md § R12, CHARTER.md § 5, and
# CI-GATES.md § Allowlist convention.
#
# Exits non-zero on any unallowlisted match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SCAN_PATHS=()
for candidate in \
    "$ROOT/gumiho-mudang-scope/scope-core/src/languages" \
    "$ROOT/gumiho-mudang-scope/scope-core/src/frameworks" \
    "$ROOT/gumiho-mudang-scope/scope-core/src/parser.rs" \
    "$ROOT/gumiho-mudang-scope/scope-core/src/extract" \
    "$ROOT/gumiho-mudang-scope/scope-graph/src/resolve"
do
    if [[ -e "$candidate" ]]; then
        SCAN_PATHS+=("$candidate")
    fi
done

if (( ${#SCAN_PATHS[@]} == 0 )); then
    echo "audit_no_spawn: no scan paths exist; check sub-crate layout" >&2
    exit 1
fi

# Collapse multi-line `use ... { ... };` blocks per file. Output format
# is `<file>:<lineno>:<text>` so it grep-greps identically to a raw
# grep. The lineno is the OPENING line of the use block, so failure
# messages still point at code humans can navigate to.
preprocess() {
    local f="$1"
    awk -v f="$f" '
        function count(s, ch,   n) { n = gsub(ch, ch, s); return n }
        BEGIN { ln = 0; buf = ""; bln = 0; depth = 0 }
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
            if ($0 ~ /^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?use[[:space:]]/ && index($0, "{") > 0) {
                d = count($0, "{") - count($0, "}")
                if (d > 0) {
                    buf = $0
                    bln = ln
                    depth = d
                    next
                }
            }
            print f ":" ln ":" $0
        }
        END {
            if (depth > 0) { print f ":" bln ":" buf }
        }
    ' "$f"
}

# Discover .rs files under scan paths.
rs_files=()
for p in "${SCAN_PATHS[@]}"; do
    if [[ -f "$p" ]]; then
        rs_files+=("$p")
    else
        while IFS= read -r line; do
            rs_files+=("$line")
        done < <(find "$p" -type f -name '*.rs')
    fi
done

# Preprocessed corpus: a stream of `<file>:<lineno>:<text>` lines with
# multi-line use statements collapsed.
preprocessed=$(
    for f in "${rs_files[@]}"; do
        preprocess "$f"
    done
)

# Forbidden patterns:
#   - `Command::new(` — direct construction
#   - `process::Command` — qualified type reference
#   - `std::process::Command` — fully qualified type reference
#   - `process::{...Command...}` — grouped import (collapsed by the
#     preprocessor; the codex sprint-0004 round-5 review caught this
#     missing arm)
#   - `Command as <Alias>` — alias declaration (after preprocessing,
#     this catches grouped + bare forms uniformly)
PATTERN='(Command::new\(|process::Command|std::process::Command|process::\{[^}]*\bCommand\b|\bCommand[[:space:]]+as[[:space:]]+[A-Za-z_])'

hits=$(echo "$preprocessed" \
    | grep -nE "$PATTERN" 2>/dev/null \
    | grep -vE '^[0-9]+:[^:]+:[0-9]+:[[:space:]]*//' \
    | sed -E 's/^[0-9]+://' \
    || true)

if [[ -z "$hits" ]]; then
    echo "no-spawn: OK (no process-spawn in plugin/extractor/resolver paths)"
    exit 0
fi

filtered=""
while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    file=${line%%:*}
    rest=${line#*:}
    lineno=${rest%%:*}
    if (( lineno > 1 )); then
        prev=$(sed -n "$((lineno - 1))p" "$file")
        if [[ "$prev" == *"scope:audit-allow process-spawn"* ]]; then
            continue
        fi
    fi
    filtered+="$line"$'\n'
done <<< "$hits"

if [[ -n "${filtered// /}" ]]; then
    echo "CI gate FAILED: Process-spawn denylist (R12)" >&2
    echo "" >&2
    echo "Process-spawn calls / aliases / imports in plugin/extractor/resolver paths:" >&2
    echo "" >&2
    printf '%s' "$filtered" >&2
    echo "" >&2
    echo "Per CHARTER.md § 5: no compiler / interpreter invocation." >&2
    echo "The plugin / extractor / resolver surface must not spawn" >&2
    echo "external processes. To exempt a legitimate site, precede the" >&2
    echo "Command::new(...) line with:" >&2
    echo "  // scope:audit-allow process-spawn — <rationale>" >&2
    echo "Wrapping the call in a helper and tagging the helper is" >&2
    echo "forbidden — the tag follows the construct, not the abstraction." >&2
    echo "" >&2
    echo 'Note: multi-line `use ... { ... };` blocks are collapsed before' >&2
    echo "matching, so grouped imports are not a bypass. The reported" >&2
    echo "line number points at the opening line of the use block." >&2
    exit 1
fi

echo "no-spawn: OK (all process-spawn sites are allowlisted)"
