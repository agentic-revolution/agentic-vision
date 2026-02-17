//! `cortex cache` — manage cached maps.

use crate::cli::doctor::cortex_home;
use crate::cli::output::{self, Styled};
use anyhow::Result;
use std::path::PathBuf;

/// Clear cached maps.
pub async fn run_clear(domain: Option<&str>) -> Result<()> {
    let s = Styled::new();
    let maps_dir = cortex_home().join("maps");

    match domain {
        Some(d) => {
            // Clear specific domain
            let map_path = maps_dir.join(format!("{d}.ctx"));
            if map_path.exists() {
                std::fs::remove_file(&map_path)?;
                if output::is_json() {
                    output::print_json(&serde_json::json!({
                        "cleared": d,
                    }));
                } else if !output::is_quiet() {
                    eprintln!("  {} Cleared cached map for '{d}'.", s.ok_sym());
                }
            } else if output::is_json() {
                output::print_json(&serde_json::json!({
                    "error": "not_found",
                    "message": format!("No cached map for '{d}'"),
                }));
            } else if !output::is_quiet() {
                eprintln!("  No cached map for '{d}'.");
            }
        }
        None => {
            // Clear all
            let mut count = 0;
            let mut size = 0u64;
            if let Ok(entries) = std::fs::read_dir(&maps_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "ctx") {
                        if let Ok(meta) = path.metadata() {
                            size += meta.len();
                        }
                        std::fs::remove_file(&path)?;
                        count += 1;
                    }
                }
            }

            if output::is_json() {
                output::print_json(&serde_json::json!({
                    "cleared_count": count,
                    "cleared_bytes": size,
                }));
            } else if !output::is_quiet() {
                if count > 0 {
                    eprintln!(
                        "  {} Cleared {count} cached map(s) ({}).",
                        s.ok_sym(),
                        output::format_size(size)
                    );
                } else {
                    eprintln!("  No cached maps to clear.");
                }
            }
        }
    }

    Ok(())
}
