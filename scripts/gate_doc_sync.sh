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
# Check 5 — every `LanguageId` slug from `as_str()` owns an
# `audit-samples/` directory under
# `gumiho-mudang-scope/scope-core/tests/fixtures/reference/`, AND
# every directory present there corresponds to a `LanguageId` slug
# (no extras).
#
# Rationale: the labelled corpus is keyed by `db_slug`. A new
# `LanguageId` arm without an `audit-samples/` directory means the
# audit pipeline cannot accumulate samples for that language —
# sprint 0002 (e) ([`SELF-CORRECTION-CYCLE.md` § "Sprint-by-sprint
# additions expected"](gumiho-mudang-scope/docs/SELF-CORRECTION-CYCLE.md))
# wires this lockstep in.
#
# Drift shape caught: arm-set ≠ directory-set (either direction).
check_audit_samples_layout() {
    local id_rs="gumiho-mudang-scope/scope-core/src/languages/id.rs"
    local corpus_root="gumiho-mudang-scope/scope-core/tests/fixtures/reference"
    [[ -f "$id_rs" ]] || return 0
    [[ -d "$corpus_root" ]] || return 0
    # Extract slugs from the `as_str` arms. The const fn body uses
    # `Self::<Variant> => "<slug>",` lines exclusively; `from_slug`
    # uses the inverse `"<slug>" => Some(Self::<Variant>)` shape and
    # does NOT match this regex (direction differs).
    local slugs_from_code
    slugs_from_code="$(grep -oE 'Self::[A-Z][a-zA-Z]+ => "[a-z]+"' "$id_rs" \
                       | sed -E 's/.* => "([a-z]+)"/\1/' \
                       | sort -u)"
    # Directories on disk that contain `audit-samples/`. depth-2
    # matches `<corpus_root>/<slug>/audit-samples` exactly; any
    # future top-level subdir under the corpus root that happens to
    # contain its own `audit-samples/` would surface as an "extra"
    # slug, which is the right signal.
    local dirs_on_disk
    dirs_on_disk="$(/usr/bin/find "$corpus_root" -mindepth 2 -maxdepth 2 -type d -name audit-samples \
                    | sed -E "s#^$corpus_root/##; s#/audit-samples\$##" \
                    | sort -u)"
    # Guard against empty inputs — both sides should always be
    # populated in practice (id.rs always defines arms; the corpus
    # root always has at least one slug dir). If either is empty
    # the comparison is meaningless: skip rather than emit a
    # phantom verdict from `comm` on stray empty lines.
    if [[ -z "$slugs_from_code" || -z "$dirs_on_disk" ]]; then
        return 0
    fi
    local missing extra
    missing="$(comm -23 <(printf '%s\n' "$slugs_from_code") <(printf '%s\n' "$dirs_on_disk"))"
    extra="$(comm -13 <(printf '%s\n' "$slugs_from_code") <(printf '%s\n' "$dirs_on_disk"))"
    local detail=""
    if [[ -n "$missing" ]]; then
        detail+="LanguageId slugs without audit-samples/ dir:"$'\n'
        while IFS= read -r s; do [[ -n "$s" ]] && detail+="  $s"$'\n'; done <<< "$missing"
    fi
    if [[ -n "$extra" ]]; then
        detail+="audit-samples/ dirs without matching LanguageId slug:"$'\n'
        while IFS= read -r s; do [[ -n "$s" ]] && detail+="  $s"$'\n'; done <<< "$extra"
    fi
    if [[ -n "$detail" ]]; then
        fail_block "audit-samples-layout" \
                   "LanguageId arm set ≠ reference/<slug>/audit-samples/ directory set" \
                   "$detail"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 6 — every `LanguageId` variant in
# `scope-core/src/languages/id.rs` (a) appears as a
# `LanguageId::<Variant>` arm in `detect_in_dir` inside
# `scope-core/src/workspace/lang_version.rs`, AND (b) has a matching
# subsection under `CHARTER.md § 7. Per-language scope and non-scope`
# (heading text equals the variant name, with `CSharp` rendered as
# `C#`, and an optional ` (surface)` suffix tolerated).
#
# Rationale: sprint 0003 wires a per-language `lang_version` detector
# matrix per [`SELF-CORRECTION-CYCLE.md` § "Sprint-by-sprint
# additions expected"](gumiho-mudang-scope/docs/SELF-CORRECTION-CYCLE.md)
# row 0003 / (d). The detector dispatcher is the single source of
# truth for which languages emit a non-null `lang_version`. CHARTER
# §7 is the source of truth for which languages Scope supports at
# all. A drift in either direction (variant added without detector
# arm; variant added without CHARTER §7 acknowledgement; CHARTER §7
# section without an enum variant) is the drift shape this check
# catches. The Rust compiler enforces exhaustive `match` so the
# variant→detector half is already structurally true; this check
# adds the doc-side half on top.
#
# Drift shape caught: variant set ≠ CHARTER §7 subsection set, or
# variant absent from `detect_in_dir`.
check_lang_version_detector_modules() {
    local charter="$SCOPE_DOCS/CHARTER.md"
    local id_rs="gumiho-mudang-scope/scope-core/src/languages/id.rs"
    local detector="gumiho-mudang-scope/scope-core/src/workspace/lang_version.rs"
    [[ -f "$charter" && -f "$id_rs" && -f "$detector" ]] || return 0
    local variants
    variants="$(grep -oE 'Self::[A-Z][a-zA-Z]+ =>' "$id_rs" \
                | sed -E 's/Self::([A-Z][a-zA-Z]+) =>/\1/' \
                | sort -u)"
    [[ -z "$variants" ]] && return 0
    local charter_section
    charter_section="$(awk '/^## 7\./{flag=1; next} /^## /{flag=0} flag' "$charter")"
    local detail=""
    while IFS= read -r v; do
        [[ -z "$v" ]] && continue
        local display="$v"
        [[ "$v" == "CSharp" ]] && display="C#"
        if ! printf '%s\n' "$charter_section" \
             | grep -qE "^### ${display}( \\(surface\\))?[[:space:]]*\$"; then
            detail+="LanguageId::$v → expected '### $display' (optionally ' (surface)') subsection under CHARTER.md § 7"$'\n'
        fi
        if ! grep -qE "LanguageId::$v[[:space:]]" "$detector"; then
            detail+="LanguageId::$v → expected arm in lang_version.rs::detect_in_dir"$'\n'
        fi
    done <<< "$variants"
    # Reverse pass — every `### <Lang>` subsection under CHARTER §7
    # must map back to a `LanguageId` variant. Catches the drift the
    # forward pass cannot: an orphan heading (e.g. `### Kotlin`) added
    # to CHARTER §7 without a matching enum variant in `id.rs`. The
    # "Multi-version posture for languages" subsection is the only
    # non-language `###` heading and is exempted by name.
    while IFS= read -r heading; do
        [[ -z "$heading" ]] && continue
        # Strip leading "### " and trailing " (surface)" / whitespace.
        local name
        name="$(printf '%s' "$heading" | sed -E 's/^###[[:space:]]+//; s/[[:space:]]+\(surface\)$//; s/[[:space:]]+$//')"
        # Skip the documented non-language §7 subsection.
        [[ "$name" == "Multi-version posture for languages" ]] && continue
        # CHARTER renders C# as `C#`; map back to enum variant CSharp.
        local expected="$name"
        [[ "$name" == "C#" ]] && expected="CSharp"
        if ! printf '%s\n' "$variants" | grep -qxE "$expected"; then
            detail+="CHARTER.md § 7 subsection '### $name' has no matching LanguageId variant in id.rs"$'\n'
        fi
    done < <(printf '%s\n' "$charter_section" | grep -E '^### ')
    if [[ -n "$detail" ]]; then
        fail_block "lang-version-detector-modules" \
                   "LanguageId variant set ≠ CHARTER.md § 7 subsections or lang_version.rs::detect_in_dir arms" \
                   "$detail"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 7 — `SAMPLE_SCHEMA_VERSION` const in audit.rs ↔ documented
# version in AUDIT-LABEL-SCHEMA.md.
#
# Rationale: AUDIT-LABEL-SCHEMA.md is the wire contract for external
# labellers (LLM / LSP / hybrid). The schema version stamp appears in
# TWO places that must match: the heading
# `### Record schema (\`schema_version: "X"\`)` and the row in the
# fields table whose `Description` cell quotes the active version
# (`Always "X" on emit`). Code emission uses
# `SAMPLE_SCHEMA_VERSION: &str = "X"` from audit.rs. Sprint 0004 (g)
# bumped v1 → v2 across all three; any drift between them silently
# breaks the external-labeller contract.
#
# Drift shape caught: const value ≠ heading value, or const value ≠
# table-row value, or heading value ≠ table-row value.
check_schema_version_const_doc() {
    local audit_rs="gumiho-mudang-cli/src/commands/audit.rs"
    local schema_doc="$SCOPE_DOCS/AUDIT-LABEL-SCHEMA.md"
    [[ -f "$audit_rs" && -f "$schema_doc" ]] || return 0

    # Extract the const value. Wrap each pipeline with `|| true` so
    # `set -e` does not kill the gate on a deliberate no-match — the
    # absence detail is surfaced via the `[[ -z ... ]]` guards below.
    local const_value
    const_value="$(grep -oE 'SAMPLE_SCHEMA_VERSION:[[:space:]]*&str[[:space:]]*=[[:space:]]*"[^"]+"' "$audit_rs" 2>/dev/null \
                   | head -n1 \
                   | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/' || true)"
    if [[ -z "$const_value" ]]; then
        fail_block "schema-version-const-doc" \
                   "SAMPLE_SCHEMA_VERSION const not found in $audit_rs (the gate cannot validate against an absent anchor)" \
                   "expected: pub const SAMPLE_SCHEMA_VERSION: &str = \"<version>\";"
        return
    fi

    # Extract the heading value: the section header that opens the
    # record-schema table. AUDIT-LABEL-SCHEMA.md renders this at H2
    # (`## Record schema (\`schema_version: "X"\`)`) but the gate
    # tolerates H3 too in case the doc is reshuffled — the named
    # token "Record schema" is the contract.
    local heading_value
    heading_value="$(grep -oE '^#+[[:space:]]+Record schema[[:space:]]+\(`schema_version: "[^"]+"`\)' "$schema_doc" 2>/dev/null \
                     | head -n1 \
                     | sed -E 's/.*"([^"]+)".*/\1/' || true)"

    # Extract the table-row value: the row whose first cell is
    # `\`schema_version\`` and whose Description quotes the version.
    local table_value
    table_value="$(grep -E '^\| `schema_version` \|' "$schema_doc" 2>/dev/null \
                   | head -n1 \
                   | grep -oE 'Always `"[^"]+"`' \
                   | sed -E 's/.*"([^"]+)".*/\1/' || true)"

    local detail=""
    if [[ -z "$heading_value" ]]; then
        detail+="AUDIT-LABEL-SCHEMA.md missing heading \`### Record schema (\`schema_version: \"<X>\"\`)\`"$'\n'
    fi
    if [[ -z "$table_value" ]]; then
        detail+="AUDIT-LABEL-SCHEMA.md missing schema_version row with \`Always \"<X>\"\` description"$'\n'
    fi
    if [[ -n "$heading_value" && "$heading_value" != "$const_value" ]]; then
        detail+="SAMPLE_SCHEMA_VERSION=\"$const_value\" ≠ AUDIT-LABEL-SCHEMA.md heading schema_version=\"$heading_value\""$'\n'
    fi
    if [[ -n "$table_value" && "$table_value" != "$const_value" ]]; then
        detail+="SAMPLE_SCHEMA_VERSION=\"$const_value\" ≠ AUDIT-LABEL-SCHEMA.md table-row schema_version=\"$table_value\""$'\n'
    fi
    if [[ -n "$detail" ]]; then
        fail_block "schema-version-const-doc" \
                   "SAMPLE_SCHEMA_VERSION const ≠ AUDIT-LABEL-SCHEMA.md schema_version value (wire-contract drift)" \
                   "$detail"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 8 — `SampleRecord` field set in audit.rs ⊆ documented v2
# fields in AUDIT-LABEL-SCHEMA.md.
#
# Rationale: SampleRecord is the on-the-wire serde shape for the
# `*.jsonl` sample file. Adding a field to the struct without
# documenting it leaks an undocumented field into the wire contract
# and breaks external-labeller authoring (the labeller has no spec to
# code against). Documenting more than the struct exposes is fine
# (forward-reservation), so this check is one-way: every code-side
# field must appear in the doc table.
#
# Drift shape caught: a `pub <name>: …` field inside `pub struct
# SampleRecord { … }` whose `<name>` has no `\`<name>\`` cell in the
# AUDIT-LABEL-SCHEMA.md record-schema table.
check_sample_record_fields() {
    local audit_rs="gumiho-mudang-cli/src/commands/audit.rs"
    local schema_doc="$SCOPE_DOCS/AUDIT-LABEL-SCHEMA.md"
    [[ -f "$audit_rs" && -f "$schema_doc" ]] || return 0

    # Extract field names from the SampleRecord struct body. BSD awk
    # (macOS default) does not support gawk's three-argument `match()`,
    # so the body is sliced with an open-ended range pattern and the
    # field-line shape is matched and trimmed with `sub` (POSIX awk).
    local code_fields
    code_fields="$(awk '
        /pub struct SampleRecord[[:space:]]*\{/ { in_struct = 1; next }
        in_struct {
            if ($0 ~ /^\}/) { exit }
            line = $0
            sub(/^[[:space:]]*pub[[:space:]]+/, "", line)
            if (line ~ /^[a-z_][a-z0-9_]*:/) {
                sub(/:.*$/, "", line)
                print line
            }
        }
    ' "$audit_rs" | sort -u)"

    if [[ -z "$code_fields" ]]; then
        fail_block "sample-record-fields" \
                   "pub struct SampleRecord { … } not found in $audit_rs (gate cannot validate against an absent anchor)" \
                   "expected: a top-level \`pub struct SampleRecord { pub <field>: <type>, … }\` block"
        return
    fi

    # Extract documented field names from the record-schema table —
    # rows whose first cell is `\`<name>\``.
    local doc_fields
    doc_fields="$(grep -oE '^\| `[a-z_][a-z0-9_]*` \|' "$schema_doc" \
                  | sed -E 's/^\| `([a-z_][a-z0-9_]*)` \|/\1/' \
                  | sort -u)"

    local missing
    missing="$(comm -23 <(printf '%s\n' "$code_fields") <(printf '%s\n' "$doc_fields"))"
    if [[ -n "$missing" ]]; then
        local detail="SampleRecord fields without a row in AUDIT-LABEL-SCHEMA.md record-schema table:"$'\n'
        while IFS= read -r f; do [[ -n "$f" ]] && detail+="  $f"$'\n'; done <<< "$missing"
        fail_block "sample-record-fields" \
                   "SampleRecord field set ⊄ AUDIT-LABEL-SCHEMA.md documented fields (undocumented wire field)" \
                   "$detail"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 9 — `CoverageSummary` struct fields in audit.rs ↔
# documented field names in ENFORCEMENT-MAP.md § R8.
#
# Rationale: ENFORCEMENT-MAP.md § R8 is the canonical surface
# description for the audit subcommand. Its `coverage_summary`
# bullet enumerates the top-level coverage object's field names
# verbatim. CoverageSummary is the serde shape; serde renames are
# disallowed by structure (the struct is direct-serialised). A drift
# (struct field renamed, doc still cites old name) silently breaks
# any operator who reads the doc and reaches for the field by the
# documented name.
#
# Drift shape caught: struct field set ≠ doc field set, either
# direction.
check_coverage_summary_fields() {
    local audit_rs="gumiho-mudang-cli/src/commands/audit.rs"
    local map="$SCOPE_DOCS/ENFORCEMENT-MAP.md"
    [[ -f "$audit_rs" && -f "$map" ]] || return 0

    local code_fields
    code_fields="$(awk '
        /pub struct CoverageSummary[[:space:]]*\{/ { in_struct = 1; next }
        in_struct {
            if ($0 ~ /^\}/) { exit }
            line = $0
            sub(/^[[:space:]]*pub[[:space:]]+/, "", line)
            if (line ~ /^[a-z_][a-z0-9_]*:/) {
                sub(/:.*$/, "", line)
                print line
            }
        }
    ' "$audit_rs" | sort -u)"

    if [[ -z "$code_fields" ]]; then
        fail_block "coverage-summary-fields" \
                   "pub struct CoverageSummary { … } not found in $audit_rs (gate cannot validate against an absent anchor)" \
                   "expected: a top-level \`pub struct CoverageSummary { pub <field>: <type>, … }\` block"
        return
    fi

    # Extract the documented field set from the R8 coverage_summary
    # bullet. The bullet has a single line shape:
    #   …`coverage_summary` carries top-level coverage (`field_a`,
    #   `field_b`, `field_c`, …)…
    # The line itself contains many other backticked identifiers
    # (other JSON envelope keys, per-row coverage fields). Scope the
    # extraction to the parenthetical that *immediately follows* the
    # phrase "`coverage_summary` carries top-level coverage" so
    # neighbouring identifiers stay out.
    local parenthetical
    parenthetical="$(sed -nE 's/.*`coverage_summary` carries top-level coverage \(([^)]*)\).*/\1/p' "$map" 2>/dev/null \
                     | head -n1 || true)"
    if [[ -z "$parenthetical" ]]; then
        fail_block "coverage-summary-fields" \
                   "ENFORCEMENT-MAP.md § R8 missing the coverage_summary bullet (the gate's doc anchor)" \
                   "expected: a line of shape \"…\`coverage_summary\` carries top-level coverage (\`field_a\`, \`field_b\`, …)\""
        return
    fi
    local doc_fields
    doc_fields="$(printf '%s' "$parenthetical" \
                  | grep -oE '`[a-z_][a-z0-9_]*`' \
                  | sed -E 's/`//g' \
                  | sort -u)"

    local missing extra
    missing="$(comm -23 <(printf '%s\n' "$code_fields") <(printf '%s\n' "$doc_fields"))"
    extra="$(comm -13 <(printf '%s\n' "$code_fields") <(printf '%s\n' "$doc_fields"))"
    local detail=""
    if [[ -n "$missing" ]]; then
        detail+="CoverageSummary fields not in ENFORCEMENT-MAP.md § R8 coverage_summary bullet:"$'\n'
        while IFS= read -r f; do [[ -n "$f" ]] && detail+="  $f"$'\n'; done <<< "$missing"
    fi
    if [[ -n "$extra" ]]; then
        detail+="ENFORCEMENT-MAP.md § R8 coverage_summary fields not in CoverageSummary struct:"$'\n'
        while IFS= read -r f; do [[ -n "$f" ]] && detail+="  $f"$'\n'; done <<< "$extra"
    fi
    if [[ -n "$detail" ]]; then
        fail_block "coverage-summary-fields" \
                   "CoverageSummary struct field set ≠ ENFORCEMENT-MAP.md § R8 coverage_summary fields" \
                   "$detail"
    fi
}

# ─────────────────────────────────────────────────────────────
# Check 10 — `edge_audit_history` column set in schema.sql ↔
# documented column set in ENFORCEMENT-MAP.md § R0.
#
# Rationale: sprint 0004 (j) carved a writable namespace into the
# auditor-immutability rule and pinned the audit-history table's
# columns in R0's schema closure. The SQL schema and the R-entry must
# stay in lockstep — a column added to the table without an R0 update
# means the closure description lies; a column documented but not
# created means readers reach for fields that do not exist.
#
# Drift shape caught: schema.sql column set ≠ R0 bullet column set,
# either direction.
check_edge_audit_history_columns() {
    local schema_sql="gumiho-mudang-scope/scope-graph/src/sql/schema.sql"
    local map="$SCOPE_DOCS/ENFORCEMENT-MAP.md"
    [[ -f "$schema_sql" && -f "$map" ]] || return 0

    # Extract column names from the CREATE TABLE block. BSD awk
    # (macOS default) does not support gawk's three-argument
    # `match()`, so column lines are trimmed with `sub` (POSIX awk).
    local sql_columns
    sql_columns="$(awk '
        /CREATE TABLE IF NOT EXISTS edge_audit_history[[:space:]]*\(/ { in_table = 1; next }
        in_table {
            if ($0 ~ /^\)[[:space:]]*;/) { exit }
            line = $0
            sub(/^[[:space:]]+/, "", line)
            if (line ~ /^[a-z_][a-z0-9_]*[[:space:]]+(INTEGER|TEXT|REAL|BLOB|NUMERIC)/) {
                sub(/[[:space:]]+.*$/, "", line)
                print line
            }
        }
    ' "$schema_sql" | sort -u)"

    if [[ -z "$sql_columns" ]]; then
        fail_block "edge-audit-history-columns" \
                   "CREATE TABLE IF NOT EXISTS edge_audit_history (…) not found in $schema_sql (the gate cannot validate against an absent anchor)" \
                   "expected: a CREATE TABLE block for edge_audit_history with explicit column declarations"
        return
    fi

    # Extract documented column names from the R0
    # "Audit-derived rows (writable namespace)" bullet. The bullet
    # carries the full CREATE-TABLE signature inline as one backtick
    # span: `edge_audit_history (col1 TYPE NOT NULL, col2 TYPE …)`.
    # Grab the span, peel the parens, split on commas, take the first
    # identifier of each entry, drop the CHECK-clause noise.
    local doc_span
    doc_span="$(grep -E '^[[:space:]]+- \*\*Audit-derived rows \(writable namespace\)\*\*' "$map" \
                | head -n1 \
                | grep -oE '`edge_audit_history \([^`]+\)`' \
                | head -n1)"
    if [[ -z "$doc_span" ]]; then
        fail_block "edge-audit-history-columns" \
                   "ENFORCEMENT-MAP.md § R0 missing the edge_audit_history schema bullet (the gate's doc anchor)" \
                   "expected: a bullet of shape \"- **Audit-derived rows (writable namespace)**: \`edge_audit_history (col1 TYPE …, col2 TYPE …)\` — …\""
        return
    fi
    local doc_columns
    doc_columns="$(printf '%s' "$doc_span" \
                   | sed -E 's/^`edge_audit_history \(//; s/\)`$//' \
                   | tr ',' '\n' \
                   | sed -E 's/^[[:space:]]+//; s/[[:space:]]+.*$//' \
                   | grep -E '^[a-z_][a-z0-9_]*$' \
                   | sort -u)"

    local missing extra
    missing="$(comm -23 <(printf '%s\n' "$sql_columns") <(printf '%s\n' "$doc_columns"))"
    extra="$(comm -13 <(printf '%s\n' "$sql_columns") <(printf '%s\n' "$doc_columns"))"
    local detail=""
    if [[ -n "$missing" ]]; then
        detail+="schema.sql edge_audit_history columns not documented in ENFORCEMENT-MAP.md § R0:"$'\n'
        while IFS= read -r c; do [[ -n "$c" ]] && detail+="  $c"$'\n'; done <<< "$missing"
    fi
    if [[ -n "$extra" ]]; then
        detail+="ENFORCEMENT-MAP.md § R0 edge_audit_history columns not in schema.sql:"$'\n'
        while IFS= read -r c; do [[ -n "$c" ]] && detail+="  $c"$'\n'; done <<< "$extra"
    fi
    if [[ -n "$detail" ]]; then
        fail_block "edge-audit-history-columns" \
                   "schema.sql edge_audit_history column set ≠ ENFORCEMENT-MAP.md § R0 column set" \
                   "$detail"
    fi
}

# ─────────────────────────────────────────────────────────────
# Future sprint extension hooks. Each later sprint in Priority 1 adds
# ONE function here (named `check_<short_name>`) and invokes it from
# `main()`. See SELF-CORRECTION-CYCLE.md § "Extending the doc-sync
# gate" for the per-sprint table.
#
# Sprints expected to extend this script:
#   - 0006 (i): documented default aggregation policy ↔ aggregator default
#   - 0007 (c): audit-ci / audit-nightly recipes ↔ CI-GATES.md rows
#   - 0009 (k): audit-trail path doc ↔ indexer-read path

# ─────────────────────────────────────────────────────────────
main() {
    check_enforcement_map_paths
    check_ci_gates_recipes
    check_doc_relative_links
    check_cycle_docs_indexed
    check_audit_samples_layout
    check_lang_version_detector_modules
    check_schema_version_const_doc
    check_sample_record_fields
    check_coverage_summary_fields
    check_edge_audit_history_columns

    if [[ "$FAILED" -ne 0 ]]; then
        echo "doc-sync gate: FAIL" >&2
        exit 1
    fi
    echo "doc-sync gate: pass"
}

main "$@"
