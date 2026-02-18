//! `cortex install` — download Chrome for Testing.

use crate::cli::doctor::{cortex_home, find_chromium};
use crate::cli::output::{self, Styled};
use anyhow::Result;

/// Download and install Chrome for Testing into ~/.cortex/chromium/.
pub async fn run_with_force(force: bool) -> Result<()> {
    let s = Styled::new();

    // Check if already installed (unless --force)
    if !force {
        if let Some(path) = find_chromium() {
            if output::is_json() {
                output::print_json(&serde_json::json!({
                    "installed": true,
                    "path": path.display().to_string(),
                    "message": "Chromium is already installed. Use --force to reinstall."
                }));
                return Ok(());
            }
            if !output::is_quiet() {
                eprintln!(
                    "  {} Chromium is already installed at {}",
                    s.ok_sym(),
                    path.display()
                );
                eprintln!("  Use --force to reinstall.");
            }
            return Ok(());
        }
    }

    // Ensure ~/.cortex/chromium/ directory exists
    let chromium_dir = cortex_home().join("chromium");
    std::fs::create_dir_all(&chromium_dir)?;

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    if output::is_json() {
        output::print_json(&serde_json::json!({
            "status": "not_implemented",
            "message": "Chromium download not yet implemented",
            "platform": format!("{os} {arch}"),
            "target_dir": chromium_dir.display().to_string(),
        }));
        return Ok(());
    }

    if !output::is_quiet() {
        eprintln!("  Installing Chromium for Cortex...");
        eprintln!();
        eprintln!("  Platform:  {os} {arch}");
        eprintln!("  Target:    {}", chromium_dir.display());
        eprintln!();
        // TODO: Implement actual download from Chrome for Testing
        eprintln!("  {} Download not yet implemented.", s.warn_sym());
        eprintln!("  Please manually install Chromium and set CORTEX_CHROMIUM_PATH.");
        eprintln!();
        eprintln!("  Options:");
        eprintln!("    1. Install Chrome: https://www.google.com/chrome/");
        eprintln!("    2. Set env: export CORTEX_CHROMIUM_PATH=/path/to/chrome");
        eprintln!("    3. Run 'cortex doctor' to verify setup.");

        // macOS Gatekeeper hint
        if os == "macos" {
            eprintln!();
            eprintln!(
                "  {} macOS users: if Gatekeeper blocks Chromium, run:",
                s.info_sym()
            );
            eprintln!("    xattr -cr ~/.cortex/chromium/");
        }
    }

    Ok(())
}

/// Legacy entry point (no force flag).
pub async fn run() -> Result<()> {
    run_with_force(false).await
}
