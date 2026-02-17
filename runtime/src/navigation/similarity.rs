//! Vector similarity search over SiteMap feature vectors.

use crate::map::types::{NodeMatch, SiteMap, FEATURE_DIM};

/// Find k nearest neighbors by cosine similarity.
///
/// Uses brute-force scan with precomputed norms for efficiency.
pub fn nearest_neighbors(
    map: &SiteMap,
    target: &[f32; FEATURE_DIM],
    k: usize,
) -> Vec<NodeMatch> {
    map.nearest(target, k)
}

/// Compute cosine similarity between two feature vectors.
pub fn cosine_similarity(a: &[f32; FEATURE_DIM], b: &[f32; FEATURE_DIM]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Find nodes similar to a given node.
pub fn find_similar(map: &SiteMap, node_index: u32, k: usize) -> Vec<NodeMatch> {
    let features = map.node_features(node_index);
    let mut results = map.nearest(features, k + 1);
    // Remove the query node itself
    results.retain(|m| m.index != node_index);
    results.truncate(k);
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::builder::SiteMapBuilder;
    use crate::map::types::*;

    #[test]
    fn test_cosine_similarity() {
        let mut a = [0.0f32; FEATURE_DIM];
        let mut b = [0.0f32; FEATURE_DIM];
        a[0] = 1.0;
        a[1] = 0.0;
        b[0] = 1.0;
        b[1] = 0.0;
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        // Orthogonal vectors
        let mut c = [0.0f32; FEATURE_DIM];
        c[0] = 0.0;
        c[1] = 1.0;
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[test]
    fn test_nearest_neighbors() {
        let mut builder = SiteMapBuilder::new("test.com");

        for i in 0..10 {
            let mut feats = [0.0f32; FEATURE_DIM];
            feats[i % FEATURE_DIM] = 1.0;
            feats[0] += 0.1; // Small shared component
            builder.add_node(
                &format!("https://test.com/page-{i}"),
                PageType::Article,
                feats,
                200,
            );
        }

        let map = builder.build();

        let mut target = [0.0f32; FEATURE_DIM];
        target[3] = 1.0;
        target[0] = 0.1;

        let results = nearest_neighbors(&map, &target, 3);
        assert_eq!(results.len(), 3);
        // Node 3 should be most similar
        assert_eq!(results[0].index, 3);
    }

    #[test]
    fn test_find_similar() {
        let mut builder = SiteMapBuilder::new("test.com");

        // Create nodes with similar features
        let mut feats_a = [0.0f32; FEATURE_DIM];
        feats_a[0] = 1.0;
        feats_a[1] = 0.5;
        builder.add_node("https://test.com/a", PageType::Article, feats_a, 200);

        let mut feats_b = [0.0f32; FEATURE_DIM];
        feats_b[0] = 0.9;
        feats_b[1] = 0.6;
        builder.add_node("https://test.com/b", PageType::Article, feats_b, 200);

        let mut feats_c = [0.0f32; FEATURE_DIM];
        feats_c[5] = 1.0;
        builder.add_node("https://test.com/c", PageType::ProductDetail, feats_c, 200);

        let map = builder.build();

        let similar = find_similar(&map, 0, 2);
        assert!(!similar.is_empty());
        // Node b should be most similar to node a
        assert_eq!(similar[0].index, 1);
    }
}
