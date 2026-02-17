//! Classify a page using extraction results + URL patterns.

use crate::cartography::url_classifier;
use crate::extraction::loader::ExtractionResult;
use crate::map::types::PageType;

/// Classify a page using multiple signals.
/// Returns (PageType, confidence).
pub fn classify_page(extraction: &ExtractionResult, url: &str) -> (PageType, f32) {
    // 1. Try schema.org type from metadata
    if let Some((pt, conf)) = classify_from_schema(&extraction.metadata) {
        if conf > 0.8 {
            return (pt, conf);
        }
    }

    // 2. Try URL classification
    let domain = extract_domain(url);
    let (url_type, url_conf) = url_classifier::classify_url(url, &domain);

    // 3. Try DOM heuristics from extraction results
    if let Some((dom_type, dom_conf)) = classify_from_dom(extraction) {
        // If DOM and URL agree, boost confidence
        if dom_type == url_type {
            return (dom_type, (dom_conf + url_conf) / 2.0 + 0.1);
        }
        // DOM heuristics are generally more reliable than URL alone
        if dom_conf > url_conf {
            return (dom_type, dom_conf);
        }
    }

    (url_type, url_conf)
}

/// Try to classify from schema.org / JSON-LD metadata.
fn classify_from_schema(metadata: &serde_json::Value) -> Option<(PageType, f32)> {
    // Look for @type in JSON-LD
    let type_str = metadata
        .get("jsonLd")
        .and_then(|v| v.get("@type"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            metadata
                .get("schemaOrg")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str())
        })?;

    let pt = match type_str.to_lowercase().as_str() {
        "product" => PageType::ProductDetail,
        "article" | "newsarticle" | "blogposting" => PageType::Article,
        "faqpage" => PageType::Faq,
        "aboutpage" => PageType::AboutPage,
        "contactpage" => PageType::ContactPage,
        "collectionpage" | "searchresultspage" => PageType::SearchResults,
        "itemlist" | "offerlist" => PageType::ProductListing,
        "checkoutpage" => PageType::Checkout,
        "profilepage" => PageType::Account,
        "mediagallery" | "imageobject" | "videoobject" => PageType::MediaPage,
        "discussionforumposting" => PageType::Forum,
        "review" => PageType::ReviewList,
        _ => return None,
    };

    Some((pt, 0.95))
}

/// Try to classify from DOM structure.
fn classify_from_dom(extraction: &ExtractionResult) -> Option<(PageType, f32)> {
    let structure = &extraction.structure;
    let content = &extraction.content;
    let actions = &extraction.actions;

    // Count form fields
    let form_count = structure
        .get("formCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Count price-like content
    let has_prices = content
        .as_array()
        .map(|arr| {
            arr.iter()
                .any(|c| c.get("type").and_then(|t| t.as_str()) == Some("price"))
        })
        .unwrap_or(false);

    // Count heading elements
    let heading_count = content
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("heading"))
                .count()
        })
        .unwrap_or(0);

    // Count add-to-cart style actions
    let has_cart_action = actions
        .as_array()
        .map(|arr| {
            arr.iter().any(|a| {
                let opcode = a.get("opcode").and_then(|v| v.as_u64()).unwrap_or(0);
                // 0x0200 = add_to_cart
                opcode == 0x0200
            })
        })
        .unwrap_or(false);

    // Login detection
    let has_login_form = actions
        .as_array()
        .map(|arr| {
            arr.iter().any(|a| {
                let opcode = a.get("opcode").and_then(|v| v.as_u64()).unwrap_or(0);
                // 0x0400 = login
                opcode == 0x0400
            })
        })
        .unwrap_or(false);

    // Product page: has price + add-to-cart
    if has_prices && has_cart_action {
        return Some((PageType::ProductDetail, 0.85));
    }

    // Login page: has login form
    if has_login_form && form_count > 0 {
        return Some((PageType::Login, 0.85));
    }

    // Checkout: many form fields
    if form_count >= 3 && has_prices {
        return Some((PageType::Checkout, 0.7));
    }

    // Article: many headings, long text
    let text_density = structure
        .get("textDensity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    if heading_count >= 2 && text_density > 0.3 {
        return Some((PageType::Article, 0.7));
    }

    None
}

fn extract_domain(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split('/').next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_from_url_fallback() {
        let extraction = ExtractionResult::default();
        let (pt, conf) = classify_page(&extraction, "https://shop.com/product/widget-123");
        assert_eq!(pt, PageType::ProductDetail);
        assert!(conf > 0.5);
    }

    #[test]
    fn test_classify_from_schema() {
        let metadata = serde_json::json!({
            "jsonLd": {
                "@type": "Product"
            }
        });
        let result = classify_from_schema(&metadata);
        assert!(result.is_some());
        let (pt, conf) = result.unwrap();
        assert_eq!(pt, PageType::ProductDetail);
        assert!(conf > 0.9);
    }
}
