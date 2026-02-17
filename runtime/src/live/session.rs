//! Session management for multi-step browser flows.
//!
//! A session holds a persistent browser context with cookies,
//! allowing agents to perform multi-step workflows (e.g., login → navigate → purchase).

use crate::renderer::RenderContext;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// A persistent browser session.
pub struct Session {
    /// Unique session identifier.
    pub id: String,
    /// The browser context (with cookies and state).
    context: Box<dyn RenderContext>,
    /// When the session was created.
    created_at: Instant,
    /// When the session was last accessed.
    last_accessed: Instant,
    /// Session timeout duration.
    timeout: Duration,
}

impl Session {
    /// Create a new session with a browser context.
    pub fn new(id: String, context: Box<dyn RenderContext>, timeout: Duration) -> Self {
        let now = Instant::now();
        Self {
            id,
            context,
            created_at: now,
            last_accessed: now,
            timeout,
        }
    }

    /// Check if the session has expired.
    pub fn is_expired(&self) -> bool {
        self.last_accessed.elapsed() > self.timeout
    }

    /// Touch the session to update last accessed time.
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Get the browser context for this session.
    pub fn context_mut(&mut self) -> &mut dyn RenderContext {
        self.touch();
        self.context.as_mut()
    }

    /// How long the session has been alive.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Close the session and release the browser context.
    pub async fn close(self) -> Result<()> {
        self.context.close().await
    }
}

/// Manages active browser sessions.
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    default_timeout: Duration,
    next_id: Arc<Mutex<u64>>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            default_timeout,
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a new session with a browser context.
    pub async fn create(&self, context: Box<dyn RenderContext>) -> String {
        let mut counter = self.next_id.lock().await;
        *counter += 1;
        let id = format!("sess-{}", *counter);

        let session = Session::new(id.clone(), context, self.default_timeout);
        self.sessions.lock().await.insert(id.clone(), session);

        id
    }

    /// Get a mutable reference to a session.
    pub async fn get_mut<F, R>(&self, session_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;

        if session.is_expired() {
            sessions.remove(session_id);
            bail!("session expired: {}", session_id);
        }

        Ok(f(session))
    }

    /// Close and remove a session.
    pub async fn close(&self, session_id: &str) -> Result<()> {
        let session = self
            .sessions
            .lock()
            .await
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", session_id))?;
        session.close().await
    }

    /// Remove all expired sessions.
    pub async fn cleanup_expired(&self) {
        let mut sessions = self.sessions.lock().await;
        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            if let Some(session) = sessions.remove(&id) {
                let _ = session.close().await;
            }
        }
    }

    /// Number of active sessions.
    pub async fn active_count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}
