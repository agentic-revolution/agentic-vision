//! Pathfinding engine for navigating between nodes in a SiteMap.

use crate::map::types::{Path, PathConstraints, SiteMap};

/// Find the shortest path between two nodes.
///
/// Uses Dijkstra's algorithm on the SiteMap's CSR edge structure.
/// Respects path constraints (avoid auth, avoid state changes).
/// Weight mode controls what is minimized (hops, weight, state changes).
pub fn find_path(
    map: &SiteMap,
    from: u32,
    to: u32,
    constraints: &PathConstraints,
) -> Option<Path> {
    map.shortest_path(from, to, constraints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::builder::SiteMapBuilder;
    use crate::map::types::*;

    fn build_path_map() -> SiteMap {
        let mut builder = SiteMapBuilder::new("test.com");
        let feats = [0.0f32; FEATURE_DIM];

        // Create a small graph:
        // 0 -> 1 -> 2 -> 3
        // 0 -> 4 -> 3 (shortcut with higher weight)
        builder.add_node("https://test.com/", PageType::Home, feats, 255);
        builder.add_node("https://test.com/a", PageType::Article, feats, 200);
        builder.add_node("https://test.com/b", PageType::Article, feats, 200);
        builder.add_node("https://test.com/c", PageType::ProductDetail, feats, 200);
        builder.add_node("https://test.com/d", PageType::ProductDetail, feats, 200);

        // Path: 0->1->2->3 (3 hops, weight 3)
        builder.add_edge(0, 1, EdgeType::Navigation, 1, EdgeFlags::default());
        builder.add_edge(1, 2, EdgeType::Navigation, 1, EdgeFlags::default());
        builder.add_edge(2, 3, EdgeType::Navigation, 1, EdgeFlags::default());

        // Shortcut: 0->4->3 (2 hops, weight 10)
        builder.add_edge(0, 4, EdgeType::Navigation, 5, EdgeFlags::default());
        builder.add_edge(4, 3, EdgeType::Navigation, 5, EdgeFlags::default());

        builder.build()
    }

    #[test]
    fn test_find_path_by_hops() {
        let map = build_path_map();
        let constraints = PathConstraints::default(); // minimize hops

        let path = find_path(&map, 0, 3, &constraints);
        assert!(path.is_some());
        let path = path.unwrap();
        // Both paths are 2 or 3 hops; with hops minimization, shorter is preferred
        assert!(path.hops <= 3);
        assert_eq!(*path.nodes.first().unwrap(), 0);
        assert_eq!(*path.nodes.last().unwrap(), 3);
    }

    #[test]
    fn test_find_path_by_weight() {
        let map = build_path_map();
        let constraints = PathConstraints {
            minimize: PathMinimize::Weight,
            ..Default::default()
        };

        let path = find_path(&map, 0, 3, &constraints);
        assert!(path.is_some());
        let path = path.unwrap();
        // Path via 1->2 has lower total weight (3) than via 4 (10)
        assert_eq!(path.total_weight, 3.0);
        assert_eq!(path.nodes, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_find_path_no_path() {
        let map = build_path_map();
        let constraints = PathConstraints::default();

        // Node 3 has no outgoing edges to node 0
        let path = find_path(&map, 3, 0, &constraints);
        assert!(path.is_none());
    }

    #[test]
    fn test_find_path_with_auth_constraint() {
        let mut builder = SiteMapBuilder::new("test.com");
        let feats = [0.0f32; FEATURE_DIM];

        builder.add_node("https://test.com/", PageType::Home, feats, 255);
        builder.add_node("https://test.com/login", PageType::Login, feats, 200);
        builder.add_node("https://test.com/account", PageType::Account, feats, 200);
        builder.add_node("https://test.com/public", PageType::Article, feats, 200);

        // Direct path through auth
        builder.add_edge(0, 1, EdgeType::Navigation, 1, EdgeFlags::default());
        builder.add_edge(
            1,
            2,
            EdgeType::Navigation,
            1,
            EdgeFlags(EdgeFlags::REQUIRES_AUTH),
        );

        // Public path
        builder.add_edge(0, 3, EdgeType::Navigation, 1, EdgeFlags::default());

        let map = builder.build();

        // With avoid_auth, path to account should fail (no alternative)
        let constraints = PathConstraints {
            avoid_auth: true,
            ..Default::default()
        };
        let path = find_path(&map, 0, 2, &constraints);
        assert!(path.is_none());
    }
}
