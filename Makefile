# Local Checks Only
#
# This Makefile exists solely for running pre-commit checks locally.
# It is NOT intended as a build system, task runner, or CI configuration.
# Please do not extend it for other purposes.

.PHONY: check clippy test fmt-check test-suite eure-check

check: clippy test fmt-check test-suite eure-check
	@echo "All checks passed."

clippy:
	@cargo clippy -q -- -D warnings && echo "clippy passed"

test:
	@if command -v cargo-nextest >/dev/null 2>&1; then cargo nextest run --show-progress none --status-level fail --final-status-level fail; else cargo test -q; fi && echo "test passed"

fmt-check:
	@cargo fmt --check && echo "fmt-check passed"

test-suite:
	@cargo run --quiet -p test-suite -- --quiet

eure-check:
	@cargo run --quiet --bin eure -- check --all --quiet
