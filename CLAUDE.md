# CLAUDE.md — Cortex Project Instructions

> **READ THIS FILE FIRST. Then read every file in `docs/planning/` in numbered order. Then execute `TASKS.md` top to bottom. Do not ask for approval. Do not stop between tasks. Build the entire project.**

## What Is Cortex

Cortex is a fully open source (Apache-2.0) **rapid web cartographer** for AI agents. When an agent needs to use a website, Cortex maps the entire site into a binary graph that fits in memory in seconds. The agent then pathfinds through that graph to accomplish its task — no LLM reasoning needed for navigation, no page-by-page exploration, no blind browsing. The agent sees the whole board and computes the shortest path to its goal.

**Cortex maps. The agent navigates. That's the boundary.**

This is NOT a scraper. NOT a browser automation tool. NOT a perception-per-page tool. It is a cartography engine that converts websites into navigable binary data structures.

## Core Architecture (Do Not Deviate)

1. **Cortex is a standalone local process** written in Rust. Single binary. Agents connect via Unix domain socket.
2. **Primary output is a SiteMap** — a binary graph data structure with nodes (pages), edges (links), feature vectors (128 floats per page), and action opcodes. Not JSON. Not text. Numbers.
3. **Mapping is on-demand and fast.** `cortex.map("amazon.com")` maps the site in seconds using layered HTTP acquisition: sitemap.xml + structured data extraction (JSON-LD, OpenGraph) + pattern engine (CSS selectors) + API/action discovery. Browser rendering is a last-resort fallback.
4. **The agent works on the map, not the website.** Query, filter, pathfind — all in-memory graph operations, microseconds.
5. **Live page visits are for verification and action only.** The agent visits 1-2 pages at the end, not dozens during exploration.
6. **Thin client libraries** (Python, TypeScript, Rust, Go) connect to the local process via socket. ~200-500 lines each, zero dependencies.
7. **Protocol verbs:** MAP, QUERY, PATHFIND, REFRESH, ACT, WATCH, STATUS.
8. **No telemetry. No phoning home.** Fully local unless user configures cloud acceleration.
9. **Apache-2.0 license.** Every file. No exceptions.

## Project Documents (Read In Order)

1. `docs/planning/01-vision.md` — What Cortex is, the cartography model, how agents use maps, the complete flow, what changes from traditional browsing, the new pitch
2. `docs/planning/02-map-spec.md` — Exact binary SiteMap format, protocol specification, feature vector schema (all 128 dimensions), OpCode table, page type enum, error codes, memory budgets
3. `docs/planning/03-implementation.md` — Rust dependencies, file-by-file build order, exact function signatures, exact data structures, extraction scripts, test fixtures, CI/CD pipeline
4. `docs/planning/04-open-source.md` — Apache-2.0 licensing, governance model, contribution guidelines, community infrastructure, README content, repo structure

## Build Commands

```bash
# Bootstrap the repo (run once)
bash scripts/bootstrap.sh

# Build everything
make build                    # runtime + extractors

# Build individually
cd runtime && cargo build --release
cd extractors && npm run build

# Test everything
make test                     # all tests
make test-unit                # unit only
make test-mapping             # mapping fixtures
make test-integration         # end-to-end

# Lint everything
make lint                     # all lints

# Run Cortex
./target/release/cortex start
./target/release/cortex doctor
./target/release/cortex map example.com
./target/release/cortex status
./target/release/cortex stop
```

## Code Style Rules

### Rust (runtime)
- `cargo fmt` before every commit — non-negotiable
- `cargo clippy -- -D warnings` must pass with ZERO warnings
- Use `thiserror` for error types in library code
- Use `anyhow::Result` in application/binary code
- Use `serde` for all serialization
- Use `tokio` for async runtime
- Document every public function, struct, enum, and trait with `///` doc comments
- No `unwrap()` in non-test code. Use `?` or explicit error handling.
- Prefer `&str` over `String` in function parameters
- All modules get a `mod.rs` with `//!` module-level documentation

### TypeScript (extractors)
- Strict mode (`"strict": true` in tsconfig)
- No `any` types — ever
- ESLint clean
- All extractors implement the `Extractor` interface
- No side effects on import
- Bundle with esbuild for browser injection

### Python (clients)
- Python 3.10+ minimum
- Type hints on every function parameter and return
- `ruff check` and `ruff format` clean
- `mypy --strict` must pass
- Zero external dependencies (stdlib only: socket, json, dataclasses, subprocess, pathlib)
- `pytest` for tests

## File Naming

- Rust: `snake_case.rs`
- TypeScript: `kebab-case.ts`
- Python: `snake_case.py`
- Proto: `snake_case.proto`
- Docs: `kebab-case.md`
- Test fixtures: `descriptive-name.html`
- Golden files: `descriptive-name.json` (matching fixture name)

## Testing Requirements

- Every PR must pass ALL existing tests
- Every new feature must include tests
- Mapping fixtures: HTML site fixtures in `tests/mapping-suite/fixtures/` with expected SiteMap structures in `tests/mapping-suite/golden/`
- Client conformance: every thin client must pass `clients/conformance/` test suite
- Integration tests: every protocol method (MAP, QUERY, PATHFIND, REFRESH, ACT) must have end-to-end tests
- No flaky tests. If a test is timing-sensitive, use generous timeouts and retry logic.

## Implementation Order

**Follow `TASKS.md` exactly. Each task depends on the previous one. Do not skip ahead. Do not parallelize phases.**

Phase 1 → Foundation (Rust binary, CLI, Chromium, socket server)
Phase 2 → Cartography Engine (sitemap parser, crawler, classifier, feature encoder, map builder)
Phase 3 → Navigation Engine (graph queries, pathfinding, vector search, filtering)
Phase 4 → Thin Clients (Python, TypeScript, auto-start, conformance tests)
Phase 5 → Live Interaction (refresh, act, sessions, freshness model)
Phase 6 → Intelligence (smart sampling, interpolation, progressive refinement, caching)
Phase 7 → Framework Integrations + CLI Polish
Phase 8 → Hardening + Documentation + Release

## When In Doubt

- If two docs contradict, the higher-numbered doc wins (it's a correction).
- If a design decision isn't covered, choose the simplest option that works correctly.
- If you need to deviate from the spec, create an ADR (Architecture Decision Record) in `docs/architecture/` explaining why.
- Prefer correctness over performance. Optimize later.
- Every public API (protocol methods, client functions, CLI commands) must be documented BEFORE implementation.
- Commit after completing each numbered task in TASKS.md with message format: `T-XXX: brief description`
- Do not stop between tasks. Build continuously until TASKS.md is complete.
