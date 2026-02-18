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
use rustyline::{
    Cmd, ConditionalEventHandler, Event, EventContext, EventHandler, Helper, KeyEvent, RepeatCount,
};

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

/// Event handler that inserts `/` and then triggers Tab completion when
/// the line is empty. This gives the Claude Code-style experience where
/// pressing `/` immediately shows the command picker list.
pub struct SlashTrigger;

impl ConditionalEventHandler for SlashTrigger {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        if ctx.line().is_empty() {
            // On empty line: insert `/` — the Tab hint will guide them,
            // and CompletionType::List will show all commands on next Tab press.
            // We use Cmd::Insert so the character appears immediately.
            Some(Cmd::Insert(1, "/".to_string()))
        } else {
            // Mid-line: just insert `/` normally
            Some(Cmd::Insert(1, "/".to_string()))
        }
    }
}

/// Event handler that auto-triggers completion after typing `/`.
/// When Tab is pressed and line starts with `/`, show the command list.
pub struct TabCompleteOrAcceptHint;

impl ConditionalEventHandler for TabCompleteOrAcceptHint {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        // If there's a hint showing, complete the hint
        if ctx.has_hint() {
            Some(Cmd::CompleteHint)
        } else {
            // Otherwise, trigger normal completion (shows the List picker)
            Some(Cmd::Complete)
        }
    }
}

/// Bind custom key sequences to the editor for interactive command selection.
pub fn bind_keys(rl: &mut rustyline::Editor<CortexHelper, rustyline::history::DefaultHistory>) {
    // Bind Tab to smart complete (accept hint if present, else show picker list)
    rl.bind_sequence(
        KeyEvent::from('\t'),
        EventHandler::Conditional(Box::new(TabCompleteOrAcceptHint)),
    );
}

/// Find the closest matching command for a misspelled input (Levenshtein distance).
pub fn suggest_command(input: &str) -> Option<&'static str> {
    let input_lower = input.to_lowercase();
    let mut best: Option<(&str, usize)> = None;

    for (cmd, _) in COMMANDS {
        // Strip the leading /
        let cmd_name = &cmd[1..];
        let dist = levenshtein(&input_lower, cmd_name);
        if dist <= 3 && (best.is_none() || dist < best.unwrap().1) {
            best = Some((cmd, dist));
        }
    }

    best.map(|(cmd, _)| cmd)
}

/// Simple Levenshtein distance for fuzzy matching.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}
