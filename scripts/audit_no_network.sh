#!/usr/bin/env bash
# CI gate: Network denylist (R12).
#
# Rule (CI-GATES.md):
#   `std::net::*` / `tokio::net::*` / `reqwest::` / `hyper::` / `ureq::`
#   symbol references must not appear in the plugin / extractor /
#   resolver / query paths. The language layer cannot reach the network
#   at query time — CHARTER.md § 5 hard limit "No network at query time"
#   plus R12's detection layer.
#
# Path filter: same as the process-spawn denylist (see
# scripts/audit_no_spawn.sh).
#
# Mechanism: source-text grep over `use` declarations and qualified
# references, after multi-line `use ... { ... };` blocks are collapsed
# to a single logical line by `awk` preprocessing. The multi-line
# grouped-import bypass that the review flagged is
# closed by this preprocessing — the matching is identical to a raw
# grep, but operates on the collapsed corpus.
#
# `cargo-deny` is **not** wired as an architecture gate — dep-tree
# hygiene is tooling, not the R12 contract. Allowlist tag:
# `// scope:audit-allow network — <rationale>` on the line
# immediately preceding the reference.
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
    echo "audit_no_network: no scan paths exist; check sub-crate layout" >&2
    exit 1
fi

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

preprocessed=$(
    for f in "${rs_files[@]}"; do
        preprocess "$f"
    done
)

# Network-surface detection. Pattern arms (matched against the
# collapsed corpus, so single-line and multi-line forms are equivalent):
#   1. Qualified references: `std::net::`, `tokio::net::`, `reqwest::`,
#      `hyper::`, `ureq::`.
#   2. Bare top-level `use std::net;` or `use tokio::net;` — leaves
#      later `net::TcpStream` references unqualified.
#   3. Bare HTTP-crate imports: `use reqwest;`, `use hyper;`, `use ureq;`.
#   4. Grouped imports inside `{ ... }`: `use std::{..., net::..., ...}`
#      or `use std::{..., net, ...}`. Same for `tokio::{...}`.
# Use word-boundary `\buse[[:space:]]+` so `pub use ...` / `pub(crate) use ...`
# / `pub(super) use ...` are matched alongside bare `use ...`. The
# preprocessor above collapses every `use … { … };` block — public,
# private, and visibility-qualified — to one logical line.
#
# Absolute-path imports (`use ::reqwest as http;`, `use ::std::net::...`)
# are matched by allowing an optional `::` prefix on every `use …` arm.
# The alias-declaration arms `(reqwest|hyper|ureq)[[:space:]]+as[[:space:]]+\w+`
# catch aliased forms regardless of import syntax — anyone introducing
# an alias is forced to remove it; the bare crate name then matches.
# Both cases must be caught.
PATTERN='(std::net::|tokio::net::|reqwest::|hyper::|ureq::|\buse[[:space:]]+(::[[:space:]]*)?std::net\b|\buse[[:space:]]+(::[[:space:]]*)?tokio::net\b|\buse[[:space:]]+(::[[:space:]]*)?(reqwest|hyper|ureq)\b|\buse[[:space:]]+(::[[:space:]]*)?std::\{[^}]*\bnet\b|\buse[[:space:]]+(::[[:space:]]*)?tokio::\{[^}]*\bnet\b|\b(reqwest|hyper|ureq)[[:space:]]+as[[:space:]]+[A-Za-z_])'

hits=$(echo "$preprocessed" \
    | grep -nE "$PATTERN" 2>/dev/null \
    | grep -vE '^[0-9]+:[^:]+:[0-9]+:[[:space:]]*//' \
    | sed -E 's/^[0-9]+://' \
    || true)

if [[ -z "$hits" ]]; then
    echo "no-network: OK (no network symbols in plugin/extractor/resolver paths)"
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
        if [[ "$prev" == *"scope:audit-allow network"* ]]; then
            continue
        fi
    fi
    filtered+="$line"$'\n'
done <<< "$hits"

if [[ -n "${filtered// /}" ]]; then
    echo "CI gate FAILED: Network denylist (R12)" >&2
    echo "" >&2
    echo "Network symbol references in plugin/extractor/resolver paths:" >&2
    echo "" >&2
    printf '%s' "$filtered" >&2
    echo "" >&2
    echo "Per CHARTER.md § 5: no network at query time." >&2
    echo "The plugin / extractor / resolver surface must not link in" >&2
    echo "network APIs. To exempt a legitimate site, precede the line" >&2
    echo "with:" >&2
    echo "  // scope:audit-allow network — <rationale>" >&2
    echo "Wrapping the call in a helper and tagging the helper is" >&2
    echo "forbidden — the tag follows the construct, not the abstraction." >&2
    echo "" >&2
    echo 'Note: multi-line `use ... { ... };` blocks are collapsed before' >&2
    echo "matching, so grouped imports are not a bypass. The reported" >&2
    echo "line number points at the opening line of the use block." >&2
    exit 1
fi

echo "no-network: OK (all network sites are allowlisted)"
