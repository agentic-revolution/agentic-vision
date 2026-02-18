# CORTEX — Autonomous Test, Score, Self-Fix & Full Platform Audit v3

**For: OpenClaw Agent**
**Project Directory:** /path/to/cortex
**Prerequisite:** V03-V04.md, gateway (MCP server + REST API + native clients), and `cortex plug` must all be complete and compiling.

---

## Your Mission

Three jobs:

**Job 1:** Test every capability of the complete Cortex platform against 100 real websites. This includes v0.2.0 features (layered acquisition, pattern engine, HTTP actions, HTTP auth) AND v0.3.0/v0.4.0 features (drag-drop discovery, canvas extraction, HTTP-native OAuth, native WebSocket, WebMCP integration). Score each site out of 100. Fix what's fixable.

**Job 2:** Test the gateway layer — MCP server, REST API, native clients, and framework adapters. Verify that every integration path works end-to-end.

**Job 3:** Test `cortex plug` — agent auto-discovery, config injection, clean removal, and status checks. Verify it finds and connects to agents correctly on this machine.

**Job 4:** Documentation audit. Every doc, README, code comment, and config file must reflect the true current state of the complete platform. Remove all references to old architecture, deleted files, or incomplete features that are now complete.

**Do all four jobs in order. Do not ask for approval. Do not stop between tasks.**

---

## PHASE 0: Pre-Flight

### 0A: Build Verification

```bash
cd /path/to/cortex
cargo build --release
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

All must pass. Fix before proceeding.

### 0B: Verify Complete Architecture Exists

```bash
# ── Core Runtime ──
ls runtime/src/acquisition/mod.rs
ls runtime/src/acquisition/http_client.rs
ls runtime/src/acquisition/structured.rs
ls runtime/src/acquisition/head_scanner.rs
ls runtime/src/acquisition/feed_parser.rs
ls runtime/src/acquisition/api_discovery.rs
ls runtime/src/acquisition/pattern_engine.rs
ls runtime/src/acquisition/css_selectors.json
ls runtime/src/acquisition/action_discovery.rs
ls runtime/src/acquisition/js_analyzer.rs
ls runtime/src/acquisition/platform_actions.json
ls runtime/src/acquisition/http_session.rs
ls runtime/src/acquisition/auth.rs

# ── v0.3.0 (complex actions) ──
ls runtime/src/acquisition/drag_discovery.rs
ls runtime/src/acquisition/drag_platforms.json
ls runtime/src/acquisition/canvas_extractor.rs
ls runtime/src/acquisition/ws_discovery.rs
ls runtime/src/acquisition/ws_platforms.json
ls runtime/src/live/websocket.rs

# ── v0.4.0 (WebMCP) ──
ls runtime/src/acquisition/webmcp.rs

# ── Gateway ──
ls integrations/mcp-server/
ls integrations/rest-api/       # or built into runtime/src/server.rs
ls clients/python/
ls clients/typescript/

# ── Framework Adapters ──
ls integrations/langchain/      || echo "Optional: LangChain adapter"
ls integrations/crewai/         || echo "Optional: CrewAI adapter"
ls integrations/openclaw/       || echo "Optional: OpenClaw adapter"

# ── Plug Command ──
# Verify cortex plug subcommand exists
./target/release/cortex plug --help || echo "MISSING: cortex plug"
```

If any REQUIRED file is missing, stop and report. If optional adapters are missing, note but continue.

### 0C: Verify Deleted Code Is Gone

```bash
test ! -f runtime/src/cartography/crawler.rs || echo "STALE: crawler.rs"
test ! -f runtime/src/cartography/sampler.rs || echo "STALE: sampler.rs"
test ! -f runtime/src/cartography/interpolator.rs || echo "STALE: interpolator.rs"
test ! -f runtime/src/intelligence/smart_sampler.rs || echo "STALE: smart_sampler.rs"
```

If any exist, delete them and all references. `cargo build` to verify.

### 0D: Runtime Startup

```bash
./target/release/cortex doctor
./target/release/cortex start
./target/release/cortex status
```

Doctor must report all checks passing. Status must show runtime listening.

### 0E: Install Clients

```bash
cd clients/python && pip install -e ".[dev]"
python -c "from cortex_client import map, perceive, act, compare, login, status; print('Python client OK')"
```

If TypeScript client exists:
```bash
cd clients/typescript && npm install && npm run build
```

---

## PHASE 1: Documentation Audit

Before testing, ensure every document reflects the complete platform. Read each file. Fix stale content.

### Documents to Audit

**Planning Documents (docs/planning/):**

1. `01-vision.md` — Must describe the full layered acquisition + action stack. No "browser renders every page."
2. `02-map-spec.md` — Must include AUTH method, WebSocket session, WebMCP tool catalog in protocol. NodeFlags must reflect http_action_available.
3. `03-implementation.md` — Must list all current source files. No references to deleted crawler/sampler/interpolator. Must include acquisition/, drag_discovery, canvas_extractor, websocket, webmcp modules.
4. `04-open-source.md` — Must mention: contributing css_selectors.json, platform_actions.json, drag_platforms.json, ws_platforms.json. Must mention MCP server, REST API, framework adapters as contribution entry points.
5. `05-edge-cases.md` — Major update: bot detection edge cases mostly irrelevant. Add: drag-drop edge cases (unknown libraries), canvas edge cases (no accessibility tree), WebSocket edge cases (binary protocols, Socket.IO handshake). Remove anything whose fix was "improve browser stealth" for mapping.
6. `06-no-browser-revolution.md` — Verify matches actual implementation.
7. `07-killing-last-limitations.md` — Verify matches implementation.
8. `08-last-mile-inventions.md` — Verify drag-drop, canvas, OAuth, WebSocket implementations match design.
9. `09-one-command-takeover.md` — Verify `cortex plug` implementation matches design.

**Root Documents:**

10. `README.md` — Must describe the complete platform: mapping without browser, HTTP actions, WebSocket, WebMCP, MCP server, REST API, `cortex plug`. Speed numbers must reflect current performance. Installation must show one-line install. Quick start must show map → query → act flow.
11. `CLAUDE.md` — Must list all current modules. Architecture description must be current.
12. `CHANGELOG.md` — Must have entries for: v0.1.0 (initial build), v0.1.1 (polish), v0.2.0 (no-browser acquisition), v0.2.1 (patterns + HTTP actions + auth), v0.3.0 (drag-drop, canvas, OAuth HTTP, WebSocket), v0.4.0 (WebMCP), v0.4.1 (gateway — MCP server, REST API), v0.4.2 (`cortex plug`).
13. `docs/LIMITATIONS.md` — Rewrite completely. Current honest limitations only:
    - Opaque canvas apps with no accessibility tree and no API (~0.5% of web)
    - First-time OAuth consent needs user confirmation (not a browser, but a pause)
    - Binary WebSocket protocols without known schema require one-time browser learning
    - WebMCP tools only available on opted-in sites (currently ~0%)
    - Rate limiting on aggressive mapping of large sites
    - Sites behind enterprise SSO (SAML/Kerberos) not yet supported

**Guide Documents (if they exist):**

14. Quickstart guide — Verify map → query → act flow with current API
15. Integration guides — Verify MCP server setup, REST API usage, Python client usage
16. `cortex plug` guide — Verify supported agents list, inject/remove/status commands

**Code Comments:**

17. Every `//!` module doc in `runtime/src/` — verify accuracy
18. Every `///` on public functions — verify accuracy
19. All `mod.rs` files — no dead module references

**Gateway Documents:**

20. MCP server README — tool definitions must match actual implementation
21. REST API docs or OpenAPI spec — endpoints must match implementation
22. Framework adapter READMEs — usage examples must work

**Commit:** `docs: complete platform documentation audit for v0.4.2`

---

## PHASE 2: Gateway Tests

Before the 100-site test, verify the gateway layer works.

### Test 2A: MCP Server

```bash
# Start MCP server
cd integrations/mcp-server
npm start &
MCP_PID=$!

# Verify it connects to Cortex runtime
sleep 2

# Call each tool via MCP protocol
# Test cortex_map
echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "cortex_map", "arguments": {"domain": "example.com"}}, "id": 1}' | node test-mcp-client.js
# Verify: response contains node_count > 0

# Test cortex_query
echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "cortex_query", "arguments": {"domain": "example.com", "limit": 5}}, "id": 2}' | node test-mcp-client.js
# Verify: response contains results array

# Test cortex_perceive
echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "cortex_perceive", "arguments": {"url": "https://example.com"}}, "id": 3}' | node test-mcp-client.js
# Verify: response contains page data

# Test cortex_auth
echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "cortex_auth", "arguments": {"domain": "httpbin.org", "method": "api_key", "credentials": {"key": "test", "header": "Authorization"}}}, "id": 4}' | node test-mcp-client.js
# Verify: response contains session_id

# Test tool listing
echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 5}' | node test-mcp-client.js
# Verify: all tools listed with correct schemas

kill $MCP_PID

# Score: 5 points per working tool, 5 for tool listing = 30 points
```

If test-mcp-client.js doesn't exist, create a minimal one that sends JSON-RPC over stdio.

### Test 2B: REST API

```bash
# Start REST API (if separate from runtime)
# Or verify runtime has --http-port flag
./target/release/cortex start --http-port 7700

sleep 2

# Test each endpoint
# Map
curl -s -X POST http://localhost:7700/api/v1/map \
  -H "Content-Type: application/json" \
  -d '{"domain": "example.com"}' | python -c "import sys,json; d=json.load(sys.stdin); assert d.get('nodes',d.get('node_count',0)) > 0, 'map failed'"

# Query
curl -s -X POST http://localhost:7700/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"domain": "example.com", "limit": 5}' | python -c "import sys,json; d=json.load(sys.stdin); print('query ok')"

# Perceive
curl -s -X POST http://localhost:7700/api/v1/perceive \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}' | python -c "import sys,json; d=json.load(sys.stdin); print('perceive ok')"

# Status
curl -s http://localhost:7700/api/v1/status | python -c "import sys,json; d=json.load(sys.stdin); assert d.get('running',d.get('status','')) in [True,'ok','running'], 'status failed'"

# OpenAPI spec (if exists)
curl -s http://localhost:7700/api/v1/openapi.json | python -c "import sys,json; d=json.load(sys.stdin); assert 'paths' in d, 'no openapi spec'"

# Score: 5 points per working endpoint, 5 for OpenAPI = 30 points
```

### Test 2C: Python Client End-to-End

```python
from cortex_client import map, query, perceive, act, compare, status, login

# Status
s = status()
assert s is not None, "status() failed"

# Map
site = map("example.com")
assert site.node_count > 0, "map returned 0 nodes"

# Query
results = site.filter(limit=5)
assert len(results) > 0, "query returned empty"

# Perceive
page = perceive("https://example.com")
assert page is not None, "perceive returned None"

# Compare (map 2 sites and compare)
try:
    comp = compare(domains=["example.com", "iana.org"], limit=5)
    assert comp is not None
except Exception:
    pass  # compare is optional, don't fail the suite

print("Python client: all tests passed")
# Score: 5 per function = 25 points
```

### Test 2D: Framework Adapters (if they exist)

```python
# LangChain
try:
    from cortex_langchain import CortexMapTool, CortexQueryTool
    tool = CortexMapTool()
    result = tool.run("example.com")
    assert "Mapped" in result or "nodes" in result.lower()
    print("LangChain adapter: OK")
except ImportError:
    print("LangChain adapter: not installed (optional)")

# CrewAI
try:
    from cortex_crewai import CortexWebCartographer
    tool = CortexWebCartographer()
    result = tool._run(command="map", domain="example.com")
    assert result is not None
    print("CrewAI adapter: OK")
except ImportError:
    print("CrewAI adapter: not installed (optional)")

# Score: 5 per working adapter, max 15 points
```

### Gateway Score Summary

```
MCP Server:          /30
REST API:            /30
Python Client:       /25
Framework Adapters:  /15
GATEWAY TOTAL:       /100
```

Save to `gateway-test-report.json`. Fix any failures before proceeding.

**Commit:** `test: gateway test suite — MCP, REST, Python, adapters`

---

## PHASE 3: Plug Command Tests

### Test 3A: Agent Discovery

```bash
# List what cortex plug can find (don't inject yet)
./target/release/cortex plug --list

# Verify output format: each found agent shows name + config path
# Capture output and parse
FOUND=$(./target/release/cortex plug --list 2>&1)

# Check for at least some agent detection capability
echo "$FOUND" | grep -c "found\|detected\|discovered"
# Should be >= 0 (depends on what's installed on this machine)
```

### Test 3B: Injection + Removal Round-Trip

```bash
# Create a mock agent config directory for safe testing
mkdir -p /tmp/cortex-plug-test/claude
echo '{"mcpServers": {}}' > /tmp/cortex-plug-test/claude/claude_desktop_config.json

mkdir -p /tmp/cortex-plug-test/cursor
echo '{"mcpServers": {}}' > /tmp/cortex-plug-test/cursor/mcp.json

mkdir -p /tmp/cortex-plug-test/continue
echo '{}' > /tmp/cortex-plug-test/continue/config.json

# Inject using test directory (if --config-dir flag exists)
# Otherwise test against real configs with backup
./target/release/cortex plug --config-dir /tmp/cortex-plug-test 2>&1 || \
  echo "Note: --config-dir may not be implemented, testing real configs"

# Verify injection
python3 -c "
import json
with open('/tmp/cortex-plug-test/claude/claude_desktop_config.json') as f:
    config = json.load(f)
assert 'cortex' in config.get('mcpServers', {}), 'Claude injection failed'
print('Claude injection: OK')
"

python3 -c "
import json
with open('/tmp/cortex-plug-test/cursor/mcp.json') as f:
    config = json.load(f)
assert 'cortex' in config.get('mcpServers', {}), 'Cursor injection failed'
print('Cursor injection: OK')
"

# Test idempotency (run plug again, should not duplicate)
./target/release/cortex plug --config-dir /tmp/cortex-plug-test 2>&1

python3 -c "
import json
with open('/tmp/cortex-plug-test/claude/claude_desktop_config.json') as f:
    config = json.load(f)
servers = config.get('mcpServers', {})
cortex_entries = [k for k in servers if 'cortex' in k.lower()]
assert len(cortex_entries) == 1, f'Duplicate injection: {cortex_entries}'
print('Idempotency: OK')
"

# Test removal
./target/release/cortex plug --remove --config-dir /tmp/cortex-plug-test 2>&1

python3 -c "
import json
with open('/tmp/cortex-plug-test/claude/claude_desktop_config.json') as f:
    config = json.load(f)
assert 'cortex' not in config.get('mcpServers', {}), 'Removal failed'
print('Removal: OK')
"

# Test status
./target/release/cortex plug --status 2>&1

# Cleanup
rm -rf /tmp/cortex-plug-test

# Score:
# Discovery works: 15 points
# Injection works: 25 points  
# Idempotency: 15 points
# Removal works: 25 points
# Status works: 10 points
# No existing config corrupted: 10 points
# PLUG TOTAL: /100
```

Save to `plug-test-report.json`. Fix any failures.

**Commit:** `test: cortex plug test suite — inject, idempotency, remove, status`

---

## PHASE 4: The 100-Site Test Suite v3

Same 100 sites. Expanded scoring with v0.3.0/v0.4.0 capabilities.

### Sites

```python
SITES = [
    # Category 1: E-Commerce (15)
    "amazon.com", "ebay.com", "walmart.com", "bestbuy.com", "target.com",
    "etsy.com", "allbirds.com", "alibaba.com", "newegg.com", "wayfair.com",
    "homedepot.com", "costco.com", "zappos.com", "bhphotovideo.com", "nordstrom.com",
    # Category 2: News (10)
    "nytimes.com", "bbc.com", "cnn.com", "reuters.com", "theguardian.com",
    "washingtonpost.com", "techcrunch.com", "arstechnica.com", "theverge.com", "bloomberg.com",
    # Category 3: Social (10)
    "reddit.com", "x.com", "linkedin.com", "medium.com", "quora.com",
    "stackoverflow.com", "news.ycombinator.com", "dev.to", "producthunt.com", "meta.discourse.org",
    # Category 4: Docs (10)
    "docs.python.org", "developer.mozilla.org", "doc.rust-lang.org", "react.dev",
    "vuejs.org", "docs.github.com", "kubernetes.io", "docs.aws.amazon.com",
    "cloud.google.com", "learn.microsoft.com",
    # Category 5: SPA/JS-Heavy (10)
    "gmail.com", "maps.google.com", "figma.com", "notion.so", "vercel.com",
    "netflix.com", "spotify.com", "airbnb.com", "uber.com", "stripe.com",
    # Category 6: Government (10)
    "usa.gov", "gov.uk", "who.int", "un.org", "irs.gov",
    "sec.gov", "census.gov", "nasa.gov", "nih.gov", "cdc.gov",
    # Category 7: Travel (10)
    "booking.com", "expedia.com", "tripadvisor.com", "kayak.com", "hotels.com",
    "airbnb.com", "skyscanner.com", "agoda.com", "vrbo.com", "google.com/travel",
    # Category 8: Food (5)
    "yelp.com", "doordash.com", "ubereats.com", "opentable.com", "allrecipes.com",
    # Category 9: Financial (5)
    "finance.yahoo.com", "marketwatch.com", "coinmarketcap.com", "bankrate.com", "nerdwallet.com",
    # Category 10: Misc (15)
    "wikipedia.org", "craigslist.org", "archive.org", "github.com", "gitlab.com",
    "npmjs.com", "pypi.org", "crates.io", "imdb.com", "rottentomatoes.com",
    "weather.com", "zillow.com", "indeed.com", "healthline.com", "pinterest.com",
]
```

### Test Sequence Per Site (v3 Scoring)

```python
import time, json
from cortex_client import map, perceive, act, status

def test_site_v3(domain: str) -> dict:
    results = {
        "domain": domain,
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "scores": {},
        "errors": [],
        "warnings": [],
        "diagnostics": {},
        "total_score": 0,
    }

    # ── TEST 1: Mapping (15 points) ──────────────────────
    try:
        start = time.time()
        site = map(domain, max_time_ms=30000, max_nodes=10000)
        map_time = time.time() - start
        score = 0

        if site.node_count > 0: score += 3
        if site.node_count > 100: score += 2
        if site.edge_count > site.node_count: score += 2

        types_found = 0
        for pt in [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]:
            try:
                if site.filter(page_type=pt, limit=1): types_found += 1
            except: pass
        if types_found > 3: score += 3
        elif types_found > 1: score += 2
        elif types_found >= 1: score += 1

        # Speed — v3 expects <3s for most sites
        if map_time < 3: score += 5
        elif map_time < 8: score += 3
        elif map_time < 15: score += 2
        elif map_time < 30: score += 1
        else: results["warnings"].append(f"MAP: slow ({map_time:.1f}s)")

        results["scores"]["mapping"] = score
        results["diagnostics"]["map_time_s"] = round(map_time, 2)
        results["diagnostics"]["node_count"] = site.node_count
        results["diagnostics"]["edge_count"] = site.edge_count
        results["diagnostics"]["types_found"] = types_found

    except Exception as e:
        results["scores"]["mapping"] = 0
        results["errors"].append(f"MAP FAILED: {type(e).__name__}: {e}")
        # Zero all other scores and return
        for k in ["data_source", "features", "query", "pathfinding", "actions", "advanced_actions", "live"]:
            results["scores"][k] = 0
        return results

    # ── TEST 2: Data Source Quality (10 points) ──────────
    try:
        score = 0
        high_conf = site.filter(features={1: {"gt": 0.9}}, limit=5)
        if high_conf and len(high_conf) >= 3:
            score += 4
            results["diagnostics"]["jsonld_detected"] = True
        elif high_conf:
            score += 2
            results["diagnostics"]["jsonld_detected"] = True
        else:
            results["diagnostics"]["jsonld_detected"] = False

        medium_conf = site.filter(features={1: {"gt": 0.7, "lt": 0.95}}, limit=5)
        if medium_conf and len(medium_conf) >= 2:
            score += 3
            results["diagnostics"]["patterns_detected"] = True
        else:
            results["diagnostics"]["patterns_detected"] = False

        low_conf = site.filter(features={1: {"lt": 0.5}}, limit=100)
        low_ratio = len(low_conf) / max(site.node_count, 1)
        if low_ratio < 0.1: score += 3
        elif low_ratio < 0.3: score += 2
        elif low_ratio < 0.5: score += 1
        results["diagnostics"]["low_confidence_ratio"] = round(low_ratio, 3)

        results["scores"]["data_source"] = score
    except Exception as e:
        results["scores"]["data_source"] = 0
        results["errors"].append(f"DATA SOURCE CHECK FAILED: {e}")

    # ── TEST 3: Feature Quality (10 points) ──────────────
    try:
        score = 0
        non_zero_dims = 0
        for dim in [0, 16, 17, 18, 20, 25, 48, 52, 64, 80]:
            try:
                if site.filter(features={dim: {"gt": 0.01}}, limit=1): non_zero_dims += 1
            except: pass
        if non_zero_dims >= 7: score += 4
        elif non_zero_dims >= 4: score += 2

        products = site.filter(page_type=0x04, limit=5)
        if products:
            priced = site.filter(page_type=0x04, features={48: {"gt": 0}}, limit=1)
            rated = site.filter(page_type=0x04, features={52: {"gt": 0}}, limit=1)
            if priced and rated: score += 3
            elif priced or rated: score += 2
        else:
            articles = site.filter(page_type=0x05, limit=5)
            if articles:
                has_text = site.filter(page_type=0x05, features={17: {"gt": 0.1}}, limit=1)
                if has_text: score += 3
                else: score += 2
            else:
                score += 2

        tls = site.filter(features={80: {"gt": 0.5}}, limit=1)
        if tls: score += 3
        else: score += 1

        results["scores"]["features"] = score
        results["diagnostics"]["non_zero_dims"] = non_zero_dims
    except Exception as e:
        results["scores"]["features"] = 0
        results["errors"].append(f"FEATURES FAILED: {e}")

    # ── TEST 4: Querying (10 points) ─────────────────────
    try:
        score = 0
        r = site.filter(limit=10)
        if r and len(r) > 0: score += 4

        r = site.filter(features={17: {"gt": 0.1}}, limit=10)
        if r and len(r) > 0: score += 3

        goal = [0.0] * 128
        goal[0] = 0.5
        r = site.nearest(goal, k=5)
        if r and len(r) > 0: score += 3

        results["scores"]["query"] = score
    except Exception as e:
        results["scores"]["query"] = 0
        results["errors"].append(f"QUERY FAILED: {e}")

    # ── TEST 5: Pathfinding (5 points) ───────────────────
    try:
        score = 0
        nodes = site.filter(limit=20)
        if len(nodes) >= 2:
            path = site.pathfind(from_node=0, to_node=nodes[-1].index)
            if path: score += 3
            else: score += 1
        if len(nodes) >= 10:
            path = site.pathfind(from_node=nodes[2].index, to_node=nodes[8].index)
            if path: score += 2
        results["scores"]["pathfinding"] = score
    except Exception as e:
        results["scores"]["pathfinding"] = 0
        results["errors"].append(f"PATHFIND FAILED: {e}")

    # ── TEST 6: Standard Actions (10 points) ─────────────
    try:
        score = 0
        nodes_with_actions = site.filter(features={96: {"gt": 0.01}}, limit=10)
        if nodes_with_actions and len(nodes_with_actions) >= 3:
            score += 4
        elif nodes_with_actions:
            score += 2
        results["diagnostics"]["nodes_with_actions"] = len(nodes_with_actions) if nodes_with_actions else 0

        # HTTP-executable actions
        http_actions = site.filter(features={97: {"gt": 0.5}}, limit=5)
        if http_actions and len(http_actions) >= 2:
            score += 3
            results["diagnostics"]["http_actions_found"] = True
        else:
            results["diagnostics"]["http_actions_found"] = False

        # Platform detection
        products_with_actions = site.filter(page_type=0x04, features={96: {"gt": 0.01}}, limit=3)
        if products_with_actions:
            score += 3
            results["diagnostics"]["platform_actions"] = True
        else:
            score += 1
            results["diagnostics"]["platform_actions"] = False

        results["scores"]["actions"] = score
    except Exception as e:
        results["scores"]["actions"] = 0
        results["errors"].append(f"ACTIONS FAILED: {e}")

    # ── TEST 7: Advanced Actions — v0.3.0/v0.4.0 (30 points) ──
    # This is the NEW section testing drag-drop, canvas, WebSocket, WebMCP
    try:
        score = 0

        # 7A: Drag-and-Drop Discovery (8 points)
        # Check if drag-drop actions were discovered on sites that have them
        # (Trello-like sites, project boards, sortable lists)
        drag_indicators = site.filter(features={98: {"gt": 0.01}}, limit=5)  # dim 98 = drag action indicator
        if drag_indicators and len(drag_indicators) >= 1:
            score += 5
            results["diagnostics"]["drag_actions_found"] = True
            # Check if drag action has an API endpoint (not just detected, but executable)
            if any(hasattr(d, 'http_executable') and d.http_executable for d in drag_indicators):
                score += 3
                results["diagnostics"]["drag_api_found"] = True
            else:
                results["diagnostics"]["drag_api_found"] = False
        else:
            # Not all sites have drag-drop — check if this site SHOULD have it
            # Project boards, kanban, sortable lists
            has_sortable = site.filter(page_type=0x06, limit=1)  # interactive/app pages
            if not has_sortable:
                score += 5  # no drag-drop expected, full credit
                results["diagnostics"]["drag_actions_found"] = "n/a"
                results["diagnostics"]["drag_api_found"] = "n/a"
            else:
                score += 2  # should have found something
                results["diagnostics"]["drag_actions_found"] = False
                results["diagnostics"]["drag_api_found"] = False

        # 7B: Canvas/Accessibility Extraction (7 points)
        # For known canvas apps (Figma, Sheets, Maps), check if state was extracted
        canvas_domains = ["figma.com", "maps.google.com", "docs.google.com"]
        if domain in canvas_domains or any(d in domain for d in canvas_domains):
            # This IS a canvas app — did we extract anything?
            if site.node_count > 5:
                score += 4  # got meaningful data from canvas app
                results["diagnostics"]["canvas_extracted"] = True
            else:
                results["diagnostics"]["canvas_extracted"] = False
            # Check for accessibility-derived features
            acc_features = site.filter(features={99: {"gt": 0.01}}, limit=1)  # dim 99 = accessibility data indicator
            if acc_features:
                score += 3
                results["diagnostics"]["accessibility_tree_used"] = True
            else:
                results["diagnostics"]["accessibility_tree_used"] = False
        else:
            # Not a canvas app — full credit
            score += 7
            results["diagnostics"]["canvas_extracted"] = "n/a"
            results["diagnostics"]["accessibility_tree_used"] = "n/a"

        # 7C: WebSocket Discovery (8 points)
        # For known real-time apps, check if WebSocket endpoints were discovered
        ws_domains = ["slack.com", "discord.com", "notion.so", "figma.com"]
        has_ws_indicators = site.filter(features={100: {"gt": 0.01}}, limit=1)  # dim 100 = websocket indicator
        if has_ws_indicators:
            score += 5
            results["diagnostics"]["websocket_discovered"] = True
            # Check if ws endpoint is usable (has URL and protocol)
            score += 3  # assume yes if discovered
            results["diagnostics"]["websocket_connectable"] = True
        elif domain in ws_domains or any(d in domain for d in ws_domains):
            # Should have found WebSocket
            score += 2
            results["diagnostics"]["websocket_discovered"] = False
            results["warnings"].append(f"WEBSOCKET: expected WS on {domain} but none found")
        else:
            # Not a WS app — full credit
            score += 8
            results["diagnostics"]["websocket_discovered"] = "n/a"

        # 7D: WebMCP Discovery (7 points)
        # Check if any WebMCP tools were found (rare — almost no sites have adopted yet)
        webmcp_tools = site.filter(features={101: {"gt": 0.01}}, limit=1)  # dim 101 = webmcp indicator
        if webmcp_tools:
            score += 7
            results["diagnostics"]["webmcp_tools_found"] = True
        else:
            # WebMCP adoption is ~0% currently — not finding it is expected
            score += 5  # partial credit: the detection mechanism exists even if no sites use it
            results["diagnostics"]["webmcp_tools_found"] = False

        results["scores"]["advanced_actions"] = score
    except Exception as e:
        results["scores"]["advanced_actions"] = 0
        results["errors"].append(f"ADVANCED ACTIONS FAILED: {e}")

    # ── TEST 8: Live Verification (10 points) ────────────
    try:
        score = 0
        page = perceive(f"https://{domain}", include_content=True)
        if page:
            score += 5
            if hasattr(page, 'content') and page.content: score += 1
            if hasattr(page, 'encoding') and page.encoding: score += 1
        nodes = site.filter(limit=10)
        if len(nodes) >= 3:
            page = perceive(nodes[2].url)
            if page: score += 3
        results["scores"]["live"] = score
    except Exception as e:
        results["scores"]["live"] = 0
        results["errors"].append(f"LIVE FAILED: {e}")

    # ── Compute Total ────────────────────────────────────
    results["total_score"] = sum(
        results["scores"].get(k, 0)
        for k in ["mapping", "data_source", "features", "query", "pathfinding", "actions", "advanced_actions", "live"]
    )

    return results
```

### Score Breakdown (100 points total)

```
Mapping:              15 points  (map succeeds, nodes, edges, types, speed)
Data Source Quality:  10 points  (JSON-LD, patterns, low browser fallback)
Feature Quality:      10 points  (non-zero dims, commerce data, trust)
Querying:             10 points  (filter, feature range, nearest neighbor)
Pathfinding:           5 points  (root-to-deep, cross-graph)
Standard Actions:     10 points  (actions found, HTTP-executable, platform)
Advanced Actions:     30 points  (drag-drop, canvas, WebSocket, WebMCP)
Live Verification:    10 points  (perceive homepage + interior)
```

**Key difference from v2:** Advanced Actions is 30 points — the largest single category. This validates that v0.3.0/v0.4.0 features are actually working in production against real sites.

---

## PHASE 5: Run All 100 Tests

```python
import json, time

def run_full_test_suite_v3():
    all_results = []

    print("Cortex v3 — 100-Site Full Platform Test")
    print("Architecture: Layered Acquisition + Advanced Actions + WebMCP")
    print(f"{'='*70}")

    for i, domain in enumerate(SITES):
        print(f"\n[{i+1}/100] {domain}...")

        try:
            result = test_site_v3(domain)
            all_results.append(result)

            score = result["total_score"]
            ds = result["diagnostics"]

            icon = "✓" if score >= 80 else "⚠" if score >= 50 else "✗"
            flags = ""
            flags += "J" if ds.get("jsonld_detected") else "-"
            flags += "P" if ds.get("patterns_detected") else "-"
            flags += "A" if ds.get("http_actions_found") else "-"
            flags += "D" if ds.get("drag_actions_found") not in [False, "n/a", None] else "-"
            flags += "C" if ds.get("canvas_extracted") not in [False, "n/a", None] else "-"
            flags += "W" if ds.get("websocket_discovered") not in [False, "n/a", None] else "-"
            flags += "M" if ds.get("webmcp_tools_found") else "-"

            print(f"  {icon} {score:3d}/100  [{flags}]  "
                  f"nodes:{ds.get('node_count',0)}  "
                  f"time:{ds.get('map_time_s',0):.1f}s  "
                  f"errs:{len(result['errors'])}")

            if score < 80:
                for err in result["errors"][:3]:
                    print(f"    ERROR: {err}")

        except Exception as e:
            print(f"  ✗ CRASH: {e}")
            all_results.append({
                "domain": domain, "total_score": 0,
                "errors": [f"CRASH: {e}"], "warnings": [],
                "scores": {}, "diagnostics": {},
            })

        time.sleep(1)

    # Generate report
    scores = [r["total_score"] for r in all_results]
    avg = sum(scores) / len(scores)

    report = {
        "version": "v3",
        "architecture": "full_platform",
        "summary": {
            "total_sites": len(all_results),
            "average_score": round(avg, 1),
            "sites_above_90": sum(1 for s in scores if s >= 90),
            "sites_above_80": sum(1 for s in scores if s >= 80),
            "sites_below_50": sum(1 for s in scores if s < 50),
            "total_errors": sum(len(r["errors"]) for r in all_results),
            "jsonld_coverage": round(sum(1 for r in all_results if r.get("diagnostics", {}).get("jsonld_detected")) / len(all_results), 3),
            "pattern_coverage": round(sum(1 for r in all_results if r.get("diagnostics", {}).get("patterns_detected")) / len(all_results), 3),
            "http_action_coverage": round(sum(1 for r in all_results if r.get("diagnostics", {}).get("http_actions_found")) / len(all_results), 3),
            "drag_discovery_rate": round(sum(1 for r in all_results if r.get("diagnostics", {}).get("drag_actions_found") == True) / len(all_results), 3),
            "websocket_discovery_rate": round(sum(1 for r in all_results if r.get("diagnostics", {}).get("websocket_discovered") == True) / len(all_results), 3),
            "webmcp_adoption": round(sum(1 for r in all_results if r.get("diagnostics", {}).get("webmcp_tools_found")) / len(all_results), 3),
        },
        "category_scores": {},
        "results": all_results,
    }

    categories = {
        "e_commerce": SITES[0:15], "news": SITES[15:25], "social": SITES[25:35],
        "docs": SITES[35:45], "spa": SITES[45:55], "government": SITES[55:65],
        "travel": SITES[65:75], "food": SITES[75:80], "financial": SITES[80:85],
        "misc": SITES[85:100],
    }
    for cat, cat_sites in categories.items():
        cat_scores = [r["total_score"] for r in all_results if r["domain"] in cat_sites]
        if cat_scores:
            report["category_scores"][cat] = {
                "average": round(sum(cat_scores) / len(cat_scores), 1),
                "min": min(cat_scores),
                "max": max(cat_scores),
            }

    with open("test-report-v3.json", "w") as f:
        json.dump(report, f, indent=2)

    print(f"\n{'='*70}")
    print(f"RESULTS — v3 Full Platform")
    print(f"{'='*70}")
    print(f"Average:       {avg:.1f}/100")
    print(f"Sites ≥90:     {report['summary']['sites_above_90']}/100")
    print(f"Sites ≥80:     {report['summary']['sites_above_80']}/100")
    print(f"Sites <50:     {report['summary']['sites_below_50']}/100")
    print(f"")
    print(f"Coverage:")
    print(f"  JSON-LD:     {report['summary']['jsonld_coverage']:.0%}")
    print(f"  Patterns:    {report['summary']['pattern_coverage']:.0%}")
    print(f"  HTTP Acts:   {report['summary']['http_action_coverage']:.0%}")
    print(f"  Drag:        {report['summary']['drag_discovery_rate']:.0%}")
    print(f"  WebSocket:   {report['summary']['websocket_discovery_rate']:.0%}")
    print(f"  WebMCP:      {report['summary']['webmcp_adoption']:.0%}")
    print(f"")
    print(f"Category scores:")
    for cat, data in report["category_scores"].items():
        print(f"  {cat:15s} avg:{data['average']:5.1f}  min:{data['min']:3d}  max:{data['max']:3d}")
    print(f"\nSaved to test-report-v3.json")

    return report
```

---

## PHASE 6: Self-Fix Loop

Same methodology as v1/v2. New failure categories:

| Category | Root Cause | Fix Location |
|----------|-----------|--------------|
| MAP_ZERO_NODES | Sitemap missing + link discovery failed | `mapper.rs` |
| STRUCTURED_EMPTY | JSON-LD not found | `structured.rs` |
| PATTERN_MISS | CSS selectors miss this site's patterns | `css_selectors.json` |
| ACTIONS_NOT_FOUND | Forms/APIs not discovered | `action_discovery.rs` |
| PLATFORM_MISS | Known platform not detected | `platform_actions.json` |
| DRAG_MISS | Drag library not recognized | `drag_discovery.rs`, `drag_platforms.json` |
| CANVAS_OPAQUE | No accessibility tree or API | `canvas_extractor.rs` |
| WS_MISS | WebSocket endpoint not discovered | `ws_discovery.rs`, `ws_platforms.json` |
| WEBMCP_FAIL | WebMCP detection/execution error | `webmcp.rs` |
| HTTP_ERROR | reqwest connection failure | `http_client.rs` |
| PERCEIVE_FAIL | Browser fallback issue | `renderer/chromium.rs` |
| SLOW_MAP | Mapping >15s | Check Layer 3 fallback rate |
| GATEWAY_FAIL | MCP/REST not routing correctly | `integrations/` |

**Priority order:**
1. STRUCTURED_EMPTY — highest impact, fixes many sites at once
2. PATTERN_MISS — each selector addition covers multiple sites
3. PLATFORM_MISS — each platform covers thousands of sites
4. DRAG_MISS / WS_MISS — add library signatures and platform entries
5. Everything else — site-specific

**Maximum 5 fix iterations. Target: average ≥ 95.**

---

## PHASE 7: Final Report

Produce `docs/TEST-REPORT-V3.md`:

```markdown
# Cortex v3 — Full Platform Test Report

**Architecture:** Complete (Acquisition + Actions + WebSocket + WebMCP + Gateway + Plug)
**Date:** YYYY-MM-DD
**Overall Average:** XX.X/100

## Version Comparison

| Metric | v1 (browser) | v2 (no-browser) | v3 (full platform) |
|--------|-------------|-----------------|-------------------|
| Average score | 85.3 | XX.X | XX.X |
| Sites ≥ 90 | 52 | XX | XX |
| Sites < 50 | 4 | XX | XX |
| Bot-blocked | 10 | XX | XX |
| Avg map time | XXs | XXs | XXs |

## Architecture Metrics

| Metric | Value |
|--------|-------|
| JSON-LD coverage | XX% |
| Pattern engine coverage | XX% |
| HTTP action coverage | XX% |
| Drag-drop discovery rate | XX% |
| WebSocket discovery rate | XX% |
| WebMCP adoption | XX% |
| Browser fallback rate | XX% |

## Gateway Test Results

| Interface | Score | Status |
|-----------|-------|--------|
| MCP Server | XX/30 | ✓/✗ |
| REST API | XX/30 | ✓/✗ |
| Python Client | XX/25 | ✓/✗ |
| Framework Adapters | XX/15 | ✓/✗ |
| TOTAL | XX/100 | |

## Plug Test Results

| Test | Score | Status |
|------|-------|--------|
| Discovery | XX/15 | ✓/✗ |
| Injection | XX/25 | ✓/✗ |
| Idempotency | XX/15 | ✓/✗ |
| Removal | XX/25 | ✓/✗ |
| Status | XX/10 | ✓/✗ |
| Config Safety | XX/10 | ✓/✗ |
| TOTAL | XX/100 | |

## Per-Site Scores

| # | Site | Total | Map | Data | Feat | Qry | Path | Acts | Adv | Live | Flags |
|---|------|-------|-----|------|------|-----|------|------|-----|------|-------|

Flags: J=JSON-LD, P=Patterns, A=HTTP Actions, D=Drag, C=Canvas, W=WebSocket, M=WebMCP

## Remaining Limitations (Honest)

[List anything that genuinely can't be fixed]

## Fixes Applied

[List each fix with commit hash]
```

---

## PHASE 8: Final Consistency Check

```bash
# No dead references
grep -r "crawler\|sampler\|interpolat" runtime/src/ --include="*.rs" -l | grep -v test | grep -v "// " 
# Should return nothing except legitimate uses

# No stale architecture references in docs
grep -rn "browser.*render.*every\|render.*all.*pages\|browser.*sample\|interpolat" docs/ --include="*.md"
# Should return nothing

# Build is clean
cargo build --release
cargo clippy -- -D warnings
cargo test
cargo fmt --check

# MCP server builds
cd integrations/mcp-server && npm run build 2>/dev/null || echo "MCP: check build"

# Python client is clean
cd clients/python && python -m pytest 2>/dev/null || echo "Python: check tests"

# No TODOs from build phase
grep -rn "TODO\|FIXME\|not.*implemented\|placeholder" runtime/src/ --include="*.rs" | grep -v test
# Review any hits

# Verify tag
git log --oneline -5
```

Final commit: `v0.4.2: full platform test — 100 sites + gateway + plug`
Tag: `git tag v0.4.2`

---

## Rules

1. Do not ask for approval. Execute all 8 phases autonomously.
2. Test all 100 sites. Do not skip.
3. Fix code, not tests. Reality wins.
4. Documentation must match code exactly.
5. Every deleted reference must be gone. Grep to verify.
6. Run `cargo test` after every fix.
7. Commit after each meaningful change.
8. Be honest in all scores and reports.
9. Gateway and plug tests must pass BEFORE the 100-site run.
10. Stop after 5 fix iterations. Report remaining gaps honestly.
11. The final report must include v1 vs v2 vs v3 comparison.
12. Update LIMITATIONS.md with real findings only.

Begin with Phase 0, then sequentially through Phase 8.
