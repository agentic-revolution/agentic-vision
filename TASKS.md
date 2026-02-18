# TASKS.md — Sequential Build Checklist

> **Note:** These tasks were for the initial v0.1.0 build. The architecture has since evolved — see `docs/06-no-browser-revolution.md` for the layered acquisition engine and `FINAL-PUSH.md` for pattern engine, HTTP actions, and HTTP authentication additions.

> **Claude Code: Execute these tasks top to bottom. Do not skip. Do not ask for approval. Do not stop between tasks. Mark each `[x]` when done. Commit after each numbered task with message `T-XXX: description`. If a task fails, fix it before moving on.**

---

## Phase 0: Bootstrap

- [ ] **T-000** Run `bash scripts/bootstrap.sh` to create full directory structure and config files
- [ ] **T-001** Run `cd runtime && cargo check` — must compile (placeholder main.rs only)
- [ ] **T-002** Run `cd extractors && npm install` — must install devDependencies
- [ ] **T-003** Run `git init && git add -A && git commit -m "T-003: initial project scaffold"`

---

## Phase 1: Foundation

### 1A: CLI Shell

- [ ] **T-010** Implement `runtime/src/main.rs` using clap derive macros. Subcommands: `start`, `stop`, `doctor`, `status`, `map`, `query`, `pathfind`, `perceive`, `install`. Each subcommand prints "not implemented" and exits 0. Include version flag.
- [ ] **T-011** Implement `runtime/src/cli/mod.rs` — declare all submodules: `start`, `stop`, `doctor`, `status`, `map_cmd`, `query_cmd`, `pathfind_cmd`, `perceive_cmd`, `install_cmd`.
- [ ] **T-012** Implement `runtime/src/cli/doctor.rs` — Check: (1) Chromium findable at `~/.cortex/chromium/`, env `CORTEX_CHROMIUM_PATH`, or system PATH. (2) Socket path `/tmp/cortex.sock` writable. (3) Available memory >256MB. (4) Print OS, arch, memory, Chromium path. Print READY or NOT READY.
- [ ] **T-013** Run `cargo build` — must compile clean. Run `./target/debug/cortex --help` — must show all subcommands.
- [ ] **T-014** `git add -A && git commit -m "T-014: CLI shell with all subcommands"`

### 1B: Core Types

- [ ] **T-020** Implement `runtime/src/map/mod.rs` — module declarations for `types`, `builder`, `reader`, `serializer`, `deserializer`.
- [ ] **T-021** Implement `runtime/src/map/types.rs` — All structs from 02-map-spec.md: `SiteMap`, `MapHeader`, `NodeRecord`, `EdgeRecord`, `ActionRecord`, `PageType` enum (with all values), `EdgeType` enum, `NodeFlags` bitfield helpers, `OpCode` struct. Derive `Debug`, `Clone`, `Serialize`, `Deserialize` where appropriate. Implement `Display` for `PageType`. Implement constants for all 128 feature vector dimension indices (e.g., `pub const FEAT_PAGE_TYPE: usize = 0; pub const FEAT_PRICE: usize = 48;`).
- [ ] **T-022** Implement `runtime/src/map/builder.rs` — `SiteMapBuilder` with methods: `new(domain)`, `add_node(url, page_type, features, confidence) -> u32` (returns node index), `add_edge(from, to, edge_type, weight, flags)`, `add_action(node, opcode, target_node, cost, risk)`, `set_rendered(node, features, actions)`, `build() -> SiteMap`. Builder computes edge_index and action_index CSR arrays, cluster assignments (basic k-means on feature vectors with k=sqrt(node_count/10)), and feature norms.
- [ ] **T-023** Implement `runtime/src/map/serializer.rs` — `SiteMap::serialize(&self) -> Vec<u8>`. Write binary CTX format: magic bytes, header, then each table contiguously. Use `byteorder` crate for endianness (little-endian).
- [ ] **T-024** Implement `runtime/src/map/deserializer.rs` — `SiteMap::deserialize(data: &[u8]) -> Result<Self>`. Read binary CTX format back. Validate magic bytes and version.
- [ ] **T-025** Implement `runtime/src/map/reader.rs` — Query methods on SiteMap: `filter(&self, query: &NodeQuery) -> Vec<NodeMatch>` where NodeQuery supports page_type filter, feature range filters, and flag filters. `nearest(&self, target: &[f32; 128], k: usize) -> Vec<NodeMatch>` using brute-force cosine similarity. `edges_from(&self, node: u32) -> &[EdgeRecord]` using CSR index. `node_url(&self, node: u32) -> &str`.
- [ ] **T-026** Write tests in `runtime/src/map/types.rs` `#[cfg(test)]` module: create a SiteMap with builder (10 nodes, 15 edges), serialize, deserialize, verify equality. Test filter by page_type. Test nearest neighbor. Test pathfinding prep.
- [ ] **T-027** Run `cargo test` — all map tests pass.
- [ ] **T-028** Run `cargo clippy -- -D warnings` — zero warnings.
- [ ] **T-029** `git add -A && git commit -m "T-029: SiteMap types, builder, serializer, reader"`

### 1C: Chromium Management

- [ ] **T-030** Implement `scripts/install-chromium.sh` — detect platform (linux-x64, linux-arm64, mac-x64, mac-arm64), query `https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json` for latest stable Chrome for Testing URL, download to `~/.cortex/chromium/`, make executable, print success. Handle errors (no internet, wrong platform, disk space).
- [ ] **T-031** Implement `runtime/src/renderer/mod.rs` — define traits:
  ```rust
  #[async_trait]
  pub trait Renderer: Send + Sync {
      async fn new_context(&self) -> Result<Box<dyn RenderContext>>;
      async fn shutdown(&self) -> Result<()>;
      fn active_contexts(&self) -> usize;
  }
  #[async_trait]  
  pub trait RenderContext: Send + Sync {
      async fn navigate(&mut self, url: &str, timeout_ms: u64) -> Result<NavigationResult>;
      async fn execute_js(&self, script: &str) -> Result<serde_json::Value>;
      async fn get_html(&self) -> Result<String>;
      async fn get_url(&self) -> Result<String>;
      async fn close(self: Box<Self>) -> Result<()>;
  }
  pub struct NavigationResult { pub final_url: String, pub status: u16, pub redirect_chain: Vec<String>, pub load_time_ms: u64 }
  ```
- [ ] **T-032** Implement `runtime/src/renderer/chromium.rs` — `ChromiumRenderer` implementing `Renderer`. Find Chromium binary (check env CORTEX_CHROMIUM_PATH → `~/.cortex/chromium/` → system PATH → common locations). Launch headless with chromiumoxide. `ChromiumContext` implementing `RenderContext` with navigate, execute_js, get_html, close.
- [ ] **T-033** Write test: launch Chromium, navigate to `data:text/html,<h1>Hello</h1><p>World</p>`, execute JS `document.querySelector('h1').textContent`, verify returns "Hello", close.
- [ ] **T-034** Run `cargo test` — Chromium test passes.
- [ ] **T-035** `git add -A && git commit -m "T-035: Chromium renderer with trait abstraction"`

### 1D: Socket Server

- [ ] **T-040** Implement `runtime/src/protocol.rs` — Define `Method` enum (Handshake, Map, Query, Pathfind, Refresh, Act, Watch, Perceive, Status). Function `parse_request(json: &str) -> Result<(String, Method, Value)>` returns (id, method, params). Functions `format_response(id, result) -> String`, `format_error(id, code, message) -> String`. All responses are newline-terminated JSON.
- [ ] **T-041** Implement `runtime/src/server.rs` — `Server` struct with `socket_path: PathBuf`. Method `start(&self) -> Result<()>`: bind Unix domain socket, accept connections in loop, spawn tokio task per connection, read newline-delimited JSON, parse, dispatch to handler, write response. Handle `handshake` and `status` methods initially. Other methods return `E_INVALID_METHOD` temporarily.
- [ ] **T-042** Implement `runtime/src/cli/start.rs` — Start the server: remove stale socket file, create server, write PID to `~/.cortex/cortex.pid`, register SIGTERM/SIGINT handlers for clean shutdown (remove socket + PID file), run server as foreground process.
- [ ] **T-043** Implement `runtime/src/cli/stop.rs` — Read PID from `~/.cortex/cortex.pid`, send SIGTERM, wait up to 5 seconds for process to exit, clean up PID file.
- [ ] **T-044** Implement `runtime/src/cli/status.rs` — Connect to socket, send `{"id":"s","method":"status","params":{}}`, receive response, print formatted status (version, uptime, pool info).
- [ ] **T-045** Wire everything into `main.rs`: `start` calls `cli::start::run()`, `stop` calls `cli::stop::run()`, `status` calls `cli::status::run()`, `doctor` calls `cli::doctor::run()`.
- [ ] **T-046** Write integration test: start server in background task, connect via socket, send handshake, verify response, send status, verify response, shutdown.
- [ ] **T-047** Run `cargo test` — all tests pass.
- [ ] **T-048** `git add -A && git commit -m "T-048: socket server with handshake and status"`

---

## Phase 2: Cartography Engine

### 2A: Extraction Scripts

- [ ] **T-100** Implement `extractors/shared/dom-walker.ts` — `walkDom(root, visitor, options)`. Recursive DOM traversal. Options: includeShadowDom, includeHidden, maxDepth. Visitor returns false to skip subtree.
- [ ] **T-101** Implement `extractors/shared/visibility-checker.ts` — `isVisible(element): boolean`. Check computed display, visibility, opacity, dimensions, overflow clipping.
- [ ] **T-102** Implement `extractors/shared/bbox-calculator.ts` — `getBBox(element): {x,y,w,h}` using getBoundingClientRect.
- [ ] **T-103** Implement `extractors/core/content.ts` — `extractContent(document): ContentBlock[]`. Walk DOM, identify: headings (h1-h6), paragraphs, prices (regex `$X,XXX.XX` + schema.org), ratings (star patterns + schema.org), tables, images, lists, code blocks, forms. Assign IDs `cb_001`, `cb_002`. Set confidence (schema.org=0.99, aria=0.95, heuristic=0.8). Include bbox and visibility.
- [ ] **T-104** Implement `extractors/core/actions.ts` — `extractActions(document): ActionRecord[]`. Find all interactable elements: buttons, links (that trigger JS), inputs, selects, checkboxes, radio buttons. Map each to opcode from the OpCode table. Classify risk (safe/cautious/destructive). Detect expected outcome.
- [ ] **T-105** Implement `extractors/core/navigation.ts` — `extractNavigation(document): NavLink[]`. Find all `<a>` tags. Resolve relative URLs. Classify: internal/external/pagination/anchor/download. Detect pagination patterns (next/prev, page numbers). Extract breadcrumbs.
- [ ] **T-106** Implement `extractors/core/structure.ts` — `extractStructure(document): PageStructure`. Detect page regions: header, navigation, main content, sidebar, footer. Count elements by type. Calculate text density. Detect forms, tables, media.
- [ ] **T-107** Implement `extractors/core/metadata.ts` — `extractMetadata(document): Metadata`. Parse JSON-LD, schema.org microdata, OpenGraph tags, meta tags, canonical URL, robots directives, lang attribute.
- [ ] **T-108** Update `extractors/build.sh` to compile all 5 core extractors + shared utils into bundles in `extractors/dist/`.
- [ ] **T-109** Run `cd extractors && npm run build` — all bundles generated.
- [ ] **T-110** `git add -A && git commit -m "T-110: all extraction scripts"`

### 2B: Extraction Loader

- [ ] **T-111** Implement `runtime/src/extraction/mod.rs` — module declarations.
- [ ] **T-112** Implement `runtime/src/extraction/loader.rs` — `ExtractionLoader` struct. `load(name: &str) -> Result<String>`: read JS bundle from embedded path or disk path. `inject_and_run(context: &dyn RenderContext, scripts: &[&str]) -> Result<Value>`: inject all scripts into page, execute extraction entry point, collect results as JSON. Return combined extraction result with content, actions, navigation, structure, metadata.
- [ ] **T-113** Write test: create a `data:text/html` page with a heading, paragraph, link, and button. Inject content extractor. Verify extraction returns heading block and text block.
- [ ] **T-114** `git add -A && git commit -m "T-114: extraction loader"`

### 2C: Sitemap & URL Analysis

- [ ] **T-120** Implement `runtime/src/cartography/mod.rs` — module declarations.
- [ ] **T-121** Implement `runtime/src/cartography/sitemap.rs` — `parse_sitemap(xml: &str) -> Result<Vec<SitemapEntry>>`. Handle: sitemap index (recursive), urlset, lastmod, changefreq, priority. Use `quick-xml` crate. `SitemapEntry { url: String, lastmod: Option<DateTime>, priority: Option<f32> }`.
- [ ] **T-122** Implement `runtime/src/cartography/robots.rs` — `parse_robots(txt: &str, user_agent: &str) -> RobotsRules`. Extract: allowed/disallowed paths, crawl-delay, sitemap URLs. `RobotsRules { allowed: Vec<String>, disallowed: Vec<String>, crawl_delay: Option<f32>, sitemaps: Vec<String> }`. Method `is_allowed(&self, path: &str) -> bool`.
- [ ] **T-123** Implement `runtime/src/cartography/url_classifier.rs` — `classify_url(url: &str, domain: &str) -> (PageType, f32)`. Returns classified page type and confidence. Rules (in priority order): (1) exact pattern matches per known domains (amazon `/dp/` = product, `/s?` = search). (2) Generic URL patterns (`/product`, `/item`, `/p/` = product; `/cart`, `/basket` = cart; `/login`, `/signin` = login; `/checkout` = checkout; `/blog/`, `/post/`, `/article/` = article; `/about` = about; `/contact` = contact; `/faq` = faq; `/search` = search_results; `/category`, `/c/` = product_listing). (3) File extension patterns (`.pdf` = download, image extensions = media). (4) Default: unknown with confidence 0.3.
- [ ] **T-124** Write tests: parse a real sitemap.xml string (create one in test), verify entries. Classify 20 URLs from different patterns. Parse robots.txt, verify rules.
- [ ] **T-125** `git add -A && git commit -m "T-125: sitemap parser, robots parser, URL classifier"`

### 2D: Feature Encoder

- [ ] **T-130** Implement `runtime/src/cartography/page_classifier.rs` — `classify_page(extraction_result: &ExtractionResult, url: &str) -> (PageType, f32)`. Takes full extraction output. Classifies using: (1) schema.org type → PageType mapping, (2) URL classification, (3) DOM heuristics (form count, product-like elements, article structure). Returns highest-confidence classification.
- [ ] **T-131** Implement `runtime/src/cartography/feature_encoder.rs` — `encode_features(extraction: &ExtractionResult, nav_result: &NavigationResult, url: &str) -> [f32; 128]`. Fills ALL 128 dimensions from 02-map-spec.md. Each dimension has documented normalization. Dimensions without data default to 0.0. For commerce dimensions on non-commerce pages, set all to 0.0.
- [ ] **T-132** Implement `runtime/src/cartography/action_encoder.rs` — `encode_actions(extracted_actions: &[ExtractedAction]) -> Vec<ActionRecord>`. Map each extracted action to the nearest OpCode. "Add to Cart" button → OpCode(0x02, 0x00). "Submit" → OpCode(0x03, 0x05). "Login" → OpCode(0x04, 0x00). Fall back to generic navigation opcode for unrecognized actions.
- [ ] **T-133** Implement `runtime/src/cartography/interpolator.rs` — `interpolate_features(page_type: PageType, samples: &[&[f32; 128]]) -> [f32; 128]`. For unrendered pages: average the feature vectors of rendered pages with the same PageType. Set confidence dimension to 0.5. Set freshness to 0.0.
- [ ] **T-134** Write tests: encode features from a mock extraction result, verify specific dimensions. Interpolate from 3 samples, verify result is averaged. Encode actions from mock action list.
- [ ] **T-135** `git add -A && git commit -m "T-135: feature encoder, action encoder, interpolator"`

### 2E: Crawler

- [ ] **T-140** Implement `runtime/src/cartography/rate_limiter.rs` — `RateLimiter` struct. Respects crawl-delay from robots.txt. Default: max 5 concurrent requests per domain, 100ms minimum between requests. Async `acquire(&self) -> Result<()>` blocks until rate limit allows.
- [ ] **T-141** Implement `runtime/src/cartography/sampler.rs` — `select_samples(classified_urls: &[(String, PageType, f32)], max_render: usize) -> Vec<String>`. Strategy: ensure at least 2 samples per PageType present. Fill remaining budget proportionally by PageType frequency. Prefer higher-confidence classifications. Always include root/home page.
- [ ] **T-142** Implement `runtime/src/cartography/crawler.rs` — `Crawler` struct with `PoolManager` and `RateLimiter`. Method `crawl_and_discover(entry_urls: &[String], max_pages: usize) -> Vec<DiscoveredPage>`. Breadth-first: render pages in parallel (up to pool limit), extract navigation links from each, add new URLs to queue, deduplicate, stop when max_pages reached or no new URLs. Each rendered page returns: url, final_url, status, extraction_result, discovered_links.
- [ ] **T-143** Write test using wiremock: serve a mini 5-page site, crawl it, verify all 5 pages discovered, verify link structure.
- [ ] **T-144** `git add -A && git commit -m "T-144: crawler, sampler, rate limiter"`

### 2F: Mapper (The Core)

- [ ] **T-150** Implement `runtime/src/cartography/mapper.rs` — `Mapper` struct. This is the core: orchestrates the entire mapping process. Method `map(&self, request: MapRequest) -> Result<SiteMap>`:
  1. Fetch robots.txt → parse → extract sitemap URLs + rules
  2. Fetch sitemap.xml(s) → parse → collect all URLs
  3. If no sitemap: crawl from entry points to discover URLs (crawler.crawl_and_discover)
  4. Classify all URLs (url_classifier)
  5. Select samples for rendering (sampler.select_samples)
  6. Render samples in parallel using browser pool (crawler renders + extraction)
  7. For each rendered page: classify (page_classifier), encode features (feature_encoder), encode actions (action_encoder)
  8. For unrendered pages: interpolate features from rendered samples (interpolator)
  9. Build edges: from rendered pages' navigation links. For unrendered pages: infer edges from URL structure (same path prefix = likely linked)
  10. Build SiteMap using SiteMapBuilder
  11. Return SiteMap
- [ ] **T-151** Wire MAP into protocol handler in `server.rs`: receive MAP request → call mapper.map() → serialize SiteMap to file in `~/.cortex/maps/<domain>.ctx` → return file path + stats.
- [ ] **T-152** Write integration test: serve a mock 20-page site (with sitemap.xml), call MAP, verify returned SiteMap has correct node count, edge structure, and feature vectors for rendered pages.
- [ ] **T-153** Run `cargo test` — all tests pass.
- [ ] **T-154** Run `cargo clippy -- -D warnings` — zero warnings.
- [ ] **T-155** `git add -A && git commit -m "T-155: mapper - complete cartography engine"`

---

## Phase 3: Navigation Engine

- [ ] **T-200** Implement `runtime/src/navigation/mod.rs` — module declarations.
- [ ] **T-201** Implement `runtime/src/navigation/query.rs` — `QueryEngine::execute(map: &SiteMap, query: &NodeQuery) -> Vec<NodeMatch>`. Filter nodes by: page_type (single or list), feature ranges (dimension → min/max), flag requirements. Sort by any feature dimension. Limit results. Use SIMD-friendly iteration over feature matrix.
- [ ] **T-202** Implement `runtime/src/navigation/pathfinder.rs` — `find_path(map: &SiteMap, from: u32, to: u32, constraints: &PathConstraints) -> Option<Path>`. Implement Dijkstra's algorithm on the SiteMap's edge structure using `petgraph` or manual implementation on the CSR arrays. PathConstraints: avoid_flags (skip nodes with certain flags), minimize ("hops" uses unit weights, "weight" uses edge weights, "state_changes" counts edges with `changes_state` flag). Path struct: `nodes: Vec<u32>, total_weight: f32, hops: u32, required_actions: Vec<PathAction>`.
- [ ] **T-203** Implement `runtime/src/navigation/similarity.rs` — `nearest_neighbors(map: &SiteMap, target: &[f32; 128], k: usize) -> Vec<NodeMatch>`. Brute-force cosine similarity scan over feature matrix. Precomputed norms in NodeRecord.feature_norm speed up computation. Return top-k matches sorted by similarity.
- [ ] **T-204** Implement `runtime/src/navigation/cluster.rs` — `compute_clusters(map: &mut SiteMap)`. K-means clustering on feature vectors. k = max(3, sqrt(node_count / 10)). Assign cluster_assignments and compute cluster_centroids. Assign cluster_types based on dominant PageType in each cluster.
- [ ] **T-205** Wire QUERY and PATHFIND into protocol handler in `server.rs`.
- [ ] **T-206** Write tests: create a 50-node SiteMap with varied features. Test filter returns correct subset. Test pathfinding returns shortest path. Test nearest-neighbor returns correct ranked results. Test clustering produces sensible groupings.
- [ ] **T-207** Run `cargo test` — all pass.
- [ ] **T-208** `git add -A && git commit -m "T-208: navigation engine - query, pathfind, similarity, clustering"`

---

## Phase 4: Thin Clients

### 4A: Python Client

- [ ] **T-300** Implement `clients/python/cortex_client/errors.py` — `CortexError(Exception)`, `CortexConnectionError`, `CortexTimeoutError`, `CortexResourceError`, `CortexMapError`, `CortexPathError`.
- [ ] **T-301** Implement `clients/python/cortex_client/connection.py` — `Connection` class. Uses stdlib `socket` (AF_UNIX). Methods: `connect(socket_path="/tmp/cortex.sock")`, `send(method, params) -> dict` (serialize JSON + newline, read response line, deserialize), `close()`. Timeout handling (default 60s). Reconnect on broken pipe.
- [ ] **T-302** Implement `clients/python/cortex_client/autostart.py` — `ensure_running()`. Check if `/tmp/cortex.sock` exists and is responsive (send handshake). If not: find `cortex` binary (PATH, common locations), run `cortex start &` as subprocess, wait up to 15 seconds for socket, raise CortexConnectionError if timeout.
- [ ] **T-303** Implement `clients/python/cortex_client/sitemap.py` — `SiteMap` class wrapping protocol responses. Properties: `domain`, `node_count`, `edge_count`. Methods: `filter(page_type, features, flags, sort_by, limit) -> list[NodeMatch]` (sends QUERY), `nearest(goal_vector, k) -> list[NodeMatch]` (sends QUERY with similarity), `pathfind(from_node, to_node, avoid_flags, minimize) -> Path | None` (sends PATHFIND), `refresh(nodes, cluster, stale_threshold) -> RefreshResult` (sends REFRESH), `act(node, opcode, params, session_id) -> ActResult` (sends ACT), `watch(nodes, cluster, features, interval_ms) -> Iterator[WatchDelta]` (sends WATCH, reads stream). Dataclasses: `NodeMatch`, `Path`, `PathAction`, `RefreshResult`, `ActResult`, `WatchDelta`.
- [ ] **T-304** Implement `clients/python/cortex_client/protocol.py` — Helper functions building protocol messages for each method. `map_request(domain, **kwargs) -> dict`, `query_request(domain, filters, ...) -> dict`, etc.
- [ ] **T-305** Implement `clients/python/cortex_client/__init__.py` — Public API:
  ```python
  def map(domain, **kwargs) -> SiteMap
  def map_many(domains, **kwargs) -> list[SiteMap]
  def perceive(url, **kwargs) -> PageResult
  def perceive_many(urls, **kwargs) -> list[PageResult]
  def status() -> RuntimeStatus
  ```
  All functions call `ensure_running()` first, then use Connection to communicate.
- [ ] **T-306** Implement `clients/python/tests/test_sitemap.py` — Test SiteMap dataclass construction from mock data. Test filter/nearest/pathfind method signatures.
- [ ] **T-307** Implement `clients/python/tests/test_integration.py` — Integration test: requires running Cortex. Map `example.com`, verify SiteMap returned with >0 nodes. Test query. Test pathfind.
- [ ] **T-308** Run `cd clients/python && pip install -e ".[dev]" && pytest tests/ -x` — passes.
- [ ] **T-309** Run `cd clients/python && ruff check . && ruff format --check . && mypy --strict cortex_client/` — clean.
- [ ] **T-310** `git add -A && git commit -m "T-310: Python thin client"`

### 4B: TypeScript Client

- [ ] **T-320** Implement `clients/typescript/src/connection.ts` — Unix socket client using Node `net` module. Same protocol: JSON + newline over Unix socket. Connect, send, receive, close.
- [ ] **T-321** Implement `clients/typescript/src/sitemap.ts` — TypeScript interfaces: `SiteMap`, `NodeMatch`, `Path`, `NodeQuery`. Class `SiteMapClient` with filter, nearest, pathfind, refresh, act, watch methods.
- [ ] **T-322** Implement `clients/typescript/src/client.ts` — `map(domain, options?) -> SiteMap`, `perceive(url, options?) -> PageResult`, `status() -> RuntimeStatus`.
- [ ] **T-323** Implement `clients/typescript/src/index.ts` — Re-export all public types and functions.
- [ ] **T-324** Run `cd clients/typescript && npm run build` — compiles clean.
- [ ] **T-325** `git add -A && git commit -m "T-325: TypeScript thin client"`

### 4C: Client Conformance

- [ ] **T-330** Implement `clients/conformance/test_map.json` — Test cases: map a fixture site, verify node_count in expected range.
- [ ] **T-331** Implement `clients/conformance/test_query.json` — Test cases: query by page_type, verify results match expected.
- [ ] **T-332** Implement `clients/conformance/test_pathfind.json` — Test cases: pathfind between known nodes, verify path exists.
- [ ] **T-333** Implement `clients/conformance/runner.py` — Start a local HTTP server with test fixtures, start Cortex, for each test case: invoke client (Python via import, TS via subprocess), check assertions, report pass/fail.
- [ ] **T-334** Run conformance tests for Python client — all pass.
- [ ] **T-335** Run conformance tests for TypeScript client — all pass.
- [ ] **T-336** `git add -A && git commit -m "T-336: client conformance test suite"`

---

## Phase 5: Live Interaction

- [ ] **T-400** Implement `runtime/src/pool/mod.rs` — module declarations.
- [ ] **T-401** Implement `runtime/src/pool/manager.rs` — `PoolManager`. Manages browser contexts. `acquire() -> Result<ContextHandle>`: if active < max, create new; if at max, queue. `release(handle)`: return context to pool or close if over limit. Configurable `max_contexts` (default 8). Track memory usage per context.
- [ ] **T-402** Implement `runtime/src/pool/resource_governor.rs` — Enforce memory_limit_mb. Track total browser memory. When limit approached, evict oldest idle context. Per-request timeout enforcement.
- [ ] **T-403** Implement `runtime/src/live/mod.rs` — module declarations.
- [ ] **T-404** Implement `runtime/src/live/perceive.rs` — PERCEIVE handler. Render single URL, run all extractors, encode features, optionally include raw content text. Return PageResult with encoding + optional content.
- [ ] **T-405** Implement `runtime/src/live/refresh.rs` — REFRESH handler. Accept node list / cluster / stale_threshold. Re-render specified nodes in parallel. Update map in-place (new features, new freshness, new content_hash). Detect changes by comparing content_hash. Return list of changes.
- [ ] **T-406** Implement `runtime/src/live/act.rs` — ACT handler. Load the target node's URL in a browser context. Find the DOM element matching the opcode (button, input, link). Execute the action (click, fill, select). Wait for page update. Re-extract the page. Return updated features and navigation result.
- [ ] **T-407** Implement `runtime/src/live/session.rs` — Session management for multi-step flows. `Session` holds a persistent browser context with cookies. `create() -> session_id`, `act(session_id, opcode, params) -> result`, `close(session_id)`. Sessions expire after configurable timeout (default 1 hour).
- [ ] **T-408** Implement `runtime/src/live/watch.rs` — WATCH handler. Spawn background task that periodically re-renders specified nodes. Compare feature values. Stream deltas to the client connection.
- [ ] **T-409** Wire PERCEIVE, REFRESH, ACT, WATCH into protocol handler.
- [ ] **T-410** Implement `runtime/src/stealth/mod.rs`, `fingerprint.rs`, `behavior.rs` — Patch navigator.webdriver, chrome.runtime. Add random delays (50-200ms) between actions.
- [ ] **T-411** Write integration tests: perceive a page → verify encoding. Create session → act → verify state change. Refresh a map node → verify updated.
- [ ] **T-412** Run `cargo test` — all pass.
- [ ] **T-413** `git add -A && git commit -m "T-413: live interaction - perceive, refresh, act, watch, sessions"`

---

## Phase 6: Intelligence Layer

- [ ] **T-500** Implement `runtime/src/intelligence/mod.rs` — module declarations.
- [ ] **T-501** Implement `runtime/src/intelligence/cache.rs` — Map caching. Store serialized SiteMaps in `~/.cortex/maps/`. Cache key: domain + hash of mapping parameters. TTL: configurable (default 1 hour). `get_cached(domain) -> Option<SiteMap>`, `cache_map(domain, map)`, `invalidate(domain)`.
- [ ] **T-502** Implement `runtime/src/intelligence/progressive.rs` — After initial map delivery, spawn background task that continues rendering unrendered nodes. Update map on disk as more nodes are rendered. Agent can load updated map with `refresh()`.
- [ ] **T-503** Implement `runtime/src/intelligence/smart_sampler.rs` — Improved sampling: prioritize pages that are structurally important (high inbound link count), pages close to the root, and pages of underrepresented PageTypes. Use first-pass rendering results to inform second-pass sampling.
- [ ] **T-504** Implement `runtime/src/intelligence/cross_site.rs` — `merge_results(maps: &[&SiteMap], query: &NodeQuery) -> Vec<CrossSiteMatch>`. Query across multiple maps, return unified results with domain attribution. Used by `map_many` + cross-site comparison workflows.
- [ ] **T-505** Wire caching into MAP handler: check cache first, return cached map if fresh.
- [ ] **T-506** Write tests: cache a map, retrieve it, verify match. Cross-site query across 2 maps.
- [ ] **T-507** `git add -A && git commit -m "T-507: intelligence layer - caching, progressive refinement, cross-site"`

---

## Phase 7: Framework Integrations + CLI

- [ ] **T-600** Implement `integrations/langchain/cortex_langchain/tools.py`:
  ```python
  class CortexMapTool(BaseTool):
      name = "cortex_map"
      description = "Map an entire website into a navigable graph. Input: domain name."
  class CortexQueryTool(BaseTool):
      name = "cortex_query"  
      description = "Search a mapped website for pages matching criteria."
  class CortexActTool(BaseTool):
      name = "cortex_act"
      description = "Execute an action on a live webpage."
  ```
- [ ] **T-601** Implement `integrations/crewai/cortex_crewai/tools.py` — CortexWebCartographer tool for CrewAI.
- [ ] **T-602** Implement `integrations/openclaw/skills/cortex_map.py` — Map skill for OpenClaw. Follows OpenClaw skill interface.
- [ ] **T-603** Implement `integrations/openclaw/skills/cortex_navigate.py` — Query + pathfind skill for OpenClaw.
- [ ] **T-604** Implement `integrations/openclaw/skill_manifest.json` — Skill manifest declaring cortex-runtime dependency.
- [ ] **T-605** Implement `runtime/src/cli/map_cmd.rs` — `cortex map <domain> [--max-nodes N] [--max-render N] [--timeout N]`. Print: domain, node count, edge count, rendered count, duration. Save map to `~/.cortex/maps/`.
- [ ] **T-606** Implement `runtime/src/cli/query_cmd.rs` — `cortex query <domain> [--type product] [--price-lt 300] [--rating-gt 4] [--limit 10]`. Print matching nodes: index, url, type, key features.
- [ ] **T-607** Implement `runtime/src/cli/pathfind_cmd.rs` — `cortex pathfind <domain> --from <node> --to <node>`. Print path: node indices, URLs, hops, required actions.
- [ ] **T-608** Implement `runtime/src/cli/perceive_cmd.rs` — `cortex perceive <url> [--format pretty|json]`. Print encoding + optional content.
- [ ] **T-609** Wire all CLI commands into main.rs.
- [ ] **T-610** Test CLI: `cargo build --release && ./target/release/cortex map example.com` — prints map stats.
- [ ] **T-611** `git add -A && git commit -m "T-611: framework integrations + CLI polish"`

---

## Phase 8: Hardening + Docs + Release

- [ ] **T-700** Implement `runtime/src/trust/mod.rs`, `credentials.rs` — Encrypted credential vault: SQLite + AES-256-GCM. Store/retrieve credentials by domain. Key from `CORTEX_VAULT_KEY` env or machine-derived secret.
- [ ] **T-701** Implement `runtime/src/trust/pii.rs` — Scan content blocks for email, phone, SSN, credit card patterns. Flag `pii_detected` in features.
- [ ] **T-702** Implement `runtime/src/trust/sandbox.rs` — Before ACT execution: sanitize input values. Block script tags, SQL injection patterns, path traversal.
- [ ] **T-703** Implement `runtime/src/audit/mod.rs`, `logger.rs` — Write JSONL to `~/.cortex/audit.jsonl`. Each line: timestamp, method, domain/url, session_id, duration_ms, status.
- [ ] **T-704** Implement `runtime/src/audit/ledger_sync.rs` — Optional: if `CORTEX_LEDGER_URL` set, POST audit events. Async, non-blocking, failure-tolerant.
- [ ] **T-705** Write `docs/guides/quickstart.md` — From zero to first map in 60 seconds. Test it yourself by following the steps.
- [ ] **T-706** Write `docs/guides/interactive-sessions.md` — How to use ACT for multi-step flows.
- [ ] **T-707** Write `docs/guides/framework-integration.md` — LangChain, CrewAI, OpenClaw examples with working code.
- [ ] **T-708** Write `docs/guides/writing-extractors.md` — How to write and contribute custom extractors.
- [ ] **T-709** Write `docs/guides/environments.md` — Docker, CI, serverless, ARM, air-gapped instructions.
- [ ] **T-710** Write `docs/cookbooks/price-comparison.md` — Complete example: compare prices across 5 sites.
- [ ] **T-711** Write `docs/cookbooks/form-automation.md` — Complete example: fill and submit a multi-step form.
- [ ] **T-712** Write final `README.md` at repo root with: one-liner, quick start, how it works, speed table, framework list, contributing link, license badge.
- [ ] **T-713** Write `CHANGELOG.md` for v0.1.0.
- [ ] **T-714** Create `.github/workflows/ci.yml` from 03-implementation.md Section 7.
- [ ] **T-715** Create `.github/workflows/release.yml` — On tag push: build binaries for linux-x64, linux-arm64, mac-x64, mac-arm64. Create GitHub Release. Publish Python client to PyPI (if token available). Publish TS client to npm (if token available).
- [ ] **T-716** Run full test suite: `make test` — all pass.
- [ ] **T-717** Run full lint suite: `make lint` — all clean.
- [ ] **T-718** Run `cargo build --release` — builds successfully.
- [ ] **T-719** Manual smoke test: `./target/release/cortex doctor && ./target/release/cortex start && ./target/release/cortex map example.com && ./target/release/cortex stop` — all work.
- [ ] **T-720** `git add -A && git commit -m "T-720: v0.1.0 release ready"`
- [ ] **T-721** `git tag v0.1.0`

---

**Total: ~100 tasks. Estimated: 10-14 weeks continuous build.**
**Every task is atomic. Every task has a clear exit condition. No approval needed.**
