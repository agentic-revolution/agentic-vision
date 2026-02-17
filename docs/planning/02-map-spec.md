# 02 — Map Specification: Binary Formats, Protocol, and Schemas

> This document defines every binary format, protocol message, enumeration, and schema in Cortex. All implementations must match these specifications exactly.

## 1. SiteMap Binary Format

The SiteMap is the primary data structure. It represents an entire website as a navigable binary graph.

```
SiteMap {
  // Header (48 bytes)
  magic: uint32 = 0x43545800           // "CTX\0"
  format_version: uint16 = 1
  domain_length: uint16
  domain: utf8[domain_length]           // e.g. "amazon.com"
  mapped_at: uint64                     // unix timestamp seconds
  node_count: uint32
  edge_count: uint32
  cluster_count: uint16
  flags: uint16                         // bit 0: has_sitemap, bit 1: progressive_active, bit 2: cached
  padding: bytes[to align to 48]

  // Node Table: fixed-size records (32 bytes each)
  nodes: NodeRecord[node_count]

  // Edge Table: adjacency list (8 bytes each)
  edges: EdgeRecord[edge_count]
  edge_index: uint32[node_count + 1]    // CSR format: edge_index[i] to edge_index[i+1] = edges for node i

  // Feature Matrix: contiguous float array (512 bytes per node = 128 × 4 bytes)
  features: float32[node_count][128]

  // Action Catalog: variable-size (8 bytes per action)
  actions: ActionRecord[]
  action_index: uint32[node_count + 1]  // CSR format: action_index[i] to action_index[i+1] = actions for node i

  // Cluster Table
  cluster_assignments: uint16[node_count]      // which cluster each node belongs to
  cluster_centroids: float32[cluster_count][128] // centroid feature vector per cluster
  cluster_types: uint8[cluster_count]           // cluster classification

  // URL Table: variable-size strings, null-terminated
  url_data: bytes[]
  url_index: uint32[node_count]          // byte offset into url_data for each node's URL
}
```

### NodeRecord (32 bytes)

```
NodeRecord {
  page_type: uint8              // PageType enum (see below)
  confidence: uint8             // 0-255 → maps to 0.0-1.0
  freshness: uint8              // 0-255 → maps to 0.0-1.0
  flags: uint8                  // NodeFlags bitfield (see below)
  content_hash: uint32          // FNV-1a hash of extracted content (for change detection)
  rendered_at: uint32           // seconds since mapping start (0 if never rendered)
  http_status: uint16           // HTTP status code (200, 301, 404, etc.) or 0 if unknown
  depth: uint16                 // distance from root node in hops
  inbound_count: uint16         // number of edges pointing TO this node
  outbound_count: uint16        // number of edges FROM this node
  feature_norm: float32         // L2 norm of feature vector (precomputed for fast similarity)
  reserved: uint32              // future use, set to 0
}

NodeFlags (uint8 bitfield):
  bit 0: rendered             // 1 if this page was actually rendered in a browser
  bit 1: estimated            // 1 if features are interpolated, not extracted
  bit 2: stale                // 1 if freshness < 0.3
  bit 3: blocked              // 1 if bot detection prevented access
  bit 4: auth_required        // 1 if page requires authentication
  bit 5: has_form             // 1 if page contains interactive forms
  bit 6: has_price            // 1 if price data was extracted
  bit 7: has_media            // 1 if page contains video/audio/gallery
```

### EdgeRecord (8 bytes)

```
EdgeRecord {
  target_node: uint32          // target node index
  edge_type: uint8             // EdgeType enum (see below)
  weight: uint8                // traversal cost: 0=free/instant, 255=expensive/slow
  flags: uint8                 // EdgeFlags bitfield (see below)
  reserved: uint8
}

EdgeType (uint8):
  0x00  navigation             // standard nav link (menu, header, footer)
  0x01  content_link           // link within page content (article body, description)
  0x02  pagination             // next/prev/page-N links
  0x03  related                // "related items", "you might also like"
  0x04  breadcrumb             // breadcrumb navigation link
  0x05  form_submit            // form submission leads here
  0x06  action_result          // clicking a button leads here
  0x07  redirect               // HTTP redirect
  0x08  external               // link to different domain
  0x09  anchor                 // same-page anchor link

EdgeFlags (uint8 bitfield):
  bit 0: requires_auth         // traversal requires login
  bit 1: requires_form         // traversal requires form input
  bit 2: changes_state         // traversal modifies server state (POST, cart, etc.)
  bit 3: opens_new_context     // opens in new tab/window
  bit 4: is_download           // triggers a file download
  bit 5: is_nofollow           // rel="nofollow" link
```

### ActionRecord (8 bytes)

```
ActionRecord {
  opcode: uint16               // [category:uint8][action:uint8] — see OpCode Table
  target_node: int32           // node this action navigates to (-1 if stays on page, -2 if unknown)
  cost_hint: uint8             // 0=free, 1-254=relative cost, 255=unknown
  risk: uint8                  // 0=safe, 1=cautious, 2=destructive
}
```

## 2. Page Type Enum

```
PageType (uint8):
  0x00  unknown
  0x01  home
  0x02  search_results
  0x03  product_listing
  0x04  product_detail
  0x05  article
  0x06  documentation
  0x07  form_page
  0x08  login
  0x09  checkout
  0x0A  cart
  0x0B  account
  0x0C  error_page
  0x0D  captcha
  0x0E  media_page
  0x0F  comparison
  0x10  review_list
  0x11  map_location
  0x12  dashboard
  0x13  api_docs
  0x14  legal
  0x15  download_page
  0x16  social_feed
  0x17  forum
  0x18  messaging
  0x19  calendar
  0x1A  file_browser
  0x1B  pricing_page
  0x1C  about_page
  0x1D  contact_page
  0x1E  faq
  0x1F  sitemap_page
```

Classification method (in priority order):
1. Schema.org / JSON-LD type annotations on the page
2. URL pattern matching (configurable per domain)
3. DOM structure heuristics (presence of specific elements)
4. Content analysis (heading text patterns, form types)
5. Default: 0x00 unknown

## 3. OpCode Table

Actions are encoded as two-byte operation codes: `[category:uint8][action:uint8]`

```
Category 0x00 — Navigation:
  0x0000  go_back
  0x0001  go_forward
  0x0002  go_home
  0x0003  follow_link             // params: target_node_id
  0x0004  scroll_down
  0x0005  scroll_to_element       // params: element_id
  0x0006  paginate_next
  0x0007  paginate_prev
  0x0008  paginate_to             // params: page_number
  0x0009  open_tab                // params: url
  0x000A  close_tab

Category 0x01 — Search:
  0x0100  search_submit           // params: query_string
  0x0101  filter_apply            // params: filter_id, value
  0x0102  filter_remove           // params: filter_id
  0x0103  sort_by                 // params: field, direction(0=asc,1=desc)
  0x0104  clear_filters
  0x0105  autocomplete_select     // params: suggestion_index

Category 0x02 — Commerce:
  0x0200  add_to_cart             // params: variant_index, quantity
  0x0201  remove_from_cart        // params: item_index
  0x0202  update_quantity         // params: item_index, quantity
  0x0203  buy_now                 // params: variant_index
  0x0204  save_for_later
  0x0205  add_to_wishlist
  0x0206  apply_coupon            // params: coupon_code
  0x0207  select_variant          // params: variant_index
  0x0208  compare_add
  0x0209  compare_remove
  0x020A  check_availability      // params: zip_code
  0x020B  notify_when_available

Category 0x03 — Form:
  0x0300  fill_text               // params: field_id, value
  0x0301  select_option           // params: field_id, option_index
  0x0302  toggle_checkbox         // params: field_id
  0x0303  select_radio            // params: field_id, option_index
  0x0304  upload_file             // params: field_id, file_path
  0x0305  submit_form
  0x0306  clear_form
  0x0307  next_step
  0x0308  prev_step
  0x0309  fill_date               // params: field_id, date_iso
  0x030A  fill_number             // params: field_id, value

Category 0x04 — Auth:
  0x0400  login                   // params: credential_id
  0x0401  logout
  0x0402  register
  0x0403  forgot_password
  0x0404  oauth_login             // params: provider (0=google,1=facebook,2=apple,3=github)
  0x0405  mfa_submit              // params: code
  0x0406  accept_terms

Category 0x05 — Media:
  0x0500  play
  0x0501  pause
  0x0502  seek                    // params: position_seconds
  0x0503  fullscreen
  0x0504  download
  0x0505  next_image
  0x0506  prev_image
  0x0507  zoom                    // params: level (0-255)

Category 0x06 — Social:
  0x0600  like
  0x0601  share                   // params: platform
  0x0602  comment                 // params: text
  0x0603  follow
  0x0604  report
  0x0605  bookmark

Category 0x07 — System:
  0x0700  dismiss_modal
  0x0701  accept_cookies
  0x0702  reject_cookies
  0x0703  close_popup
  0x0704  dismiss_notification
  0x0705  change_language         // params: locale_code
  0x0706  change_currency         // params: currency_code
  0x0707  accept_age_gate
  0x0708  dismiss_banner
```

## 4. Feature Vector Schema (128 dimensions)

Every node has a 128-float feature vector. Dimensions are fixed and standardized:

```
Dimensions 0-15: Page Identity
  [0]   page_type_normalized              // PageType enum / 31.0 → 0.0-1.0
  [1]   page_type_confidence              // classifier confidence
  [2]   content_language_code             // ISO 639-1 numeric / 100
  [3]   page_depth_normalized             // depth from root / max_depth
  [4]   is_authenticated_area             // 0.0 or 1.0
  [5]   has_paywall                       // 0.0 or 1.0
  [6]   is_mobile_optimized              // 0.0 or 1.0
  [7]   load_time_normalized             // clamp(load_ms / 10000, 0, 1)
  [8]   is_https                         // 0.0 or 1.0
  [9]   url_path_depth                   // count of / in path / 10
  [10]  url_has_query_params             // 0.0 or 1.0
  [11]  url_has_fragment                 // 0.0 or 1.0
  [12]  is_canonical                     // 0.0 or 1.0 (matches canonical URL)
  [13]  has_structured_data              // 0.0 or 1.0 (JSON-LD / microdata present)
  [14]  meta_robots_index               // 0.0 (noindex) or 1.0 (index)
  [15]  reserved_identity               // 0.0

Dimensions 16-47: Content Metrics
  [16]  text_density                     // text_chars / total_element_count, clamp 0-1
  [17]  text_length_log                  // log10(char_count) / 6 (max ~1M chars)
  [18]  heading_count_normalized         // min(heading_count / 20, 1.0)
  [19]  paragraph_count_normalized       // min(para_count / 50, 1.0)
  [20]  image_count_normalized           // min(img_count / 30, 1.0)
  [21]  video_present                    // 0.0 or 1.0
  [22]  table_count_normalized           // min(table_count / 10, 1.0)
  [23]  list_count_normalized            // min(list_count / 20, 1.0)
  [24]  form_field_count_normalized      // min(field_count / 30, 1.0)
  [25]  link_count_internal_normalized   // min(internal_links / 100, 1.0)
  [26]  link_count_external_normalized   // min(external_links / 50, 1.0)
  [27]  ad_density                       // estimated ad area / total area
  [28]  content_uniqueness               // 1.0 - (boilerplate_ratio)
  [29]  reading_level_normalized         // Flesch-Kincaid grade / 20, clamp 0-1
  [30]  sentiment_normalized             // (raw_sentiment + 1) / 2 → 0.0-1.0
  [31-46] topic_embedding                // 16-dim topic vector (TF-IDF or similar)
  [47]  structured_data_richness         // count of schema.org properties / 50

Dimensions 48-63: Commerce Features (0.0 for non-commerce pages)
  [48]  price_raw                        // actual price in USD (NOT normalized — for direct comparison)
  [49]  price_original                   // original price if discounted (0 if no discount)
  [50]  discount_percentage              // 0.0-1.0
  [51]  availability                     // 0.0=out of stock, 0.5=limited, 1.0=in stock
  [52]  rating_normalized                // rating / max_rating (e.g. 4.6/5.0 = 0.92)
  [53]  review_count_log                 // log10(review_count) / 6
  [54]  review_sentiment                 // 0.0-1.0
  [55]  shipping_free                    // 0.0 or 1.0
  [56]  shipping_speed_normalized        // 0.0=slow, 0.5=standard, 1.0=same day
  [57]  return_policy_score              // 0.0=no returns, 0.5=paid, 1.0=free returns
  [58]  seller_reputation                // 0.0-1.0
  [59]  variant_count_normalized         // min(variants / 20, 1.0)
  [60]  comparison_available             // 0.0 or 1.0
  [61]  price_trend                      // -1.0=falling, 0.0=stable, 1.0=rising
  [62]  category_price_percentile        // where this price falls: 0.0=cheapest, 1.0=most expensive
  [63]  deal_score                       // composite: discount × rating × availability

Dimensions 64-79: Navigation Features
  [64]  outbound_link_count_normalized   // min(outbound / 100, 1.0)
  [65]  pagination_present               // 0.0 or 1.0
  [66]  pagination_position              // 0.0=first page, 1.0=last page
  [67]  breadcrumb_depth                 // breadcrumb items / 10
  [68]  nav_menu_items_normalized        // min(menu_items / 30, 1.0)
  [69]  search_available                 // 0.0 or 1.0
  [70]  filter_count_normalized          // min(filters / 20, 1.0)
  [71]  sort_options_normalized          // min(sort_options / 10, 1.0)
  [72]  related_content_count            // min(related_items / 20, 1.0)
  [73]  estimated_next_relevance         // predicted relevance of outbound links
  [74]  is_dead_end                      // 0.0 or 1.0 (no useful outbound links)
  [75]  site_section_depth               // depth within current section
  [76]  site_section_breadth             // siblings in current section / 100
  [77]  goal_distance_estimate           // -1=unknown, 0=at goal, 1.0=far from goal
  [78]  loop_risk                        // probability of revisiting already-seen nodes
  [79]  exit_probability                 // probability this leads to external domain

Dimensions 80-95: Trust & Safety
  [80]  tls_valid                        // 0.0 or 1.0
  [81]  domain_age_normalized            // log(days_since_registration) / 12
  [82]  domain_reputation                // 0.0-1.0 (based on known reputation lists)
  [83]  dark_pattern_count               // min(count / 5, 1.0)
  [84]  pii_exposure_risk                // 0.0-1.0
  [85]  content_consistency              // structured data vs visible content match
  [86]  bot_challenge_present            // 0.0 or 1.0
  [87]  bot_challenge_severity           // 0.0=none, 0.5=js challenge, 1.0=captcha
  [88]  cookie_consent_blocking          // 0.0 or 1.0 (blocks content)
  [89]  popup_count_normalized           // min(popups / 5, 1.0)
  [90]  redirect_count                   // redirects / 5, clamp 0-1
  [91]  mixed_content                    // 0.0 or 1.0 (HTTP on HTTPS page)
  [92]  tracker_count_normalized         // min(trackers / 20, 1.0)
  [93]  content_freshness                // 0.0=ancient, 1.0=just published
  [94]  authority_score                  // 0.0-1.0 (domain authority)
  [95]  scam_probability                 // 0.0-1.0

Dimensions 96-111: Action Features
  [96]  action_count_normalized          // min(actions / 20, 1.0)
  [97]  safe_action_ratio                // safe_actions / total_actions
  [98]  cautious_action_ratio            // cautious / total
  [99]  destructive_action_ratio         // destructive / total
  [100] auth_required_ratio              // auth_required / total
  [101] form_completeness                // filled_fields / total_fields
  [102] form_steps_remaining             // remaining / total steps
  [103] cart_item_count_normalized       // min(items / 20, 1.0)
  [104] cart_total_normalized            // log10(cart_total) / 5
  [105] checkout_steps_remaining         // remaining / total
  [106] primary_cta_present              // 0.0 or 1.0
  [107] primary_cta_category             // cta's opcode category / 7.0
  [108] download_available               // 0.0 or 1.0
  [109] share_available                  // 0.0 or 1.0
  [110] save_available                   // 0.0 or 1.0
  [111] undo_available                   // 0.0 or 1.0

Dimensions 112-127: Session & Context
  [112] session_page_count               // pages visited in session / 50
  [113] session_action_count             // actions taken / 100
  [114] session_duration_normalized      // seconds / 3600
  [115] unique_domains_visited           // domains / 20
  [116] flow_step_current                // current_step / total_steps (0 if not in flow)
  [117] flow_step_total_normalized       // total_steps / 10
  [118] flow_completion                  // current / total (0.0-1.0)
  [119] backtrack_count                  // times agent went back / 10
  [120] revisit_ratio                    // revisited_pages / total_visited
  [121] data_extracted_normalized        // subjective: how much useful data found
  [122] goal_similarity                  // cosine similarity to goal vector if provided
  [123] time_budget_remaining            // 0.0=expired, 1.0=full budget
  [124] page_budget_remaining            // 0.0=spent, 1.0=full budget
  [125] error_count_normalized           // errors / 10
  [126] blocked_count_normalized         // blocked / 10
  [127] session_health                   // composite 0.0-1.0
```

## 5. Socket Protocol

Communication between agent clients and Cortex local process uses JSON-over-Unix-domain-socket with newline-delimited messages.

**Socket path:** `/tmp/cortex.sock` (Linux/macOS), `\\.\pipe\cortex` (Windows)

### Message Format

Request:
```json
{"id": "req_<8_random_alphanum>", "method": "<METHOD>", "params": {}}
```

Response:
```json
{"id": "req_<matching>", "result": {}}
```

Error:
```json
{"id": "req_<matching>", "error": {"code": "<ERROR_CODE>", "message": "<human_readable>"}}
```

Streaming (WATCH only):
```json
{"id": "req_x", "stream": true, "delta": {}}
...
{"id": "req_x", "stream": false, "result": {}}
```

### Methods

**HANDSHAKE** — Version negotiation
```json
// Request
{"id": "r1", "method": "handshake", "params": {"client_version": "0.1.0", "protocol_version": 1}}
// Response
{"id": "r1", "result": {"server_version": "0.1.0", "protocol_version": 1, "compatible": true}}
```

**MAP** — Build a site map
```json
// Request
{"id": "r2", "method": "map", "params": {
  "domain": "amazon.com",
  "entry_points": [],            // optional additional entry URLs
  "max_nodes": 50000,            // node limit
  "max_render": 200,             // max pages to actually render
  "max_time_ms": 10000,          // time budget
  "respect_robots": true
}}
// Response: binary SiteMap as base64 in JSON wrapper, or file path
{"id": "r2", "result": {"map_path": "/tmp/cortex/maps/amazon.com.ctx", "node_count": 47832, "edge_count": 142891, "rendered": 187, "estimated": 47645, "duration_ms": 3421}}
```

**QUERY** — Search the map
```json
// Request
{"id": "r3", "method": "query", "params": {
  "map": "amazon.com",           // domain name (loads from cache) or path
  "filters": {
    "page_type": [4],            // PageType: product_detail
    "feature_range": {
      "48": {"lt": 300.0},       // price < $300
      "52": {"gt": 0.8}          // rating > 0.8
    },
    "flags": {"has_price": true}
  },
  "sort_by": {"feature": 48, "direction": "asc"},  // sort by price ascending
  "limit": 20
}}
// Response
{"id": "r3", "result": {"nodes": [
  {"index": 4821, "url": "https://amazon.com/dp/B0...", "page_type": 4, "features": {"48": 268.0, "52": 0.92}, "confidence": 0.95},
  ...
], "total_matches": 347}}
```

**PATHFIND** — Find shortest path between nodes
```json
// Request
{"id": "r4", "method": "pathfind", "params": {
  "map": "amazon.com",
  "from_node": 0,                // root
  "to_node": 4821,               // target product
  "avoid_flags": ["auth_required"],
  "minimize": "hops"             // or "weight" or "state_changes"
}}
// Response
{"id": "r4", "result": {"path": [0, 12, 341, 4821], "total_weight": 3, "hops": 3, "requires_actions": [
  {"at_node": 0, "opcode": [0, 3], "params": {"target": 12}},
  {"at_node": 12, "opcode": [1, 0], "params": {"query": "headphones"}},
  {"at_node": 341, "opcode": [0, 3], "params": {"target": 4821}}
]}}
```

**REFRESH** — Re-render specific nodes
```json
// Request
{"id": "r5", "method": "refresh", "params": {
  "map": "amazon.com",
  "nodes": [4821, 4822, 4823],  // specific nodes
  // OR
  "cluster": 12,                 // entire cluster
  // OR
  "stale_threshold": 0.5         // all nodes with freshness < 0.5
}}
// Response
{"id": "r5", "result": {"refreshed": 3, "changed": 1, "changes": [
  {"node": 4821, "field": "features.48", "old": 278.0, "new": 268.0}
], "duration_ms": 1842}}
```

**ACT** — Execute an action on a live page
```json
// Request
{"id": "r6", "method": "act", "params": {
  "map": "amazon.com",
  "node": 4821,                  // which page
  "opcode": [2, 0],              // add_to_cart
  "action_params": {"variant": 0, "quantity": 1},
  "session_id": "sess_abc123"    // optional, for multi-step flows
}}
// Response
{"id": "r6", "result": {"success": true, "resulting_node": 4821, "node_changed": true, "new_features": {"103": 0.05}}}
```

**WATCH** — Monitor nodes for changes
```json
// Request
{"id": "r7", "method": "watch", "params": {
  "map": "amazon.com",
  "nodes": [4821],               // or "cluster": 12
  "features": [48, 51],          // watch price and availability
  "interval_ms": 60000           // check every 60 seconds
}}
// Streaming responses
{"id": "r7", "stream": true, "delta": {"node": 4821, "feature": 48, "old": 268.0, "new": 259.0, "timestamp": 1740001234}}
...
```

**PERCEIVE** — Single page perception (for live verification/action)
```json
// Request
{"id": "r8", "method": "perceive", "params": {
  "url": "https://amazon.com/dp/B0EXAMPLE",
  "include_content": true        // include text content, not just encoding
}}
// Response
{"id": "r8", "result": {
  "encoding": {"page_type": 4, "features": [0.04, 0.34, ...], "actions": [...]},
  "content": {"title": "Sony WH-1000XM5...", "blocks": [...]}   // only if include_content=true
}}
```

**STATUS** — Runtime info
```json
{"id": "r9", "method": "status", "params": {}}
{"id": "r9", "result": {"version": "0.1.0", "uptime_s": 3421, "maps_cached": 3, "pool": {"active": 2, "max": 8, "memory_mb": 340}, "cache_mb": 47}}
```

## 6. Error Codes

```
E_CONN_REFUSED          Cortex process not running
E_CONN_TIMEOUT          Socket connection timeout
E_INVALID_METHOD        Unknown method name
E_INVALID_PARAMS        Missing or invalid parameters
E_PROTOCOL_VERSION      Incompatible protocol version

E_MAP_TIMEOUT           Mapping exceeded time budget
E_MAP_BLOCKED           Site completely blocked access
E_MAP_DNS_FAILED        Domain DNS resolution failed
E_MAP_TOO_LARGE         Site exceeds max_nodes limit
E_MAP_NO_CONTENT        Site returned no usable content
E_MAP_NOT_FOUND         Requested map not in cache

E_RENDER_TIMEOUT        Page render exceeded timeout
E_RENDER_CRASH          Browser context crashed
E_RENDER_BLOCKED        Bot detection blocked rendering

E_QUERY_INVALID         Invalid filter/sort specification
E_PATHFIND_NO_PATH      No path exists between nodes with given constraints
E_PATHFIND_INVALID      Invalid node indices

E_ACT_FAILED            Action execution failed
E_ACT_BLOCKED           Action blocked by sandbox
E_ACT_STALE             Page state changed since last refresh

E_POOL_EXHAUSTED        All browser contexts busy
E_MEMORY_LIMIT          Memory limit reached
E_SESSION_EXPIRED       Session timed out
E_SESSION_NOT_FOUND     Invalid session ID
```

## 7. Memory Budgets

| Site Size | Nodes | Map Size | Features Matrix | Total |
|-----------|-------|----------|----------------|-------|
| 100 pages | 100 | ~10KB | 50KB | ~70KB |
| 1,000 pages | 1,000 | ~50KB | 500KB | ~600KB |
| 10,000 pages | 10,000 | ~400KB | 5MB | ~6MB |
| 50,000 pages | 50,000 | ~2MB | 25MB | ~30MB |
| 100,000 pages | 100,000 | ~4MB | 50MB | ~60MB |

Target: agent can hold 10 site maps simultaneously in under 300MB.
