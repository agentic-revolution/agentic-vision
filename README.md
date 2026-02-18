# Cortex

**Rapid web cartographer for AI agents.** Map entire websites into navigable binary graphs in seconds. Agents navigate the map, not the site.

[![CI](https://github.com/cortex-ai/cortex/actions/workflows/ci.yml/badge.svg)](https://github.com/cortex-ai/cortex/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

## Quick Start

```bash
# Install
cargo install cortex-runtime

# Map a site (auto-installs Chromium and starts daemon on first run)
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

No manual setup needed. `cortex map` handles Chromium installation and daemon lifecycle automatically on first run. For diagnostics, run `cortex doctor`.

## How It Works

1. **Map** — Cortex maps a domain via layered HTTP acquisition: sitemap/robots.txt discovery, HTTP GET with JSON-LD/OpenGraph extraction, CSS-selector pattern engine, and platform API discovery. Each page is encoded into a 128-dimension feature vector. **No browser needed for mapping** — browser is a last-resort fallback for the ~5% of pages with no structured data.

2. **Navigate** — Query the map by page type, feature ranges, or vector similarity. Pathfind between any two nodes using Dijkstra's algorithm on the edge graph.

3. **Act** — Execute actions on live pages. Many actions (add-to-cart, search, form submission) work via HTTP POST. Complex interactions (drag-drop, canvas) fall back to browser sessions.

## Architecture

```
Agent  ──→  Client (Python/TS)  ──→  Unix Socket  ──→  Cortex Runtime
                                                          ├── Acquisition Engine (HTTP-first)
                                                          ├── Cartography Engine
                                                          ├── Navigation Engine
                                                          ├── Live Interaction
                                                          ├── Intelligence Layer
                                                          └── Chromium Pool (fallback only)
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
│       ├── acquisition/  # HTTP-first data acquisition (JSON-LD, patterns, actions)
│       ├── cartography/  # Classification, feature encoding, mapping
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
├── integrations/
│   ├── mcp-server/  # MCP server for Claude, Cursor, Continue, Windsurf
│   ├── langchain/   # LangChain adapter
│   ├── crewai/      # CrewAI adapter
│   └── openclaw/    # OpenClaw skills
└── docs/             # Guides and cookbooks
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `cortex map <domain>` | Map a website into a binary graph |
| `cortex query <domain>` | Search a mapped site by type/features |
| `cortex pathfind <domain>` | Find shortest path between nodes |
| `cortex perceive <url>` | Analyze a single live page |
| `cortex doctor` | Check environment and diagnose issues |
| `cortex start` / `stop` / `restart` | Manage the background daemon |
| `cortex status` | Show runtime status and cached maps |
| `cortex install` | Download Chromium for Testing |
| `cortex cache clear` | Clear cached maps |
| `cortex completions <shell>` | Generate shell completions (bash/zsh/fish) |
| `cortex plug` | Auto-discover AI agents and inject Cortex as an MCP tool |
| `cortex start --http-port 7700` | Start with REST API on the specified port |

Global flags: `--json`, `--quiet`, `--verbose`, `--no-color`

## Known Limitations

See [docs/LIMITATIONS.md](docs/LIMITATIONS.md) for an honest list of current limitations including SPA support, CAPTCHA handling, and feature vector accuracy.

## Contributing

See [docs/guides/writing-extractors.md](docs/guides/writing-extractors.md) for how to write custom extractors. You can also contribute platform patterns to `runtime/src/acquisition/css_selectors.json` and `runtime/src/acquisition/platform_actions.json`.

## License

Apache-2.0
