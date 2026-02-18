# Known Limitations

Cortex is production-ready for many use cases, but has limitations you should understand before deploying. We believe in documenting these honestly.

## 1. SPAs with Client-Side Routing Are Partially Supported

Cortex renders the initial page and extracts visible routes from the DOM, but deeply nested routes that require user interaction to reveal (e.g., tabs, accordions, infinite scroll) may be missed. Sites using hash-based routing (`#/page`) are detected, but complex client-side routers with dynamic imports are only partially discovered.

## 2. CAPTCHAs Are Not Solved

If a site presents a CAPTCHA during mapping, that node is marked as blocked (`NodeFlags::BLOCKED`). Cortex does not integrate CAPTCHA-solving services. The agent can detect blocked nodes via the flag and decide how to proceed.

## 3. Feature Vectors Are Heuristic

The 128-dimension encoding captures common web page properties (prices, ratings, content density, navigation structure) but cannot represent every possible page attribute. Specialized domains like medical records, legal documents, or scientific papers may benefit from custom extractors that populate domain-specific feature dimensions.

## 4. Interpolated Features Are Estimates

Unrendered pages have feature vectors estimated from similar rendered pages of the same `PageType`. These estimates are accurate for structural properties (link count, form presence) but may be inaccurate for dynamic content like prices and availability, which vary per product. Use `NodeFlags::RENDERED` vs `NodeFlags::ESTIMATED` to distinguish real from estimated features.

## 5. Mapping Speed Depends on the Site

| Scenario | Typical Time |
|----------|-------------|
| Site with sitemap.xml | 1-3 seconds |
| Site without sitemap (crawl) | 5-15 seconds |
| Very large site (>100K pages) | 30+ seconds |
| Site behind Cloudflare/bot protection | May fail |

The `--timeout` flag controls the maximum mapping budget.

## 6. Not All Actions Can Be Mapped

Complex JavaScript interactions (drag-and-drop, canvas-based UIs, custom web components, WebGL) may not be detectable by the action catalog. The catalog covers standard HTML elements: links, buttons, forms, selects, and common ARIA patterns. Custom widgets that don't use standard elements may appear as generic "click" actions or be missed entirely.

## 7. Currency Is Not Converted

Price features (dimension 48) store raw numeric values in whatever currency the page displays. Cross-site price comparison across currencies is the agent's responsibility. The feature vector does not include currency type information.

## 8. Rate Limiting Is Best-Effort

Cortex respects `robots.txt` crawl-delay directives and self-limits to 5 concurrent requests per domain. However, aggressive mapping of sensitive sites may still trigger server-side rate limits or IP bans. If a site returns 429 responses, Cortex backs off with exponential delay, but persistent rate limiting will result in a partial map.

## 9. HTTP/2 Protocol Errors on Some Sites

Some sites (notably washingtonpost.com, hotels.com, opentable.com, costco.com) fail with `ERR_HTTP2_PROTOCOL_ERROR` during both mapping and perceive operations. This appears to be an incompatibility between the headless Chrome HTTP/2 implementation and certain CDN/WAF configurations. A future version will add HTTP/1.1 fallback when HTTP/2 connections fail.

## 10. Bot Detection Blocks Headless Chrome on ~10% of Sites

Sites using advanced bot detection (Cloudflare Bot Management, Akamai Bot Manager, PerimeterX, DataDome) may block headless Chrome entirely, resulting in empty or captcha-only responses. Affected sites include major e-commerce (bestbuy.com, booking.com), social platforms (reddit.com), and travel aggregators (tripadvisor.com). Cortex includes HTTP fallback for link discovery (fetching homepage HTML via standard HTTP client), but this produces lower-quality maps without rendered features. Future versions will improve stealth mode and add proxy rotation support.

## 11. SPA Sites Without Server-Side Rendering Return Minimal Data

Pure client-rendered SPAs (quora.com, stackoverflow.com, producthunt.com, doordash.com, npmjs.com) serve minimal HTML shells with no discoverable links. Both headless Chrome extraction and HTTP fallback fail to find internal navigation links. The CDP JavaScript evaluation fallback (`document.querySelectorAll('a[href]')`) helps for some SPAs but not those that render links only after authentication or complex user interaction. Future versions will add longer SPA rendering wait times and interaction-based link discovery.

## 12. Timeout Fallback Maps Have Reduced Quality

When mapping times out, Cortex builds a fallback map using HTTP-only link discovery and sitemap.xml parsing (no browser rendering). These fallback maps have interpolated or default feature vectors (not real extracted features), which reduces accuracy for feature-based queries and filtering. The `timeout_fallback` field in the MAP response indicates when this has occurred.
