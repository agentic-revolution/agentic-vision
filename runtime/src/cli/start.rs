//! Start the Cortex daemon process.

use crate::cli::output::{self, Styled};
use crate::server::Server;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::info;

/// Default socket path.
pub const SOCKET_PATH: &str = "/tmp/cortex.sock";

/// Get the PID file path.
pub fn pid_file_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cortex/cortex.pid")
}

/// Check if Cortex is already running. Returns the PID if so.
pub fn check_already_running() -> Option<i32> {
    let pid_path = pid_file_path();
    if !pid_path.exists() {
        return None;
    }
    let pid_str = std::fs::read_to_string(&pid_path).ok()?;
    let pid: i32 = pid_str.trim().parse().ok()?;

    // Check if the process is actually alive
    #[cfg(unix)]
    {
        let output = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();
        if matches!(output, Ok(o) if o.status.success()) {
            return Some(pid);
        }
    }

    // Stale PID file — clean up
    let _ = std::fs::remove_file(&pid_path);
    None
}

/// Start the Cortex daemon: bind socket, write PID, serve requests.
pub async fn run() -> Result<()> {
    let s = Styled::new();

    // Check if already running
    if let Some(pid) = check_already_running() {
        eprintln!(
            "  {} Cortex is already running (PID {pid}).",
            s.warn_sym()
        );
        eprintln!("  Use 'cortex restart' or 'cortex stop' first.");
        std::process::exit(1);
    }

    // Clean up stale socket file
    let socket_path = PathBuf::from(SOCKET_PATH);
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).ok();
    }

    // Ensure ~/.cortex/ exists
    let pid_path = pid_file_path();
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("cortex=info".parse().unwrap()),
        )
        .init();

    info!("starting Cortex v{}", env!("CARGO_PKG_VERSION"));

    // Write PID file
    std::fs::write(&pid_path, std::process::id().to_string())
        .context("failed to write PID file")?;

    if !output::is_quiet() {
        eprintln!(
            "  {} Cortex v{} started (PID {})",
            s.ok_sym(),
            env!("CARGO_PKG_VERSION"),
            std::process::id()
        );
        eprintln!("  Listening on {SOCKET_PATH}");
    }

    // Set up SIGTERM/SIGINT handling
    let server = Server::new(&socket_path);
    let shutdown = server.shutdown_handle();

    let shutdown_signal = shutdown.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("received shutdown signal");
        shutdown_signal.notify_one();
    });

    // Run server
    let result = server.start().await;

    // Clean up on exit
    let _ = std::fs::remove_file(&pid_path);
    let _ = std::fs::remove_file(&socket_path);

    if !output::is_quiet() {
        eprintln!("  {} Cortex stopped.", s.ok_sym());
    }

    result
}
