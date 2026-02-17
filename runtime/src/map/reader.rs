//! Query and read operations on a SiteMap.

use crate::map::types::*;

impl SiteMap {
    /// Filter nodes by criteria.
    pub fn filter(&self, query: &NodeQuery) -> Vec<NodeMatch> {
        let mut results = Vec::new();

        for (i, node) in self.nodes.iter().enumerate() {
            // Filter by page type
            if let Some(ref types) = query.page_types {
                if !types.contains(&node.page_type) {
                    continue;
                }
            }

            // Filter by feature ranges
            let features = &self.features[i];
            let mut skip = false;
            for range in &query.feature_ranges {
                if range.dimension >= FEATURE_DIM {
                    continue;
                }
                let val = features[range.dimension];
                if let Some(min) = range.min {
                    if val < min {
                        skip = true;
                        break;
                    }
                }
                if let Some(max) = range.max {
                    if val > max {
                        skip = true;
                        break;
                    }
                }
            }
            if skip {
                continue;
            }

            // Filter by required flags
            if let Some(ref req) = query.require_flags {
                if node.flags.0 & req.0 != req.0 {
                    continue;
                }
            }

            // Filter by excluded flags
            if let Some(ref exc) = query.exclude_flags {
                if node.flags.0 & exc.0 != 0 {
                    continue;
                }
            }

            // Collect key features for the result
            let mut key_features = Vec::new();
            for range in &query.feature_ranges {
                if range.dimension < FEATURE_DIM {
                    key_features.push((range.dimension, features[range.dimension]));
                }
            }

            results.push(NodeMatch {
                index: i as u32,
                url: self.urls[i].clone(),
                page_type: node.page_type,
                confidence: node.confidence as f32 / 255.0,
                features: key_features,
                similarity: None,
            });
        }

        // Sort
        if let Some(sort_dim) = query.sort_by_feature {
            if sort_dim < FEATURE_DIM {
                results.sort_by(|a, b| {
                    let va = self.features[a.index as usize][sort_dim];
                    let vb = self.features[b.index as usize][sort_dim];
                    if query.sort_ascending {
                        va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
                    }
                });
            }
        }

        // Limit
        if query.limit > 0 && results.len() > query.limit {
            results.truncate(query.limit);
        }

        results
    }

    /// Find k nearest nodes by cosine similarity to target vector.
    pub fn nearest(&self, target: &[f32; FEATURE_DIM], k: usize) -> Vec<NodeMatch> {
        let target_norm: f32 = target.iter().map(|f| f * f).sum::<f32>().sqrt();
        if target_norm == 0.0 {
            return Vec::new();
        }

        let mut scored: Vec<(u32, f32)> = self
            .features
            .iter()
            .enumerate()
            .map(|(i, feat)| {
                let node_norm = self.nodes[i].feature_norm;
                if node_norm == 0.0 {
                    return (i as u32, -1.0);
                }
                let dot: f32 = feat.iter().zip(target.iter()).map(|(a, b)| a * b).sum();
                let similarity = dot / (node_norm * target_norm);
                (i as u32, similarity)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(k)
            .map(|(idx, sim)| NodeMatch {
                index: idx,
                url: self.urls[idx as usize].clone(),
                page_type: self.nodes[idx as usize].page_type,
                confidence: self.nodes[idx as usize].confidence as f32 / 255.0,
                features: Vec::new(),
                similarity: Some(sim),
            })
            .collect()
    }

    /// Get all edges from a node using the CSR index.
    pub fn edges_from(&self, node: u32) -> &[EdgeRecord] {
        let n = node as usize;
        if n >= self.nodes.len() {
            return &[];
        }
        let start = self.edge_index[n] as usize;
        let end = self.edge_index[n + 1] as usize;
        &self.edges[start..end]
    }

    /// Get the URL for a node.
    pub fn node_url(&self, node: u32) -> &str {
        &self.urls[node as usize]
    }

    /// Get the feature vector for a node.
    pub fn node_features(&self, node: u32) -> &[f32; FEATURE_DIM] {
        &self.features[node as usize]
    }

    /// Get actions available on a node using the CSR index.
    pub fn actions_for(&self, node: u32) -> &[ActionRecord] {
        let n = node as usize;
        if n >= self.nodes.len() {
            return &[];
        }
        let start = self.action_index[n] as usize;
        let end = self.action_index[n + 1] as usize;
        &self.actions[start..end]
    }

    /// Update a node with fresh data.
    pub fn update_node(
        &mut self,
        index: u32,
        record: NodeRecord,
        features: [f32; FEATURE_DIM],
    ) {
        let idx = index as usize;
        if idx < self.nodes.len() {
            self.nodes[idx] = record;
            self.features[idx] = features;
        }
    }

    /// Find shortest path between two nodes using Dijkstra's algorithm.
    pub fn shortest_path(
        &self,
        from: u32,
        to: u32,
        constraints: &PathConstraints,
    ) -> Option<Path> {
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;

        let n = self.nodes.len();
        if from as usize >= n || to as usize >= n {
            return None;
        }

        let mut dist = vec![f32::INFINITY; n];
        let mut prev = vec![u32::MAX; n];
        dist[from as usize] = 0.0;

        // Min-heap: (cost, node)
        let mut heap = BinaryHeap::new();
        heap.push(Reverse((OrderedF32(0.0), from)));

        while let Some(Reverse((OrderedF32(cost), node))) = heap.pop() {
            if node == to {
                break;
            }
            if cost > dist[node as usize] {
                continue;
            }

            for edge in self.edges_from(node) {
                let target = edge.target_node;
                if target as usize >= n {
                    continue;
                }

                // Apply constraints
                if constraints.avoid_auth && edge.flags.requires_auth() {
                    continue;
                }
                if constraints.avoid_state_changes && edge.flags.changes_state() {
                    continue;
                }

                let edge_cost = match constraints.minimize {
                    PathMinimize::Hops => 1.0,
                    PathMinimize::Weight => edge.weight as f32,
                    PathMinimize::StateChanges => {
                        if edge.flags.changes_state() {
                            10.0
                        } else {
                            1.0
                        }
                    }
                };

                let new_cost = cost + edge_cost;
                if new_cost < dist[target as usize] {
                    dist[target as usize] = new_cost;
                    prev[target as usize] = node;
                    heap.push(Reverse((OrderedF32(new_cost), target)));
                }
            }
        }

        if dist[to as usize].is_infinite() {
            return None;
        }

        // Reconstruct path
        let mut path_nodes = Vec::new();
        let mut current = to;
        while current != from {
            path_nodes.push(current);
            current = prev[current as usize];
            if current == u32::MAX {
                return None;
            }
        }
        path_nodes.push(from);
        path_nodes.reverse();

        Some(Path {
            hops: (path_nodes.len() - 1) as u32,
            total_weight: dist[to as usize],
            nodes: path_nodes,
            required_actions: Vec::new(),
        })
    }
}

/// Wrapper for f32 to implement Ord for use in BinaryHeap.
#[derive(Clone, Copy, PartialEq)]
struct OrderedF32(f32);

impl Eq for OrderedF32 {}

impl PartialOrd for OrderedF32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}
