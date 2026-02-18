//! WATCH handler — monitor nodes for changes over time.
//!
//! When watching nodes that have JSON-LD data, prefers HTTP GET + structured
//! data comparison instead of browser rendering. Only falls back to browser
//! for nodes without structured data.

use crate::acquisition::http_client::HttpClient;
use crate::acquisition::structured;
use crate::cartography::feature_encoder;
use crate::live::refresh;
use crate::map::types::{SiteMap, FEATURE_DIM};
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
pub fn select_watch_nodes(map: &SiteMap, request: &WatchRequest) -> Vec<u32> {
    let refresh_req = refresh::RefreshRequest {
        nodes: request.nodes.clone(),
        cluster: request.cluster,
        stale_threshold: None,
    };
    refresh::select_nodes_to_refresh(map, &refresh_req)
}

/// Fetch updated features for a node via HTTP (no browser).
///
/// Prefers HTTP GET + structured data for nodes that had JSON-LD data.
/// Returns updated feature vector, or None if HTTP fetch fails.
pub async fn fetch_node_features_http(
    url: &str,
    client: &HttpClient,
) -> Option<[f32; FEATURE_DIM]> {
    let resp = client.get(url, 10000).await.ok()?;
    if resp.status != 200 {
        return None;
    }

    let sd = structured::extract_structured_data(&resp.body, &resp.final_url);

    // Only use HTTP path if we got meaningful structured data
    if sd.has_jsonld || sd.has_opengraph {
        let head = crate::acquisition::http_client::HeadResponse {
            url: resp.url,
            status: resp.status,
            content_type: resp
                .headers
                .iter()
                .find(|(k, _)| k == "content-type")
                .map(|(_, v)| v.clone()),
            content_language: resp
                .headers
                .iter()
                .find(|(k, _)| k == "content-language")
                .map(|(_, v)| v.clone()),
            last_modified: resp
                .headers
                .iter()
                .find(|(k, _)| k == "last-modified")
                .map(|(_, v)| v.clone()),
            cache_control: resp
                .headers
                .iter()
                .find(|(k, _)| k == "cache-control")
                .map(|(_, v)| v.clone()),
        };

        Some(feature_encoder::encode_features_from_structured_data(
            &sd, url, &head,
        ))
    } else {
        None // No structured data — caller should fall back to browser
    }
}
