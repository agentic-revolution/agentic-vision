// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tab completion for the Cortex interactive REPL.
//!
//! Provides context-aware completion for slash commands, domain names
//! (from the map cache), and page type names.

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;

use crate::cli::doctor::cortex_home;

/// All available REPL slash commands.
pub const COMMANDS: &[(&str, &str)] = &[
    ("/map", "Map a website into a navigable graph"),
    ("/query", "Search current map by type/features"),
    ("/pathfind", "Find shortest path between nodes"),
    ("/perceive", "Analyze a single live page"),
    ("/status", "Show runtime status"),
    ("/doctor", "Check environment and diagnose issues"),
    ("/maps", "List all cached maps"),
    ("/use", "Switch active domain"),
    ("/settings", "View current configuration"),
    ("/plug", "Show AI agent connections"),
    ("/cache", "Manage cached maps (clear)"),
    ("/clear", "Clear the screen"),
    ("/help", "Show available commands"),
    ("/exit", "Quit the REPL"),
];

/// Page type names for --type completion.
const PAGE_TYPES: &[&str] = &[
    "home",
    "product_detail",
    "product_listing",
    "article",
    "search_results",
    "login",
    "cart",
    "checkout",
    "account",
    "documentation",
    "form",
    "about",
    "contact",
    "faq",
    "pricing",
];

/// Cortex REPL helper providing tab completion.
pub struct CortexHelper;

impl CortexHelper {
    pub fn new() -> Self {
        Self
    }

    /// Get list of cached domain names from ~/.cortex/maps/*.ctx.
    fn cached_domains(&self) -> Vec<String> {
        let maps_dir = cortex_home().join("maps");
        let mut domains = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&maps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "ctx") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        domains.push(stem.to_string());
                    }
                }
            }
        }
        domains.sort();
        domains
    }
}

impl Completer for CortexHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let input = &line[..pos];

        // Complete command names if input starts with /
        if !input.contains(' ') {
            let matches: Vec<Pair> = COMMANDS
                .iter()
                .filter(|(cmd, _)| cmd.starts_with(input))
                .map(|(cmd, desc)| Pair {
                    display: format!("{cmd:<16} {desc}"),
                    replacement: format!("{cmd} "),
                })
                .collect();
            return Ok((0, matches));
        }

        // Split into command and args
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0];
        let args = if parts.len() > 1 { parts[1] } else { "" };

        match cmd {
            // Domain completion for /map, /query, /use, /pathfind
            "/map" | "/query" | "/use" | "/pathfind" => {
                if !args.starts_with('-') {
                    let domains = self.cached_domains();
                    let prefix_start = input.len() - args.len();
                    let matches: Vec<Pair> = domains
                        .iter()
                        .filter(|d| d.starts_with(args.trim()))
                        .map(|d| Pair {
                            display: d.clone(),
                            replacement: format!("{d} "),
                        })
                        .collect();
                    return Ok((prefix_start, matches));
                }

                // --type completion for /query
                if args.contains("--type ") || args.contains("--type=") {
                    let type_start = args.rfind("--type").unwrap_or(0);
                    let after_type = if args[type_start..].contains('=') {
                        &args[args.rfind('=').unwrap() + 1..]
                    } else if args[type_start..].contains(' ') {
                        let space_after = args[type_start..].find(' ').unwrap();
                        &args[type_start + space_after + 1..]
                    } else {
                        ""
                    };
                    let prefix_start = input.len() - after_type.len();
                    let matches: Vec<Pair> = PAGE_TYPES
                        .iter()
                        .filter(|t| t.starts_with(after_type.trim()))
                        .map(|t| Pair {
                            display: t.to_string(),
                            replacement: format!("{t} "),
                        })
                        .collect();
                    return Ok((prefix_start, matches));
                }

                Ok((pos, Vec::new()))
            }

            "/cache" => {
                let matches: Vec<Pair> = vec![Pair {
                    display: "clear".to_string(),
                    replacement: "clear ".to_string(),
                }];
                let prefix_start = input.len() - args.len();
                Ok((
                    prefix_start,
                    matches
                        .into_iter()
                        .filter(|p| p.replacement.starts_with(args.trim()))
                        .collect(),
                ))
            }

            _ => Ok((pos, Vec::new())),
        }
    }
}

impl Hinter for CortexHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if pos < line.len() || line.is_empty() {
            return None;
        }
        // Show first matching command as ghost text
        if line.starts_with('/') && !line.contains(' ') {
            for (cmd, _) in COMMANDS {
                if cmd.starts_with(line) && *cmd != line {
                    return Some(cmd[line.len()..].to_string());
                }
            }
        }
        None
    }
}

impl Highlighter for CortexHelper {}
impl Validator for CortexHelper {}
impl Helper for CortexHelper {}
