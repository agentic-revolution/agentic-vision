# Cortex

**Rapid web cartographer for AI agents.** Map entire websites into navigable binary graphs in seconds. Agents navigate the map, not the site.

[![CI](https://github.com/cortex-ai/cortex/actions/workflows/ci.yml/badge.svg)](https://github.com/cortex-ai/cortex/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

## Quick Start

```bash
# Install
cargo install cortex-runtime
cortex install    # Download Chromium for Testing
cortex doctor     # Verify setup

# Map a site
cortex start
cortex map example.com

# Query from Python
python3 -c "
import cortex_client
site = cortex_client.map('example.com')
print(f'{site.node_count} nodes, {site.edge_count} edges')
for p in site.filter(page_type=4, limit=5):
    print(f'  {p.url}')
"
```

## How It Works

1. **Map** — Cortex crawls a domain, renders sample pages with headless Chromium, extracts structured data, and encodes every page into a 128-dimension feature vector. Unrendered pages are interpolated from rendered samples.

2. **Navigate** — Query the map by page type, feature ranges, or vector similarity. Pathfind between any two nodes using Dijkstra's algorithm on the edge graph.

3. **Act** — Execute actions on live pages (click buttons, fill forms, add to cart) through persistent browser sessions with cookie state.

## Architecture

```
Agent  ──→  Client (Python/TS)  ──→  Unix Socket  ──→  Cortex Runtime
                                                          ├── Cartography Engine
                                                          ├── Navigation Engine
                                                          ├── Live Interaction
                                                          ├── Intelligence Layer
                                                          └── Chromium Pool
```

## Feature Vector (128 dimensions)

| Range | Category | Examples |
|-------|----------|----------|
| 0-15 | Page identity | page_type, confidence, depth, load_time |
| 16-47 | Content metrics | text_density, heading_count, image_count |
| 48-63 | Commerce | price, rating, stock_status, discount |
| 64-79 | Navigation | link_count, form_count, pagination |
| 80-95 | Trust/safety | https, pii_detected, auth_required |
| 96-111 | Actions | clickable_count, form_fields, cta_count |
| 112-127 | Session | cookies, login_state, cart_items |

## Framework Integrations

| Framework | Package | Status |
|-----------|---------|--------|
| LangChain | `cortex-langchain` | Ready |
| CrewAI | `cortex-crewai` | Ready |
| OpenClaw | `cortex-openclaw` | Ready |

## Clients

| Language | Package | Install |
|----------|---------|---------|
| Python | `cortex-client` | `pip install cortex-client` |
| TypeScript | `@cortex-ai/client` | `npm install @cortex-ai/client` |

## Project Structure

```
cortex/
├── runtime/          # Rust runtime (the core)
│   └── src/
│       ├── map/          # SiteMap types, builder, serializer, reader
│       ├── cartography/  # Crawling, classification, encoding
│       ├── navigation/   # Query, pathfind, similarity, clustering
│       ├── live/         # Perceive, refresh, act, watch, sessions
│       ├── intelligence/ # Caching, progressive refinement
│       ├── trust/        # Credentials, PII, sanitization
│       ├── audit/        # JSONL logging, remote sync
│       └── cli/          # CLI commands
├── extractors/       # TypeScript browser extraction scripts
├── clients/
│   ├── python/       # Python thin client
│   └── typescript/   # TypeScript thin client
├── integrations/     # LangChain, CrewAI, OpenClaw
└── docs/             # Guides and cookbooks
```

## Contributing

See [docs/guides/writing-extractors.md](docs/guides/writing-extractors.md) for how to write custom extractors.

## License

Apache-2.0
