// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

//! `cortex plug` — auto-discover and inject into AI agents.
//!
//! Scans the machine for known AI agent configurations and injects
//! Cortex as an MCP server into each one. Supports `--list`, `--remove`,
//! `--status`, and `--agent <name>` for fine-grained control.

use anyhow::Result;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Run the plug command.
pub async fn run(
    list_only: bool,
    remove: bool,
    status_only: bool,
    agent: Option<&str>,
) -> Result<()> {
    let quiet = crate::cli::output::is_quiet();
    let json_mode = crate::cli::output::is_json();

    if !quiet && !json_mode {
        println!();
        println!("  Cortex Plug \u{2014} One command, every agent.");
        println!();
    }

    let probes = build_probes();
    let mut connected = 0u32;
    let mut needs_restart: Vec<&str> = Vec::new();
    let mut json_results: Vec<serde_json::Value> = Vec::new();

    if !quiet && !json_mode && !list_only && !status_only {
        println!("  Scanning for agents...");
        println!();
    }

    for probe in &probes {
        // Filter to specific agent if requested
        if let Some(target) = agent {
            if !probe.name.eq_ignore_ascii_case(target)
                && !probe.short_name.eq_ignore_ascii_case(target)
            {
                continue;
            }
        }

        let config_path = match probe.detect() {
            Some(p) => p,
            None => {
                if list_only || status_only {
                    if json_mode {
                        json_results.push(json!({
                            "agent": probe.name,
                            "detected": false,
                        }));
                    } else if !quiet {
                        println!("  \u{2717} {:<20} not found", probe.name);
                    }
                }
                continue;
            }
        };

        if list_only {
            if json_mode {
                json_results.push(json!({
                    "agent": probe.name,
                    "detected": true,
                    "config_path": config_path.display().to_string(),
                }));
            } else if !quiet {
                println!(
                    "  \u{2713} {:<20} found at {}",
                    probe.name,
                    config_path.display()
                );
            }
            connected += 1;
            continue;
        }

        if status_only {
            let has_cortex = check_cortex_present(&config_path);
            if json_mode {
                json_results.push(json!({
                    "agent": probe.name,
                    "detected": true,
                    "config_path": config_path.display().to_string(),
                    "cortex_connected": has_cortex,
                }));
            } else if !quiet {
                let symbol = if has_cortex { "\u{2713}" } else { "\u{25cb}" };
                let status = if has_cortex {
                    "connected"
                } else {
                    "not connected"
                };
                println!("  {} {:<20} {}", symbol, probe.name, status);
            }
            if has_cortex {
                connected += 1;
            }
            continue;
        }

        if remove {
            match remove_mcp_server(&config_path) {
                Ok(RemovalResult::Removed) => {
                    if json_mode {
                        json_results.push(json!({
                            "agent": probe.name,
                            "action": "removed",
                        }));
                    } else if !quiet {
                        println!(
                            "  \u{2713} {:<20} \u{2192} Removed from {}",
                            probe.name,
                            config_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        );
                    }
                }
                Ok(RemovalResult::NotPresent) => {
                    if !quiet && !json_mode {
                        println!("  \u{25cb} {:<20} was not connected", probe.name);
                    }
                }
                Err(e) => {
                    if !quiet && !json_mode {
                        println!("  \u{26a0} {:<20} removal failed: {}", probe.name, e);
                    }
                }
            }
            continue;
        }

        // Inject
        match inject_mcp_server(&config_path) {
            Ok(InjectionResult::Injected) => {
                connected += 1;
                if json_mode {
                    json_results.push(json!({
                        "agent": probe.name,
                        "action": "injected",
                        "config_path": config_path.display().to_string(),
                        "needs_restart": probe.needs_restart,
                    }));
                } else if !quiet {
                    println!("  \u{2713} {:<20} found", probe.name);
                    println!(
                        "    \u{2192} Injected MCP server into {}",
                        config_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    );
                    if probe.needs_restart {
                        println!("    \u{2192} Restart {} to activate", probe.name);
                        needs_restart.push(probe.name);
                    } else {
                        println!("    \u{2192} Active immediately");
                    }
                }
            }
            Ok(InjectionResult::AlreadyPresent) => {
                connected += 1;
                if json_mode {
                    json_results.push(json!({
                        "agent": probe.name,
                        "action": "already_present",
                    }));
                } else if !quiet {
                    println!("  \u{2713} {:<20} already connected", probe.name);
                }
            }
            Err(e) => {
                if json_mode {
                    json_results.push(json!({
                        "agent": probe.name,
                        "action": "error",
                        "error": e.to_string(),
                    }));
                } else if !quiet {
                    println!("  \u{26a0} {:<20} injection failed: {}", probe.name, e);
                }
            }
        }

        if !quiet && !json_mode {
            println!();
        }
    }

    // Summary
    if json_mode {
        crate::cli::output::print_json(&json!({
            "agents": json_results,
            "connected": connected,
        }));
    } else if !quiet {
        if !list_only && !status_only && !remove {
            println!("  \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");
            println!("  {} agent(s) connected. Cortex is ready.", connected);
            println!();
            if !needs_restart.is_empty() {
                println!("  Restart to activate: {}", needs_restart.join(", "));
                println!();
            }
            println!("  Test it:");
            println!("    Claude:     \"Map amazon.com and find headphones under $300\"");
            println!("    Terminal:   cortex map amazon.com");
            println!("    Python:     from cortex_client import map");
            println!();
        } else if remove {
            println!();
            println!("  Done. Cortex disconnected from agents.");
            println!("  Runtime still running. Stop with: cortex stop");
            println!();
        }
    }

    Ok(())
}

// ── Agent Probes ────────────────────────────────────────────────

/// An agent probe knows how to detect and locate an agent's config.
struct AgentProbe {
    name: &'static str,
    short_name: &'static str,
    needs_restart: bool,
    detect_fn: fn() -> Option<PathBuf>,
}

impl AgentProbe {
    fn detect(&self) -> Option<PathBuf> {
        (self.detect_fn)()
    }
}

fn home_dir() -> PathBuf {
    dirs::home_dir().expect("cannot determine home directory")
}

fn build_probes() -> Vec<AgentProbe> {
    vec![
        AgentProbe {
            name: "Claude Desktop",
            short_name: "claude-desktop",
            needs_restart: true,
            detect_fn: detect_claude_desktop,
        },
        AgentProbe {
            name: "Claude Code",
            short_name: "claude-code",
            needs_restart: false,
            detect_fn: detect_claude_code,
        },
        AgentProbe {
            name: "Cursor",
            short_name: "cursor",
            needs_restart: true,
            detect_fn: detect_cursor,
        },
        AgentProbe {
            name: "Windsurf",
            short_name: "windsurf",
            needs_restart: true,
            detect_fn: detect_windsurf,
        },
        AgentProbe {
            name: "Continue",
            short_name: "continue",
            needs_restart: false,
            detect_fn: detect_continue,
        },
        AgentProbe {
            name: "Cline",
            short_name: "cline",
            needs_restart: false,
            detect_fn: detect_cline,
        },
    ]
}

fn detect_claude_desktop() -> Option<PathBuf> {
    let candidates = if cfg!(target_os = "macos") {
        vec![home_dir().join("Library/Application Support/Claude/claude_desktop_config.json")]
    } else {
        vec![home_dir().join(".config/claude/claude_desktop_config.json")]
    };
    // Return path even if file doesn't exist yet — parent dir must exist
    for p in candidates {
        if let Some(parent) = p.parent() {
            if parent.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn detect_claude_code() -> Option<PathBuf> {
    let settings = home_dir().join(".claude/settings.json");
    let parent = settings.parent()?;
    if parent.exists() {
        Some(settings)
    } else {
        // Check if `claude` is in PATH
        if which::which("claude").is_ok() {
            // Create the directory if it doesn't exist
            let _ = std::fs::create_dir_all(parent);
            Some(settings)
        } else {
            None
        }
    }
}

fn detect_cursor() -> Option<PathBuf> {
    let config = home_dir().join(".cursor/mcp.json");
    if home_dir().join(".cursor").exists() {
        Some(config)
    } else {
        None
    }
}

fn detect_windsurf() -> Option<PathBuf> {
    let config = home_dir().join(".codeium/windsurf/mcp_config.json");
    if home_dir().join(".codeium").exists() {
        Some(config)
    } else {
        None
    }
}

fn detect_continue() -> Option<PathBuf> {
    let config = home_dir().join(".continue/config.json");
    if home_dir().join(".continue").exists() {
        Some(config)
    } else {
        None
    }
}

fn detect_cline() -> Option<PathBuf> {
    // Cline stores config in VS Code's globalStorage
    let base = if cfg!(target_os = "macos") {
        home_dir()
            .join("Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev")
    } else {
        home_dir().join(".config/Code/User/globalStorage/saoudrizwan.claude-dev")
    };
    if base.exists() {
        Some(base.join("settings/cline_mcp_settings.json"))
    } else {
        None
    }
}

// ── MCP Config Injection ────────────────────────────────────────

/// The MCP server config that gets injected into agent configs.
fn cortex_mcp_entry() -> serde_json::Value {
    json!({
        "command": "npx",
        "args": ["-y", "@cortex/mcp-server"],
        "env": {
            "CORTEX_HOST": "127.0.0.1",
            "CORTEX_PORT": "7700"
        }
    })
}

/// Result of an injection attempt.
enum InjectionResult {
    Injected,
    AlreadyPresent,
}

/// Result of a removal attempt.
enum RemovalResult {
    Removed,
    NotPresent,
}

/// Inject the Cortex MCP server entry into an agent's JSON config.
fn inject_mcp_server(config_path: &Path) -> Result<InjectionResult> {
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        serde_json::from_str(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };

    let obj = config
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("config is not a JSON object"))?;

    let servers = obj.entry("mcpServers").or_insert(json!({}));

    if servers.get("cortex").is_some() {
        return Ok(InjectionResult::AlreadyPresent);
    }

    servers
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("mcpServers is not a JSON object"))?
        .insert("cortex".to_string(), cortex_mcp_entry());

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(InjectionResult::Injected)
}

/// Remove the Cortex MCP server entry from an agent's JSON config.
fn remove_mcp_server(config_path: &Path) -> Result<RemovalResult> {
    if !config_path.exists() {
        return Ok(RemovalResult::NotPresent);
    }

    let content = std::fs::read_to_string(config_path)?;
    let mut config: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(servers) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        if servers.remove("cortex").is_some() {
            std::fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
            return Ok(RemovalResult::Removed);
        }
    }

    Ok(RemovalResult::NotPresent)
}

/// Check if Cortex is already present in an agent's config.
fn check_cortex_present(config_path: &Path) -> bool {
    if !config_path.exists() {
        return false;
    }
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return false;
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    config
        .get("mcpServers")
        .and_then(|v| v.get("cortex"))
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_inject_into_empty_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{{}}").unwrap();
        let path = tmp.path().to_path_buf();

        let result = inject_mcp_server(&path).unwrap();
        assert!(matches!(result, InjectionResult::Injected));

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["cortex"]["command"]
            .as_str()
            .is_some());
    }

    #[test]
    fn test_inject_idempotent() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{{}}").unwrap();
        let path = tmp.path().to_path_buf();

        inject_mcp_server(&path).unwrap();
        let result = inject_mcp_server(&path).unwrap();
        assert!(matches!(result, InjectionResult::AlreadyPresent));
    }

    #[test]
    fn test_inject_preserves_existing_servers() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{"mcpServers": {{"other": {{"command": "other-server"}}}}}}"#
        )
        .unwrap();
        let path = tmp.path().to_path_buf();

        inject_mcp_server(&path).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["other"]["command"].as_str().is_some());
        assert!(content["mcpServers"]["cortex"]["command"]
            .as_str()
            .is_some());
    }

    #[test]
    fn test_remove_mcp_server() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{{}}").unwrap();
        let path = tmp.path().to_path_buf();

        inject_mcp_server(&path).unwrap();
        let result = remove_mcp_server(&path).unwrap();
        assert!(matches!(result, RemovalResult::Removed));

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["cortex"].is_null());
    }

    #[test]
    fn test_remove_not_present() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{{}}").unwrap();
        let path = tmp.path().to_path_buf();

        let result = remove_mcp_server(&path).unwrap();
        assert!(matches!(result, RemovalResult::NotPresent));
    }

    #[test]
    fn test_check_cortex_present() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{{}}").unwrap();
        let path = tmp.path().to_path_buf();

        assert!(!check_cortex_present(&path));
        inject_mcp_server(&path).unwrap();
        assert!(check_cortex_present(&path));
    }
}
