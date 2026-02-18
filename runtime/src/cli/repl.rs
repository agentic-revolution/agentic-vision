// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

//! Interactive REPL for Cortex — Claude Code-style slash command interface.
//!
//! Launch with `cortex` (no subcommand) to enter the interactive mode.
//! Type `/help` for available commands, Tab for completion.

use crate::cli::doctor::cortex_home;
use crate::cli::output::Styled;
use crate::cli::repl_commands;
use crate::cli::repl_complete;
use anyhow::Result;
use rustyline::config::CompletionType;
use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};

/// History file location.
fn history_path() -> std::path::PathBuf {
    cortex_home().join("repl_history")
}

/// Print the welcome banner with runtime status summary.
async fn print_banner() {
    let s = Styled::new();

    eprintln!();
    eprintln!(
        "  {} {} {}",
        s.green("\u{25c9}"),
        s.bold(&format!("Cortex v{}", env!("CARGO_PKG_VERSION"))),
        s.dim("— Web Cartographer for AI Agents")
    );

    // Quick status check
    let daemon_status = check_daemon_status().await;
    let cache_count = count_cached_maps();

    eprintln!("    {} | Cached maps: {}", daemon_status, cache_count,);

    eprintln!();
    eprintln!(
        "    Press {} to browse commands, {} to complete, {} to quit.",
        s.cyan("/"),
        s.dim("Tab"),
        s.dim("/exit")
    );
    eprintln!();
}

/// Check if daemon is running and return a status string.
async fn check_daemon_status() -> String {
    let s = Styled::new();
    let socket_path = "/tmp/cortex.sock";

    if !std::path::Path::new(socket_path).exists() {
        return format!("Daemon: {}", s.yellow("not running"));
    }

    // Try to connect
    match tokio::net::UnixStream::connect(socket_path).await {
        Ok(_) => {
            // Read PID
            let pid = std::fs::read_to_string(cortex_home().join("cortex.pid"))
                .ok()
                .and_then(|p| p.trim().parse::<i32>().ok());

            match pid {
                Some(p) => format!("Daemon: {} (pid {})", s.green("running"), p),
                None => format!("Daemon: {}", s.green("running")),
            }
        }
        Err(_) => format!("Daemon: {}", s.yellow("socket exists but unresponsive")),
    }
}

/// Count cached .ctx files.
fn count_cached_maps() -> usize {
    let maps_dir = cortex_home().join("maps");
    std::fs::read_dir(&maps_dir)
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "ctx"))
                .count()
        })
        .unwrap_or(0)
}

/// Run the interactive REPL.
pub async fn run() -> Result<()> {
    // Print welcome banner
    print_banner().await;

    // Configure rustyline with List completion (shows all matches like Bash)
    let config = Config::builder()
        .history_ignore_space(true)
        .auto_add_history(true)
        .completion_type(CompletionType::List)
        .completion_prompt_limit(20)
        .build();

    let helper = repl_complete::CortexHelper::new();
    let mut rl: Editor<repl_complete::CortexHelper, rustyline::history::DefaultHistory> =
        Editor::with_config(config)?;
    rl.set_helper(Some(helper));

    // Bind custom keys for smart Tab completion and command picker
    repl_complete::bind_keys(&mut rl);

    // Load history
    let hist_path = history_path();
    if hist_path.exists() {
        let _ = rl.load_history(&hist_path);
    }

    // Session state
    let mut state = repl_commands::ReplState::new();

    // Main REPL loop
    let prompt = format!(
        " {} ",
        if Styled::new().ok_sym() == "OK" {
            "cortex>"
        } else {
            "\x1b[36mcortex>\x1b[0m"
        }
    );

    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Dispatch command
                match repl_commands::execute(line, &mut state).await {
                    Ok(true) => {
                        // /exit was called
                        let s = Styled::new();
                        eprintln!("  {} Goodbye!", s.dim("\u{2728}"));
                        break;
                    }
                    Ok(false) => {
                        // Continue REPL
                    }
                    Err(e) => {
                        let s = Styled::new();
                        eprintln!("  {} {e:#}", s.fail_sym());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C — don't exit, just show hint
                let s = Styled::new();
                eprintln!("  {} Type {} to quit.", s.dim("(Ctrl+C)"), s.bold("/exit"));
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D — exit
                let s = Styled::new();
                eprintln!("  {} Goodbye!", s.dim("\u{2728}"));
                break;
            }
            Err(err) => {
                eprintln!("  Error: {err}");
                break;
            }
        }
    }

    // Save history
    let _ = std::fs::create_dir_all(hist_path.parent().unwrap_or(std::path::Path::new(".")));
    let _ = rl.save_history(&hist_path);

    Ok(())
}
