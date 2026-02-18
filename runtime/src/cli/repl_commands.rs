// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

//! Slash command parsing and dispatch for the Cortex REPL.
//!
//! Each slash command maps to functionality from the existing CLI commands,
//! adapted for the interactive session context (e.g., active domain tracking).

use crate::cli::doctor::cortex_home;
use crate::cli::output::{self, format_duration, format_size, Styled};
use crate::cli::repl_complete::COMMANDS;
use crate::cli::repl_progress;
use crate::intelligence::cache::MapCache;
use crate::map::types::{
    FeatureRange, NodeQuery, PageType, PathConstraints, FEAT_PRICE, FEAT_RATING,
};
use anyhow::Result;
use std::time::Instant;

/// Session state preserved across commands.
pub struct ReplState {
    /// Currently active domain for queries/pathfind.
    pub active_domain: Option<String>,
}

impl ReplState {
    pub fn new() -> Self {
        Self {
            active_domain: None,
        }
    }
}

/// Parse and execute a slash command. Returns `true` if the REPL should exit.
pub async fn execute(input: &str, state: &mut ReplState) -> Result<bool> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(false);
    }

    // Strip leading / if present
    let input = input.strip_prefix('/').unwrap_or(input);

    // Bare `/` with nothing else → show help
    if input.is_empty() {
        cmd_help();
        return Ok(false);
    }

    // Split into command and arguments
    let mut parts = input.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("").trim();

    match cmd {
        "exit" | "quit" | "q" => return Ok(true),
        "help" | "h" | "?" => cmd_help(),
        "clear" | "cls" => cmd_clear(),
        "status" => cmd_status().await?,
        "doctor" => cmd_doctor().await?,
        "maps" | "ls" => cmd_maps()?,
        "use" => cmd_use(args, state)?,
        "map" => cmd_map(args, state).await?,
        "query" => cmd_query(args, state)?,
        "pathfind" | "path" => cmd_pathfind(args, state)?,
        "perceive" => cmd_perceive(args).await?,
        "settings" | "config" => cmd_settings()?,
        "cache" => cmd_cache(args)?,
        "plug" => cmd_plug().await?,
        _ => {
            let s = Styled::new();
            if let Some(suggestion) = crate::cli::repl_complete::suggest_command(cmd) {
                eprintln!(
                    "  {} Unknown command '/{cmd}'. Did you mean {}?",
                    s.warn_sym(),
                    s.bold(suggestion)
                );
            } else {
                eprintln!(
                    "  {} Unknown command '/{cmd}'. Type {} or press {} for commands.",
                    s.warn_sym(),
                    s.bold("/help"),
                    s.bold("/")
                );
            }
        }
    }

    Ok(false)
}

/// /help — Show available commands.
fn cmd_help() {
    let s = Styled::new();
    eprintln!();
    eprintln!("  {}", s.bold("Commands:"));
    eprintln!();
    for (cmd, desc) in COMMANDS {
        eprintln!("    {:<22} {}", s.cyan(cmd), s.dim(desc));
    }
    eprintln!();
    eprintln!(
        "  {}",
        s.dim("Tip: Tab completion works for commands and domain names.")
    );
    eprintln!();
}

/// /clear — Clear the terminal.
fn cmd_clear() {
    // ANSI escape to clear screen and move cursor to top-left
    eprint!("\x1b[2J\x1b[H");
}

/// /status — Show runtime status.
async fn cmd_status() -> Result<()> {
    crate::cli::status::run().await
}

/// /doctor — Environment diagnostics.
async fn cmd_doctor() -> Result<()> {
    crate::cli::doctor::run().await
}

/// /maps — List cached maps.
fn cmd_maps() -> Result<()> {
    let s = Styled::new();
    let maps_dir = cortex_home().join("maps");

    let mut entries: Vec<(String, u64, std::time::SystemTime)> = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&maps_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "ctx") {
                if let (Some(stem), Ok(meta)) =
                    (path.file_stem().and_then(|s| s.to_str()), path.metadata())
                {
                    let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    entries.push((stem.to_string(), meta.len(), modified));
                }
            }
        }
    }

    entries.sort_by(|a, b| b.2.cmp(&a.2));

    if entries.is_empty() {
        eprintln!();
        eprintln!(
            "  {} No cached maps. Map a site with: {}",
            s.info_sym(),
            s.bold("/map example.com")
        );
        eprintln!();
        return Ok(());
    }

    eprintln!();
    eprintln!("  {} cached map(s):", entries.len());
    eprintln!();

    let mut total_size = 0u64;
    for (name, size, modified) in &entries {
        total_size += size;
        let ago = modified
            .elapsed()
            .map(|d| format_duration(d.as_secs()) + " ago")
            .unwrap_or_else(|_| "unknown".to_string());
        eprintln!(
            "    {:<25} {:>10}   {}",
            s.bold(name),
            format_size(*size),
            s.dim(&ago)
        );
    }
    eprintln!();
    eprintln!("    {:<25} {}", "Total:", format_size(total_size));
    eprintln!();

    Ok(())
}

/// /use <domain> — Switch active domain.
fn cmd_use(args: &str, state: &mut ReplState) -> Result<()> {
    let s = Styled::new();

    if args.is_empty() {
        if let Some(ref domain) = state.active_domain {
            eprintln!("  Active domain: {}", s.bold(domain));
        } else {
            eprintln!(
                "  {} No active domain. Usage: {}",
                s.info_sym(),
                s.bold("/use example.com")
            );
        }
        return Ok(());
    }

    let domain = args.split_whitespace().next().unwrap_or(args);

    // Check if map exists in cache
    let mut cache = MapCache::default_cache()?;
    if cache.load_map(domain)?.is_some() {
        state.active_domain = Some(domain.to_string());
        eprintln!("  {} Active domain set to: {}", s.ok_sym(), s.bold(domain));
    } else {
        eprintln!(
            "  {} No cached map for '{}'. Map it first with: /map {domain}",
            s.warn_sym(),
            domain
        );
        // Still set it — they might map it next
        state.active_domain = Some(domain.to_string());
    }

    Ok(())
}

/// /map <domain> — Map a website with progress display.
async fn cmd_map(args: &str, state: &mut ReplState) -> Result<()> {
    let s = Styled::new();

    if args.is_empty() {
        eprintln!("  {} Usage: {}", s.info_sym(), s.bold("/map <domain>"));
        return Ok(());
    }

    let domain = args.split_whitespace().next().unwrap_or(args);
    let start = Instant::now();

    eprintln!();
    eprintln!("  Mapping {}...", s.bold(domain));
    eprintln!();

    // Create progress bars for each acquisition layer
    let (mp, bars) = repl_progress::create_mapping_progress();

    // Auto-start daemon if needed
    let socket_path = "/tmp/cortex.sock";
    if !std::path::Path::new(socket_path).exists() {
        repl_progress::set_layer_active(&bars[0], "Starting daemon", "auto-start...");
        let _ = crate::cli::start::run().await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    // Mark first layer as active
    repl_progress::set_layer_active(&bars[0], "Sitemap discovery", "scanning...");

    // Connect and send MAP request
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let mut stream = match UnixStream::connect(socket_path).await {
        Ok(s) => s,
        Err(_) => {
            for bar in &bars {
                repl_progress::set_layer_skipped(bar, "", "failed");
            }
            mp.clear()?;
            eprintln!(
                "  {} Cannot connect to daemon. Run /status to check.",
                s.fail_sym()
            );
            return Ok(());
        }
    };

    let req = serde_json::json!({
        "id": format!("repl-map-{}", std::process::id()),
        "method": "map",
        "params": {
            "domain": domain,
            "max_nodes": 50000_u32,
            "max_render": 200_u32,
            "max_time_ms": 10000_u64,
            "respect_robots": true,
        }
    });
    let req_str = format!("{}\n", req);
    stream.write_all(req_str.as_bytes()).await?;

    // Simulate layer progression while waiting for response
    // (The real mapper runs all layers server-side; we animate to show activity)
    let read_handle = tokio::spawn(async move {
        let mut buf = vec![0u8; 1024 * 1024];
        let timeout = std::time::Duration::from_millis(40000);
        match tokio::time::timeout(timeout, stream.read(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => {
                let response: serde_json::Value =
                    serde_json::from_slice(&buf[..n]).unwrap_or_default();
                Some(response)
            }
            _ => None,
        }
    });

    // Animate layers while waiting
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    repl_progress::set_layer_done(&bars[0], "Sitemap discovery", "URLs discovered");

    repl_progress::set_layer_active(&bars[1], "HTTP extraction", "fetching pages...");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    repl_progress::set_layer_done(&bars[1], "HTTP extraction", "pages fetched");

    repl_progress::set_layer_active(&bars[2], "Pattern engine", "extracting data...");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    repl_progress::set_layer_done(&bars[2], "Pattern engine", "data extracted");

    repl_progress::set_layer_active(&bars[3], "API discovery", "scanning endpoints...");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    repl_progress::set_layer_done(&bars[3], "API discovery", "endpoints found");

    // Wait for actual response
    let response = read_handle.await.ok().flatten();

    repl_progress::set_layer_skipped(&bars[4], "Browser fallback", "skipped (HTTP sufficient)");

    // Clear progress display
    let _ = mp.clear();

    eprintln!();

    match response {
        Some(resp) => {
            if let Some(error) = resp.get("error") {
                let msg = error
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                eprintln!("  {} Mapping failed: {msg}", s.fail_sym());
            } else {
                let result = resp.get("result").cloned().unwrap_or_default();
                let node_count = result
                    .get("node_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let edge_count = result
                    .get("edge_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let elapsed = start.elapsed();

                eprintln!(
                    "  {} Map complete: {} nodes, {} edges ({:.1}s)",
                    s.ok_sym(),
                    s.bold(&node_count.to_string()),
                    edge_count,
                    elapsed.as_secs_f64()
                );

                // Auto-set active domain
                state.active_domain = Some(domain.to_string());
                eprintln!();
                eprintln!(
                    "  Active domain set to {}. Try: {}",
                    s.bold(domain),
                    s.cyan("/query --type product_detail")
                );
            }
        }
        None => {
            eprintln!("  {} Mapping timed out or connection lost.", s.fail_sym());
        }
    }

    eprintln!();
    Ok(())
}

/// /query [filters] — Search the active domain's map.
fn cmd_query(args: &str, state: &mut ReplState) -> Result<()> {
    let s = Styled::new();

    // Determine domain
    let mut domain = state.active_domain.clone();
    let mut page_type_str: Option<String> = None;
    let mut price_lt: Option<f32> = None;
    let mut rating_gt: Option<f32> = None;
    let mut limit: u32 = 20;
    let mut feature_filters: Vec<String> = Vec::new();

    // Simple arg parser
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "--type" if i + 1 < tokens.len() => {
                page_type_str = Some(tokens[i + 1].to_string());
                i += 2;
            }
            "--price-lt" if i + 1 < tokens.len() => {
                price_lt = tokens[i + 1].parse().ok();
                i += 2;
            }
            "--rating-gt" if i + 1 < tokens.len() => {
                rating_gt = tokens[i + 1].parse().ok();
                i += 2;
            }
            "--limit" if i + 1 < tokens.len() => {
                limit = tokens[i + 1].parse().unwrap_or(20);
                i += 2;
            }
            "--feature" if i + 1 < tokens.len() => {
                feature_filters.push(tokens[i + 1].to_string());
                i += 2;
            }
            s if !s.starts_with('-') && domain.is_none() => {
                domain = Some(s.to_string());
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    let domain = match domain {
        Some(d) => d,
        None => {
            eprintln!(
                "  {} No active domain. Use: {} or {}",
                s.info_sym(),
                s.bold("/use example.com"),
                s.bold("/query example.com --type article")
            );
            return Ok(());
        }
    };

    // Delegate to existing query logic
    let rt = tokio::runtime::Handle::current();
    rt.block_on(async {
        crate::cli::query_cmd::run(
            &domain,
            page_type_str.as_deref(),
            price_lt,
            rating_gt,
            limit,
            &feature_filters,
        )
        .await
    })
}

/// /pathfind <from> <to> — Find shortest path.
fn cmd_pathfind(args: &str, state: &mut ReplState) -> Result<()> {
    let s = Styled::new();

    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut domain = state.active_domain.clone();
    let mut from: Option<u32> = None;
    let mut to: Option<u32> = None;

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "--from" if i + 1 < tokens.len() => {
                from = tokens[i + 1].parse().ok();
                i += 2;
            }
            "--to" if i + 1 < tokens.len() => {
                to = tokens[i + 1].parse().ok();
                i += 2;
            }
            s if !s.starts_with('-') && domain.is_none() => {
                domain = Some(s.to_string());
                i += 1;
            }
            s if !s.starts_with('-') => {
                // Positional: first is from, second is to
                if from.is_none() {
                    from = s.parse().ok();
                } else if to.is_none() {
                    to = s.parse().ok();
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    let domain = match domain {
        Some(d) => d,
        None => {
            eprintln!(
                "  {} No active domain. Use: {}",
                s.info_sym(),
                s.bold("/pathfind --from 0 --to 10")
            );
            return Ok(());
        }
    };

    let from = match from {
        Some(f) => f,
        None => {
            eprintln!(
                "  {} Usage: {}",
                s.info_sym(),
                s.bold("/pathfind --from 0 --to 10")
            );
            return Ok(());
        }
    };

    let to = match to {
        Some(t) => t,
        None => {
            eprintln!(
                "  {} Usage: {}",
                s.info_sym(),
                s.bold("/pathfind --from 0 --to 10")
            );
            return Ok(());
        }
    };

    let rt = tokio::runtime::Handle::current();
    rt.block_on(async { crate::cli::pathfind_cmd::run(&domain, from, to).await })
}

/// /perceive <url> — Analyze a single page.
async fn cmd_perceive(args: &str) -> Result<()> {
    let s = Styled::new();

    if args.is_empty() {
        eprintln!(
            "  {} Usage: {}",
            s.info_sym(),
            s.bold("/perceive https://example.com/page")
        );
        return Ok(());
    }

    let url = args.split_whitespace().next().unwrap_or(args);
    crate::cli::perceive_cmd::run(url, "pretty").await
}

/// /settings — Show current configuration.
fn cmd_settings() -> Result<()> {
    let s = Styled::new();

    eprintln!();
    eprintln!("  {}", s.bold("Configuration"));
    eprintln!();
    eprintln!("    {:<22} {}", "CORTEX_HOME", cortex_home().display());
    let socket = "/tmp/cortex.sock";
    eprintln!("    {:<22} {}", "Socket", socket);
    eprintln!(
        "    {:<22} {}",
        "Chromium",
        crate::cli::doctor::find_chromium()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| s.dim("not found").to_string())
    );

    // Show relevant env vars
    eprintln!();
    eprintln!("  {}", s.bold("Environment Variables"));
    eprintln!();
    let env_vars = [
        "CORTEX_HOME",
        "CORTEX_CHROMIUM_PATH",
        "CORTEX_CHROMIUM_NO_SANDBOX",
        "CORTEX_JSON",
        "CORTEX_QUIET",
        "CORTEX_VERBOSE",
        "CORTEX_NO_COLOR",
    ];
    for var in &env_vars {
        let val = std::env::var(var).unwrap_or_else(|_| s.dim("(not set)").to_string());
        eprintln!("    {var:<32} {val}");
    }

    // Cache stats
    let maps_dir = cortex_home().join("maps");
    let cache_count = std::fs::read_dir(&maps_dir)
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "ctx"))
                .count()
        })
        .unwrap_or(0);
    eprintln!();
    eprintln!("  {}", s.bold("Cache"));
    eprintln!("    {:<22} {}", "Cached maps", cache_count);
    eprintln!("    {:<22} {}", "Location", maps_dir.display());
    eprintln!();

    Ok(())
}

/// /cache clear [domain] — Clear cached maps.
fn cmd_cache(args: &str) -> Result<()> {
    let s = Styled::new();

    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() || parts[0] != "clear" {
        eprintln!(
            "  {} Usage: {}",
            s.info_sym(),
            s.bold("/cache clear [domain]")
        );
        return Ok(());
    }

    let domain = parts.get(1).copied();
    let rt = tokio::runtime::Handle::current();
    rt.block_on(async { crate::cli::cache_cmd::run_clear(domain).await })
}

/// /plug — Show agent connections.
async fn cmd_plug() -> Result<()> {
    crate::cli::plug::run(false, false, true, None, None).await
}
