//! Shared CLI output formatting with colors, symbols, and structured display.

use std::io::Write;

/// Check if color output is enabled.
pub fn color_enabled() -> bool {
    // Respect NO_COLOR env (https://no-color.org/)
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    // Respect --no-color flag via our global flag
    if std::env::var("CORTEX_NO_COLOR").is_ok() {
        return false;
    }
    // Default: enable color if stdout is a terminal
    atty_stdout()
}

/// Check if stdout is a TTY.
fn atty_stdout() -> bool {
    unsafe { libc_isatty(1) != 0 }
}

#[cfg(unix)]
extern "C" {
    fn isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

#[cfg(unix)]
unsafe fn libc_isatty(fd: i32) -> i32 {
    unsafe { isatty(fd) }
}

#[cfg(not(unix))]
unsafe fn libc_isatty(_fd: i32) -> i32 {
    0
}

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Colored string builder.
pub struct Styled {
    use_color: bool,
}

impl Styled {
    pub fn new() -> Self {
        Self {
            use_color: color_enabled(),
        }
    }

    /// Green checkmark symbol.
    pub fn ok_sym(&self) -> &str {
        if self.use_color {
            "\x1b[32m\u{2713}\x1b[0m"
        } else {
            "OK"
        }
    }

    /// Red X symbol.
    pub fn fail_sym(&self) -> &str {
        if self.use_color {
            "\x1b[31m\u{2717}\x1b[0m"
        } else {
            "!!"
        }
    }

    /// Yellow warning symbol.
    pub fn warn_sym(&self) -> &str {
        if self.use_color {
            "\x1b[33m\u{26a0}\x1b[0m"
        } else {
            "??"
        }
    }

    /// Blue circle (info/neutral) symbol.
    pub fn info_sym(&self) -> &str {
        if self.use_color {
            "\x1b[34m\u{25cb}\x1b[0m"
        } else {
            "--"
        }
    }

    pub fn green(&self, s: &str) -> String {
        if self.use_color {
            format!("{GREEN}{s}{RESET}")
        } else {
            s.to_string()
        }
    }

    pub fn red(&self, s: &str) -> String {
        if self.use_color {
            format!("{RED}{s}{RESET}")
        } else {
            s.to_string()
        }
    }

    pub fn yellow(&self, s: &str) -> String {
        if self.use_color {
            format!("{YELLOW}{s}{RESET}")
        } else {
            s.to_string()
        }
    }

    pub fn blue(&self, s: &str) -> String {
        if self.use_color {
            format!("{BLUE}{s}{RESET}")
        } else {
            s.to_string()
        }
    }

    pub fn cyan(&self, s: &str) -> String {
        if self.use_color {
            format!("{CYAN}{s}{RESET}")
        } else {
            s.to_string()
        }
    }

    pub fn dim(&self, s: &str) -> String {
        if self.use_color {
            format!("{DIM}{s}{RESET}")
        } else {
            s.to_string()
        }
    }

    pub fn bold(&self, s: &str) -> String {
        if self.use_color {
            format!("{BOLD}{s}{RESET}")
        } else {
            s.to_string()
        }
    }
}

/// Print a branded header for CLI output.
pub fn print_header(s: &Styled) {
    eprintln!(
        "  {} {}",
        s.bold("Cortex"),
        s.dim(&format!("v{}", env!("CARGO_PKG_VERSION")))
    );
    eprintln!();
}

/// Print a section header (e.g., "System", "Browser", "Runtime").
pub fn print_section(s: &Styled, title: &str) {
    eprintln!("  {}", s.bold(title));
}

/// Print a check result line with symbol and label/value.
pub fn print_check(symbol: &str, label: &str, value: &str) {
    eprintln!("    {symbol} {label:<16} {value}");
}

/// Print an indented detail/fix line under a check.
pub fn print_detail(msg: &str) {
    eprintln!("                        {msg}");
}

/// Print a status summary line at the bottom.
pub fn print_status(s: &Styled, status: &str, msg: &str) {
    eprintln!();
    eprintln!("  {}: {status} ({msg})", s.bold("Status"));
}

/// Format bytes into human-readable size (e.g., "28.7 MB").
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a duration in seconds into human-readable (e.g., "2h 14m").
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        format!("{m}m {s}s")
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h {m}m")
    }
}

/// Simple progress bar string.
pub fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return format!("[{}]", " ".repeat(width));
    }
    let filled = (current * width) / total;
    let empty = width - filled;
    let pct = (current * 100) / total;
    format!(
        "[{}{}{:>4}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
        format!("] {pct}%")
    )
}

/// Check if --quiet mode is active.
pub fn is_quiet() -> bool {
    std::env::var("CORTEX_QUIET").is_ok()
}

/// Check if --verbose mode is active.
pub fn is_verbose() -> bool {
    std::env::var("CORTEX_VERBOSE").is_ok()
}

/// Check if --json mode is active.
pub fn is_json() -> bool {
    std::env::var("CORTEX_JSON").is_ok()
}

/// Print JSON output to stdout and return.
pub fn print_json(value: &serde_json::Value) {
    if let Ok(s) = serde_json::to_string_pretty(value) {
        println!("{s}");
    }
}
