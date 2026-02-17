//! Stop the running Cortex daemon.

use crate::cli::output::{self, Styled};
use crate::cli::start::{pid_file_path, SOCKET_PATH};
use anyhow::{Context, Result};
use std::time::Duration;

/// Stop the Cortex daemon by reading PID file and sending SIGTERM.
pub async fn run() -> Result<()> {
    let s = Styled::new();
    let pid_path = pid_file_path();

    if !pid_path.exists() {
        if !output::is_quiet() {
            eprintln!("  Cortex is not running.");
        }
        // Exit 0, not error — per spec
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_path).context("failed to read PID file")?;
    let pid: i32 = pid_str.trim().parse().context("invalid PID in PID file")?;

    // Check if process is actually alive
    #[cfg(unix)]
    {
        let alive = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !alive {
            // Stale PID file — clean up
            let _ = std::fs::remove_file(&pid_path);
            let _ = std::fs::remove_file(SOCKET_PATH);
            if !output::is_quiet() {
                eprintln!("  Cleaned up stale PID file (process {pid} was not running).");
            }
            return Ok(());
        }
    }

    if !output::is_quiet() {
        eprint!("  Stopping Cortex (PID {pid})...");
    }

    // Send SIGTERM
    #[cfg(unix)]
    {
        let output = std::process::Command::new("kill")
            .arg(pid.to_string())
            .output()
            .context("failed to send SIGTERM")?;
        if !output.status.success() {
            let _ = std::fs::remove_file(&pid_path);
            if !output::is_quiet() {
                eprintln!(
                    " {}",
                    s.warn_sym()
                );
                eprintln!(
                    "  Process may have already exited. Cleaned up PID file."
                );
            }
            return Ok(());
        }
    }

    // Wait up to 5 seconds for the process to exit
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        #[cfg(unix)]
        {
            let output = std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output();
            match output {
                Ok(o) if !o.status.success() => {
                    // Process has exited
                    let _ = std::fs::remove_file(&pid_path);
                    let _ = std::fs::remove_file(SOCKET_PATH);
                    if !output::is_quiet() {
                        eprintln!(" {}", s.ok_sym());
                        eprintln!("  Cortex stopped.");
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    // Timed out
    let _ = std::fs::remove_file(&pid_path);
    if !output::is_quiet() {
        eprintln!(" {}", s.warn_sym());
        eprintln!("  Cortex may still be running. PID file removed.");
        eprintln!("  If the problem persists, try: kill -9 {pid}");
    }
    Ok(())
}
