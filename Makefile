# Local Checks Only
#
# This Makefile exists solely for running pre-commit checks locally.
# It is NOT intended as a build system, task runner, or CI configuration.
# Please do not extend it for other purposes.

# Disable incremental compilation to prevent cache bloat
export CARGO_INCREMENTAL=0

.PHONY: check clippy eure-ls-wasm test test-no-default-features fmt-check test-suite eure-check

check: fmt-check clippy test-no-default-features test test-suite eure-check eure-ls-wasm
	@echo "All checks passed."

clippy:
	@cargo clippy -q --offline -- -D warnings && echo "clippy passed"

eure-ls-wasm:
	@rustup target add wasm32-unknown-unknown && cargo clippy -p eure-ls --target wasm32-unknown-unknown --offline -- -D warnings && echo "eure-ls-wasm passed"

test:
	@if command -v cargo-nextest >/dev/null 2>&1; then cargo nextest run --all-targets --all-features --offline --show-progress none --status-level fail --final-status-level fail; else cargo test -q --all-targets --all-features; fi && echo "test passed"

test-no-default-features:
	@if command -v cargo-nextest >/dev/null 2>&1; then cargo nextest run --all-targets --no-default-features --offline --show-progress none --status-level fail --final-status-level fail; else cargo test -q --all-targets --no-default-features; fi && echo "test passed"

fmt-check:
	@cargo fmt --check && echo "fmt-check passed"

test-suite:
	@cargo run --quiet --offline -p test-suite -- --quiet

eure-check:
	@cargo run --quiet --offline --bin eure -- check --all --quiet
