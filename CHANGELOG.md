# Changelog

## v0.1.0 (2026-02-17)

Initial release of Cortex — the rapid web cartographer for AI agents.

### Features

- **SiteMap binary format (CTX)**: 128-dimension feature vectors, PageType classification, OpCode action encoding, CSR edge structure, k-means clustering
- **Cartography engine**: Robots.txt parsing, sitemap discovery, URL classification, page rendering with Chromium, feature encoding, action encoding, sample-based mapping with interpolation
- **Navigation engine**: Query by page type / feature ranges / flags, Dijkstra pathfinding, cosine similarity nearest-neighbor search, k-means clustering
- **Live interaction**: Perceive (single page encoding), Refresh (re-render nodes), Act (execute browser actions), Watch (monitor changes), persistent browser sessions
- **Intelligence layer**: Map caching with TTL, progressive rendering queue, smart sampling, cross-site query merging
- **Trust & safety**: Credential vault (SQLite), PII detection (email, phone, SSN, credit card), input sanitization (XSS, SQL injection, path traversal)
- **Audit logging**: JSONL append-only log, optional remote ledger sync
- **Stealth**: Browser fingerprint patching, human-like timing delays
- **Python client**: Zero-dependency stdlib-only client with auto-start, SiteMap wrapper, full type annotations
- **TypeScript client**: Unix socket client with full type exports
- **Framework integrations**: LangChain tools, CrewAI tool, OpenClaw skills
- **CLI**: `start`, `stop`, `doctor`, `status`, `map`, `query`, `pathfind`, `perceive`, `install` commands
- **CI/CD**: GitHub Actions for Rust/Python/TypeScript, multi-platform release builds
