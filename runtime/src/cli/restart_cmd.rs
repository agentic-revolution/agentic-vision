//! `cortex restart` — stop and start the daemon in one step.

use crate::cli::output;
use anyhow::Result;

/// Restart the Cortex daemon by stopping and starting.
pub async fn run() -> Result<()> {
    if !output::is_quiet() {
        eprintln!("  Restarting Cortex...");
    }

    // Stop first (ignoring errors if not running)
    crate::cli::stop::run().await.ok();

    // Brief pause to let the socket clean up
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Start
    crate::cli::start::run().await
}
