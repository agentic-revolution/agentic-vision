//! Query engine for filtering and searching nodes in a SiteMap.

use crate::map::types::{NodeMatch, NodeQuery, SiteMap};

/// Execute a query against a SiteMap.
///
/// Supports filtering by:
/// - page_type (single or list)
/// - feature ranges (dimension -> min/max)
/// - flag requirements
/// - sorting by any feature dimension
/// - result limiting
pub fn execute(map: &SiteMap, query: &NodeQuery) -> Vec<NodeMatch> {
    map.filter(query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::builder::SiteMapBuilder;
    use crate::map::types::*;

    fn build_test_map() -> SiteMap {
        let mut builder = SiteMapBuilder::new("test.com");

        // Home page
        let mut feats = [0.0f32; FEATURE_DIM];
        feats[FEAT_PAGE_TYPE] = 1.0 / 31.0;
        feats[FEAT_PRICE] = 0.0;
        feats[FEAT_RATING] = 0.0;
        builder.add_node("https://test.com/", PageType::Home, feats, 255);

        // Products
        for i in 0..5 {
            let mut feats = [0.0f32; FEATURE_DIM];
            feats[FEAT_PAGE_TYPE] = 4.0 / 31.0;
            feats[FEAT_PRICE] = (50.0 + i as f32 * 30.0) / 1000.0;
            feats[FEAT_RATING] = (3.0 + i as f32 * 0.3) / 5.0;
            builder.add_node(
                &format!("https://test.com/p/{i}"),
                PageType::ProductDetail,
                feats,
                200,
            );
        }

        // Articles
        for i in 0..3 {
            let mut feats = [0.0f32; FEATURE_DIM];
            feats[FEAT_PAGE_TYPE] = 5.0 / 31.0;
            feats[FEAT_TEXT_DENSITY] = 0.5 + i as f32 * 0.1;
            builder.add_node(
                &format!("https://test.com/blog/{i}"),
                PageType::Article,
                feats,
                180,
            );
        }

        // Add edges
        for i in 1..9 {
            builder.add_edge(0, i, EdgeType::Navigation, 1, EdgeFlags::default());
        }

        builder.build()
    }

    #[test]
    fn test_query_by_page_type() {
        let map = build_test_map();
        let query = NodeQuery {
            page_types: Some(vec![PageType::ProductDetail]),
            limit: 100,
            ..Default::default()
        };
        let results = execute(&map, &query);
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.page_type == PageType::ProductDetail));
    }

    #[test]
    fn test_query_with_feature_range() {
        let map = build_test_map();
        let query = NodeQuery {
            page_types: Some(vec![PageType::ProductDetail]),
            feature_ranges: vec![FeatureRange {
                dimension: FEAT_PRICE,
                min: Some(0.1),
                max: None,
            }],
            limit: 100,
            ..Default::default()
        };
        let results = execute(&map, &query);
        // Only products with price >= 0.1 (i.e., >= $100 in normalized scale)
        assert!(results.len() < 5);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_query_with_sort() {
        let map = build_test_map();
        let query = NodeQuery {
            page_types: Some(vec![PageType::ProductDetail]),
            sort_by_feature: Some(FEAT_RATING),
            sort_ascending: false,
            limit: 3,
            ..Default::default()
        };
        let results = execute(&map, &query);
        assert_eq!(results.len(), 3);
        // Should be sorted by rating descending
        for i in 0..results.len() - 1 {
            let a = map.node_features(results[i].index)[FEAT_RATING];
            let b = map.node_features(results[i + 1].index)[FEAT_RATING];
            assert!(a >= b);
        }
    }

    #[test]
    fn test_query_multiple_types() {
        let map = build_test_map();
        let query = NodeQuery {
            page_types: Some(vec![PageType::Home, PageType::Article]),
            limit: 100,
            ..Default::default()
        };
        let results = execute(&map, &query);
        assert_eq!(results.len(), 4); // 1 home + 3 articles
    }
}
