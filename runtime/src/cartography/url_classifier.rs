//! Classify URLs by pattern into PageType.

use crate::map::types::PageType;

/// Classify a URL into a page type and confidence score.
pub fn classify_url(url: &str, _domain: &str) -> (PageType, f32) {
    let path = extract_path(url).to_lowercase();

    // Root/home page
    if path == "/" || path.is_empty() {
        return (PageType::Home, 0.9);
    }

    // Product patterns
    if path.contains("/dp/")
        || path.contains("/product/")
        || path.contains("/item/")
        || path.contains("/p/")
        || path.contains("/products/")
        || path.contains("/pd/")
    {
        return (PageType::ProductDetail, 0.8);
    }

    // Search
    if path.contains("/search") || path.contains("/s?") || path.starts_with("/s/") {
        return (PageType::SearchResults, 0.8);
    }

    // Category / product listing
    if path.contains("/category/")
        || path.contains("/c/")
        || path.contains("/collections/")
        || path.contains("/shop/")
    {
        return (PageType::ProductListing, 0.7);
    }

    // Cart
    if path.contains("/cart") || path.contains("/basket") || path.contains("/bag") {
        return (PageType::Cart, 0.9);
    }

    // Checkout
    if path.contains("/checkout") {
        return (PageType::Checkout, 0.9);
    }

    // Login / Auth
    if path.contains("/login")
        || path.contains("/signin")
        || path.contains("/sign-in")
        || path.contains("/auth")
    {
        return (PageType::Login, 0.85);
    }

    // Account
    if path.contains("/account") || path.contains("/profile") || path.contains("/settings") {
        return (PageType::Account, 0.7);
    }

    // Article / Blog
    if path.contains("/blog/")
        || path.contains("/post/")
        || path.contains("/article/")
        || path.contains("/news/")
        || path.contains("/stories/")
    {
        return (PageType::Article, 0.75);
    }

    // Documentation
    if path.contains("/docs/")
        || path.contains("/documentation/")
        || path.contains("/wiki/")
        || path.contains("/guide/")
    {
        return (PageType::Documentation, 0.7);
    }

    // About
    if path.contains("/about") {
        return (PageType::AboutPage, 0.85);
    }

    // Contact
    if path.contains("/contact") {
        return (PageType::ContactPage, 0.85);
    }

    // FAQ
    if path.contains("/faq") || path.contains("/help") {
        return (PageType::Faq, 0.8);
    }

    // Pricing
    if path.contains("/pricing") || path.contains("/plans") {
        return (PageType::PricingPage, 0.85);
    }

    // Legal
    if path.contains("/privacy")
        || path.contains("/terms")
        || path.contains("/tos")
        || path.contains("/legal")
    {
        return (PageType::Legal, 0.8);
    }

    // Download
    if path.contains("/download") {
        return (PageType::DownloadPage, 0.8);
    }
    if path.ends_with(".pdf") || path.ends_with(".zip") || path.ends_with(".tar.gz") {
        return (PageType::DownloadPage, 0.9);
    }

    // Media
    if path.ends_with(".jpg")
        || path.ends_with(".png")
        || path.ends_with(".gif")
        || path.ends_with(".mp4")
    {
        return (PageType::MediaPage, 0.9);
    }

    // Forum
    if path.contains("/forum") || path.contains("/discuss") || path.contains("/community") {
        return (PageType::Forum, 0.7);
    }

    // Sitemap page
    if path.contains("/sitemap") {
        return (PageType::SitemapPage, 0.8);
    }

    // Archive / listing (blog-like)
    if path.contains("/archive") || path.contains("/tags/") || path.contains("/categories/") {
        return (PageType::ProductListing, 0.5);
    }

    // Default
    (PageType::Unknown, 0.3)
}

fn extract_path(url: &str) -> &str {
    // Simple path extraction without parsing the full URL
    if let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    {
        if let Some(slash_pos) = rest.find('/') {
            return &rest[slash_pos..];
        }
        return "/";
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_urls() {
        assert_eq!(
            classify_url("https://amazon.com/", "amazon.com").0,
            PageType::Home
        );
        assert_eq!(
            classify_url("https://amazon.com/dp/B0EXAMPLE", "amazon.com").0,
            PageType::ProductDetail
        );
        assert_eq!(
            classify_url("https://example.com/blog/my-post", "example.com").0,
            PageType::Article
        );
        assert_eq!(
            classify_url("https://example.com/about", "example.com").0,
            PageType::AboutPage
        );
        assert_eq!(
            classify_url("https://example.com/contact", "example.com").0,
            PageType::ContactPage
        );
        assert_eq!(
            classify_url("https://shop.com/cart", "shop.com").0,
            PageType::Cart
        );
        assert_eq!(
            classify_url("https://shop.com/checkout", "shop.com").0,
            PageType::Checkout
        );
        assert_eq!(
            classify_url("https://example.com/login", "example.com").0,
            PageType::Login
        );
        assert_eq!(
            classify_url("https://example.com/unknown-page", "example.com").0,
            PageType::Unknown
        );
    }
}
