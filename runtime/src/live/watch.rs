//! WATCH handler — monitor nodes for changes over time.

use crate::live::refresh;
use crate::map::types::SiteMap;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// A change detected during watching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchDelta {
    /// The node index that changed.
    pub node: u32,
    /// Features that changed: dimension → (old_value, new_value).
    pub changed_features: Vec<(usize, f32, f32)>,
    /// When the change was detected (Unix timestamp).
    pub timestamp: f64,
}

/// Parameters for a watch operation.
#[derive(Debug, Clone)]
pub struct WatchRequest {
    /// Domain to watch.
    pub domain: String,
    /// Specific nodes to watch.
    pub nodes: Option<Vec<u32>>,
    /// Watch all nodes in a cluster.
    pub cluster: Option<u32>,
    /// Which feature dimensions to monitor.
    pub features: Option<Vec<usize>>,
    /// Polling interval in milliseconds.
    pub interval_ms: u64,
}

/// Compare feature vectors and produce a delta if changed.
pub fn compute_delta(
    node: u32,
    old_features: &[f32; 128],
    new_features: &[f32; 128],
    watch_features: Option<&[usize]>,
    threshold: f32,
) -> Option<WatchDelta> {
    let mut changed = Vec::new();

    let dimensions: Box<dyn Iterator<Item = usize>> = match watch_features {
        Some(dims) => Box::new(dims.iter().copied()),
        None => Box::new(0..128),
    };

    for dim in dimensions {
        let old = old_features[dim];
        let new = new_features[dim];
        if (old - new).abs() > threshold {
            changed.push((dim, old, new));
        }
    }

    if changed.is_empty() {
        return None;
    }

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    Some(WatchDelta {
        node,
        changed_features: changed,
        timestamp,
    })
}

/// Select which nodes to watch based on the request.
pub fn select_watch_nodes(
    map: &SiteMap,
    request: &WatchRequest,
) -> Vec<u32> {
    let refresh_req = refresh::RefreshRequest {
        nodes: request.nodes.clone(),
        cluster: request.cluster,
        stale_threshold: None,
    };
    refresh::select_nodes_to_refresh(map, &refresh_req)
}
