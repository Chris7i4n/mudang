.DEFAULT_GOAL := check

.PHONY: check test fmt

check:
	cargo clippy --all-targets -- -D warnings
	cargo fmt --all -- --check

test:
	cargo test

fmt:
	cargo fmt --all
