# Known Limitations

Cortex v0.2 uses a layered HTTP acquisition architecture — most mapping is done without a browser. This eliminates many v0.1 limitations (bot detection, SPA shells, HTTP/2 browser errors) but introduces new ones.

## 1. Sites Without Structured Data or Useful HTML Patterns (~2-5% of sites)

Some sites have no JSON-LD, no OpenGraph tags, and no consistent HTML patterns that the CSS selector database can match. These sites fall through to Layer 3 (browser rendering). If the site also blocks headless Chrome, the resulting map will have low-confidence features.

## 2. Actions That Require a Browser (~10-15% of actions)

Drag-and-drop, canvas-based UIs, complex multi-step wizards, and custom web components cannot be executed via HTTP POST. These require browser-based action execution. The action catalog marks such actions with `browser_required=true`.

## 3. OAuth Authentication Requires a One-Time Browser Session

Password-based login works entirely via HTTP (form discovery + POST). However, OAuth flows (Google, GitHub, etc.) require a brief browser session for the consent screen. After the initial OAuth grant, subsequent requests use the captured session cookies without a browser.

## 4. Canvas/WebGL Applications Cannot Be Mapped via HTTP

Applications like Figma, Google Sheets, and other canvas-based tools render their content entirely in WebGL/Canvas. There is no HTML structure to extract. These applications require full browser rendering for any useful data.

## 5. CAPTCHA Solving Is Not Supported

Cortex does not integrate CAPTCHA-solving services. However, CAPTCHAs are rarely triggered in v0.2 because mapping uses standard HTTP requests (not headless Chrome), which are indistinguishable from search engine crawlers.

## 6. WebSocket-Based Real-Time Apps Are Not Fully Supported

Applications that rely on WebSocket connections for content delivery (real-time dashboards, chat applications, collaborative editors) cannot have their dynamic content captured via HTTP GET. Static content and navigation structure are still mapped correctly.

## 7. Feature Vectors Are Heuristic

The 128-dimension encoding captures common web page properties (prices, ratings, content density, navigation structure) but cannot represent every possible page attribute. Specialized domains may benefit from custom extractors that populate domain-specific feature dimensions.

## 8. Rate Limiting Is Best-Effort

Cortex respects `robots.txt` crawl-delay directives and self-limits concurrent requests per domain. However, aggressive mapping of sensitive sites may still trigger server-side rate limits. If a site returns 429 responses, Cortex backs off with exponential delay, but persistent rate limiting will result in a partial map.

## 9. Currency Is Not Converted

Price features (dimension 48) store raw numeric values in whatever currency the page displays. Cross-site price comparison across currencies is the agent's responsibility.

## 10. Platform Detection Coverage Is Incomplete

The platform action database (`platform_actions.json`) currently covers Shopify, WooCommerce, Magento, and BigCommerce. Sites on other e-commerce platforms (Squarespace Commerce, Wix Stores, custom builds) will not get platform-specific action templates, though generic form-based action discovery still works.
