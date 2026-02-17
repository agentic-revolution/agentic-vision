//! JSONL audit logger — append-only log of all operations.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// A single audit event.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub method: String,
    pub domain: Option<String>,
    pub url: Option<String>,
    pub session_id: Option<String>,
    pub duration_ms: u64,
    pub status: String,
}

/// Append-only JSONL audit logger.
pub struct AuditLogger {
    file: File,
}

impl AuditLogger {
    /// Open or create the audit log file.
    pub fn open(path: &PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open audit log: {}", path.display()))?;

        Ok(Self { file })
    }

    /// Open the default audit log at ~/.cortex/audit.jsonl.
    pub fn default_logger() -> Result<Self> {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".cortex")
            .join("audit.jsonl");
        Self::open(&path)
    }

    /// Log an audit event.
    pub fn log(&mut self, event: &AuditEvent) -> Result<()> {
        let json = serde_json::to_string(event)?;
        writeln!(self.file, "{json}")?;
        Ok(())
    }

    /// Log a method call with timing.
    pub fn log_method(
        &mut self,
        method: &str,
        domain: Option<&str>,
        url: Option<&str>,
        session_id: Option<&str>,
        duration_ms: u64,
        status: &str,
    ) -> Result<()> {
        self.log(&AuditEvent {
            timestamp: Utc::now().to_rfc3339(),
            method: method.to_string(),
            domain: domain.map(String::from),
            url: url.map(String::from),
            session_id: session_id.map(String::from),
            duration_ms,
            status: status.to_string(),
        })
    }
}
