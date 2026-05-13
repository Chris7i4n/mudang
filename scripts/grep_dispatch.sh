#!/usr/bin/env bash
# CI gate: Indexer-only dispatch (R7).
#
# Rule (CI-GATES.md):
#   Plugin code does not read file content for self-activation
#   (no `read_to_string` etc. in plugin trait impls).
#
# Post-R7, dispatch is the const-fn table built by
# `register_languages!` in `scope-core/src/languages/dispatch.rs`.
# Plugins (`scope-core/src/languages/*.rs`) receive a parsed tree +
# source slice from the indexer; they must not read file content to
# decide whether to handle a file. The dispatch table is the single
# source of truth — extension/shebang routing happens at compile time.
#
# Mechanism: two checks.
#   (1) `languages/` contains no content readers
#       (`read_to_string`, `read_to_end`, `read_line`, `BufRead`).
#   (2) `dispatch_extension` and `dispatch_shebang` are defined
#       exactly once each (only in `languages/dispatch.rs`).
#
# Exits non-zero on any violation.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCAN_DIR="$ROOT/gumiho-mudang-scope/scope-core/src/languages"

if [[ ! -d "$SCAN_DIR" ]]; then
    echo "dispatch: scan dir not found: $SCAN_DIR" >&2
    exit 1
fi

# (1) content-reader scan
READERS='\b(read_to_string|read_to_end|read_line|BufRead)\b'
reader_hits=$(grep -RnE "$READERS" --include='*.rs' "$SCAN_DIR" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
    || true)

if [[ -n "$reader_hits" ]]; then
    echo "CI gate FAILED: Indexer-only dispatch (R7)" >&2
    echo "" >&2
    echo "Plugin code reads file content (forbidden for self-activation):" >&2
    echo "" >&2
    echo "$reader_hits" >&2
    echo "" >&2
    echo "Plugins must not read content. Dispatch goes through the const-fn" >&2
    echo "table in languages/dispatch.rs (register_languages! macro)." >&2
    exit 1
fi

# (2) dispatch fn defined exactly once each
extension_defs=$(grep -RlE '\bfn dispatch_extension\b' --include='*.rs' \
    "$ROOT/gumiho-mudang-scope/scope-core/src/" 2>/dev/null | sort -u || true)
shebang_defs=$(grep -RlE '\bfn dispatch_shebang\b' --include='*.rs' \
    "$ROOT/gumiho-mudang-scope/scope-core/src/" 2>/dev/null | sort -u || true)

# The macro hides the `fn` token inside the expansion site. Treat the
# canonical dispatch module as the sole definition source: require the
# `register_languages!` invocation to live in `languages/dispatch.rs`,
# and that no other file in the workspace declares those fns directly.
macro_call=$(grep -RlE '^register_languages!\(' --include='*.rs' \
    "$ROOT/gumiho-mudang-scope/scope-core/src/" 2>/dev/null | sort -u || true)

EXPECTED_MACRO="$ROOT/gumiho-mudang-scope/scope-core/src/languages/dispatch.rs"

if [[ "$macro_call" != "$EXPECTED_MACRO" ]]; then
    echo "CI gate FAILED: Indexer-only dispatch (R7)" >&2
    echo "" >&2
    echo "register_languages!() must be invoked exactly once, in" >&2
    echo "  $EXPECTED_MACRO" >&2
    echo "Found:" >&2
    printf '  %s\n' $macro_call >&2
    exit 1
fi

# Any non-empty `fn dispatch_extension` / `fn dispatch_shebang` outside
# the macro definition site is a violation. The macro definition itself
# lives in dispatch.rs, so dispatch.rs is allowed; everything else is not.
for f in $extension_defs $shebang_defs; do
    if [[ "$f" != "$EXPECTED_MACRO" ]]; then
        echo "CI gate FAILED: Indexer-only dispatch (R7)" >&2
        echo "" >&2
        echo "dispatch_extension / dispatch_shebang defined outside" >&2
        echo "$EXPECTED_MACRO:" >&2
        echo "  $f" >&2
        exit 1
    fi
done

echo "dispatch: OK (no content readers in languages/, register_languages! lives in dispatch.rs)"
