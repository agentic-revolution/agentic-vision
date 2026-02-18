//! Discovers WebSocket endpoints from HTML and JavaScript source code.
//!
//! Scans for WebSocket connection patterns in page source and JS bundles:
//!
//! 1. **Standard WebSocket** — `new WebSocket("wss://...")`
//! 2. **Socket.IO** — `io("wss://...",` or `io.connect("...")`
//! 3. **SockJS** — `new SockJS("...")`
//! 4. **SignalR** — `new signalR.HubConnectionBuilder().withUrl("...")`
//!
//! Also checks known platform configurations for major real-time apps
//! (Slack, Discord, etc.) via an embedded JSON registry.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;

const WS_PLATFORMS_JSON: &str = include_str!("ws_platforms.json");

/// The WebSocket protocol/library in use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsProtocol {
    /// Standard WebSocket API.
    Raw,
    /// Socket.IO protocol.
    SocketIO,
    /// SockJS protocol.
    SockJS,
    /// ASP.NET SignalR.
    SignalR,
    /// Unknown protocol wrapper.
    Unknown,
}

/// How the WebSocket connection authenticates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsAuth {
    /// Auth via cookies (sent automatically).
    Cookie,
    /// Auth via token in query parameter.
    QueryParam,
    /// Auth via token in the first message.
    FirstMessage,
    /// Auth via HTTP header (upgrade request).
    Header,
    /// No authentication required.
    None,
}

/// A discovered WebSocket endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEndpoint {
    /// The WebSocket URL (wss:// or ws://).
    pub url: String,
    /// The protocol/library used.
    pub protocol: WsProtocol,
    /// Authentication method.
    pub auth_method: WsAuth,
    /// Which source this was discovered from.
    pub discovered_from: String,
    /// Confidence that this is a real endpoint, in [0.0, 1.0].
    pub confidence: f32,
}

// ── Platform configuration types ────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WsPlatformConfig {
    ws_url_pattern: Option<String>,
    ws_url: Option<String>,
    protocol: String,
    auth: String,
    #[allow(dead_code)]
    ping: Option<serde_json::Value>,
    #[allow(dead_code)]
    auth_message: Option<serde_json::Value>,
    #[allow(dead_code)]
    heartbeat: Option<serde_json::Value>,
    #[allow(dead_code)]
    send_message: Option<serde_json::Value>,
}

type WsPlatformRegistry = std::collections::HashMap<String, WsPlatformConfig>;

fn ws_platform_registry() -> &'static WsPlatformRegistry {
    static REGISTRY: OnceLock<WsPlatformRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| serde_json::from_str(WS_PLATFORMS_JSON).unwrap_or_default())
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Discover WebSocket endpoints from HTML source and JS bundles.
///
/// Scans for `new WebSocket(...)`, Socket.IO, SockJS, and SignalR patterns.
/// Also checks the known platform registry.
///
/// # Arguments
///
/// * `html` - Raw HTML source of the page.
/// * `js_bundles` - JavaScript bundle source strings to scan.
/// * `domain` - The domain being mapped (for platform lookup).
///
/// # Returns
///
/// A vector of discovered [`WsEndpoint`] items, deduplicated by URL.
pub fn discover_ws_endpoints(html: &str, js_bundles: &[String], domain: &str) -> Vec<WsEndpoint> {
    let mut endpoints = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    // Check known platforms first
    for (platform_domain, config) in ws_platform_registry() {
        if domain.contains(platform_domain.as_str()) || platform_domain.contains(domain) {
            let url = config
                .ws_url
                .clone()
                .or_else(|| config.ws_url_pattern.clone())
                .unwrap_or_default();
            if !url.is_empty() && seen_urls.insert(url.clone()) {
                let protocol = parse_protocol(&config.protocol);
                let auth_method = parse_auth(&config.auth);
                endpoints.push(WsEndpoint {
                    url,
                    protocol,
                    auth_method,
                    discovered_from: format!("platform:{platform_domain}"),
                    confidence: 0.95,
                });
            }
        }
    }

    // Scan HTML and JS sources
    let sources: Vec<(&str, String)> = std::iter::once((html, "html".to_string()))
        .chain(
            js_bundles
                .iter()
                .enumerate()
                .map(|(i, s)| (s.as_str(), format!("js_bundle_{i}"))),
        )
        .collect();

    for (source, source_name) in &sources {
        // Pattern 1: new WebSocket("wss://..." or "ws://...")
        scan_standard_ws(source, source_name, &mut endpoints, &mut seen_urls);

        // Pattern 2: Socket.IO
        scan_socketio(source, source_name, &mut endpoints, &mut seen_urls);

        // Pattern 3: SockJS
        scan_sockjs(source, source_name, &mut endpoints, &mut seen_urls);

        // Pattern 4: SignalR
        scan_signalr(source, source_name, &mut endpoints, &mut seen_urls);
    }

    endpoints
}

/// Check if a domain has a known WebSocket endpoint.
pub fn has_known_ws(domain: &str) -> bool {
    ws_platform_registry()
        .keys()
        .any(|k| domain.contains(k.as_str()) || k.contains(domain))
}

/// Get the known WebSocket configuration for a domain, if any.
pub(crate) fn get_known_ws_config(domain: &str) -> Option<&'static WsPlatformConfig> {
    ws_platform_registry()
        .iter()
        .find(|(k, _)| domain.contains(k.as_str()) || k.contains(domain))
        .map(|(_, v)| v)
}

// ── Private scanning functions ──────────────────────────────────────────────

fn scan_standard_ws(
    source: &str,
    source_name: &str,
    endpoints: &mut Vec<WsEndpoint>,
    seen: &mut HashSet<String>,
) {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"new\s+WebSocket\(\s*['"]((wss?://[^'"]+))['"]"#).expect("ws regex is valid")
    });

    for caps in re.captures_iter(source) {
        let url = caps.get(1).map_or("", |m| m.as_str()).to_string();
        if !url.is_empty() && seen.insert(url.clone()) {
            endpoints.push(WsEndpoint {
                url,
                protocol: WsProtocol::Raw,
                auth_method: WsAuth::None,
                discovered_from: source_name.to_string(),
                confidence: 0.90,
            });
        }
    }
}

fn scan_socketio(
    source: &str,
    source_name: &str,
    endpoints: &mut Vec<WsEndpoint>,
    seen: &mut HashSet<String>,
) {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"io(?:\.connect)?\(\s*['"]((?:wss?|https?)://[^'"]+)['"]"#)
            .expect("socketio regex is valid")
    });

    for caps in re.captures_iter(source) {
        let url = caps.get(1).map_or("", |m| m.as_str()).to_string();
        if !url.is_empty() && seen.insert(url.clone()) {
            // Convert http(s) to ws(s) for Socket.IO
            let ws_url = url
                .replace("https://", "wss://")
                .replace("http://", "ws://");
            endpoints.push(WsEndpoint {
                url: ws_url,
                protocol: WsProtocol::SocketIO,
                auth_method: WsAuth::Cookie,
                discovered_from: source_name.to_string(),
                confidence: 0.85,
            });
        }
    }
}

fn scan_sockjs(
    source: &str,
    source_name: &str,
    endpoints: &mut Vec<WsEndpoint>,
    seen: &mut HashSet<String>,
) {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"new\s+SockJS\(\s*['"]([^'"]+)['"]"#).expect("sockjs regex is valid")
    });

    for caps in re.captures_iter(source) {
        let url = caps.get(1).map_or("", |m| m.as_str()).to_string();
        if !url.is_empty() && seen.insert(url.clone()) {
            endpoints.push(WsEndpoint {
                url,
                protocol: WsProtocol::SockJS,
                auth_method: WsAuth::Cookie,
                discovered_from: source_name.to_string(),
                confidence: 0.85,
            });
        }
    }
}

fn scan_signalr(
    source: &str,
    source_name: &str,
    endpoints: &mut Vec<WsEndpoint>,
    seen: &mut HashSet<String>,
) {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"\.withUrl\(\s*['"]([^'"]+)['"]"#).expect("signalr regex is valid")
    });

    // Only match if signalR or HubConnectionBuilder is also present
    if !source.contains("signalR") && !source.contains("HubConnection") {
        return;
    }

    for caps in re.captures_iter(source) {
        let url = caps.get(1).map_or("", |m| m.as_str()).to_string();
        if !url.is_empty() && seen.insert(url.clone()) {
            endpoints.push(WsEndpoint {
                url,
                protocol: WsProtocol::SignalR,
                auth_method: WsAuth::Cookie,
                discovered_from: source_name.to_string(),
                confidence: 0.85,
            });
        }
    }
}

fn parse_protocol(s: &str) -> WsProtocol {
    match s {
        "raw" => WsProtocol::Raw,
        "socketio" | "socket.io" => WsProtocol::SocketIO,
        "sockjs" => WsProtocol::SockJS,
        "signalr" => WsProtocol::SignalR,
        _ => WsProtocol::Unknown,
    }
}

fn parse_auth(s: &str) -> WsAuth {
    match s {
        "cookie" => WsAuth::Cookie,
        "query_param" | "query_param_token" => WsAuth::QueryParam,
        "first_message" | "auth_message" => WsAuth::FirstMessage,
        "header" => WsAuth::Header,
        "none" => WsAuth::None,
        _ => WsAuth::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_standard_websocket() {
        let html = r#"<script>const ws = new WebSocket("wss://api.example.com/stream");</script>"#;
        let endpoints = discover_ws_endpoints(html, &[], "example.com");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].url, "wss://api.example.com/stream");
        assert_eq!(endpoints[0].protocol, WsProtocol::Raw);
    }

    #[test]
    fn test_discover_socketio() {
        let js = r#"const socket = io.connect("https://realtime.example.com", {transports: ['websocket']});"#;
        let endpoints = discover_ws_endpoints("", &[js.to_string()], "example.com");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].protocol, WsProtocol::SocketIO);
        assert!(endpoints[0].url.starts_with("wss://"));
    }

    #[test]
    fn test_discover_sockjs() {
        let js = r#"var sock = new SockJS("/ws/notifications");"#;
        let endpoints = discover_ws_endpoints("", &[js.to_string()], "example.com");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].protocol, WsProtocol::SockJS);
    }

    #[test]
    fn test_discover_signalr() {
        let js = r#"
            const connection = new signalR.HubConnectionBuilder()
                .withUrl("/hubs/chat")
                .build();
        "#;
        let endpoints = discover_ws_endpoints("", &[js.to_string()], "example.com");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].protocol, WsProtocol::SignalR);
    }

    #[test]
    fn test_discover_known_platform() {
        let endpoints = discover_ws_endpoints("", &[], "slack.com");
        assert!(!endpoints.is_empty());
        assert!(endpoints[0].confidence >= 0.9);
    }

    #[test]
    fn test_empty_html() {
        let endpoints = discover_ws_endpoints("", &[], "unknown-domain.com");
        assert!(endpoints.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let html = r#"
            <script>new WebSocket("wss://api.example.com/ws");</script>
            <script>new WebSocket("wss://api.example.com/ws");</script>
        "#;
        let endpoints = discover_ws_endpoints(html, &[], "example.com");
        assert_eq!(endpoints.len(), 1); // Deduplicated
    }

    #[test]
    fn test_has_known_ws() {
        assert!(has_known_ws("slack.com"));
        assert!(has_known_ws("discord.com"));
        assert!(!has_known_ws("random-blog.com"));
    }
}
