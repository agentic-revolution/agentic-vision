//! HTTP session management for authenticated requests.
//!
//! An `HttpSession` stores cookies, auth headers, and CSRF tokens obtained
//! during authentication. It can be applied to any `HttpClient` request to
//! make authenticated API calls and page fetches.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Monotonic counter for generating unique session IDs.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// An authenticated HTTP session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Domain this session is valid for.
    pub domain: String,
    /// Session cookies (name -> value).
    pub cookies: HashMap<String, String>,
    /// Authentication headers to include (header-name -> value).
    pub auth_headers: HashMap<String, String>,
    /// CSRF token if discovered.
    pub csrf_token: Option<String>,
    /// Type of authentication used.
    pub auth_type: AuthType,
    /// Unix timestamp when session was created.
    pub created_at: f64,
    /// Unix timestamp when session expires, if known.
    pub expires_at: Option<f64>,
}

/// Type of authentication used to establish a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    /// Password-based login (form POST).
    Password,
    /// OAuth flow (browser-assisted).
    OAuth(String), // provider name
    /// API key in header.
    ApiKey,
    /// Bearer token.
    Bearer,
    /// No authentication.
    None,
}

impl HttpSession {
    /// Create a new session for the given domain with the specified auth type.
    ///
    /// Generates a unique session ID from the current timestamp and an atomic
    /// counter. Cookies and auth headers start empty.
    pub fn new(domain: &str, auth_type: AuthType) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        let session_id = format!("sess-{ts}-{counter}");
        let created_at = ts as f64 / 1000.0;

        Self {
            session_id,
            domain: domain.to_string(),
            cookies: HashMap::new(),
            auth_headers: HashMap::new(),
            csrf_token: None,
            auth_type,
            created_at,
            expires_at: None,
        }
    }

    /// Format cookies as a `Cookie` header value.
    ///
    /// Returns a string like `name1=val1; name2=val2`. The order of cookies
    /// is sorted by name for deterministic output.
    pub fn cookie_header(&self) -> String {
        let mut pairs: Vec<_> = self.cookies.iter().collect();
        pairs.sort_by_key(|(k, _)| (*k).clone());
        pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Check whether this session has expired.
    ///
    /// Returns `false` if no expiry is set.
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            now >= expires
        } else {
            false
        }
    }

    /// Add a cookie to this session.
    pub fn add_cookie(&mut self, name: &str, value: &str) {
        self.cookies.insert(name.to_string(), value.to_string());
    }

    /// Add an authentication header to this session.
    pub fn add_auth_header(&mut self, name: &str, value: &str) {
        self.auth_headers
            .insert(name.to_string(), value.to_string());
    }

    /// Set the expiry timestamp for this session.
    pub fn set_expires(&mut self, unix_timestamp: f64) {
        self.expires_at = Some(unix_timestamp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = HttpSession::new("example.com", AuthType::Password);

        assert!(session.session_id.starts_with("sess-"));
        assert_eq!(session.domain, "example.com");
        assert_eq!(session.auth_type, AuthType::Password);
        assert!(session.cookies.is_empty());
        assert!(session.auth_headers.is_empty());
        assert!(session.csrf_token.is_none());
        assert!(session.expires_at.is_none());
        assert!(session.created_at > 0.0);
    }

    #[test]
    fn test_cookie_header_format() {
        let mut session = HttpSession::new("example.com", AuthType::None);
        session.add_cookie("session_id", "abc123");
        session.add_cookie("csrftoken", "xyz789");

        let header = session.cookie_header();
        // Sorted by name: csrftoken comes before session_id
        assert_eq!(header, "csrftoken=xyz789; session_id=abc123");
    }

    #[test]
    fn test_is_expired() {
        let mut session = HttpSession::new("example.com", AuthType::Bearer);

        // No expiry set — not expired.
        assert!(!session.is_expired());

        // Expiry in the past — expired.
        session.set_expires(0.0);
        assert!(session.is_expired());

        // Expiry far in the future — not expired.
        session.set_expires(f64::MAX);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_add_cookies_and_headers() {
        let mut session = HttpSession::new("example.com", AuthType::ApiKey);

        session.add_cookie("sid", "value1");
        session.add_cookie("pref", "dark");
        assert_eq!(session.cookies.len(), 2);
        assert_eq!(session.cookies.get("sid").unwrap(), "value1");
        assert_eq!(session.cookies.get("pref").unwrap(), "dark");

        session.add_auth_header("X-Api-Key", "key123");
        session.add_auth_header("X-Custom", "custom_val");
        assert_eq!(session.auth_headers.len(), 2);
        assert_eq!(session.auth_headers.get("X-Api-Key").unwrap(), "key123");
        assert_eq!(session.auth_headers.get("X-Custom").unwrap(), "custom_val");
    }
}
