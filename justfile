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
#
# Tests run in --release because integration tests have timing
# assertions calibrated to optimised binaries (e.g. incremental
# indexing budget < 2 s). Debug builds blow these budgets.

test:
    cargo test --workspace --release

test-fast:
    cargo nextest run --workspace --profile dev-fast

test-changed:
    cargo nextest run --workspace --changed-since main --profile dev --release

# Per-crate convenience.
test-scope:
    cargo test -p gumiho-mudang-scope --release

test-lsp:
    cargo test -p gumiho-mudang-lsp --release

# CLI crate is also where cross-crate integration tests live.
test-cli:
    cargo test -p gumiho-mudang-cli --release

# Alias for test-cli — readable when the intent is "integration tests".
test-integration: test-cli

# --- Gates ---

# Pre-commit gate. Run before pushing.
gate: fmt-check clippy test

# --- Tooling ---

tools-install:
    cargo install cargo-nextest cargo-deny --locked

# --- Cleanup ---

clean:
    cargo clean
