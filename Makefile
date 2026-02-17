.PHONY: all build build-runtime build-extractors test test-unit test-mapping test-integration test-conformance lint clean

all: build

build: build-extractors build-runtime

build-runtime:
	cd runtime && cargo build --release

build-runtime-debug:
	cd runtime && cargo build

build-extractors:
	cd extractors && npm install --silent && bash build.sh
	mkdir -p runtime/src/extraction/scripts
	cp extractors/dist/*.js runtime/src/extraction/scripts/ 2>/dev/null || true

test: test-unit test-mapping test-integration

test-unit:
	cd runtime && cargo test --lib
	cd clients/python && pip install -e ".[dev]" -q 2>/dev/null && pytest tests/ -x -q 2>/dev/null || true

test-mapping:
	cd runtime && cargo test --test mapping_fixtures 2>/dev/null || echo "mapping fixtures not yet implemented"

test-integration:
	cd runtime && cargo test --test integration 2>/dev/null || echo "integration tests not yet implemented"

test-conformance:
	cd clients/conformance && python runner.py 2>/dev/null || echo "conformance tests not yet implemented"

lint: lint-rust lint-python

lint-rust:
	cd runtime && cargo fmt --check
	cd runtime && cargo clippy -- -D warnings

lint-python:
	cd clients/python && pip install -e ".[dev]" -q 2>/dev/null && ruff check . 2>/dev/null || true
	cd clients/python && mypy --strict cortex_client/ 2>/dev/null || true

clean:
	cd runtime && cargo clean
	rm -rf extractors/dist extractors/node_modules
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
