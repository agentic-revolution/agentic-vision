Read docs/planning/05-edge-cases.md completely. This is a comprehensive audit of every edge case, UI polish item, and hardening task that must be completed before Cortex v0.1.0 is published. Work through it section by section:

1. CLI Polish (Sections 1.1-1.8): Refactor every CLI command output to match the exact formatting shown — branded headers, colored symbols (✓ ✗ ⚠ ○), progress bars for long operations, specific fix instructions on every error. Add --json, --quiet, --verbose, --no-color flags to every command. Add the `cortex restart` command. Make `cortex map` auto-install Chromium and auto-start the process on first run so users never have to run install/start separately. Fix `cortex doctor` to perform all 14 checks listed. Every error message must answer: what happened, why, how to fix it.

2. Client Library Polish (Section 2): Add meaningful __repr__ to every Python dataclass. Normalize domain input in the map() function (strip protocol, trailing slash, extract domain from full URL). Add async variants (amap, aperceive). Validate goal vector dimensions in nearest(). Make every error include a .code attribute. Add docstrings with examples to every public method.

3. Mapping Edge Cases (Section 3): Handle SPAs (detect and extract client-side routes), cookie consent auto-dismiss, lazy-loaded images (scroll + data-src), shadow DOM traversal, sitemap stream parsing for huge sitemaps, redirect loop detection, 404 node handling, rate limit backoff on 429 responses, partial map saving on ctrl+C interrupt, cached map reuse with --fresh override. Ship a known-platforms.json for Shopify/WordPress/Wix URL patterns.

4. Protocol Hardening (Section 4): Add 30s inactivity timeout per connection. Handle malformed JSON gracefully (return error, keep connection). Deduplicate concurrent MAP requests for same domain with a per-domain lock. Handle client disconnect by cancelling in-progress work. Enforce request ID uniqueness.

5. Feature Vector Edge Cases (Section 5): Implement the missing data strategy — 0.0 for absent features, use NodeFlags to distinguish "zero value" from "unknown". Handle non-USD currency (store raw, don't convert). Handle price ranges (store low end). Map text ratings to numeric. Zero vector for pages with no text content.

6. Installation Polish (Section 6): macOS Gatekeeper detection and xattr fix instruction in doctor. Linux shared library detection with per-distro install commands. Docker root detection with --no-sandbox auto-flag. Alpine musl detection and warning. First-run auto-setup in any command that needs the runtime. Add `cortex update` command with version check and rollback support. Add `cortex cache clear` command.

7. Security Hardening (Section 7): HTML-encode all ACT values before injection. Add CORTEX_VAULT_KEY_FILE support. Add audit log rotation (max 100MB). Add rate limiting on client connections (max 10 req/s). Re-perceive before ACT to detect stale pages.

8. Performance (Section 8): Add LRU eviction for in-memory map cache. Add browser context health check (kill idle >5min). Add max context age (30min). RwLock on maps for concurrent read/write safety. Add memory leak test.

9. Polish Checklist (Section 10): Implement every unchecked item — tab completion scripts, man pages, consistent exit codes, map file integrity checksums, compressed map storage, fuzz tests for protocol and sitemap parsers.

10. Documentation: Create docs/LIMITATIONS.md with the 8 honest limitations from Section 11. Update README with any new commands or flags. Update quickstart to reflect the auto-install first-run experience.

Do not ask for approval. Fix each issue directly in the codebase. Run tests after each section to ensure nothing breaks. Commit after completing each section with message format: `polish: <section description>`. If a fix requires changing a public API, update both the runtime and all client libraries to match. Begin now.
