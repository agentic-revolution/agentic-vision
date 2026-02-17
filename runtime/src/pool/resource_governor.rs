//! Resource governor — enforces memory limits on the browser pool.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Tracks and enforces resource limits for the browser pool.
pub struct ResourceGovernor {
    /// Current estimated memory usage in bytes.
    memory_usage: Arc<AtomicU64>,
    /// Maximum allowed memory in bytes.
    memory_limit: u64,
    /// Per-request timeout in milliseconds.
    request_timeout_ms: u64,
}

impl ResourceGovernor {
    /// Create a new resource governor with a memory limit in megabytes.
    pub fn new(memory_limit_mb: u64, request_timeout_ms: u64) -> Self {
        Self {
            memory_usage: Arc::new(AtomicU64::new(0)),
            memory_limit: memory_limit_mb * 1024 * 1024,
            request_timeout_ms,
        }
    }

    /// Check if acquiring another context would exceed the memory limit.
    pub fn can_acquire(&self, estimated_mb: u64) -> bool {
        let current = self.memory_usage.load(Ordering::SeqCst);
        current + (estimated_mb * 1024 * 1024) <= self.memory_limit
    }

    /// Record memory allocation for a context.
    pub fn record_allocation(&self, bytes: u64) {
        self.memory_usage.fetch_add(bytes, Ordering::SeqCst);
    }

    /// Record memory deallocation when a context is released.
    pub fn record_deallocation(&self, bytes: u64) {
        self.memory_usage.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// Current memory usage in bytes.
    pub fn memory_usage_bytes(&self) -> u64 {
        self.memory_usage.load(Ordering::SeqCst)
    }

    /// Current memory usage in megabytes.
    pub fn memory_usage_mb(&self) -> f64 {
        self.memory_usage_bytes() as f64 / (1024.0 * 1024.0)
    }

    /// Per-request timeout in milliseconds.
    pub fn request_timeout_ms(&self) -> u64 {
        self.request_timeout_ms
    }

    /// Memory limit in megabytes.
    pub fn memory_limit_mb(&self) -> u64 {
        self.memory_limit / (1024 * 1024)
    }
}
