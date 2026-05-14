#!/usr/bin/env bash
# Labeller-workspace isolation gate (R14).
#
# Enforces the build-system boundary that turns CHARTER §3 invariant 6
# ("Deterministic, read-only at query time. No network calls.") and
# CHARTER §5 hard limits ("Network calls during query", "No toolchain
# required", "Invoking the language's compiler or interpreter") into
# mechanical guarantees instead of discipline.
#
# Three narrow-grep checks:
#   1. Root `Cargo.toml` lists `gumiho-mudang-labeller` under
#      `[workspace] exclude = [...]`.
#   2. No Scope-side crate manifest declares a `path = "..."` dependency
#      pointing into `gumiho-mudang-labeller/`.
#   3. No labeller-side manifest under `gumiho-mudang-labeller/` declares
#      a `path = "..."` dependency pointing into `gumiho-mudang-scope/`,
#      `gumiho-mudang-cli/`, or `gumiho-mudang-lsp/`.
#
# Any of the three failing means the workspace boundary has leaked. The
# remediation is on the offending manifest, not on this gate.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FAILED=0

fail_block() {
    local check_name="$1"
    local rationale="$2"
    local hits="$3"
    echo "✗ labeller-workspace-isolation: $check_name" >&2
    echo "  rationale: $rationale" >&2
    if [[ -n "$hits" ]]; then
        echo "  hits:" >&2
        while IFS= read -r line; do
            echo "    $line" >&2
        done <<< "$hits"
    fi
    echo >&2
    FAILED=1
}

# Check 1 — root Cargo.toml excludes the labeller workspace.
#
# The exclusion is the build-system fact. Without it cargo would treat
# `gumiho-mudang-labeller` as an implicit member of the root workspace,
# pulling labeller dependencies into the root `Cargo.lock`.
if ! grep -nE '^\s*exclude\s*=\s*\[[^]]*"gumiho-mudang-labeller"[^]]*\]' Cargo.toml >/dev/null 2>&1; then
    fail_block "root-cargo-toml-excludes-labeller" \
               "root Cargo.toml must declare \`exclude = [..., \"gumiho-mudang-labeller\", ...]\` under \`[workspace]\`" \
               ""
fi

# Check 2 — Scope-side manifests must not depend on the labeller workspace.
#
# A `path = "../gumiho-mudang-labeller/..."` (or any relative variant) in
# a Scope crate's Cargo.toml would re-import the labeller dependencies
# into the Scope build. Forbidden in every direction.
hits=$(grep -RnE 'path\s*=\s*"[^"]*gumiho-mudang-labeller[^"]*"' \
       --include='Cargo.toml' \
       gumiho-mudang-scope gumiho-mudang-cli gumiho-mudang-lsp 2>/dev/null || true)
# Also scan the root Cargo.toml directly; the dir-targeted scan above
# would miss it because `Cargo.toml` is not under any of those dirs.
hits_root=$(grep -nE 'path\s*=\s*"[^"]*gumiho-mudang-labeller[^"]*"' Cargo.toml 2>/dev/null || true)
if [[ -n "$hits_root" ]]; then
    hits=$(printf '%s\n%s' "Cargo.toml:$hits_root" "$hits")
fi
if [[ -n "$hits" ]]; then
    fail_block "scope-side-path-dep-on-labeller" \
               "Scope-side manifests must not declare a path dependency on any crate inside gumiho-mudang-labeller/" \
               "$hits"
fi

# Check 3 — labeller-side manifests must not depend on Scope crates.
#
# A `path = "../gumiho-mudang-scope/..."` (or similar) in a labeller
# manifest would import Scope-side types directly, defeating the
# "consume only the schema doc" contract documented in
# AUDIT-LABEL-SCHEMA.md. Forbidden in every direction.
if [[ -d gumiho-mudang-labeller ]]; then
    hits=$(grep -RnE 'path\s*=\s*"[^"]*(gumiho-mudang-scope|gumiho-mudang-cli|gumiho-mudang-lsp)[^"]*"' \
           --include='Cargo.toml' \
           gumiho-mudang-labeller 2>/dev/null || true)
    if [[ -n "$hits" ]]; then
        fail_block "labeller-side-path-dep-on-scope" \
                   "labeller-side manifests must not declare a path dependency on any Scope crate — the contract is AUDIT-LABEL-SCHEMA.md, not a code dependency" \
                   "$hits"
    fi
fi

if [[ "$FAILED" -ne 0 ]]; then
    exit 1
fi
echo "labeller-workspace-isolation: OK (boundary enforced in both directions)"
