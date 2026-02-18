//! RSS/Atom feed discovery and parsing.
//!
//! Finds RSS and Atom links in homepage HTML, tries common feed paths,
//! and parses discovered feeds to extract URLs.

use super::http_client::HttpClient;

/// An entry discovered from an RSS/Atom feed.
#[derive(Debug, Clone)]
pub struct FeedEntry {
    /// The URL of the feed entry.
    pub url: String,
    /// Title of the entry (if available).
    pub title: Option<String>,
    /// Publication date (if available).
    pub published: Option<String>,
}

/// Discover and parse RSS/Atom feeds for a domain.
///
/// 1. Finds `<link rel="alternate" type="application/rss+xml">` in HTML.
/// 2. Tries common feed paths: /feed, /rss, /atom.xml, /feed.xml, /rss.xml.
/// 3. Parses discovered feeds and returns entries.
pub async fn discover_feeds(html: &str, domain: &str, client: &HttpClient) -> Vec<FeedEntry> {
    // Extract feed URLs in a blocking task (uses scraper which is not Send)
    let html_owned = html.to_string();
    let domain_owned = domain.to_string();
    let feed_urls =
        tokio::task::spawn_blocking(move || discover_feed_urls_sync(&html_owned, &domain_owned))
            .await
            .unwrap_or_default();

    let mut entries = Vec::new();

    for feed_url in &feed_urls {
        if let Ok(resp) = client.get(feed_url, 5000).await {
            if resp.status == 200 {
                let mut parsed = parse_feed(&resp.body);
                entries.append(&mut parsed);
                if entries.len() >= 500 {
                    break;
                }
            }
        }
    }

    entries
}

/// Find feed URLs from `<link>` tags in HTML (sync, uses scraper).
fn discover_feed_urls_sync(html: &str, domain: &str) -> Vec<String> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let mut urls = Vec::new();

    // RSS feeds
    if let Ok(sel) = Selector::parse(r#"link[type="application/rss+xml"]"#) {
        for el in document.select(&sel) {
            if let Some(href) = el.value().attr("href") {
                let resolved = resolve_url(href, domain);
                if !urls.contains(&resolved) {
                    urls.push(resolved);
                }
            }
        }
    }

    // Atom feeds
    if let Ok(sel) = Selector::parse(r#"link[type="application/atom+xml"]"#) {
        for el in document.select(&sel) {
            if let Some(href) = el.value().attr("href") {
                let resolved = resolve_url(href, domain);
                if !urls.contains(&resolved) {
                    urls.push(resolved);
                }
            }
        }
    }

    // Add common paths
    let common_paths = ["/feed", "/rss", "/atom.xml", "/feed.xml", "/rss.xml"];
    for path in &common_paths {
        let url = format!("https://{domain}{path}");
        if !urls.contains(&url) {
            urls.push(url);
        }
    }

    urls
}

fn resolve_url(href: &str, domain: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("https://{domain}{href}")
    } else {
        format!("https://{domain}/{href}")
    }
}

/// Parse RSS 2.0 or Atom feed XML into entries.
fn parse_feed(xml: &str) -> Vec<FeedEntry> {
    let mut entries = Vec::new();

    // Try RSS 2.0 first
    if xml.contains("<rss") || xml.contains("<channel>") {
        entries = parse_rss(xml);
    }

    // Try Atom
    if entries.is_empty() && (xml.contains("<feed") || xml.contains("<entry>")) {
        entries = parse_atom(xml);
    }

    entries
}

fn parse_rss(xml: &str) -> Vec<FeedEntry> {
    let mut entries = Vec::new();
    let mut in_item = false;
    let mut current_url = String::new();
    let mut current_title: Option<String> = None;
    let mut current_date: Option<String> = None;
    let mut current_tag = String::new();

    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "item" {
                    in_item = true;
                    current_url.clear();
                    current_title = None;
                    current_date = None;
                }
                current_tag = name;
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                if in_item {
                    let text = e.unescape().unwrap_or_default().to_string();
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        match current_tag.as_str() {
                            "link" => current_url = trimmed,
                            "title" => current_title = Some(trimmed),
                            "pubDate" | "dc:date" => current_date = Some(trimmed),
                            _ => {}
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "item" && in_item {
                    if !current_url.is_empty() {
                        entries.push(FeedEntry {
                            url: current_url.clone(),
                            title: current_title.clone(),
                            published: current_date.clone(),
                        });
                    }
                    in_item = false;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    entries
}

fn parse_atom(xml: &str) -> Vec<FeedEntry> {
    let mut entries = Vec::new();
    let mut in_entry = false;
    let mut current_url = String::new();
    let mut current_title: Option<String> = None;
    let mut current_date: Option<String> = None;
    let mut current_tag = String::new();

    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e))
            | Ok(quick_xml::events::Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "entry" {
                    in_entry = true;
                    current_url.clear();
                    current_title = None;
                    current_date = None;
                }
                if in_entry && name == "link" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"href" {
                            current_url = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
                current_tag = name;
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                if in_entry {
                    let text = e.unescape().unwrap_or_default().to_string();
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        match current_tag.as_str() {
                            "title" => current_title = Some(trimmed),
                            "published" | "updated" => current_date = Some(trimmed),
                            _ => {}
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "entry" && in_entry {
                    if !current_url.is_empty() {
                        entries.push(FeedEntry {
                            url: current_url.clone(),
                            title: current_title.clone(),
                            published: current_date.clone(),
                        });
                    }
                    in_entry = false;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_feed_urls_sync() {
        let html = r#"
        <html><head>
        <link rel="alternate" type="application/rss+xml" href="/feed.xml" title="RSS" />
        <link rel="alternate" type="application/atom+xml" href="https://example.com/atom" />
        </head><body></body></html>
        "#;

        let urls = discover_feed_urls_sync(html, "example.com");
        assert!(urls.iter().any(|u| u.contains("feed.xml")));
        assert!(urls.iter().any(|u| u.contains("atom")));
    }

    #[test]
    fn test_parse_rss() {
        let xml = r#"<?xml version="1.0"?>
        <rss version="2.0">
        <channel>
        <title>Test</title>
        <item>
            <title>Post 1</title>
            <link>https://example.com/post-1</link>
            <pubDate>Mon, 01 Jan 2026 00:00:00 GMT</pubDate>
        </item>
        <item>
            <title>Post 2</title>
            <link>https://example.com/post-2</link>
        </item>
        </channel>
        </rss>"#;

        let entries = parse_rss(xml);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].url, "https://example.com/post-1");
        assert_eq!(entries[0].title.as_deref(), Some("Post 1"));
        assert!(entries[0].published.is_some());
    }

    #[test]
    fn test_parse_atom() {
        let xml = r#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test</title>
        <entry>
            <title>Entry 1</title>
            <link href="https://example.com/entry-1" />
            <published>2026-01-15T00:00:00Z</published>
        </entry>
        </feed>"#;

        let entries = parse_atom(xml);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com/entry-1");
        assert_eq!(entries[0].title.as_deref(), Some("Entry 1"));
    }

    #[test]
    fn test_resolve_url() {
        assert_eq!(
            resolve_url("/feed.xml", "example.com"),
            "https://example.com/feed.xml"
        );
        assert_eq!(
            resolve_url("https://example.com/rss", "example.com"),
            "https://example.com/rss"
        );
    }
}
