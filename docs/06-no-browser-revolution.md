# CORTEX — The No-Browser Revolution

**Why are we fighting websites when they're already handing us the data?**

---

## The Realization

Every single limitation in Cortex v0.1.0 traces back to one thing: **opening a browser.**

```
Bot detection        → because we opened a browser
SPA empty shells     → because the browser didn't finish rendering JS
HTTP/2 errors        → because the browser connection setup failed
Fingerprint issues   → because the browser has a detectable fingerprint
Slow mapping         → because browser rendering takes 200-500ms per page
Memory usage         → because each browser context uses 50-100MB
```

But why are we opening a browser at all?

To get the content of a web page, we render it in Chromium, wait for JavaScript, then extract text, prices, ratings, links. This is the hard way.

The easy way: **websites already publish all of this data in machine-readable format, specifically designed for machines to read, without a browser.**

They do this for Google. We can use the exact same channels.

---

## What Websites Already Give Us (For Free)

### 1. Sitemap.xml — The Complete URL Map

```
GET https://amazon.com/sitemap.xml
→ 50,000+ URLs with lastmod, priority, changefreq
```

One HTTP request. No browser. We already use this. It gives us every URL on the site.

### 2. JSON-LD / Schema.org — The Product Database

This is the breakthrough. Almost every modern website embeds structured data in their HTML specifically for search engines. It's in the raw HTML source — no JavaScript execution needed.

```html
<!-- This is in the raw HTML of almost every product page -->
<script type="application/ld+json">
{
  "@type": "Product",
  "name": "Sony WH-1000XM5 Wireless Headphones",
  "brand": {"@type": "Brand", "name": "Sony"},
  "description": "Industry-leading noise canceling...",
  "sku": "WH1000XM5/B",
  "offers": {
    "@type": "Offer",
    "price": 278.00,
    "priceCurrency": "USD",
    "availability": "https://schema.org/InStock",
    "seller": {"@type": "Organization", "name": "Amazon.com"}
  },
  "aggregateRating": {
    "@type": "AggregateRating",
    "ratingValue": 4.6,
    "reviewCount": 12847
  },
  "image": "https://...",
  "category": "Electronics > Headphones"
}
</script>
```

**This is literally our feature vector.** Price, currency, availability, rating, review count, category, brand — all structured, all typed, all in the raw HTML.

A simple HTTP GET + HTML parse gives us this. No browser. No JavaScript. No bot detection.

### How much of the web has JSON-LD?

```
E-commerce sites:     ~85-95%  (Google Shopping requires it)
News/article sites:   ~80-90%  (Google News requires it)
Recipe sites:         ~90%+    (Google recipe cards require it)
Local businesses:     ~70-80%  (Google Maps / Local Pack)
Job listings:         ~80%+    (Google Jobs requires it)
Event sites:          ~75%+    (Google Events requires it)
Review sites:         ~85%+    (Google review snippets)
FAQ pages:            ~60%+    (Google FAQ snippets)
Documentation:        ~40-50%  (varies)
Social/forums:        ~30-40%  (lower, but still present)

Overall e-commerce:   ~90% of product pages have structured data
```

Google essentially forced the web to become machine-readable. Websites that don't have structured data don't appear in rich search results, which means less traffic, which means less money. So they all added it.

### 3. Meta Tags — Page Classification

```html
<meta property="og:type" content="product" />
<meta property="og:title" content="Sony WH-1000XM5" />
<meta property="og:description" content="..." />
<meta property="og:image" content="https://..." />
<meta property="og:price:amount" content="278.00" />
<meta property="og:price:currency" content="USD" />
<meta name="robots" content="index, follow" />
<link rel="canonical" href="https://amazon.com/dp/..." />
```

OpenGraph, Twitter Cards, standard meta tags — these classify the page type and summarize its content. In the raw HTML. No browser.

### 4. RSS / Atom Feeds — Content Streams

```
GET https://techcrunch.com/feed/
→ Latest 50 articles with title, author, date, summary, category
```

News sites, blogs, podcasts — they all publish feeds. Complete content metadata without visiting a single page.

### 5. robots.txt — Site Structure Hints

Already using this. Tells us URL patterns, site sections, crawl policies.

### 6. HTTP Headers — Page Metadata

```
HEAD https://amazon.com/dp/B0EXAMPLE
→ Content-Type: text/html
→ Last-Modified: 2026-02-17
→ Content-Language: en-US
→ X-Frame-Options: SAMEORIGIN
→ Cache-Control: max-age=300
```

A HEAD request (not even GET) tells us: content type, language, freshness, caching policy. No body downloaded.

### 7. Public APIs — The Real Data

Many sites have public or semi-public APIs:

```
# Wikipedia
GET https://en.wikipedia.org/api/rest_v1/page/summary/Sony_WH-1000XM5

# GitHub
GET https://api.github.com/repos/cortex-ai/cortex

# Reddit
GET https://www.reddit.com/r/headphones.json

# Hacker News
GET https://hacker-news.firebaseio.com/v0/topstories.json

# Stack Overflow
GET https://api.stackexchange.com/2.3/questions?site=stackoverflow

# npm
GET https://registry.npmjs.org/express

# PyPI
GET https://pypi.org/pypi/requests/json

# CoinMarketCap
GET https://api.coinmarketcap.com/v1/ticker/
```

These return clean JSON with everything we need. No HTML parsing. No browser. No bot detection.

---

## The New Architecture: Layered Data Acquisition

Instead of "open browser, render page, extract data", Cortex uses a layered approach. Each layer is faster and stealthier than the next. Only fall to the next layer if the previous one didn't get enough data.

```
LAYER 0: Metadata Layer (instant, zero detection risk)
  ├── Sitemap.xml           → all URLs
  ├── robots.txt            → site structure, crawl rules
  ├── HEAD requests         → page existence, content type, language, freshness
  └── RSS/Atom feeds        → content summaries for news/blog sites

LAYER 1: Structured Data Layer (fast, near-zero detection risk)
  ├── HTTP GET raw HTML     → parse JSON-LD, microdata, meta tags, OpenGraph
  ├── NO JavaScript executed
  ├── NO images/CSS/fonts downloaded
  ├── Simple HTTP client (reqwest), NOT a browser
  ├── Looks identical to Googlebot to the server
  └── 10-50ms per page (vs 200-500ms with browser)

LAYER 2: API Layer (fast, zero detection risk)
  ├── Public APIs           → if site has an API, use it directly
  ├── GraphQL endpoints     → many SPAs expose their data API
  ├── Discover API endpoints from HTML source (fetch URLs in JS bundles)
  └── Returns clean JSON, no parsing needed

LAYER 3: Browser Layer (slow, detection risk — LAST RESORT)
  ├── Only used when Layers 0-2 don't provide enough data
  ├── Only used for ACTION EXECUTION (add to cart, fill form, login)
  ├── Only used for pages with zero structured data AND no API
  ├── Full stealth when used
  └── Maybe 5-10% of pages ever need this
```

### The Speed Difference

```
Layer 0 (sitemap + HEAD):
  50,000 URLs classified in <2 seconds
  Just HTTP HEAD requests, ~1ms each, massively parallel

Layer 1 (structured data):
  200 pages' structured data extracted in <3 seconds
  Simple HTTP GET + HTML parse, ~15ms each, massively parallel
  No browser, no JS, no images, no CSS

Layer 2 (API):
  Full product catalog from API in <1 second
  Single API call returns thousands of products

Layer 3 (browser):
  50 pages rendered in 10-25 seconds
  Full Chromium render with stealth
  Only for pages that truly need it

COMBINED: Full site map with real data in 3-5 seconds
  Of which maybe 0 seconds is browser time for most sites
```

### What Each Layer Provides to the Feature Vector

```
Layer 0 (metadata):
  [0]  page_type (from URL pattern)        ✓
  [2]  content_language (from headers)      ✓
  [3]  page_depth (from URL)               ✓
  [8]  is_https (from URL)                 ✓
  [9]  url_path_depth                      ✓
  [10] url_has_query_params                ✓
  [13] has_structured_data (HEAD sniff)     partial
  [14] meta_robots_index                    ✓ (from robots.txt)

Layer 1 (structured data — the big one):
  ALL of dims 48-63 (commerce):
    [48] price                             ✓ (from JSON-LD offers.price)
    [49] original_price                    ✓ (from JSON-LD offers)
    [50] discount_percentage               ✓ (calculated)
    [51] availability                      ✓ (from JSON-LD offers.availability)
    [52] rating                            ✓ (from JSON-LD aggregateRating)
    [53] review_count                      ✓ (from JSON-LD aggregateRating.reviewCount)
    [55] shipping_free                     ✓ (from JSON-LD offers.shippingDetails)
    [58] seller_reputation                 ✓ (from JSON-LD seller)
    
  Plus:
    [0]  page_type (from @type)            ✓ (Product, Article, FAQPage, etc.)
    [5]  has_paywall (from isAccessibleForFree) ✓
    [17] text_length (from description)    partial
    [30] sentiment (from review text)      partial
    [80] tls_valid                         ✓
    [93] content_freshness (from dateModified) ✓

  Navigation (from <a> tags in raw HTML):
    All outbound links                     ✓ (no JS needed for standard links)
    Breadcrumbs                            ✓
    Pagination                             ✓

Layer 2 (API):
  Can provide COMPLETE product catalogs:
    Every product, every price, every rating, every variant
    Plus: inventory levels, shipping options, review text
    More data than even a browser render provides

Layer 3 (browser — only when needed):
  [24] form_field_count                    ✓ (needs rendered DOM)
  [96-111] action features                 ✓ (needs interactable elements)
  Visual features that need rendering      ✓
  Dynamic content loaded by JS only        ✓
```

**Layer 1 alone fills 30-40 of the 128 feature dimensions with REAL data (not estimates). For e-commerce pages, it fills nearly all the dimensions that matter for agent decision-making (price, rating, availability).**

---

## What This Eliminates

| v0.1.0 Limitation | Still a problem? | Why not? |
|-------------------|-----------------|----------|
| Bot detection (10 sites) | **No** | Layers 0-1 use simple HTTP. No browser fingerprint to detect. Looks like Googlebot. |
| SPA empty shells (6 sites) | **No** | JSON-LD is in the raw HTML before JS runs. Or we hit the API directly (Layer 2). |
| HTTP/2 errors (4 sites) | **No** | reqwest handles HTTP/1.1 and H2 correctly. No Chromium connection issues. |
| Slow mapping | **No** | HTTP GET is 10-50x faster than browser render. |
| Memory usage | **No** | No browser contexts for Layers 0-2. Just HTTP connections. |
| CAPTCHAs | **No** | No browser = no CAPTCHA trigger. Sites don't CAPTCHA Googlebot. |
| Rate limiting | **Reduced** | Still need to be polite, but HTTP GET is much lighter than full page renders. |
| Complex actions not mapped | **Partially** | Actions still need browser (Layer 3), but mapping doesn't. |
| Currency not converted | **Better** | JSON-LD includes priceCurrency field — we KNOW the currency now. |
| Interpolation inaccurate | **Mostly gone** | Layer 1 gives real data for most pages, not estimates. |
| Feature vectors heuristic | **Better** | JSON-LD provides exact typed data instead of DOM heuristics. |

**9 of 12 limitations eliminated. 2 reduced. 1 unchanged (complex actions still need a browser for execution, but not for mapping).**

---

## The New Mapping Flow

```python
async def map_site(domain: str) -> SiteMap:
    
    # ── LAYER 0: Metadata (instant) ─────────────────
    robots = await http_get(f"https://{domain}/robots.txt")
    rules = parse_robots(robots)
    
    sitemap_urls = rules.sitemaps or [f"https://{domain}/sitemap.xml"]
    urls = []
    for sitemap_url in sitemap_urls:
        xml = await http_get(sitemap_url)
        urls.extend(parse_sitemap(xml))
    
    # If no sitemap, discover URLs from homepage HTML
    if not urls:
        html = await http_get(f"https://{domain}/")
        urls = extract_links(html)
        # Crawl one level deep for more URLs
        for link_batch in chunk(urls, 50):
            pages = await http_get_many(link_batch)
            for page in pages:
                urls.extend(extract_links(page))
        urls = deduplicate(urls)
    
    # Classify all URLs by pattern
    classified = classify_urls(urls, domain)
    # → [(url, PageType, confidence), ...]
    
    # HEAD requests for metadata
    heads = await http_head_many(urls)  # parallel, ~1ms each
    # → content-type, language, status, last-modified per URL
    
    # ── LAYER 1: Structured Data (fast) ─────────────
    # Sample pages for structured data extraction
    sample_urls = select_samples(classified, max_samples=500)
    
    # Simple HTTP GET — NOT a browser render
    pages_html = await http_get_many(sample_urls)  # parallel, ~15ms each
    
    structured_data = []
    for url, html in zip(sample_urls, pages_html):
        sd = extract_structured_data(html)
        # sd contains: JSON-LD objects, microdata, OpenGraph, meta tags
        # Also: all <a href> links (for edges), <form> elements (for action hints)
        structured_data.append((url, sd))
    
    # ── LAYER 2: API Discovery (if applicable) ──────
    # Check for known APIs
    api_data = await try_public_api(domain)
    # Returns product catalog, article list, etc. if API exists
    
    # ── BUILD THE MAP ────────────────────────────────
    builder = SiteMapBuilder(domain)
    
    for url, page_type, confidence in classified:
        # Start with URL-based classification
        features = base_features_from_url(url, page_type)
        
        # Enrich with HEAD response metadata
        if url in heads:
            features = enrich_with_headers(features, heads[url])
        
        # Enrich with structured data (this is the gold)
        if url in structured_data:
            features = enrich_with_structured_data(features, structured_data[url])
            # This fills: price, rating, availability, description, category,
            # brand, currency, images, review count, date, author...
            # With REAL data, not estimates.
        
        # Enrich with API data if available
        if api_data and url in api_data:
            features = enrich_with_api(features, api_data[url])
        
        node_id = builder.add_node(url, page_type, features, confidence)
    
    # Add edges from links found in HTML
    for url, sd in structured_data:
        for link in sd.links:
            if link.target in builder.url_index:
                builder.add_edge(
                    builder.url_index[url],
                    builder.url_index[link.target],
                    link.edge_type,
                    link.weight
                )
    
    return builder.build()
    
    # TOTAL TIME: 2-4 seconds for a 50,000-page site
    # BROWSER CONTEXTS USED: 0
    # BOT DETECTION TRIGGERED: 0
    # CAPTCHAS ENCOUNTERED: 0
```

### The Browser is Now Optional

```python
# Map a site — no browser needed
site = cortex.map("amazon.com")  # Uses Layers 0-2 only. Fast. Silent. Invisible.

# Query the map — never needed a browser
products = site.filter(page_type=PRODUCT, features={48: {"lt": 300}})

# Pathfind — never needed a browser
path = site.pathfind(0, products[0].index)

# Verify a specific page live — NOW uses browser (Layer 3, optional)
live = cortex.perceive(products[0].url)

# Execute an action — NOW uses browser (Layer 3, required for actions)
cortex.act(products[0].index, opcode=ADD_TO_CART)
```

The browser exists ONLY for:
1. **Live verification** — confirming that mapped data is current
2. **Action execution** — adding to cart, filling forms, logging in
3. **Fallback rendering** — pages with zero structured data that need JS

For mapping and navigation? No browser. Zero. The map is built from the data websites already publish for search engines.

---

## Structured Data Extraction (Layer 1 Detail)

The core of this approach. How to extract structured data from raw HTML without a browser:

```rust
pub fn extract_structured_data(html: &str) -> StructuredData {
    let mut result = StructuredData::default();
    
    // 1. JSON-LD (most valuable — complete structured objects)
    // Find all <script type="application/ld+json"> blocks
    for block in find_jsonld_blocks(html) {
        match serde_json::from_str::<Value>(block) {
            Ok(value) => {
                match value["@type"].as_str() {
                    Some("Product") => result.products.push(parse_product(&value)),
                    Some("Article") | Some("NewsArticle") | Some("BlogPosting") => {
                        result.articles.push(parse_article(&value));
                    }
                    Some("BreadcrumbList") => {
                        result.breadcrumbs = parse_breadcrumbs(&value);
                    }
                    Some("WebPage") | Some("WebSite") => {
                        result.page_info = parse_webpage(&value);
                    }
                    Some("FAQPage") => result.faq = parse_faq(&value),
                    Some("Recipe") => result.recipes.push(parse_recipe(&value)),
                    Some("Event") => result.events.push(parse_event(&value)),
                    Some("Organization") => result.org = parse_org(&value),
                    Some("ItemList") => result.lists.push(parse_item_list(&value)),
                    Some("Review") => result.reviews.push(parse_review(&value)),
                    _ => result.other_jsonld.push(value),
                }
            }
            Err(_) => {} // malformed JSON-LD, skip
        }
    }
    
    // 2. OpenGraph meta tags
    result.og = parse_opengraph(html);
    // og:type, og:title, og:description, og:image, og:price:amount, etc.
    
    // 3. Standard meta tags
    result.meta = parse_meta_tags(html);
    // description, keywords, robots, author, viewport, etc.
    
    // 4. Twitter Card tags
    result.twitter = parse_twitter_cards(html);
    
    // 5. Links (for graph edges)
    result.links = extract_all_links(html);
    // Every <a href="..."> tag, resolved to absolute URLs
    
    // 6. Microdata (schema.org via HTML attributes)
    result.microdata = parse_microdata(html);
    // itemscope, itemtype, itemprop attributes
    
    // 7. Headings structure (for content outline)
    result.headings = extract_headings(html);
    // h1, h2, h3... gives us content structure without rendering
    
    // 8. Forms (for action hints)
    result.forms = extract_forms(html);
    // <form> elements with their fields — tells us what actions exist
    
    result
}
```

### Mapping JSON-LD @type to PageType

```rust
fn jsonld_type_to_page_type(jsonld_type: &str) -> (PageType, f32) {
    match jsonld_type {
        "Product"                            => (PageType::ProductDetail, 0.99),
        "ProductGroup"                       => (PageType::ProductDetail, 0.95),
        "Article" | "NewsArticle"            => (PageType::Article, 0.99),
        "BlogPosting"                        => (PageType::Article, 0.95),
        "FAQPage"                            => (PageType::Faq, 0.99),
        "WebPage"                            => (PageType::Unknown, 0.5), // too generic
        "SearchResultsPage"                  => (PageType::SearchResults, 0.99),
        "CollectionPage"                     => (PageType::ProductListing, 0.90),
        "CheckoutPage"                       => (PageType::Checkout, 0.99),
        "AboutPage"                          => (PageType::AboutPage, 0.99),
        "ContactPage"                        => (PageType::ContactPage, 0.99),
        "ProfilePage"                        => (PageType::Account, 0.80),
        "Recipe"                             => (PageType::Article, 0.95), // food article
        "Event"                              => (PageType::Article, 0.80),
        "QAPage"                             => (PageType::Forum, 0.90),
        "RealEstateListing"                  => (PageType::ProductDetail, 0.90),
        "JobPosting"                         => (PageType::ProductDetail, 0.85),
        "MedicalWebPage"                     => (PageType::Documentation, 0.85),
        "SoftwareApplication"                => (PageType::ProductDetail, 0.90),
        _                                    => (PageType::Unknown, 0.3),
    }
}
```

**Confidence 0.99** — the website is TELLING us what this page is. No heuristics. No guessing.

### Mapping JSON-LD to Feature Vector

```rust
fn features_from_product_jsonld(product: &JsonLdProduct) -> PartialFeatures {
    let mut f = PartialFeatures::new();
    
    // Page identity
    f.set(0, PageType::ProductDetail as f32 / 31.0);  // page_type
    f.set(1, 0.99);  // confidence (JSON-LD is authoritative)
    f.set(13, 1.0);  // has_structured_data
    
    // Commerce (the critical dimensions)
    if let Some(price) = product.offers.price {
        f.set(48, price);                               // price_raw
        f.set_flag(HAS_PRICE);
    }
    if let Some(original) = product.offers.original_price {
        f.set(49, original);                             // price_original
        if let Some(price) = product.offers.price {
            f.set(50, 1.0 - (price / original));         // discount_percentage
        }
    }
    if let Some(avail) = &product.offers.availability {
        f.set(51, match avail.as_str() {
            "https://schema.org/InStock" => 1.0,
            "https://schema.org/LimitedAvailability" => 0.5,
            "https://schema.org/OutOfStock" => 0.0,
            "https://schema.org/PreOrder" => 0.7,
            _ => 0.5,
        });
    }
    if let Some(rating) = &product.aggregate_rating {
        if let (Some(val), Some(best)) = (rating.value, rating.best) {
            f.set(52, val / best);                       // rating_normalized
        }
        if let Some(count) = rating.review_count {
            f.set(53, (count as f32).log10() / 6.0);    // review_count_log
        }
    }
    if let Some(currency) = &product.offers.price_currency {
        // We KNOW the currency now — can normalize to USD
        if let Some(price) = product.offers.price {
            f.set(48, convert_to_usd(price, currency));  // normalized price
        }
    }
    
    // Content
    if let Some(desc) = &product.description {
        f.set(17, (desc.len() as f32).log10() / 6.0);   // text_length_log
    }
    
    // Trust
    f.set(93, freshness_from_date(&product.date_modified)); // content_freshness
    
    f
}
```

---

## Revised Performance Numbers

```
THE ORIGINAL TASK: "Find cheapest Sony WH-1000XM5 across 5 retailers"

v0.1.0 (browser-based mapping):
  Map 5 sites (browser):     ~15-25s (3-5s each, limited by rendering)
  Query maps:                <1ms
  Verify live:               ~2s (1-2 pages)
  LLM calls:                 2
  TOTAL:                     17-28s
  BOT BLOCKS:                2-3 sites likely blocked
  SCORE:                     ~77/100

v0.2.0 (no-browser mapping):
  Layer 0 (sitemaps):         ~1s (5 parallel fetches)
  Layer 1 (structured data):  ~1.5s (100 HTTP GETs across 5 sites, parallel)
  Build 5 maps:               <0.5s (in-memory, no rendering)
  Query maps:                 <1ms
  Verify live (optional):     ~1s (1 page, browser)
  LLM calls:                  2
  TOTAL:                      ~3-4s
  BOT BLOCKS:                 0 (no browser used for mapping)
  SCORE:                      ~95/100
```

---

## What Still Needs a Browser (and When)

```
MAPPING:     No browser needed (Layers 0-2)
QUERYING:    No browser needed (in-memory graph operations)
PATHFINDING: No browser needed (in-memory graph operations)
PERCEIVING:  Browser needed (for live verification — but this is optional and rare)
ACTING:      Browser needed (for add-to-cart, form fill, login — this is the only REQUIRED browser use)
WATCHING:    No browser needed (periodic HTTP GET for change detection via structured data)
```

The browser becomes a **surgical tool for action execution**, not a mapping instrument. It's like the difference between using Google Maps to plan your route (no car needed) versus actually driving to the store (car needed). You only need the car for the last mile.

---

## Implementation Changes

### What Changes in the Codebase

```
NEW: runtime/src/acquisition/mod.rs             ← new module: layered data acquisition
NEW: runtime/src/acquisition/http_client.rs      ← async HTTP client (reqwest-based)
NEW: runtime/src/acquisition/structured.rs       ← JSON-LD, OpenGraph, meta tags, links, forms parser
NEW: runtime/src/acquisition/api_discovery.rs    ← detect and use public APIs
NEW: runtime/src/acquisition/head_scanner.rs     ← parallel HEAD requests for metadata
NEW: runtime/src/acquisition/feed_parser.rs      ← RSS/Atom feed parser
NEW: runtime/src/acquisition/pattern_engine.rs   ← CSS selector + regex extraction for sparse structured data
NEW: runtime/src/acquisition/css_selectors.json  ← database of CSS selectors for price, rating, availability
NEW: runtime/src/acquisition/action_discovery.rs ← HTML form + JS endpoint + platform action discovery
NEW: runtime/src/acquisition/js_analyzer.rs      ← JavaScript source analysis for API endpoints
NEW: runtime/src/acquisition/platform_actions.json ← platform-specific action templates (Shopify, WooCommerce, etc.)
NEW: runtime/src/acquisition/http_session.rs     ← HTTP session management with cookie jar
NEW: runtime/src/acquisition/auth.rs             ← HTTP authentication (password, OAuth, API key)

CHANGED: runtime/src/cartography/mapper.rs    ← use acquisition layers, browser is Layer 3 fallback
CHANGED: runtime/src/cartography/feature_encoder.rs ← accept StructuredData as input, not just DOM extraction
CHANGED: runtime/src/live/watch.rs            ← use HTTP GET for change detection, not browser render

DELETED: runtime/src/cartography/crawler.rs       ← replaced by acquisition engine
DELETED: runtime/src/cartography/sampler.rs        ← replaced by acquisition engine
DELETED: runtime/src/cartography/interpolator.rs   ← replaced by pattern engine
DELETED: runtime/src/intelligence/smart_sampler.rs ← replaced by acquisition engine

UNCHANGED: runtime/src/renderer/              ← still exists for Layer 3 and ACT
UNCHANGED: runtime/src/navigation/            ← unchanged, operates on SiteMap regardless of how it was built
UNCHANGED: runtime/src/map/                   ← unchanged, SiteMap format is the same
UNCHANGED: extractors/                        ← still exist for Layer 3 browser rendering
UNCHANGED: clients/                           ← unchanged API, map() still returns SiteMap
```

### The Python Client API Doesn't Change

```python
# This code works identically in v0.1.0 and v0.2.0
# The only difference is v0.2.0 is faster and doesn't get blocked

from cortex_client import map

site = map("amazon.com")
products = site.filter(page_type=0x04, features={48: {"lt": 300}})
path = site.pathfind(0, products[0].index)
```

The agent doesn't know or care that the map was built without a browser. The SiteMap format is identical. The protocol is identical. The improvement is entirely behind the scenes.

---

## The Final Architecture

```
                    ┌─────────────────────────────────────┐
                    │           CORTEX RUNTIME             │
                    │                                       │
  Agent ──socket──→ │  ┌─────────────────────────────────┐ │
                    │  │     ACQUISITION ENGINE           │ │
                    │  │                                   │ │
                    │  │  Layer 0: Metadata               │ │
                    │  │    sitemap.xml, robots.txt,      │ │
                    │  │    HEAD requests, RSS feeds      │ │
                    │  │            ↓                      │ │
                    │  │  Layer 1: Structured Data        │ │
                    │  │    HTTP GET → JSON-LD, OpenGraph,│ │
                    │  │    microdata, meta tags, links   │ │
                    │  │            ↓                      │ │
                    │  │  Layer 2: API Discovery          │ │
                    │  │    Public APIs, GraphQL endpoints│ │
                    │  │            ↓                      │ │
                    │  │  Layer 3: Browser (last resort)  │ │
                    │  │    Chromium render for pages     │ │
                    │  │    with no structured data       │ │
                    │  └────────────┬──────────────────────┘ │
                    │               ↓                        │
                    │  ┌─────────────────────────────────┐ │
                    │  │     MAP BUILDER                  │ │
                    │  │  Features from all layers merged │ │
                    │  │  → Binary SiteMap                │ │
                    │  └────────────┬──────────────────────┘ │
                    │               ↓                        │
                    │  ┌─────────────────────────────────┐ │
                    │  │     NAVIGATION ENGINE            │ │
                    │  │  Query, Pathfind, Similarity     │ │
                    │  │  (unchanged from v0.1.0)        │ │
                    │  └─────────────────────────────────┘ │
                    │                                       │
                    │  ┌─────────────────────────────────┐ │
                    │  │     ACTION ENGINE                │ │
                    │  │  Browser-based: ACT, sessions   │ │
                    │  │  (only part that needs Chromium) │ │
                    │  └─────────────────────────────────┘ │
                    └─────────────────────────────────────┘
```

---

> **THE NORTH STAR (FINAL)**
>
> Websites already publish machine-readable descriptions of themselves for search engines. Cortex reads those descriptions to build its maps — no browser, no bot detection, no CAPTCHAs, no fingerprinting. The browser exists only for the moment the agent needs to ACT: click a button, fill a form, complete a purchase. Everything else is data that's already sitting there, waiting to be read.
>
> We don't fight the web. We read what it's already saying.
