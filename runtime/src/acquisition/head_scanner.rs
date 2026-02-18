//! Parallel HEAD request scanner for URL metadata.
//!
//! Quickly determines status, content-type, language, and freshness
//! for discovered URLs without downloading bodies.

use super::http_client::{HeadResponse, HttpClient};

/// Result of scanning a URL with HEAD.
#[derive(Debug, Clone)]
pub struct HeadResult {
    /// The scanned URL.
    pub url: String,
    /// HTTP status code (0 if request failed).
    pub status: u16,
    /// Content type (e.g., "text/html").
    pub content_type: Option<String>,
    /// Content language.
    pub content_language: Option<String>,
    /// Whether the page is fresh (has recent Last-Modified or no-cache).
    pub is_fresh: bool,
    /// Whether this is an HTML page.
    pub is_html: bool,
}

/// Scan URLs with parallel HEAD requests.
///
/// Returns metadata for each URL without downloading page bodies.
/// Non-HTML URLs and error responses are marked accordingly.
pub async fn scan_heads(urls: &[String], client: &HttpClient) -> Vec<HeadResult> {
    let responses = client.head_many(urls, 20).await;

    responses
        .into_iter()
        .zip(urls.iter())
        .map(|(result, url)| match result {
            Ok(resp) => head_response_to_result(resp),
            Err(_) => HeadResult {
                url: url.clone(),
                status: 0,
                content_type: None,
                content_language: None,
                is_fresh: false,
                is_html: false,
            },
        })
        .collect()
}

/// Filter URLs to only those that are HTML pages (status 200 + text/html).
pub fn filter_html_urls(results: &[HeadResult]) -> Vec<String> {
    results
        .iter()
        .filter(|r| r.status == 200 && r.is_html)
        .map(|r| r.url.clone())
        .collect()
}

fn head_response_to_result(resp: HeadResponse) -> HeadResult {
    let is_html = resp
        .content_type
        .as_deref()
        .map(|ct| ct.contains("text/html") || ct.contains("application/xhtml"))
        .unwrap_or(true); // assume HTML if no content-type

    let is_fresh = resp
        .cache_control
        .as_deref()
        .map(|cc| cc.contains("no-cache") || cc.contains("must-revalidate"))
        .unwrap_or(false)
        || resp.last_modified.is_some();

    HeadResult {
        url: resp.url,
        status: resp.status,
        content_type: resp.content_type,
        content_language: resp.content_language,
        is_fresh,
        is_html,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_response_to_result_html() {
        let resp = HeadResponse {
            url: "https://example.com/".to_string(),
            status: 200,
            content_type: Some("text/html; charset=utf-8".to_string()),
            content_language: Some("en".to_string()),
            last_modified: Some("Tue, 15 Jan 2026 12:00:00 GMT".to_string()),
            cache_control: None,
        };

        let result = head_response_to_result(resp);
        assert!(result.is_html);
        assert!(result.is_fresh);
        assert_eq!(result.status, 200);
    }

    #[test]
    fn test_head_response_to_result_non_html() {
        let resp = HeadResponse {
            url: "https://example.com/image.png".to_string(),
            status: 200,
            content_type: Some("image/png".to_string()),
            content_language: None,
            last_modified: None,
            cache_control: None,
        };

        let result = head_response_to_result(resp);
        assert!(!result.is_html);
        assert!(!result.is_fresh);
    }

    #[test]
    fn test_filter_html_urls() {
        let results = vec![
            HeadResult {
                url: "https://example.com/page".to_string(),
                status: 200,
                content_type: Some("text/html".to_string()),
                content_language: None,
                is_fresh: false,
                is_html: true,
            },
            HeadResult {
                url: "https://example.com/image.png".to_string(),
                status: 200,
                content_type: Some("image/png".to_string()),
                content_language: None,
                is_fresh: false,
                is_html: false,
            },
            HeadResult {
                url: "https://example.com/missing".to_string(),
                status: 404,
                content_type: Some("text/html".to_string()),
                content_language: None,
                is_fresh: false,
                is_html: true,
            },
        ];

        let html_urls = filter_html_urls(&results);
        assert_eq!(html_urls.len(), 1);
        assert_eq!(html_urls[0], "https://example.com/page");
    }
}
