#!/usr/bin/env bash
# CI gate: No filesystem in plugin (R4).
#
# Rule (CI-GATES.md):
#   `std::fs::*`, `std::path::PathBuf::from`, `File::open` constructors
#   in plugin code (`scope-core/src/languages/`).
#
# Plugins receive parsed `tree-sitter::Tree` + source `&str` from the
# indexer. Reading files from inside a plugin breaks reproducibility
# (the indexer is the only filesystem reader) and shape-tests the
# C2/C3 split. See ARCHITECTURAL-REFACTOR.md § R4.
#
# Allowlist: precede the call with `// scope:audit-allow filesystem`.
# Tags survive grep because each violation is reported with its grep
# context; tagged lines are stripped first.
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCAN_DIR="$ROOT/gumiho-mudang-scope/scope-core/src/languages"

if [[ ! -d "$SCAN_DIR" ]]; then
    echo "no-fs: scan dir not found: $SCAN_DIR" >&2
    exit 1
fi

PATTERN='(std::fs::|fs::(read|write|metadata|canonicalize|create|remove|copy|rename|File)|PathBuf::from|File::open|File::create)'

# Strip allowlisted call sites: any line where the previous non-blank
# line is the allowlist tag.
hits=$(grep -RnE "$PATTERN" --include='*.rs' "$SCAN_DIR" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
    || true)

if [[ -z "$hits" ]]; then
    echo "no-fs: OK (languages/ contains no filesystem constructors)"
    exit 0
fi

# Per-hit allowlist check: read file, look at line above for tag.
filtered=""
while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    file=${line%%:*}
    rest=${line#*:}
    lineno=${rest%%:*}
    if (( lineno > 1 )); then
        prev=$(sed -n "$((lineno - 1))p" "$file")
        if [[ "$prev" == *"scope:audit-allow filesystem"* ]]; then
            continue
        fi
    fi
    filtered+="$line"$'\n'
done <<< "$hits"

if [[ -n "${filtered// /}" ]]; then
    echo "CI gate FAILED: No filesystem in plugin (R4)" >&2
    echo "" >&2
    echo "Filesystem access in plugin code:" >&2
    echo "" >&2
    printf '%s' "$filtered" >&2
    echo "" >&2
    echo "Plugins must not read or write files. The indexer is the only" >&2
    echo "filesystem reader. To exempt a legitimate site, precede the call" >&2
    echo "with: // scope:audit-allow filesystem" >&2
    exit 1
fi

echo "no-fs: OK (all filesystem call sites in languages/ are allowlisted)"
