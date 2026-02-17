//! Breadth-first crawler using the browser pool.

use crate::cartography::rate_limiter::RateLimiter;
use crate::extraction::loader::{ExtractionLoader, ExtractionResult};
use crate::renderer::{NavigationResult, RenderContext, Renderer};
use anyhow::{Context, Result};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;

/// A page discovered during crawling.
#[derive(Debug, Clone)]
pub struct DiscoveredPage {
    pub url: String,
    pub final_url: String,
    pub status: u16,
    pub extraction: ExtractionResult,
    pub nav_result: NavigationResult,
    pub discovered_links: Vec<String>,
}

/// Breadth-first crawler that discovers pages using a browser.
pub struct Crawler {
    renderer: Arc<dyn Renderer>,
    extractor_loader: Arc<ExtractionLoader>,
    rate_limiter: Arc<RateLimiter>,
}

impl Crawler {
    pub fn new(
        renderer: Arc<dyn Renderer>,
        extractor_loader: Arc<ExtractionLoader>,
        rate_limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            renderer,
            extractor_loader,
            rate_limiter,
        }
    }

    /// Crawl from entry URLs, discovering linked pages breadth-first.
    pub async fn crawl_and_discover(
        &self,
        entry_urls: &[String],
        max_pages: usize,
    ) -> Vec<DiscoveredPage> {
        let queue = Arc::new(Mutex::new(VecDeque::from_iter(entry_urls.iter().cloned())));
        let visited = Arc::new(Mutex::new(HashSet::new()));
        let results = Arc::new(Mutex::new(Vec::new()));

        loop {
            // Check if we've reached the limit
            {
                let r = results.lock().await;
                if r.len() >= max_pages {
                    break;
                }
            }

            // Get next URL from queue
            let url = {
                let mut q = queue.lock().await;
                q.pop_front()
            };

            let Some(url) = url else {
                break; // Queue empty
            };

            // Skip if already visited
            {
                let mut v = visited.lock().await;
                if v.contains(&url) {
                    continue;
                }
                v.insert(url.clone());
            }

            // Acquire rate limiter
            let _guard = self.rate_limiter.acquire().await;

            // Render the page
            match self.render_page(&url).await {
                Ok(page) => {
                    // Add discovered links to queue
                    {
                        let mut q = queue.lock().await;
                        let v = visited.lock().await;
                        for link in &page.discovered_links {
                            if !v.contains(link) {
                                q.push_back(link.clone());
                            }
                        }
                    }

                    results.lock().await.push(page);
                }
                Err(e) => {
                    warn!("failed to render {url}: {e}");
                }
            }
        }

        let results = results.lock().await.clone();
        results
    }

    /// Render a single page and extract everything.
    pub async fn render_page(&self, url: &str) -> Result<DiscoveredPage> {
        let mut context = self
            .renderer
            .new_context()
            .await
            .context("creating browser context")?;

        let nav_result = context
            .navigate(url, 30_000)
            .await
            .context("navigating to page")?;

        let extraction = self
            .extractor_loader
            .inject_and_run(context.as_ref())
            .await
            .context("running extractors")?;

        // Extract links from navigation results
        let discovered_links = extract_links(&extraction.navigation, url);

        let page = DiscoveredPage {
            url: url.to_string(),
            final_url: nav_result.final_url.clone(),
            status: nav_result.status,
            extraction,
            nav_result,
            discovered_links,
        };

        context.close().await.ok();

        Ok(page)
    }
}

/// Extract internal links from navigation extraction results.
fn extract_links(navigation: &serde_json::Value, source_url: &str) -> Vec<String> {
    let domain = extract_domain(source_url);

    let Some(arr) = navigation.as_array() else {
        return Vec::new();
    };

    arr.iter()
        .filter_map(|item| {
            let link_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            // Only follow internal links
            if link_type != "internal" && link_type != "pagination" {
                return None;
            }
            let url = item.get("url").and_then(|u| u.as_str())?;
            // Verify it's the same domain
            if extract_domain(url) == domain {
                Some(normalize_url(url))
            } else {
                None
            }
        })
        .collect()
}

fn extract_domain(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split('/').next().unwrap_or("").to_string()
}

fn normalize_url(url: &str) -> String {
    // Strip fragment
    url.split('#').next().unwrap_or(url).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_links() {
        let nav = serde_json::json!([
            {"type": "internal", "url": "https://example.com/page1"},
            {"type": "internal", "url": "https://example.com/page2#section"},
            {"type": "external", "url": "https://other.com/page"},
            {"type": "pagination", "url": "https://example.com/page3"}
        ]);

        let links = extract_links(&nav, "https://example.com/");
        assert_eq!(links.len(), 3);
        assert!(links.contains(&"https://example.com/page1".to_string()));
        assert!(links.contains(&"https://example.com/page2".to_string())); // fragment stripped
        assert!(links.contains(&"https://example.com/page3".to_string()));
    }
}
