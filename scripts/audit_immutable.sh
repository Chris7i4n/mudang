#!/usr/bin/env bash
# CI gate: Immutable source (R9).
#
# Rule (CI-GATES.md):
#   `&mut str` / `&mut tree_sitter::Tree` / `&mut Tree` / `&mut Source*`
#   tokens in scope-core/src/languages/, scope-core/src/extract/, and
#   scope-core/src/parser.rs are forbidden at fn signatures and let
#   bindings. Source-related data crosses the plugin/extractor surface
#   read-only.
#
# Mechanism: source-text grep over the path-filtered set, line-by-line.
# Tagged exceptions: prefix the line with `// scope:audit-allow
# mutable-source — <rationale>` on the immediately preceding line.
#
# Per ARCHITECTURAL-REFACTOR.md § R9 + LANGUAGE-PLAYBOOK.md F2.
#
# Exits non-zero on any unallowlisted match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SCAN_PATHS=(
    "$ROOT/gumiho-mudang-scope/scope-core/src/languages"
    "$ROOT/gumiho-mudang-scope/scope-core/src/extract"
    "$ROOT/gumiho-mudang-scope/scope-core/src/parser.rs"
)

for p in "${SCAN_PATHS[@]}"; do
    if [[ ! -e "$p" ]]; then
        echo "audit_immutable: scan path not found: $p" >&2
        exit 1
    fi
done

# Forbidden tokens. `&mut Source` matches `&mut SourceFile`, `&mut SourceMap`,
# as well as path-qualified forms like `&mut crate::SourceFile` and
# `&mut source::SourceMap` — the codex sprint-0004 round-3 review flagged
# unqualified-only matching as a bypass. `tree_sitter::Tree` is the
# fully-qualified path; `Tree` alone is also matched but only in `&mut Tree`
# (not e.g. `Vec<Tree>` or unrelated `MyTree`).
#
# An optional `'<lifetime>` between `&` and `mut` must match — Rust permits
# `&'a mut str`, `&'_ mut Tree`, etc. Codex sprint-0004 review flagged the
# original `&mut[[:space:]]+...` pattern as bypassable by any lifetime-qualified
# signature; the lifetime arm is now part of the contract.
#
# Pattern structure:
#   &                            literal reference
#   ( '<lifetime> )?             optional lifetime
#   mut                          literal mut
#   ((\w+::)*                    zero or more path segments
#       Source[A-Z]              `Source` followed by an uppercase letter
#       |str\b                   or bare `str`
#       |String\b                or bare `String`
#       |tree_sitter::Tree\b     or fully qualified Tree
#       |Tree\b                  or bare Tree
#   )
PATTERN='&([[:space:]]*'"'"'[a-z_0-9]+)?[[:space:]]*mut[[:space:]]+((([A-Za-z_][A-Za-z_0-9]*)::)*Source[A-Za-z0-9_]*\b|str\b|String\b|tree_sitter::Tree\b|Tree\b)'

hits=$(grep -RnE "$PATTERN" --include='*.rs' "${SCAN_PATHS[@]}" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
    || true)

if [[ -z "$hits" ]]; then
    echo "immutable: OK (no &mut on source-related types at plugin/extractor surface)"
    exit 0
fi

# Per-hit allowlist check.
filtered=""
while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    file=${line%%:*}
    rest=${line#*:}
    lineno=${rest%%:*}
    if (( lineno > 1 )); then
        prev=$(sed -n "$((lineno - 1))p" "$file")
        if [[ "$prev" == *"scope:audit-allow mutable-source"* ]]; then
            continue
        fi
    fi
    filtered+="$line"$'\n'
done <<< "$hits"

if [[ -n "${filtered// /}" ]]; then
    echo "CI gate FAILED: Immutable source (R9)" >&2
    echo "" >&2
    echo "Mutable source-related references in plugin/extractor surface:" >&2
    echo "" >&2
    printf '%s' "$filtered" >&2
    echo "" >&2
    echo "Source data must cross the plugin/extractor surface read-only." >&2
    echo "Per LANGUAGE-PLAYBOOK.md F2: plugins must not write back to source." >&2
    echo "To exempt a legitimate site, precede the line with:" >&2
    echo "  // scope:audit-allow mutable-source — <rationale>" >&2
    exit 1
fi

echo "immutable: OK (all &mut source-type sites are allowlisted)"
