//! Optional remote ledger sync — POST audit events to a remote endpoint.

use crate::audit::logger::AuditEvent;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Async ledger sync that sends audit events to a remote URL.
pub struct LedgerSync {
    tx: mpsc::Sender<AuditEvent>,
}

impl LedgerSync {
    /// Create a new ledger sync that POSTs events to the given URL.
    ///
    /// Returns None if CORTEX_LEDGER_URL is not set.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("CORTEX_LEDGER_URL").ok()?;
        let (tx, rx) = mpsc::channel(1000);

        tokio::spawn(async move {
            sync_loop(url, rx).await;
        });

        Some(Self { tx })
    }

    /// Queue an event for remote sync.
    pub async fn send(&self, event: AuditEvent) {
        // Non-blocking, failure-tolerant
        let _ = self.tx.try_send(event);
    }
}

async fn sync_loop(url: String, mut rx: mpsc::Receiver<AuditEvent>) {
    let client = reqwest::Client::new();

    while let Some(event) = rx.recv().await {
        // Best-effort POST, ignore failures
        let _ = client
            .post(&url)
            .json(&event)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
    }
}
