# Contributing to Cortex

## Quick Start
1. Fork & clone
2. `bash scripts/bootstrap.sh` (if not already bootstrapped)
3. `cd runtime && cargo build`
4. `cd extractors && npm install`
5. Make changes, run `make test`, submit PR

## Easiest Contribution: Write an Extractor
See `docs/guides/writing-extractors.md`.

## Code Style
- Rust: `cargo fmt` + `cargo clippy -- -D warnings`
- TypeScript: strict mode, no `any`
- Python: `ruff` + `mypy --strict`, zero external dependencies

## Pull Requests
- Tests required for all changes
- CI must pass
- One maintainer review required
