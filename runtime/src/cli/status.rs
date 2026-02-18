//! Show status of the running Cortex daemon.

use crate::cli::doctor::cortex_home;
use crate::cli::output::{self, Styled};
use crate::cli::start::SOCKET_PATH;
use anyhow::{bail, Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Connect to socket and display runtime status.
pub async fn run() -> Result<()> {
    let s = Styled::new();

    let stream = match UnixStream::connect(SOCKET_PATH).await {
        Ok(s) => s,
        Err(_) => {
            if output::is_json() {
                output::print_json(&serde_json::json!({
                    "running": false,
                    "error": "not running"
                }));
                return Ok(());
            }
            eprintln!("  Cortex is not running. Start with 'cortex start'.");
            std::process::exit(1);
        }
    };

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send status request
    let req = r#"{"id":"status","method":"status","params":{}}"#;
    writer
        .write_all(format!("{req}\n").as_bytes())
        .await
        .context("failed to send status request")?;
    writer.flush().await?;

    // Read response
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .context("failed to read status response")?;

    let resp: serde_json::Value =
        serde_json::from_str(line.trim()).context("invalid status response")?;

    if output::is_json() {
        output::print_json(&resp);
        return Ok(());
    }

    if let Some(result) = resp.get("result") {
        let version = result
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let uptime = result.get("uptime_s").and_then(|v| v.as_u64()).unwrap_or(0);
        let maps = result
            .get("maps_cached")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Read PID from PID file
        let pid = std::fs::read_to_string(crate::cli::start::pid_file_path())
            .ok()
            .and_then(|s| s.trim().parse::<i32>().ok());
        let pid_str = pid.map(|p| format!("PID {p}, ")).unwrap_or_default();

        eprintln!();
        eprintln!(
            "  {} — running ({pid_str}uptime {})",
            s.bold(&format!("Cortex v{version}")),
            output::format_duration(uptime)
        );
        eprintln!();

        // Browser Pool
        if let Some(pool) = result.get("pool") {
            let active = pool.get("active").and_then(|v| v.as_u64()).unwrap_or(0);
            let max = pool.get("max").and_then(|v| v.as_u64()).unwrap_or(0);
            let mem = pool.get("memory_mb").and_then(|v| v.as_u64()).unwrap_or(0);
            output::print_section(&s, "Browser Pool");
            output::print_check(" ", "Active:", &format!("{active} / {max} contexts"));
            output::print_check(" ", "Memory:", &format!("{mem} MB"));
            eprintln!();
        }

        // Cached Maps
        output::print_section(&s, &format!("Cached Maps ({maps})"));
        let maps_dir = cortex_home().join("maps");
        if let Ok(entries) = std::fs::read_dir(&maps_dir) {
            let mut map_entries: Vec<(String, u64, std::time::SystemTime)> = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "ctx") {
                    if let (Some(stem), Ok(meta)) =
                        (path.file_stem().and_then(|s| s.to_str()), path.metadata())
                    {
                        let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        map_entries.push((stem.to_string(), meta.len(), modified));
                    }
                }
            }
            map_entries.sort_by(|a, b| b.2.cmp(&a.2)); // most recent first

            let mut total_size = 0u64;
            for (name, size, modified) in map_entries.iter().take(10) {
                total_size += size;
                let ago = modified
                    .elapsed()
                    .map(|d| output::format_duration(d.as_secs()) + " ago")
                    .unwrap_or_else(|_| "unknown".to_string());
                eprintln!(
                    "    {:<20} {:>10}   {:>14}",
                    name,
                    output::format_size(*size),
                    ago
                );
            }
            if !map_entries.is_empty() {
                eprintln!("    {:<20} {}", "Total:", output::format_size(total_size));
            } else {
                eprintln!("    (none)");
            }
        }

        eprintln!();

        // Audit log
        let audit_path = cortex_home().join("audit.jsonl");
        if audit_path.exists() {
            if let Ok(meta) = audit_path.metadata() {
                eprintln!(
                    "  Audit log: {} ({})",
                    audit_path.display(),
                    output::format_size(meta.len())
                );
            }
        }
    } else if let Some(error) = resp.get("error") {
        let code = error
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let msg = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        bail!("status error [{code}]: {msg}");
    }

    Ok(())
}
