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
Cortex rapidly maps the ENTIRE site into a binary graph (2-5 seconds)
  ↓
Agent loads the graph into memory (30MB for a 50K-page site)
  ↓
Agent pathfinds/queries/filters the graph (microseconds, no LLM)
  ↓
Agent visits 1-2 live pages to verify and act (1-2 seconds)
  ↓
Done. Total: 3-7 seconds, 2 LLM calls (task parsing + response formatting)
```

## The Chess Engine Analogy

A chess engine doesn't look at one square, think about it, look at the next square. It loads the entire 64-square board as an array and evaluates millions of positions per second. The board IS numbers. The engine computes against the board — it never "reads" it.

Cortex does this for websites. The site IS a graph of numbers. The agent computes against the graph. It never "reads" pages.

## Complete Flow Example

Task: "Find the cheapest Sony WH-1000XM5 across 5 retailers."

```python
import cortex

# Step 0: LLM parses the task ONCE (only LLM call during browsing)
goal = llm.parse("Sony WH-1000XM5, cheapest, across 5 sites")

# Step 1: Map all 5 sites in parallel (~3 seconds)
maps = cortex.map_many(["amazon.com", "bestbuy.com", "walmart.com", "bhphoto.com", "newegg.com"])
# → 5 SiteMap objects, ~150,000 nodes total, ~100MB in memory

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

| System | Total Time | LLM Calls | Pages Visited Live | Decision Method |
|--------|-----------|-----------|-------------------|-----------------|
| OpenClaw-style | 20-35s | 10-15 | 8-12 (serial, blind) | LLM at every step |
| Cortex | **3-7s** | **2** | **1-5 (verify only)** | **Graph pathfinding** |

## How Cortex Maps a Site

Cortex doesn't render every page. It builds the graph intelligently:

**Level 0 — Sitemap + robots.txt (instant, <200ms):**
Most sites publish sitemap.xml. One HTTP request gives Cortex thousands of URLs. robots.txt reveals URL structure patterns.

**Level 1 — URL pattern analysis (instant, <10ms):**
URLs encode structure. `/dp/B0XXXXX` = product. `/s?k=query` = search results. `/gp/cart` = cart. Cortex classifies every URL by pattern without visiting it.

**Level 2 — Sample rendering (2-5 seconds):**
Cortex renders 50-200 sample pages (a few per URL pattern) in parallel browsers. Each sample provides: real feature vector, discovered links, action catalog, page type verification.

**Level 3 — Feature interpolation (instant):**
For the thousands of pages NOT rendered, Cortex estimates feature vectors from the rendered samples. A rendered product page at $278 tells Cortex what product pages on this site look like. Unrendered product pages get estimated features with lower confidence.

**Level 4 — Progressive refinement (background):**
After returning the initial map, Cortex keeps rendering more pages in the background. The map gets more accurate over time. The agent can refresh at any point.

### Mapping Times

| Site Size | Sitemap | Sample Renders | Map Delivery | Quality |
|-----------|---------|---------------|-------------|---------|
| Small (100 pages) | Yes | 30 | <1s | Near-complete |
| Medium (10K pages) | Yes | 100-200 | 1-3s | Skeleton + samples |
| Large (100K+ pages) | Yes | 200-500 | 2-5s | Skeleton + samples + interpolation |
| Any size | No | More sampling | 3-10s | Partial, expanding via progressive refinement |

## The Dynamic Problem

Websites change. Prices update, stock fluctuates, pages appear and disappear. The map handles this through freshness scoring and targeted refresh:

- Every node has a `freshness` score (0.0 = never rendered / stale, 1.0 = just rendered)
- The agent can filter by freshness: "only trust nodes with freshness > 0.7"
- Targeted refresh: `cortex.refresh(map, cluster=ELECTRONICS)` re-renders only the relevant section
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
