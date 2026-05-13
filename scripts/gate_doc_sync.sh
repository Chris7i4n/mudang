#!/usr/bin/env bash
# Doc-sync gate.
#
# Refuses every drift shape between named code surfaces and named
# governing-doc passages. ENFORCEMENT-MAP.md § R13 owns the contract.
#
# This gate is the mechanical half of preventing doc-↔-code drift
# across the self-correction loop (`SELF-CORRECTION-CYCLE.md`). It is
# also general — any pair of (governing-doc value, code value) that
# must match can be wired here.
#
# Each check below targets ONE specific drift shape, modelled on the
# `gate_charter.sh` narrow-grep pattern. Loose substring scans are
# avoided on purpose: they fire on charter-aligned prose (migration
# notes, language-feature terminology, directional terms) and erode
# the gate's signal.
#
# Extension protocol (for later sprints in Priority 1):
#
#   1. Add a `check_<short_name>()` function below.
#   2. Invoke it from `main()` alongside the other checks.
#   3. The check stays narrow: assert ONE drift shape; cite the doc
#      path AND the code path in its rationale.
#   4. Commit lives on the sprint branch that introduces the
#      code-↔-doc pair, in the same commit that ships the surface.
#      The CI-GATES.md row's `Mechanism` cell is refined to mention
#      the new check by name (per ENFORCEMENT-MAP.md § 7.5 refinement
#      vs. new-technique discipline).
#
# Exits non-zero on any match. Output identifies the failing check
# and the canonical replacement so the operator can resolve the
# regression without guesswork.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SCOPE_DOCS="gumiho-mudang-scope/docs"

FAILED=0

# Compute the GitHub-style anchor for a single markdown heading line.
#
# Input  : `## 5. Hard limits — Scope will never cross these`
# Output : `5-hard-limits--scope-will-never-cross-these`
#
# Algorithm (matches GitHub's renderer for the cases this repo
# uses — top-level numbered `## N. Title`, sub-sections `### Title`,
# em-dash separators):
#   1. Strip the leading `#` markers and following whitespace.
#   2. Lowercase.
#   3. Replace spaces with dashes.
#   4. Drop every char that is not [a-z0-9_-].
#
# Locale forced to C so multi-byte chars (em-dash, en-dash) get
# stripped predictably.
compute_anchor() {
    local line="$1"
    line="$(printf '%s' "$line" | sed -E 's/^#+[[:space:]]*//')"
    line="$(printf '%s' "$line" | LC_ALL=C tr '[:upper:]' '[:lower:]')"
    line="${line// /-}"
    line="$(printf '%s' "$line" | LC_ALL=C sed -E 's/[^a-z0-9_-]+//g')"
    printf '%s' "$line"
}

# Print a failure block. Reports the violating drift shape +
# rationale, then appends the offending values verbatim so the
# operator can navigate to the source of truth and the drifted copy.
fail_block() {
    local check_name="$1"
    local rationale="$2"
    local detail="$3"
    echo "✗ doc-sync regression: $check_name" >&2
    echo "  rationale: $rationale" >&2
    echo "  detail:" >&2
    while IFS= read -r line; do
        echo "    $line" >&2
    done <<< "$detail"
    echo >&2
    FAILED=1
}

# ─────────────────────────────────────────────────────────────
# Check 1 — every `### R<n>` entry in ENFORCEMENT-MAP.md references
# at least one file path that exists on disk.
#
# Rationale: R-entries link to "Where in the tree" code surfaces.
# Renaming or removing the code without updating the R-entry breaks
# the rule → implementation map. R-entry edits in the same commit as
# code moves are mandated by sprints/README.md § 7.5; this check
# enforces it mechanically.
#
# Drift shape caught: an R-entry cites a file path that no longer
# exists.
check_enforcement_map_paths() {
    local map="$SCOPE_DOCS/ENFORCEMENT-MAP.md"
    [[ -f "$map" ]] || return 0
    # Extract backticked paths that look like file paths
    # (contain `/` AND end in `.rs`, `.sh`, `.sql`, `.toml`, `.md`).
    # Skip placeholder-shaped paths (contain `<`, `>`, `*`, `{`, `}`)
    # since those are template / glob illustrations, not real paths.
    # Try fallback resolution under `gumiho-mudang-scope/` for paths
    # written as sub-crate-relative (e.g. `scope-core/src/edge.rs`
    # actually lives at `gumiho-mudang-scope/scope-core/src/edge.rs`).
    local missing=""
    while IFS= read -r path; do
        # Strip leading ./ and trailing ) ] , . ;
        path="${path#./}"
        path="${path%[).,:;\`]}"
        # Skip placeholders
        [[ "$path" == *"<"* || "$path" == *">"* ]] && continue
        [[ "$path" == *"*"* ]] && continue
        [[ "$path" == *"{"* || "$path" == *"}"* ]] && continue
        # Require slash + recognised extension
        if [[ "$path" =~ \.(rs|sh|sql|toml|md)$ ]] && [[ "$path" == */* ]]; then
            if [[ ! -e "$path" \
               && ! -e "gumiho-mudang-scope/$path" \
               && ! -e "gumiho-mudang-scope/docs/$path" ]]; then
                missing+="$path"$'\n'
            fi
        fi
    done < <(grep -oE '`[^`]+\.(rs|sh|sql|toml|md)`' "$map" | tr -d '`' | sort -u)
    if [[ -n "$missing" ]]; then
        fail_block "enforcement-map-paths" \
                   "ENFORCEMENT-MAP.md cites file path(s) that do not exist (tried both repo-root and gumiho-mudang-scope/ prefixes)" \
                   "$missing"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 2 — every CI-GATES.md row marked `active` has a `just`
# recipe defined in the root justfile.
#
# Rationale: CI-GATES.md "Local invocation" cell is the operator's
# contract for running the gate locally. If the recipe is gone, the
# gate is unrunnable and the doc lies.
#
# Drift shape caught: gate row says `just X` but `X` is not in
# `justfile`.
check_ci_gates_recipes() {
    local gates="$SCOPE_DOCS/CI-GATES.md"
    local just="justfile"
    # If CI-GATES.md is absent there is nothing to check — bail early.
    [[ -f "$gates" ]] || return 0
    # If CI-GATES.md exists but justfile is missing/renamed, every
    # "Local invocation" cell in the active rows is unrunnable. That
    # IS the drift the gate must catch — fail loud, do not silently
    # skip.
    if [[ ! -f "$just" ]]; then
        fail_block "ci-gates-recipes" \
                   "CI-GATES.md is present but root justfile is missing — every active row's \`just <recipe>\` is unrunnable" \
                   "expected: justfile (repo root)"
        return
    fi
    local missing=""
    # Pull every "Local invocation" cell that contains `just <name>`
    # AND whose row ends with `| active |`.
    while IFS= read -r line; do
        # Only rows tagged active
        [[ "$line" == *"| active |"* ]] || continue
        # Extract `just <name>` invocations
        while read -r recipe; do
            [[ -n "$recipe" ]] || continue
            if ! grep -qE "^${recipe}:" "$just"; then
                missing+="$recipe (in CI-GATES.md but not in justfile)"$'\n'
            fi
        done < <(echo "$line" | grep -oE 'just [a-z0-9_-]+' | awk '{print $2}')
    done < <(grep -E '^\| ' "$gates")
    if [[ -n "$missing" ]]; then
        fail_block "ci-gates-recipes" \
                   "CI-GATES.md active row references a just recipe not in justfile" \
                   "$missing"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 3 — every markdown link `](./*.md)` or `](../*.md)` (and
# deeper relative paths) under gumiho-mudang-scope/docs/ resolves to
# an existing file on disk.
#
# Rationale: cross-link discipline is the connective tissue of the
# governing-doc layer. A broken link is a silent rule-routing
# failure: readers land on a 404 and miss the source of truth. The
# CLAUDE.md doc-routing table relies on this.
#
# Drift shape caught: `]( …relative path… )` pointing nowhere.
check_doc_relative_links() {
    local broken=""
    # Iterate over every .md file under the docs tree, skipping
    # template files (placeholder links by design).
    while IFS= read -r doc; do
        [[ "$doc" == *"_TEMPLATE.md" ]] && continue
        local dir
        dir="$(dirname "$doc")"
        # Strip inline code spans + fenced code blocks BEFORE
        # extracting links so backtick-protected `](pattern)` literals
        # used illustratively in prose are not mistaken for real
        # links. awk strips fenced ``` blocks; sed strips `…` spans.
        local stripped
        stripped="$(awk 'BEGIN{f=0} /^```/{f=!f;next} f==0{print}' "$doc" | sed -E 's/`[^`]*`//g')"
        # Extract every `](relative-path)` link target from the
        # stripped content.
        while IFS= read -r target; do
            [[ -n "$target" ]] || continue
            [[ "$target" =~ ^https?:// ]] && continue
            [[ "$target" =~ ^# ]] && continue
            # Skip placeholder-shaped targets
            [[ "$target" == *"<"* || "$target" == *">"* ]] && continue
            [[ "$target" == *"*"* ]] && continue
            [[ "$target" == *"{"* || "$target" == *"}"* ]] && continue
            # Split path + anchor.
            local path="${target%%#*}"
            local anchor=""
            if [[ "$target" == *"#"* ]]; then
                anchor="${target#*#}"
            fi
            [[ -z "$path" ]] && continue
            # File-existence check first.
            if [[ ! -e "$dir/$path" ]]; then
                broken+="$doc → $target (file not found)"$'\n'
                continue
            fi
            # Anchor validation — when present, every named fragment
            # must match a heading in the target file. A relative
            # link `target.md#anchor` where the file exists but the
            # fragment is gone is still drift: the reader lands on
            # the file but not on the section that was promised.
            if [[ -n "$anchor" ]]; then
                local target_file="$dir/$path"
                # target_file may not be a .md (e.g. justfile). Only
                # validate anchors against markdown files; non-md
                # files have no headings to match.
                if [[ "$target_file" == *.md ]]; then
                    local found=0
                    while IFS= read -r heading_line; do
                        if [[ "$(compute_anchor "$heading_line")" == "$anchor" ]]; then
                            found=1
                            break
                        fi
                    done < <(grep -E '^#+[[:space:]]' "$target_file" 2>/dev/null || true)
                    if [[ "$found" -eq 0 ]]; then
                        broken+="$doc → $target (file exists; anchor #$anchor not found in headings)"$'\n'
                    fi
                fi
            fi
        done < <(echo "$stripped" | grep -oE '\]\([^)]+\)' | sed -E 's/^\]\(//; s/\)$//')
    done < <(find "$SCOPE_DOCS" -name '*.md' -type f)
    if [[ -n "$broken" ]]; then
        fail_block "doc-relative-links" \
                   "markdown link points to a non-existent path (inline + fenced code spans skipped; templates skipped; placeholder shapes skipped)" \
                   "$broken"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 4 — SELF-CORRECTION-CYCLE.md and SELF-CORRECTION-STATE.md
# are cross-linked from the doc index README.
#
# Rationale: governing docs that are not indexed are not discovered.
# Per gumiho-mudang-scope/docs/README.md § "Where to put a new note",
# every governing doc is reachable from the index.
#
# Drift shape caught: cycle doc / state doc exists on disk but is
# not referenced from the docs README.
check_cycle_docs_indexed() {
    local index="$SCOPE_DOCS/README.md"
    [[ -f "$index" ]] || return 0
    local cycle="SELF-CORRECTION-CYCLE.md"
    local state="SELF-CORRECTION-STATE.md"
    local missing=""
    if [[ -f "$SCOPE_DOCS/$cycle" ]] && ! grep -q "$cycle" "$index"; then
        missing+="$cycle not referenced from $index"$'\n'
    fi
    if [[ -f "$SCOPE_DOCS/$state" ]] && ! grep -q "$state" "$index"; then
        missing+="$state not referenced from $index"$'\n'
    fi
    if [[ -n "$missing" ]]; then
        fail_block "cycle-docs-indexed" \
                   "self-correction docs exist but the docs README does not link them" \
                   "$missing"
    fi
}

# ─────────────────────────────────────────────────────────────
# Future sprint extension hooks. Each later sprint in Priority 1 adds
# ONE function here (named `check_<short_name>`) and invokes it from
# `main()`. See SELF-CORRECTION-CYCLE.md § "Extending the doc-sync
# gate" for the per-sprint table.
#
# Sprints expected to extend this script:
#   - 0004 (g): SCHEMA_VERSION const ↔ schema_version doc value
#   - 0004 (g): SampleRecord field set ⊆ AUDIT-LABEL-SCHEMA.md fields
#   - 0004 (h): coverage_summary fields ↔ doc fields
#   - 0004 (j): edge_audit_history columns ↔ doc columns
#   - 0006 (i): documented default aggregation policy ↔ aggregator default
#   - 0009 (k): audit-trail path doc ↔ indexer-read path

# ─────────────────────────────────────────────────────────────
main() {
    check_enforcement_map_paths
    check_ci_gates_recipes
    check_doc_relative_links
    check_cycle_docs_indexed

    if [[ "$FAILED" -ne 0 ]]; then
        echo "doc-sync gate: FAIL" >&2
        exit 1
    fi
    echo "doc-sync gate: pass"
}

main "$@"
