#!/usr/bin/env bash
# CI gate: WorkspaceContext shape (R4).
#
# Rule (CI-GATES.md):
#   `LanguageWorkspaceContext` must NOT expose `edition`, `target`,
#   `python_requires`, `go_directive`, `tsconfig_target`,
#   `framework_versions`. These belong to per-package metadata or the
#   framework layer (`FrameworkWorkspaceContext`), never the language
#   plugin surface. See ENFORCEMENT-MAP.md § R4 and
#   LANGUAGE-PLAYBOOK.md C2.
#
# Mechanism: extract the `pub trait LanguageWorkspaceContext` block
# from `scope-core/src/workspace_context.rs` (start line → first
# top-level `}` after it) and grep the slice for forbidden tokens.
#
# Exits non-zero on any match.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/gumiho-mudang-scope/scope-core/src/workspace_context.rs"

if [[ ! -f "$SRC" ]]; then
    echo "audit_context_shape: source not found: $SRC" >&2
    exit 1
fi

FORBIDDEN=(
    edition
    target
    python_requires
    go_directive
    tsconfig_target
    framework_versions
)

# Extract the trait body. Start at the `pub trait LanguageWorkspaceContext`
# declaration; stop at the first column-0 `}` after it.
body=$(awk '
    /^pub trait LanguageWorkspaceContext/ { inside = 1 }
    inside { print }
    inside && /^}/ { exit }
' "$SRC")

if [[ -z "$body" ]]; then
    echo "audit_context_shape: failed to locate LanguageWorkspaceContext trait block" >&2
    exit 1
fi

# Strip comment lines so prose mentioning forbidden tokens (the doc
# comment immediately above the trait) does not trip the gate.
code=$(printf '%s\n' "$body" | sed -E 's:^[[:space:]]*//.*$::')

violations=()
for tok in "${FORBIDDEN[@]}"; do
    if printf '%s\n' "$code" | grep -qE "\\b$tok\\b"; then
        violations+=("$tok")
    fi
done

if (( ${#violations[@]} > 0 )); then
    echo "CI gate FAILED: WorkspaceContext shape (R4)" >&2
    echo "" >&2
    echo "LanguageWorkspaceContext exposes forbidden field(s):" >&2
    for v in "${violations[@]}"; do
        echo "  - $v" >&2
    done
    echo "" >&2
    echo "Per ENFORCEMENT-MAP.md § R4, these belong to per-package" >&2
    echo "metadata or FrameworkWorkspaceContext, never the language" >&2
    echo "plugin surface." >&2
    exit 1
fi

echo "context-shape: OK (LanguageWorkspaceContext exposes none of: ${FORBIDDEN[*]})"
