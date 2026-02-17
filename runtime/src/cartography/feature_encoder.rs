//! Encode extraction results into 128-float feature vectors.

use crate::extraction::loader::ExtractionResult;
use crate::map::types::*;
use crate::renderer::NavigationResult;

/// Encode extraction results + navigation info into a 128-float feature vector.
pub fn encode_features(
    extraction: &ExtractionResult,
    nav_result: &NavigationResult,
    url: &str,
    page_type: PageType,
    confidence: f32,
) -> [f32; FEATURE_DIM] {
    let mut feats = [0.0f32; FEATURE_DIM];

    // ── Page Identity (0-15) ──
    feats[FEAT_PAGE_TYPE] = (page_type as u8) as f32 / 31.0;
    feats[FEAT_PAGE_TYPE_CONFIDENCE] = confidence;
    feats[FEAT_LOAD_TIME] = normalize_load_time(nav_result.load_time_ms);
    feats[FEAT_IS_HTTPS] = if url.starts_with("https://") { 1.0 } else { 0.0 };
    feats[FEAT_URL_PATH_DEPTH] = count_path_depth(url) as f32 / 10.0;
    feats[FEAT_URL_HAS_QUERY] = if url.contains('?') { 1.0 } else { 0.0 };
    feats[FEAT_URL_HAS_FRAGMENT] = if url.contains('#') { 1.0 } else { 0.0 };
    feats[FEAT_HAS_STRUCTURED_DATA] = has_structured_data(&extraction.metadata);
    feats[FEAT_META_ROBOTS_INDEX] = meta_robots_index(&extraction.metadata);
    feats[FEAT_REDIRECT_COUNT] = nav_result.redirect_chain.len() as f32 / 5.0;

    // ── Content Metrics (16-47) ──
    encode_content_features(&extraction.content, &extraction.structure, &mut feats);

    // ── Commerce Features (48-63) ──
    encode_commerce_features(&extraction.content, &extraction.metadata, &mut feats);

    // ── Navigation Features (64-79) ──
    encode_navigation_features(&extraction.navigation, &extraction.structure, &mut feats);

    // ── Trust & Safety (80-95) ──
    feats[FEAT_TLS_VALID] = if url.starts_with("https://") { 1.0 } else { 0.0 };
    feats[FEAT_CONTENT_FRESHNESS] = 1.0; // Just mapped, so fresh

    // ── Action Features (96-111) ──
    encode_action_features(&extraction.actions, &mut feats);

    // Session dimensions (112-127) default to 0.0 at mapping time

    feats
}

fn normalize_load_time(ms: u64) -> f32 {
    // Normalize: 0ms=1.0 (best), 10000ms=0.0 (worst)
    1.0 - (ms as f32 / 10_000.0).clamp(0.0, 1.0)
}

fn count_path_depth(url: &str) -> usize {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let path = rest.split('?').next().unwrap_or(rest);
    let path = path.split('#').next().unwrap_or(path);
    // Count path segments after domain
    if let Some(slash_pos) = path.find('/') {
        path[slash_pos..]
            .split('/')
            .filter(|s| !s.is_empty())
            .count()
    } else {
        0
    }
}

fn has_structured_data(metadata: &serde_json::Value) -> f32 {
    let has_jsonld = metadata.get("jsonLd").is_some_and(|v| !v.is_null());
    let has_schema = metadata.get("schemaOrg").is_some_and(|v| !v.is_null());
    let has_og = metadata.get("openGraph").is_some_and(|v| !v.is_null());
    if has_jsonld || has_schema {
        1.0
    } else if has_og {
        0.5
    } else {
        0.0
    }
}

fn meta_robots_index(metadata: &serde_json::Value) -> f32 {
    let robots = metadata
        .get("robots")
        .and_then(|v| v.as_str())
        .unwrap_or("index");
    if robots.contains("noindex") {
        0.0
    } else {
        1.0
    }
}

fn encode_content_features(
    content: &serde_json::Value,
    structure: &serde_json::Value,
    feats: &mut [f32; FEATURE_DIM],
) {
    let text_density = structure
        .get("textDensity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;
    feats[FEAT_TEXT_DENSITY] = text_density;

    if let Some(arr) = content.as_array() {
        let heading_count = arr
            .iter()
            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("heading"))
            .count();
        let paragraph_count = arr
            .iter()
            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("paragraph"))
            .count();
        let image_count = arr
            .iter()
            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("image"))
            .count();
        let table_count = arr
            .iter()
            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("table"))
            .count();
        let list_count = arr
            .iter()
            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("list"))
            .count();

        feats[FEAT_HEADING_COUNT] = (heading_count as f32 / 10.0).clamp(0.0, 1.0);
        feats[FEAT_PARAGRAPH_COUNT] = (paragraph_count as f32 / 20.0).clamp(0.0, 1.0);
        feats[FEAT_IMAGE_COUNT] = (image_count as f32 / 20.0).clamp(0.0, 1.0);
        feats[FEAT_TABLE_COUNT] = (table_count as f32 / 5.0).clamp(0.0, 1.0);
        feats[FEAT_LIST_COUNT] = (list_count as f32 / 10.0).clamp(0.0, 1.0);

        // Text length estimate
        let total_text_len: usize = arr
            .iter()
            .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
            .map(|s| s.len())
            .sum();
        feats[FEAT_TEXT_LENGTH_LOG] =
            ((total_text_len as f32 + 1.0).ln() / 12.0).clamp(0.0, 1.0);
    }

    // Form field count from structure
    let form_fields = structure
        .get("formFieldCount")
        .or_else(|| structure.get("formCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    feats[FEAT_FORM_FIELD_COUNT] = (form_fields as f32 / 20.0).clamp(0.0, 1.0);

    // Video present
    let has_video = structure
        .get("videoCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0;
    feats[FEAT_VIDEO_PRESENT] = if has_video { 1.0 } else { 0.0 };
}

fn encode_commerce_features(
    content: &serde_json::Value,
    metadata: &serde_json::Value,
    feats: &mut [f32; FEATURE_DIM],
) {
    // Try to extract price from content
    if let Some(arr) = content.as_array() {
        for item in arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("price") {
                if let Some(val) = item.get("value").and_then(|v| v.as_f64()) {
                    feats[FEAT_PRICE] = (val as f32 / 1000.0).clamp(0.0, 1.0);
                }
                if let Some(original) = item.get("original").and_then(|v| v.as_f64()) {
                    feats[FEAT_PRICE_ORIGINAL] = (original as f32 / 1000.0).clamp(0.0, 1.0);
                    if original > 0.0 {
                        if let Some(val) = item.get("value").and_then(|v| v.as_f64()) {
                            feats[FEAT_DISCOUNT_PCT] =
                                ((1.0 - val / original) as f32).clamp(0.0, 1.0);
                        }
                    }
                }
            }
            if item.get("type").and_then(|t| t.as_str()) == Some("rating") {
                if let Some(val) = item.get("value").and_then(|v| v.as_f64()) {
                    feats[FEAT_RATING] = (val as f32 / 5.0).clamp(0.0, 1.0);
                }
                if let Some(count) = item.get("reviewCount").and_then(|v| v.as_f64()) {
                    feats[FEAT_REVIEW_COUNT_LOG] =
                        ((count as f32 + 1.0).ln() / 10.0).clamp(0.0, 1.0);
                }
            }
        }
    }

    // Try schema.org metadata for availability
    if let Some(offers) = metadata
        .get("jsonLd")
        .and_then(|v| v.get("offers"))
        .or_else(|| metadata.get("schemaOrg").and_then(|v| v.get("offers")))
    {
        let avail = offers
            .get("availability")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        feats[FEAT_AVAILABILITY] = if avail.contains("InStock") {
            1.0
        } else if avail.contains("OutOfStock") {
            0.0
        } else {
            0.5
        };
    }
}

fn encode_navigation_features(
    navigation: &serde_json::Value,
    structure: &serde_json::Value,
    feats: &mut [f32; FEATURE_DIM],
) {
    if let Some(arr) = navigation.as_array() {
        let internal_count = arr
            .iter()
            .filter(|n| n.get("type").and_then(|t| t.as_str()) == Some("internal"))
            .count();
        let external_count = arr
            .iter()
            .filter(|n| n.get("type").and_then(|t| t.as_str()) == Some("external"))
            .count();
        let has_pagination = arr
            .iter()
            .any(|n| n.get("type").and_then(|t| t.as_str()) == Some("pagination"));
        let breadcrumb_count = arr
            .iter()
            .filter(|n| n.get("type").and_then(|t| t.as_str()) == Some("breadcrumb"))
            .count();

        feats[FEAT_LINK_COUNT_INTERNAL] = (internal_count as f32 / 100.0).clamp(0.0, 1.0);
        feats[FEAT_LINK_COUNT_EXTERNAL] = (external_count as f32 / 50.0).clamp(0.0, 1.0);
        feats[FEAT_OUTBOUND_LINKS] =
            ((internal_count + external_count) as f32 / 100.0).clamp(0.0, 1.0);
        feats[FEAT_PAGINATION_PRESENT] = if has_pagination { 1.0 } else { 0.0 };
        feats[FEAT_BREADCRUMB_DEPTH] = (breadcrumb_count as f32 / 5.0).clamp(0.0, 1.0);
    }

    // Search available
    let has_search = structure
        .get("hasSearch")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    feats[FEAT_SEARCH_AVAILABLE] = if has_search { 1.0 } else { 0.0 };

    // Dead end detection: no outbound links
    feats[FEAT_IS_DEAD_END] = if feats[FEAT_OUTBOUND_LINKS] < 0.01 {
        1.0
    } else {
        0.0
    };
}

fn encode_action_features(actions: &serde_json::Value, feats: &mut [f32; FEATURE_DIM]) {
    if let Some(arr) = actions.as_array() {
        let total = arr.len() as f32;
        feats[FEAT_ACTION_COUNT] = (total / 20.0).clamp(0.0, 1.0);

        if total > 0.0 {
            let safe_count = arr
                .iter()
                .filter(|a| a.get("risk").and_then(|r| r.as_u64()) == Some(0))
                .count() as f32;
            let cautious_count = arr
                .iter()
                .filter(|a| a.get("risk").and_then(|r| r.as_u64()) == Some(1))
                .count() as f32;
            let destructive_count = arr
                .iter()
                .filter(|a| a.get("risk").and_then(|r| r.as_u64()) == Some(2))
                .count() as f32;

            feats[FEAT_SAFE_ACTION_RATIO] = safe_count / total;
            feats[FEAT_CAUTIOUS_ACTION_RATIO] = cautious_count / total;
            feats[FEAT_DESTRUCTIVE_ACTION_RATIO] = destructive_count / total;
        }

        // Check for primary CTA
        let has_cta = arr.iter().any(|a| {
            let opcode = a.get("opcode").and_then(|v| v.as_u64()).unwrap_or(0);
            // Commerce or auth actions are primary CTAs
            let category = (opcode >> 8) as u8;
            category == 0x02 || category == 0x04
        });
        feats[FEAT_PRIMARY_CTA_PRESENT] = if has_cta { 1.0 } else { 0.0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_basic_features() {
        let extraction = ExtractionResult {
            content: serde_json::json!([
                {"type": "heading", "text": "Hello World"},
                {"type": "paragraph", "text": "Some content here"}
            ]),
            actions: serde_json::json!([
                {"opcode": 0x0000, "risk": 0},
                {"opcode": 0x0200, "risk": 1}
            ]),
            navigation: serde_json::json!([
                {"type": "internal", "url": "/about"},
                {"type": "external", "url": "https://other.com"}
            ]),
            structure: serde_json::json!({
                "textDensity": 0.6,
                "formCount": 0
            }),
            metadata: serde_json::json!({}),
        };

        let nav = NavigationResult {
            final_url: "https://example.com/page".to_string(),
            status: 200,
            redirect_chain: vec![],
            load_time_ms: 500,
        };

        let feats =
            encode_features(&extraction, &nav, "https://example.com/page", PageType::Article, 0.8);

        assert!(feats[FEAT_PAGE_TYPE] > 0.0);
        assert_eq!(feats[FEAT_PAGE_TYPE_CONFIDENCE], 0.8);
        assert_eq!(feats[FEAT_IS_HTTPS], 1.0);
        assert!(feats[FEAT_TEXT_DENSITY] > 0.0);
        assert!(feats[FEAT_HEADING_COUNT] > 0.0);
        assert!(feats[FEAT_ACTION_COUNT] > 0.0);
        assert!(feats[FEAT_CONTENT_FRESHNESS] == 1.0);
    }

    #[test]
    fn test_path_depth() {
        assert_eq!(count_path_depth("https://example.com/"), 0);
        assert_eq!(count_path_depth("https://example.com/a"), 1);
        assert_eq!(count_path_depth("https://example.com/a/b/c"), 3);
    }
}
