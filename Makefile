# Local Checks Only
#
# This Makefile exists solely for running pre-commit checks locally.
# It is NOT intended as a build system, task runner, or CI configuration.
# Please do not extend it for other purposes.

.PHONY: check clippy test fmt-check test-suite eure-check

check: clippy test fmt-check test-suite eure-check
	@echo "All checks passed."

clippy:
	cargo clippy

test:
	cargo test

fmt-check:
	cargo fmt --check

test-suite:
	cargo run -p test-suite

eure-check:
	cargo run --bin eure -- check --all
