# gumiho-mudang justfile
# Run `just` to list available recipes.

default:
    @just --list

# --- Build ---

build:
    cargo build --workspace

release:
    cargo build --release --workspace

release-fast:
    cargo build --profile release-fast --workspace

# --- Lint / format ---

check:
    cargo check --workspace --all-targets

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# --- Test ---

test:
    cargo test --workspace

test-fast:
    cargo nextest run --workspace --profile dev-fast

test-changed:
    cargo nextest run --workspace --changed-since main --profile dev

# Per-crate convenience.
test-scope:
    cargo test -p gumiho-mudang-scope

test-lsp:
    cargo test -p gumiho-mudang-lsp

# CLI crate is also where cross-crate integration tests live.
test-cli:
    cargo test -p gumiho-mudang-cli

# Alias for test-cli — readable when the intent is "integration tests".
test-integration: test-cli

# --- Gates ---

# Pre-commit gate. Run before pushing.
gate: fmt-check clippy test

# --- Refactor CI gates (gumiho-mudang-scope/docs/CI-GATES.md) ---
#
# Until repo-wide CI lands, `just gate-refactor` is the durable
# contract: every gate flipped to `active` in CI-GATES.md is run by
# this recipe. See sprint 0001 DOD #3.

# Run every active refactor gate.
gate-refactor: ci-edge-sealed test-builder test-typestate ci-context-shape ci-no-fs ci-dispatch ci-trait-shape ci-no-spawn ci-no-network ci-immutable ci-no-framework-scm ci-patterns ci-output-schema audit-confidence test-malformed gate-charter

ci-edge-sealed:
    ./scripts/grep_edge_sealed.sh

test-builder:
    cargo test -p scope-core --test compile_fail_builder

test-typestate:
    cargo test -p scope-graph --test compile_fail_typestate

ci-context-shape:
    ./scripts/audit_context_shape.sh

ci-no-fs:
    ./scripts/grep_no_fs.sh

ci-dispatch:
    ./scripts/grep_dispatch.sh

ci-trait-shape:
    ./scripts/audit_trait_shape.sh

ci-no-spawn:
    ./scripts/audit_no_spawn.sh

ci-no-network:
    ./scripts/audit_no_network.sh

ci-immutable:
    ./scripts/audit_immutable.sh

ci-no-framework-scm:
    ./scripts/audit_no_framework_scm.sh

ci-patterns:
    ./scripts/audit_patterns.sh

ci-output-schema:
    ./scripts/audit_output_schema.sh

# R8 (sprint 0007) — Confidence audit subcommand regression gate.
#
# Runs the integration suite that exercises every chunk-4-to-6 surface:
# JSONL emit shape, --label parsing, SHA-256 drift gate, schema_version
# rejection, --emit-sample/--label mutex, tier gate pass + fail, JSON +
# TSV report shape. This is the *mechanical regression* gate — wiring
# break of the subcommand, SampleRecord field-order drift (would break
# external labellers), drift-gate removal, tier-target loosening, etc.
#
# This is NOT the continuous re-audit cycle. Committed labelled samples
# + cross-reindex join key + precision-drift detection over time are
# post-refactor work — see POST-REFACTOR-PLAN.md § Priority 1 —
# Self-correction cycle.
audit-confidence:
    cargo test -p gumiho-mudang-cli --test test_audit_confidence

# R6 (sprint 0008) — Malformed-source resilience harness.
#
# Runs the integration test that walks every fixture under
# `scope-core/tests/fixtures/malformed/<lang>/<case>/` and asserts the
# four R6 acceptance contracts: no panic, parseable prefix produces
# ≥ 1 symbol, `skipped_ranges` non-empty when partially malformed,
# `insta` snapshot pins the recorded reason + range. Snapshot files
# under `scope-core/tests/snapshots/malformed_sources/` are the
# authoritative line-range record; regressions surface as snapshot
# diffs.
test-malformed:
    cargo test -p scope-core --test malformed_sources

# Sprint 0009 (Phase E close) — Charter sweep gate.
#
# Refuses re-introduction of any compatibility-shim shape that sprint
# 0009 chunks 1 and 2 retired. Each check in the script targets a
# specific shim — the `gumiho_mudang_scope::core::*` namespace-synth,
# the `pub type Edge = RawEdge` alias, `INSERT OR IGNORE`, the
# `scope impact` CLI command, the `command_label: &'static str`
# deprecation-alias parameter, etc. Charter § 2 (single-operator
# posture) + § 3 invariant 8 (no backward-compatibility shims) are
# the source of truth; this script is the mechanical successor to
# the chunk-2 manual grep pass.
gate-charter:
    ./scripts/gate_charter.sh

# --- Install ---

# Install the `mudang` binary into ~/.cargo/bin using the
# release-fast profile (lto fat + codegen-units=1).
install:
    cargo install --path gumiho-mudang-cli --profile release-fast --locked --force

uninstall:
    cargo uninstall gumiho-mudang-cli

# --- Tooling ---

tools-install:
    cargo install cargo-nextest cargo-deny --locked

# --- Cleanup ---

clean:
    cargo clean
