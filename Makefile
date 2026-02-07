.PHONY: build test fmt lint smoke run-example clean check

# Default example for run-example target
EX ?= hello.ags

## Build the entire workspace
build:
	cargo build --workspace

## Run all tests
test:
	cargo test --workspace

## Format all code
fmt:
	cargo fmt --all

## Check formatting without modifying files
fmt-check:
	cargo fmt --check --all

## Run clippy lints
lint:
	cargo clippy --workspace

## Run a specific example (usage: make run-example EX=hello.ags)
run-example:
	cargo run -p agentus-cli -- exec examples/$(EX)

## Fast verification: build + test + run 2 examples
smoke: build test
	@echo "--- Running hello.ags ---"
	@cargo run -p agentus-cli -- exec examples/hello.ags
	@echo "--- Running agent_basic.ags ---"
	@cargo run -p agentus-cli -- exec examples/agent_basic.ags
	@echo ""
	@echo "=== Smoke test PASSED ==="

## Full check: fmt-check + lint + test + smoke examples
check: fmt-check lint test
	@cargo run -p agentus-cli -- exec examples/hello.ags > /dev/null
	@cargo run -p agentus-cli -- exec examples/agent_basic.ags > /dev/null
	@cargo run -p agentus-cli -- exec examples/tools.ags > /dev/null
	@echo "=== Full check PASSED ==="

## Clean build artifacts
clean:
	cargo clean
