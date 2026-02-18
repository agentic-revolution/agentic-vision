# Changelog

## v0.2.1 (2026-02-18)

### Improvements

- **Pattern engine**: Expanded CSS selector coverage in `css_selectors.json` for broader price, rating, and availability extraction
- **Platform actions**: Added detection patterns for Magento and BigCommerce in `platform_actions.json`
- **HTTP action execution**: Broader form detection across site types
- **Test suite v2**: 100-site test suite with data source quality and action discovery scoring

## v0.2.0 (2026-02-18)

### Major Changes — Layered Acquisition Engine (No-Browser Mapping)

- **HTTP-first site mapping**: Sites are mapped via layered HTTP acquisition instead of browser rendering. Sitemap.xml, robots.txt, HEAD scanning, and feed discovery (Layer 0), then HTTP GET with JSON-LD/OpenGraph/meta tag parsing (Layer 1), pattern engine CSS selectors (Layer 1.5), API discovery for known platforms (Layer 2), and action discovery (Layer 2.5). Browser rendering is Layer 3 fallback only.
- **Structured data extraction** (`acquisition/structured.rs`): JSON-LD, OpenGraph, meta tags, headings, forms, and links extracted from raw HTML without rendering
- **Pattern engine** (`acquisition/pattern_engine.rs`): CSS selector database (`css_selectors.json`) for extracting prices, ratings, availability from HTML when structured data is sparse
- **HTTP action discovery** (`acquisition/action_discovery.rs`): Discovers forms, JS endpoints, and platform-specific actions. Many actions (add-to-cart, search) executable via HTTP POST
- **Platform action templates** (`acquisition/platform_actions.json`): Pre-built action definitions for Shopify, WooCommerce, Magento, BigCommerce
- **HTTP session & authentication** (`acquisition/http_session.rs`, `acquisition/auth.rs`): Password login, OAuth, API key authentication — password login works without a browser
- **JS analyzer** (`acquisition/js_analyzer.rs`): Extracts API endpoints and platform indicators from JavaScript sources
- **Deleted**: `crawler.rs`, `sampler.rs`, `interpolator.rs`, `smart_sampler.rs` — replaced by acquisition engine

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
