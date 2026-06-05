.PHONY: help dev build test lint fmt audit

help:
	@echo "Usage: make <target>"
	@echo ""
	@echo "  dev     Start the WASM dev server (hot-reload at http://localhost:8080)"
	@echo "  build   Compile the native desktop binary"
	@echo "  test    Run unit and integration tests"
	@echo "  lint    Run clippy (zero warnings enforced)"
	@echo "  fmt     Format source code"
	@echo "  audit   Run cargo audit + cargo deny"

dev:
	docker compose up --build

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

audit:
	cargo audit && cargo deny check
