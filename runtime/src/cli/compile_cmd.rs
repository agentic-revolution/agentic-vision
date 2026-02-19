//! CLI handler for `cortex compile <domain>`.

use crate::cli::output::{self, Styled};
use crate::compiler::{codegen, schema};
use crate::intelligence::cache::MapCache;
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::time::Instant;

/// Run the compile command.
pub async fn run(domain: &str, _all: bool, output_dir: Option<&str>) -> Result<()> {
    let s = Styled::new();
    let start = Instant::now();

    // Load the cached map
    let mut cache = MapCache::default_cache()?;
    let site_map = match cache.load_map(domain)? {
        Some(map) => map,
        None => bail!("no cached map for '{domain}'. Run `cortex map {domain}` first."),
    };

    // Infer schema
    let compiled = schema::infer_schema(&site_map, domain);

    // Determine output directory
    let out_dir = if let Some(dir) = output_dir {
        PathBuf::from(dir)
    } else {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".cortex").join("compiled").join(domain)
    };

    // Generate all files
    let _files = codegen::generate_all(&compiled, &out_dir)?;
    let elapsed = start.elapsed();

    if !output::is_quiet() {
        // Collect which targets were generated
        let mut targets: Vec<&str> = Vec::new();
        if out_dir.join("models.py").exists() || compiled.stats.total_models > 0 {
            targets.push("python");
        }
        if out_dir.join("client").exists() || out_dir.join("index.ts").exists() {
            targets.push("typescript");
        }
        if out_dir.join("openapi.yaml").exists() || out_dir.join("openapi.json").exists() {
            targets.push("openapi");
        }
        if targets.is_empty() {
            targets.extend_from_slice(&["python", "typescript", "openapi"]);
        }
        let targets_str = targets
            .iter()
            .map(|t| s.cyan(t))
            .collect::<Vec<_>>()
            .join(" \u{00b7} ");
        let time_str = if elapsed.as_millis() < 1000 {
            format!("{}ms", elapsed.as_millis())
        } else {
            format!("{:.1}s", elapsed.as_secs_f64())
        };
        eprintln!(
            "  {} Generated {} in {}",
            s.ok_sym(),
            targets_str,
            s.yellow(&time_str),
        );
    }

    if output::is_json() {
        output::print_json(&serde_json::json!({
            "domain": domain,
            "models": compiled.stats.total_models,
            "fields": compiled.stats.total_fields,
            "actions": compiled.actions.len(),
            "relationships": compiled.relationships.len(),
            "output_dir": out_dir.display().to_string(),
            "duration_ms": elapsed.as_millis() as u64,
        }));
    }

    Ok(())
}
