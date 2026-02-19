//! CLI handler for `cortex wql "<query>"`.

use crate::cli::output::{self, Styled};
use crate::intelligence::cache::MapCache;
use crate::wql::{executor, parser, planner};
use anyhow::Result;
use std::time::Instant;

/// Run a WQL query.
pub async fn run(query_str: &str) -> Result<()> {
    let s = Styled::new();
    let start = Instant::now();

    // Parse the WQL query
    let query = parser::parse(query_str)?;

    // Create the execution plan
    let plan = planner::plan(&query, None)?;

    // Load all cached maps
    let mut cache = MapCache::default_cache()?;
    let maps = cache.load_all_maps()?;

    // Execute
    let rows = executor::execute(&plan, &maps)?;
    let elapsed = start.elapsed();

    if output::is_json() {
        output::print_json(&serde_json::json!({
            "query": query_str,
            "results": rows.len(),
            "rows": rows,
            "duration_us": elapsed.as_micros() as u64,
        }));
    } else if rows.is_empty() {
        eprintln!("  No results.");
    } else {
        // Gather all field names
        let mut all_fields: Vec<String> = Vec::new();
        for row in &rows {
            for key in row.fields.keys() {
                if !all_fields.contains(key) {
                    all_fields.push(key.clone());
                }
            }
        }
        all_fields.sort();

        // Compute column widths for clean alignment
        let mut widths: Vec<usize> = all_fields.iter().map(|f| f.len()).collect();
        for row in &rows {
            for (i, field) in all_fields.iter().enumerate() {
                let val_len = row
                    .fields
                    .get(field)
                    .map(|v| v.to_string().len())
                    .unwrap_or(1);
                if val_len > widths[i] {
                    widths[i] = val_len;
                }
            }
        }

        // Print clean rows — no header, no separators
        for row in &rows {
            let mut parts: Vec<String> = Vec::new();
            for (i, field) in all_fields.iter().enumerate() {
                let val = row
                    .fields
                    .get(field)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string());
                // Right-align numeric fields (prices), left-align text
                let is_numeric = val.starts_with('$')
                    || val.starts_with('-')
                    || val.chars().next().is_some_and(|c| c.is_ascii_digit());
                if is_numeric {
                    parts.push(s.green(&format!("{:>width$}", val, width = widths[i])));
                } else {
                    parts.push(format!("{:<width$}", val, width = widths[i]));
                }
            }
            eprintln!("  {}", parts.join("   "));
        }

        // Compact footer: "5 rows · <1 µs"
        let time_str = if elapsed.as_micros() < 1 {
            "<1 \u{00b5}s".to_string()
        } else if elapsed.as_micros() < 1000 {
            format!("{} \u{00b5}s", elapsed.as_micros())
        } else if elapsed.as_millis() < 1000 {
            format!("{}ms", elapsed.as_millis())
        } else {
            format!("{:.1}s", elapsed.as_secs_f64())
        };
        eprintln!(
            "  {} rows \u{00b7} {}",
            s.blue(&format!("{}", rows.len())),
            s.yellow(&time_str),
        );
    }

    Ok(())
}
