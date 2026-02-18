# 01 — Cortex Vision: The Web Cartographer

## What Cortex Is

Cortex is a rapid web cartographer for AI agents. When an agent needs to use a website, Cortex maps the entire site into a binary graph in seconds. The agent navigates that graph — not the website — making decisions at machine speed through pathfinding and numeric operations. No LLM in the browsing loop. No page-by-page exploration.

## The Problem With Everything Else

Every existing agent browsing tool works the same way:

```
Visit page → LLM reads page → LLM decides next action → Visit next page → Repeat
```

OpenClaw, browser-use, Playwright wrappers, screenshot+vision — they all put the LLM in a sequential loop. The LLM reads every page (as text, JSON, or screenshot), reasons about what to do, then visits the next page. This is slow (20-35 seconds for a 5-site comparison), expensive (10-15 LLM calls), and fundamentally serial.

The bottleneck isn't page loading. It's the LLM reading and reasoning at every step.

## How Cortex Works

```
Agent says: "Map this site"
  ↓
Cortex maps the ENTIRE site via layered HTTP acquisition (1-4 seconds)
  Layer 0: Sitemap + robots.txt + HEAD scan + feed discovery (HTTP only)
  Layer 1: HTTP GET sample pages → parse JSON-LD / OpenGraph / meta tags
  Layer 1.5: Pattern engine (CSS selectors) for pages with sparse structured data
  Layer 2: API discovery for known platforms (Shopify, WooCommerce, etc.)
  Layer 2.5: Action discovery — HTML forms + JS endpoints + platform templates
  Layer 3: Browser render ONLY for pages where Layers 0-2.5 gave <20% data
  ↓
Agent loads the graph into memory (30MB for a 50K-page site)
  ↓
Agent pathfinds/queries/filters the graph (microseconds, no LLM)
  ↓
Agent visits 1-2 live pages to verify and act (1-2 seconds)
  ↓
Done. Total: 2-6 seconds, 2 LLM calls (task parsing + response formatting)
```

Most sites never need a browser during mapping. JSON-LD, OpenGraph, and CSS-selector-based extraction from raw HTML provide sufficient data for the graph. The browser is reserved for live actions (ACT) and pages with very low structured data coverage.

## The Chess Engine Analogy

A chess engine doesn't look at one square, think about it, look at the next square. It loads the entire 64-square board as an array and evaluates millions of positions per second. The board IS numbers. The engine computes against the board — it never "reads" it.

Cortex does this for websites. The site IS a graph of numbers. The agent computes against the graph. It never "reads" pages.

## Complete Flow Example

Task: "Find the cheapest Sony WH-1000XM5 across 5 retailers."

```python
import cortex

# Step 0: LLM parses the task ONCE (only LLM call during browsing)
goal = llm.parse("Sony WH-1000XM5, cheapest, across 5 sites")

# Step 1: Map all 5 sites in parallel via HTTP (~2 seconds, no browser needed)
maps = cortex.map_many(["amazon.com", "bestbuy.com", "walmart.com", "bhphoto.com", "newegg.com"])
# → 5 SiteMap objects, ~150,000 nodes total, ~100MB in memory
# → Structured data (JSON-LD, OpenGraph) + pattern engine extraction, no browser launched

# Step 2: Query all maps (microseconds — no LLM, pure graph operations)
candidates = []
for site_map in maps:
    results = site_map.filter(
        type=PRODUCT_DETAIL,
        feature[PRICE] < 300.0,
        feature[RATING] > 0.8,
        feature[CATEGORY] == ELECTRONICS_AUDIO
    )
    candidates.extend(results)
# → ~30 matching product nodes found across all 5 sites

# Step 3: Sort by price (microseconds)
candidates.sort(key=lambda n: n.features[PRICE])
top_5 = candidates[:5]

# Step 4: Verify top 5 live (~2 seconds, parallel)
verified = cortex.perceive_many([n.url for n in top_5])

# Step 5: LLM formats response (only other LLM call)
answer = llm.format(verified, goal)
# → "Cheapest: Walmart at $268.00..."

# TOTAL: ~5 seconds, 2 LLM calls
# Agent "browsed" 150,000 pages via maps, visited 5 live for verification
```

## Speed Comparison

| System | Total Time | LLM Calls | Pages Visited Live | Browser for Mapping | Decision Method |
|--------|-----------|-----------|-------------------|--------------------|-----------------|
| OpenClaw-style | 20-35s | 10-15 | 8-12 (serial, blind) | Yes (every page) | LLM at every step |
| Cortex v0.1 (browser) | 3-7s | 2 | 1-5 (verify only) | Yes (samples) | Graph pathfinding |
| Cortex v0.2 (HTTP) | **2-6s** | **2** | **1-5 (verify only)** | **No (HTTP-first)** | **Graph pathfinding** |

## How Cortex Maps a Site

Cortex uses **layered acquisition** — each layer adds data via HTTP without a browser. The browser is a last-resort fallback.

**Layer 0 — Metadata discovery (instant, <300ms):**
Sitemap.xml, robots.txt, HEAD scan of discovered URLs, RSS/Atom feed discovery. All via plain HTTP. One sitemap request can yield thousands of URLs. HEAD scans filter to HTML-only pages efficiently.

**Layer 1 — Structured data extraction (0.5-2 seconds):**
HTTP GET a diverse sample of pages (up to 30). Parse JSON-LD, OpenGraph, meta tags, and semantic HTML from the raw response. No rendering needed — structured data is in the HTML source. This provides page type, prices, ratings, availability, breadcrumbs, and links.

**Layer 1.5 — Pattern engine fallback (included in Layer 1 time):**
For pages where structured data covers less than 50% of features, the pattern engine applies CSS selectors from `css_selectors.json` to extract prices, ratings, titles, and other data directly from HTML.

**Layer 2 — API discovery (0-500ms, optional):**
For known platforms (Shopify, WooCommerce, Magento, BigCommerce), Cortex probes documented API endpoints to retrieve structured product/content data directly.

**Layer 2.5 — Action discovery (included in Layer 1 time):**
HTML forms, JS endpoint patterns, and platform-specific action templates from `platform_actions.json` are analyzed to build the action catalog. Many actions (add-to-cart, search) can be executed via HTTP POST without a browser.

**Layer 3 — Browser render (0-3 seconds, only if needed):**
Pages where Layers 0-2.5 produced less than 20% data completeness are rendered in a headless browser. Typically fewer than 10% of sampled pages need this fallback.

### Mapping Times

| Site Size | Sitemap | HTTP Samples | Browser Fallback | Map Delivery | Quality |
|-----------|---------|-------------|-----------------|-------------|---------|
| Small (100 pages) | Yes | 20-30 | 0-2 pages | <1s | Near-complete |
| Medium (10K pages) | Yes | 30 | 0-5 pages | 1-2s | Skeleton + structured data |
| Large (100K+ pages) | Yes | 30 | 0-10 pages | 2-4s | Skeleton + structured data + patterns |
| Any size | No | 30 (crawled links) | 0-10 pages | 2-6s | Partial, expanding via progressive refinement |

## The Dynamic Problem

Websites change. Prices update, stock fluctuates, pages appear and disappear. The map handles this through freshness scoring and targeted refresh:

- Every node has a `freshness` score (0.0 = never fetched / stale, 1.0 = just fetched)
- The agent can filter by freshness: "only trust nodes with freshness > 0.7"
- Targeted refresh: `cortex.refresh(map, cluster=ELECTRONICS)` re-fetches only the relevant section
- Change detection: content hashes detect when a node's data has changed
- Map caching: repeat visits to the same site reuse the cached map, refreshing only stale areas

## What Cortex Is NOT

- NOT a scraper (it doesn't extract text for you to read)
- NOT a browser automation tool (the agent doesn't drive a browser step by step)
- NOT an API wrapper (there's no cloud service to call)
- NOT an LLM-powered browsing agent (it doesn't reason — it maps)
- NOT page-by-page perception (the unit of work is a whole site, not a page)

## The New Pitch

**Old (every other tool):** "Help your AI agent browse the web"
**Cortex:** "Give your AI agent a complete map of any website in seconds. Navigate the map, not the site."

Browsing is exploring blind. Mapping is knowing everything. Cortex eliminates exploration.
