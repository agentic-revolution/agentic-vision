//! `cortex perceive <url>` — perceive a single live page.

use crate::cli::output;
use anyhow::Result;

/// Run the perceive command.
pub async fn run(url: &str, format: &str) -> Result<()> {
    if output::is_json() {
        output::print_json(&serde_json::json!({
            "error": "daemon_required",
            "message": "Perceive command requires a running Cortex daemon with browser pool",
            "hint": "Start with: cortex start",
            "url": url,
            "format": format,
        }));
        return Ok(());
    }

    if !output::is_quiet() {
        eprintln!("  Perceiving {url}...");
        eprintln!();
        eprintln!("  Perceive command requires a running Cortex daemon with browser pool.");
        eprintln!("  Start the daemon with: cortex start");
        if output::is_verbose() {
            eprintln!("  Output format: {format}");
        }
    }

    Ok(())
}
