//! REFRESH handler — re-render nodes and update the map.

use crate::map::types::SiteMap;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Result of a refresh operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResult {
    /// Number of nodes that were re-rendered.
    pub updated_count: u32,
    /// Indices of nodes whose content changed.
    pub changed_nodes: Vec<u32>,
}

/// Parameters for a refresh operation.
#[derive(Debug, Clone)]
pub struct RefreshRequest {
    /// Specific node indices to refresh.
    pub nodes: Option<Vec<u32>>,
    /// Refresh all nodes in a cluster.
    pub cluster: Option<u32>,
    /// Refresh nodes older than this many seconds.
    pub stale_threshold: Option<f64>,
}

/// Determine which nodes to refresh based on the request parameters.
pub fn select_nodes_to_refresh(
    map: &SiteMap,
    request: &RefreshRequest,
) -> Vec<u32> {
    if let Some(ref nodes) = request.nodes {
        return nodes.clone();
    }

    if let Some(cluster_id) = request.cluster {
        return map
            .cluster_assignments
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == cluster_id as u16)
            .map(|(i, _)| i as u32)
            .collect();
    }

    if let Some(threshold) = request.stale_threshold {
        return map
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                // freshness is a u8 (0-255 maps to 0.0-1.0)
                let freshness = node.freshness as f64 / 255.0;
                freshness < threshold
            })
            .map(|(i, _)| i as u32)
            .collect();
    }

    // Default: refresh all nodes
    (0..map.nodes.len() as u32).collect()
}

/// Compute a content hash for a feature vector (to detect changes).
pub fn feature_hash(features: &[f32; 128]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for &f in features.iter() {
        f.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Compare old and new features to detect changes.
pub fn detect_changes(
    old_features: &[f32; 128],
    new_features: &[f32; 128],
    threshold: f32,
) -> bool {
    old_features
        .iter()
        .zip(new_features.iter())
        .any(|(&old, &new)| (old - new).abs() > threshold)
}
