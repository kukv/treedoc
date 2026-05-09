.DEFAULT_GOAL := help

.PHONY: help test fmt lint check release-dry release-patch release-minor release-major

help:
	@echo "Targets:"
	@echo "  test            Run cargo test"
	@echo "  fmt             Apply cargo fmt"
	@echo "  lint            Run cargo clippy with -D warnings"
	@echo "  check           fmt --check + clippy + test (CI parity)"
	@echo ""
	@echo "  release-dry     Dry-run a patch release (no commit/tag/push)"
	@echo "  release-patch   Bump patch  (0.1.0 -> 0.1.1) and push tag"
	@echo "  release-minor   Bump minor  (0.1.0 -> 0.2.0) and push tag"
	@echo "  release-major   Bump major  (0.1.0 -> 1.0.0) and push tag"

test:
	mise exec -- cargo test

fmt:
	mise exec -- cargo fmt --all

lint:
	mise exec -- cargo clippy --all-targets -- -D warnings

check:
	mise exec -- cargo fmt --all -- --check
	mise exec -- cargo clippy --all-targets -- -D warnings
	mise exec -- cargo test

release-dry:
	mise exec -- cargo release patch

release-patch:
	mise exec -- cargo release patch --execute --no-confirm

release-minor:
	mise exec -- cargo release minor --execute --no-confirm

release-major:
	mise exec -- cargo release major --execute --no-confirm
