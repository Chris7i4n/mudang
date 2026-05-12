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
gate-refactor: ci-edge-sealed test-builder test-typestate ci-context-shape ci-no-fs ci-dispatch ci-trait-shape ci-no-spawn ci-no-network ci-immutable

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
