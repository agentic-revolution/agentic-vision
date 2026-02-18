//! Environment readiness check — the single most important command.
//!
//! Performs 14 diagnostic checks covering system, browser, runtime, cache, and
//! optional toolchain presence. Every failure includes a specific fix instruction.

use crate::cli::output::{self, Styled};
use crate::cli::start::{pid_file_path, SOCKET_PATH};
use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Run the full 14-check doctor diagnostic.
pub async fn run() -> Result<()> {
    if output::is_json() {
        return run_json().await;
    }

    let s = Styled::new();
    let mut ready = true;
    let mut has_warning = false;

    // Header
    output::print_header(&s);

    // ── System ──────────────────────────────────────────────────────────
    output::print_section(&s, "System");

    // 1. OS / architecture
    let os = format_os();
    let arch = std::env::consts::ARCH;
    output::print_check(s.ok_sym(), "OS:", &format!("{os} ({arch})"));

    // 2-3. Memory
    let (total_mb, avail_mb) = get_memory_mb();
    match avail_mb {
        Some(a) if a >= 256 => {
            let display = if let Some(t) = total_mb {
                format!(
                    "{:.1} GB total, {:.1} GB available",
                    t as f64 / 1024.0,
                    a as f64 / 1024.0
                )
            } else {
                format!("{:.1} GB available", a as f64 / 1024.0)
            };
            output::print_check(s.ok_sym(), "Memory:", &display);
        }
        Some(a) => {
            output::print_check(
                s.warn_sym(),
                "Memory:",
                &format!("{a} MB available (recommend >= 256 MB)"),
            );
            has_warning = true;
        }
        None => {
            output::print_check(s.warn_sym(), "Memory:", "could not determine");
            has_warning = true;
        }
    }

    // 11. Disk space
    let cortex_dir = cortex_home();
    match get_free_disk_mb(&cortex_dir) {
        Some(free_mb) if free_mb >= 100 => {
            output::print_check(
                s.ok_sym(),
                "Disk:",
                &format!(
                    "{} free at {}",
                    output::format_size(free_mb * 1_048_576),
                    cortex_dir.display()
                ),
            );
        }
        Some(free_mb) => {
            output::print_check(
                s.fail_sym(),
                "Disk:",
                &format!("{free_mb} MB free (< 100 MB minimum)"),
            );
            output::print_detail("Free up disk space or change CORTEX_HOME.");
            ready = false;
        }
        None => {
            output::print_check(s.warn_sym(), "Disk:", "could not determine free space");
            has_warning = true;
        }
    }

    eprintln!();

    // ── Browser ─────────────────────────────────────────────────────────
    output::print_section(&s, "Browser");

    // 4-5. Chromium installed + version
    let chromium_path = find_chromium();
    match &chromium_path {
        Some(path) => {
            let version = get_chromium_version(path);
            let ver_str = version.as_deref().unwrap_or("unknown version");
            output::print_check(
                s.ok_sym(),
                "Chromium:",
                &format!("{ver_str} at {}", path.display()),
            );

            // 6. Headless launch test
            match test_headless_launch(path) {
                Ok(ms) => {
                    output::print_check(
                        s.ok_sym(),
                        "Headless test:",
                        &format!("launched and closed in {ms}ms"),
                    );
                }
                Err(e) => {
                    let msg = e.to_string();
                    output::print_check(s.fail_sym(), "Headless test:", &format!("FAILED — {msg}"));
                    if msg.contains("shared librar") || msg.contains("libnss") {
                        suggest_shared_libs();
                    }
                    if is_docker() {
                        output::print_detail("Running in Docker? Try CORTEX_CHROMIUM_NO_SANDBOX=1");
                    }
                    ready = false;
                }
            }
        }
        None => {
            output::print_check(s.fail_sym(), "Chromium:", "NOT FOUND");
            output::print_detail("Fix: run 'cortex install'");
            output::print_detail("Or set CORTEX_CHROMIUM_PATH=/path/to/chrome");
            ready = false;
        }
    }

    // 7. Shared libraries (Linux only)
    #[cfg(target_os = "linux")]
    check_shared_libs(&s, &mut ready);

    // 7b. musl libc detection (Alpine Linux)
    #[cfg(target_os = "linux")]
    {
        if is_musl_libc() {
            output::print_check(
                s.warn_sym(),
                "C library:",
                "musl libc detected (Alpine Linux)",
            );
            output::print_detail("Chromium does not run natively on musl. Install gcompat:");
            output::print_detail("  apk add gcompat");
        }
    }

    eprintln!();

    // ── Runtime ─────────────────────────────────────────────────────────
    output::print_section(&s, "Runtime");

    // 8. Socket path writable
    let socket_path = PathBuf::from(SOCKET_PATH);
    let socket_dir = socket_path.parent().unwrap_or(&socket_path);
    if socket_dir.exists() {
        output::print_check(
            s.ok_sym(),
            "Socket path:",
            &format!("{SOCKET_PATH} (writable)"),
        );
    } else {
        output::print_check(
            s.fail_sym(),
            "Socket path:",
            &format!("directory {} does not exist", socket_dir.display()),
        );
        ready = false;
    }

    // 9-10. Process status
    let pid_path = pid_file_path();
    let process_status = check_process_status(&pid_path, SOCKET_PATH);
    match &process_status {
        ProcessStatus::RunningResponding(pid) => {
            output::print_check(s.ok_sym(), "Process:", &format!("running (PID {pid})"));
        }
        ProcessStatus::RunningNotResponding(pid) => {
            output::print_check(
                s.warn_sym(),
                "Process:",
                &format!("running (PID {pid}) but not responding on socket"),
            );
            output::print_detail("This usually means a crashed process.");
            output::print_detail("Fix: run 'cortex stop' then 'cortex start'");
            has_warning = true;
        }
        ProcessStatus::StalePid(pid) => {
            output::print_check(
                s.warn_sym(),
                "Process:",
                &format!("stale PID file (PID {pid} is dead)"),
            );
            output::print_detail("Fix: run 'cortex start' (will clean up automatically)");
            has_warning = true;
            // Clean up stale PID
            let _ = std::fs::remove_file(&pid_path);
        }
        ProcessStatus::NotRunning => {
            output::print_check(s.fail_sym(), "Process:", "not running");
        }
        ProcessStatus::SocketConflict => {
            output::print_check(s.fail_sym(), "Process:", "socket in use by another process");
            output::print_detail(&format!("Another process is listening on {SOCKET_PATH}"));
            output::print_detail("Remove the socket file or choose a different path.");
            ready = false;
        }
    }

    eprintln!();

    // ── Cache ───────────────────────────────────────────────────────────
    output::print_section(&s, "Cache");

    // 14. Cached maps
    let maps_dir = cortex_home().join("maps");
    let (map_count, map_names, total_size) = scan_cached_maps(&maps_dir);
    if map_count > 0 {
        let names = if map_names.len() <= 5 {
            map_names.join(", ")
        } else {
            format!(
                "{}, ... (+{} more)",
                map_names[..5].join(", "),
                map_names.len() - 5
            )
        };
        output::print_check(
            s.info_sym(),
            "Maps cached:",
            &format!("{map_count} ({names})"),
        );
        output::print_check(
            s.info_sym(),
            "Cache size:",
            &output::format_size(total_size),
        );
    } else {
        output::print_check(s.info_sym(), "Maps cached:", "none");
    }

    eprintln!();

    // ── Optional ────────────────────────────────────────────────────────
    output::print_section(&s, "Optional");

    // 12. Node.js
    match check_tool_version("node", &["--version"]) {
        Some(ver) => output::print_check(
            s.ok_sym(),
            "Node.js:",
            &format!("{ver} (for extractor development)"),
        ),
        None => output::print_check(
            s.info_sym(),
            "Node.js:",
            "not found (only needed for custom extractors)",
        ),
    }

    // 13. Python
    match check_tool_version("python3", &["--version"]) {
        Some(ver) => {
            output::print_check(s.ok_sym(), "Python:", &format!("{ver} (for cortex-client)"))
        }
        None => output::print_check(
            s.info_sym(),
            "Python:",
            "not found (only needed for Python client)",
        ),
    }

    // Status summary
    if ready && !has_warning {
        output::print_status(&s, &s.green("READY"), "start with 'cortex start'");
    } else if ready && has_warning {
        output::print_status(&s, &s.yellow("READY"), "some warnings above");
    } else {
        output::print_status(&s, &s.red("NOT READY"), "fix issues above");
    }

    Ok(())
}

/// JSON output mode for doctor.
async fn run_json() -> Result<()> {
    let chromium_path = find_chromium();
    let chromium_version = chromium_path.as_ref().and_then(get_chromium_version);
    let (total_mb, avail_mb) = get_memory_mb();
    let pid_path = pid_file_path();
    let process = check_process_status(&pid_path, SOCKET_PATH);
    let maps_dir = cortex_home().join("maps");
    let (map_count, map_names, total_size) = scan_cached_maps(&maps_dir);
    let node_ver = check_tool_version("node", &["--version"]);
    let python_ver = check_tool_version("python3", &["--version"]);

    let json = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "memory_total_mb": total_mb,
        "memory_available_mb": avail_mb,
        "chromium_path": chromium_path.map(|p| p.display().to_string()),
        "chromium_version": chromium_version,
        "socket_path": SOCKET_PATH,
        "process_status": format!("{process:?}"),
        "maps_cached": map_count,
        "map_names": map_names,
        "cache_size_bytes": total_size,
        "node_version": node_ver,
        "python_version": python_ver,
    });
    output::print_json(&json);
    Ok(())
}

// ── Helper functions ────────────────────────────────────────────────────────

/// Format OS name nicely.
fn format_os() -> String {
    match std::env::consts::OS {
        "macos" => {
            if let Ok(out) = Command::new("sw_vers").arg("-productVersion").output() {
                if out.status.success() {
                    let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    return format!("macOS {ver}");
                }
            }
            "macOS".to_string()
        }
        "linux" => {
            if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
                for line in contents.lines() {
                    if let Some(name) = line.strip_prefix("PRETTY_NAME=") {
                        return name.trim_matches('"').to_string();
                    }
                }
            }
            "Linux".to_string()
        }
        other => other.to_string(),
    }
}

/// Get the Cortex home directory (~/.cortex/).
pub fn cortex_home() -> PathBuf {
    if let Ok(p) = std::env::var("CORTEX_HOME") {
        return PathBuf::from(p);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cortex")
}

/// Find Chromium binary by checking multiple locations.
pub fn find_chromium() -> Option<PathBuf> {
    // 1. Check CORTEX_CHROMIUM_PATH env
    if let Ok(p) = std::env::var("CORTEX_CHROMIUM_PATH") {
        let path = PathBuf::from(&p);
        if path.exists() {
            return Some(path);
        }
    }

    // 2. Check ~/.cortex/chromium/
    if let Some(home) = dirs::home_dir() {
        let candidates = if cfg!(target_os = "macos") {
            vec![
                home.join(".cortex/chromium/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"),
                home.join(".cortex/chromium/chrome"),
            ]
        } else {
            vec![
                home.join(".cortex/chromium/chrome"),
                home.join(".cortex/chromium/chrome-linux64/chrome"),
            ]
        };
        for c in candidates {
            if c.exists() {
                return Some(c);
            }
        }
    }

    // 3. Check system PATH
    for name in &["google-chrome", "chromium", "chromium-browser"] {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
    }

    // 4. Common macOS locations
    if cfg!(target_os = "macos") {
        let common = PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
        if common.exists() {
            return Some(common);
        }
    }

    None
}

/// Get Chromium version string.
fn get_chromium_version(path: &PathBuf) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if output.status.success() {
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(raw.replace("Google Chrome ", "").replace("Chromium ", ""))
    } else {
        None
    }
}

/// Test that Chromium can launch headless and close.
fn test_headless_launch(chromium_path: &PathBuf) -> Result<u64> {
    let start = std::time::Instant::now();
    let mut cmd = Command::new(chromium_path);
    cmd.args(["--headless", "--disable-gpu", "--dump-dom", "about:blank"]);

    if is_docker() || std::env::var("CORTEX_CHROMIUM_NO_SANDBOX").is_ok() {
        cmd.arg("--no-sandbox");
    }

    let output = cmd
        .output()
        .map_err(|e| anyhow::anyhow!("failed to launch: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "{}",
            stderr.lines().next().unwrap_or("unknown error")
        ));
    }

    Ok(start.elapsed().as_millis() as u64)
}

/// Get total and available memory in MB.
fn get_memory_mb() -> (Option<u64>, Option<u64>) {
    #[cfg(target_os = "macos")]
    {
        let total = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<u64>()
                    .ok()
            })
            .map(|b| b / 1_048_576);

        let avail = Command::new("vm_stat").output().ok().and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            let mut free = 0u64;
            for line in s.lines() {
                if line.starts_with("Pages free") || line.starts_with("Pages inactive") {
                    if let Some(val) = line.split(':').nth(1) {
                        if let Ok(n) = val.trim().trim_end_matches('.').parse::<u64>() {
                            free += n * 4096;
                        }
                    }
                }
            }
            if free > 0 {
                Some(free / 1_048_576)
            } else {
                total
            }
        });

        (total, avail)
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("free").args(["-m"]).output().ok();
        if let Some(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if line.starts_with("Mem:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let total = parts.get(1).and_then(|v| v.parse().ok());
                    let avail = parts.get(6).and_then(|v| v.parse().ok());
                    return (total, avail);
                }
            }
        }
        (None, None)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        (None, None)
    }
}

/// Get free disk space in MB at a given path.
fn get_free_disk_mb(path: &std::path::Path) -> Option<u64> {
    let check_path = if path.exists() {
        path.to_path_buf()
    } else if let Some(parent) = path.parent() {
        if parent.exists() {
            parent.to_path_buf()
        } else {
            PathBuf::from("/")
        }
    } else {
        PathBuf::from("/")
    };

    let output = Command::new("df")
        .args(["-m", &check_path.display().to_string()])
        .output()
        .ok()?;

    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = s.lines().nth(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                return parts[3].parse().ok();
            }
        }
    }
    None
}

/// Process status enum for the runtime check.
#[derive(Debug)]
enum ProcessStatus {
    RunningResponding(i32),
    RunningNotResponding(i32),
    StalePid(i32),
    NotRunning,
    SocketConflict,
}

/// Check if the Cortex process is running and responding.
fn check_process_status(pid_path: &PathBuf, socket_path: &str) -> ProcessStatus {
    let pid = match std::fs::read_to_string(pid_path) {
        Ok(s) => match s.trim().parse::<i32>() {
            Ok(pid) => pid,
            Err(_) => {
                let _ = std::fs::remove_file(pid_path);
                return if PathBuf::from(socket_path).exists() {
                    ProcessStatus::SocketConflict
                } else {
                    ProcessStatus::NotRunning
                };
            }
        },
        Err(_) => {
            return if PathBuf::from(socket_path).exists() {
                ProcessStatus::SocketConflict
            } else {
                ProcessStatus::NotRunning
            };
        }
    };

    let alive = is_process_alive(pid);
    if !alive {
        return ProcessStatus::StalePid(pid);
    }

    if PathBuf::from(socket_path).exists() {
        #[cfg(unix)]
        {
            match std::os::unix::net::UnixStream::connect(socket_path) {
                Ok(_) => ProcessStatus::RunningResponding(pid),
                Err(_) => ProcessStatus::RunningNotResponding(pid),
            }
        }
        #[cfg(not(unix))]
        {
            ProcessStatus::RunningNotResponding(pid)
        }
    } else {
        ProcessStatus::RunningNotResponding(pid)
    }
}

/// Check if a process with the given PID is alive.
fn is_process_alive(pid: i32) -> bool {
    #[cfg(unix)]
    {
        let output = Command::new("kill").args(["-0", &pid.to_string()]).output();
        matches!(output, Ok(o) if o.status.success())
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Check if the system uses musl libc (Alpine Linux).
#[cfg(target_os = "linux")]
fn is_musl_libc() -> bool {
    // Check ldd --version output for "musl"
    if let Ok(output) = Command::new("ldd").arg("--version").output() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stderr.contains("musl") || stdout.contains("musl") {
            return true;
        }
    }
    // Check if /lib/ld-musl-*.so.1 exists
    if let Ok(entries) = std::fs::read_dir("/lib") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("ld-musl") {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if running inside Docker.
fn is_docker() -> bool {
    PathBuf::from("/.dockerenv").exists()
        || std::fs::read_to_string("/proc/1/cgroup")
            .map(|s| s.contains("docker") || s.contains("containerd"))
            .unwrap_or(false)
}

/// Scan cached maps directory and return (count, names, total_size_bytes).
fn scan_cached_maps(maps_dir: &PathBuf) -> (usize, Vec<String>, u64) {
    let mut names = Vec::new();
    let mut total = 0u64;

    if let Ok(entries) = std::fs::read_dir(maps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "ctx") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
                if let Ok(meta) = path.metadata() {
                    total += meta.len();
                }
            }
        }
    }

    names.sort();
    let count = names.len();
    (count, names, total)
}

/// Check if a tool exists and return its version string.
fn check_tool_version(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if output.status.success() {
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let clean = raw
            .replace("Python ", "")
            .replace("python ", "")
            .replace("v", "");
        Some(if clean.is_empty() { raw } else { clean })
    } else {
        None
    }
}

/// Suggest shared library installation commands.
fn suggest_shared_libs() {
    output::print_detail("Missing shared libraries for Chromium.");
    output::print_detail(
        "Fix (Ubuntu/Debian): sudo apt install libnss3 libatk1.0-0 libatk-bridge2.0-0",
    );
    output::print_detail("Fix (Alpine):        apk add nss atk at-spi2-atk");
}

/// Check shared libraries on Linux.
#[cfg(target_os = "linux")]
fn check_shared_libs(s: &Styled, ready: &mut bool) {
    let libs = [
        "libnss3",
        "libatk1.0-0",
        "libatk-bridge2.0-0",
        "libcups2",
        "libxcomposite1",
        "libxrandr2",
    ];
    let mut missing = Vec::new();
    for lib in &libs {
        if Command::new("ldconfig")
            .args(["-p"])
            .output()
            .ok()
            .map(|o| !String::from_utf8_lossy(&o.stdout).contains(lib))
            .unwrap_or(true)
        {
            missing.push(*lib);
        }
    }
    if !missing.is_empty() {
        output::print_check(
            s.warn_sym(),
            "Shared libs:",
            &format!("missing: {}", missing.join(", ")),
        );
        output::print_detail(&format!(
            "Fix (Ubuntu/Debian): sudo apt install {}",
            missing.join(" ")
        ));
        *ready = false;
    }
}
