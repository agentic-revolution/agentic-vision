//! `cortex map <domain>` — map a website into a navigable graph.

use crate::cli::output::{self, Styled};
use crate::intelligence::cache::MapCache;
use crate::map::types::SiteMap;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::Instant;

/// Run the map command.
pub async fn run(domain: &str, max_nodes: u32, max_render: u32, timeout: u64, fresh: bool) -> Result<()> {
    let s = Styled::new();
    let start = Instant::now();

    // Check for cached map first (unless --fresh)
    if !fresh {
        let cache = MapCache::default_cache()?;
        if let Some(path) = cache.get(domain) {
            let data = std::fs::read(path)?;
            let map = SiteMap::deserialize(&data).context("failed to load cached map")?;

            if output::is_json() {
                print_map_json(&map, start.elapsed());
                return Ok(());
            }

            if !output::is_quiet() {
                let age = path
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.elapsed().ok())
                    .map(|d| output::format_duration(d.as_secs()))
                    .unwrap_or_else(|| "unknown".to_string());
                eprintln!(
                    "  Using cached map ({age} old). Use --fresh to re-map."
                );
                eprintln!();
            }

            print_map_stats(&s, &map, start.elapsed());
            return Ok(());
        }
    }

    if output::is_json() {
        output::print_json(&serde_json::json!({
            "error": "daemon_required",
            "message": "Map command requires a running Cortex daemon with browser pool",
            "hint": "Start with: cortex start"
        }));
        return Ok(());
    }

    // TODO: When browser pool is wired up, perform actual mapping here.
    if !output::is_quiet() {
        eprintln!("  Mapping {domain}...");
        eprintln!();
        eprintln!(
            "  Map command requires a running Cortex daemon with browser pool."
        );
        eprintln!("  Start the daemon with: cortex start");
        eprintln!("  Then run: cortex map {domain}");
        eprintln!();

        if output::is_verbose() {
            eprintln!("  Configuration:");
            eprintln!("    max_nodes:  {max_nodes}");
            eprintln!("    max_render: {max_render}");
            eprintln!("    timeout:    {timeout}ms");
        }
    }

    Ok(())
}

/// Print map stats in branded format.
fn print_map_stats(s: &Styled, map: &SiteMap, elapsed: std::time::Duration) {
    let rendered = map.nodes.iter().filter(|n| n.flags.is_rendered()).count();
    let estimated = map.nodes.len() - rendered;

    // Count page types
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for node in &map.nodes {
        let name = format!("{:?}", node.page_type).to_lowercase();
        *type_counts.entry(name).or_default() += 1;
    }
    let mut type_vec: Vec<(String, usize)> = type_counts.into_iter().collect();
    type_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let total_nodes = map.nodes.len();

    eprintln!("  Map complete in {:.1}s", elapsed.as_secs_f64());
    eprintln!();
    eprintln!(
        "  {}",
        s.bold(&format!(
            "{:<45}",
            map.header.domain
        ))
    );
    eprintln!(
        "  Nodes:     {} ({} rendered, {} estimated)",
        total_nodes, rendered, estimated
    );
    eprintln!("  Edges:     {}", map.edges.len());
    if !map.cluster_centroids.is_empty() {
        eprintln!("  Clusters:  {}", map.cluster_centroids.len());
    }
    eprintln!("  Actions:   {}", map.actions.len());
    eprintln!();

    if !type_vec.is_empty() {
        eprintln!("  Top page types:");
        for (name, count) in type_vec.iter().take(5) {
            let pct = if total_nodes > 0 {
                (count * 100) / total_nodes
            } else {
                0
            };
            eprintln!("    {:<20} {:>6}  ({pct}%)", name, count);
        }
    }

    eprintln!();
    eprintln!("  Query with: cortex query {} --type product_detail", map.header.domain);
}

/// Print map stats as JSON.
fn print_map_json(map: &SiteMap, elapsed: std::time::Duration) {
    let rendered = map.nodes.iter().filter(|n| n.flags.is_rendered()).count();

    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for node in &map.nodes {
        let name = format!("{:?}", node.page_type).to_lowercase();
        *type_counts.entry(name).or_default() += 1;
    }

    output::print_json(&serde_json::json!({
        "domain": map.header.domain,
        "nodes": map.nodes.len(),
        "edges": map.edges.len(),
        "rendered": rendered,
        "clusters": map.cluster_centroids.len(),
        "actions": map.actions.len(),
        "page_types": type_counts,
        "duration_ms": elapsed.as_millis(),
    }));
}
