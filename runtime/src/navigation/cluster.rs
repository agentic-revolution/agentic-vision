//! Cluster detection and management for SiteMap nodes.

use crate::map::types::{PageType, SiteMap, FEATURE_DIM};

/// Recompute clusters for a SiteMap.
///
/// Uses k-means clustering on feature vectors.
/// k = max(3, sqrt(node_count / 10))
pub fn compute_clusters(map: &mut SiteMap) {
    let n = map.nodes.len();
    if n == 0 {
        return;
    }

    let k = if n < 30 {
        1.max(n / 3)
    } else {
        3.max(((n as f64 / 10.0).sqrt()) as usize)
    };
    let k = k.min(n);

    // Initialize centroids by evenly spacing through the data
    let mut centroids: Vec<[f32; FEATURE_DIM]> = Vec::with_capacity(k);
    for i in 0..k {
        let idx = i * n / k;
        centroids.push(map.features[idx]);
    }

    let mut assignments = vec![0u16; n];

    // Run k-means for up to 20 iterations
    for _ in 0..20 {
        let mut changed = false;

        // Assign each point to nearest centroid
        for (i, feat) in map.features.iter().enumerate() {
            let mut best_cluster = 0u16;
            let mut best_dist = f32::MAX;
            for (c, centroid) in centroids.iter().enumerate() {
                let dist: f32 = feat
                    .iter()
                    .zip(centroid.iter())
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum();
                if dist < best_dist {
                    best_dist = dist;
                    best_cluster = c as u16;
                }
            }
            if assignments[i] != best_cluster {
                assignments[i] = best_cluster;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // Recompute centroids
        let mut sums = vec![[0.0f32; FEATURE_DIM]; k];
        let mut counts = vec![0u32; k];
        for (i, feat) in map.features.iter().enumerate() {
            let c = assignments[i] as usize;
            counts[c] += 1;
            for (d, &val) in feat.iter().enumerate() {
                sums[c][d] += val;
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for (d, sum_val) in sums[c].iter().enumerate() {
                    centroids[c][d] = sum_val / counts[c] as f32;
                }
            }
        }
    }

    map.cluster_assignments = assignments;
    map.cluster_centroids = centroids;
    map.header.cluster_count = k as u16;
}

/// Get the dominant PageType for a cluster.
pub fn cluster_type(map: &SiteMap, cluster_id: u16) -> PageType {
    let mut type_counts = std::collections::HashMap::new();

    for (i, &assignment) in map.cluster_assignments.iter().enumerate() {
        if assignment == cluster_id {
            *type_counts.entry(map.nodes[i].page_type).or_insert(0u32) += 1;
        }
    }

    type_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(pt, _)| pt)
        .unwrap_or(PageType::Unknown)
}

/// Get all node indices belonging to a cluster.
pub fn cluster_members(map: &SiteMap, cluster_id: u16) -> Vec<u32> {
    map.cluster_assignments
        .iter()
        .enumerate()
        .filter(|(_, &a)| a == cluster_id)
        .map(|(i, _)| i as u32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::builder::SiteMapBuilder;
    use crate::map::types::*;

    #[test]
    fn test_compute_clusters() {
        let mut builder = SiteMapBuilder::new("test.com");

        // Group A: high dimension 0, zero on dimension 5
        for i in 0..10 {
            let mut feats = [0.0f32; FEATURE_DIM];
            feats[0] = 10.0 + (i as f32 * 0.1);
            feats[5] = 0.0;
            builder.add_node(
                &format!("https://test.com/a/{i}"),
                PageType::Article,
                feats,
                200,
            );
        }

        // Group B: high dimension 5, zero on dimension 0
        for i in 0..10 {
            let mut feats = [0.0f32; FEATURE_DIM];
            feats[0] = 0.0;
            feats[5] = 10.0 + (i as f32 * 0.1);
            builder.add_node(
                &format!("https://test.com/p/{i}"),
                PageType::ProductDetail,
                feats,
                200,
            );
        }

        let mut map = builder.build();
        compute_clusters(&mut map);

        assert!(!map.cluster_centroids.is_empty());
        assert_eq!(map.cluster_assignments.len(), 20);

        // No node from group B should share a cluster with any node from group A
        // (groups are far apart in feature space)
        let group_a_clusters: std::collections::HashSet<u16> =
            (0..10).map(|i| map.cluster_assignments[i]).collect();
        let group_b_clusters: std::collections::HashSet<u16> =
            (10..20).map(|i| map.cluster_assignments[i]).collect();

        // The two groups should have no overlapping clusters
        assert!(
            group_a_clusters.is_disjoint(&group_b_clusters),
            "Group A clusters {:?} should not overlap with Group B clusters {:?}",
            group_a_clusters,
            group_b_clusters,
        );
    }

    #[test]
    fn test_cluster_type() {
        let mut builder = SiteMapBuilder::new("test.com");
        let feats = [0.0f32; FEATURE_DIM];

        builder.add_node("https://test.com/a1", PageType::Article, feats, 200);
        builder.add_node("https://test.com/a2", PageType::Article, feats, 200);
        builder.add_node("https://test.com/p1", PageType::ProductDetail, feats, 200);

        let mut map = builder.build();
        // Force all into cluster 0
        map.cluster_assignments = vec![0, 0, 0];

        let dominant = cluster_type(&map, 0);
        assert_eq!(dominant, PageType::Article); // 2 articles vs 1 product
    }

    #[test]
    fn test_cluster_members() {
        let mut builder = SiteMapBuilder::new("test.com");
        let feats = [0.0f32; FEATURE_DIM];

        builder.add_node("https://test.com/a", PageType::Article, feats, 200);
        builder.add_node("https://test.com/b", PageType::Article, feats, 200);
        builder.add_node("https://test.com/c", PageType::Article, feats, 200);

        let mut map = builder.build();
        map.cluster_assignments = vec![0, 1, 0];

        let members = cluster_members(&map, 0);
        assert_eq!(members, vec![0, 2]);

        let members = cluster_members(&map, 1);
        assert_eq!(members, vec![1]);
    }
}
