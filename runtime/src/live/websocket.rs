//! Native WebSocket client for real-time page interaction.
//!
//! Provides a thin wrapper around `tokio-tungstenite` for opening, sending,
//! receiving, and closing WebSocket connections discovered by
//! [`crate::acquisition::ws_discovery`]. This avoids spinning up a browser
//! just to interact with sites that use WebSockets for their primary data
//! transport (Slack, Discord, real-time dashboards, etc.).
//!
//! ## Execution priority
//!
//! In the action execution stack, WebSocket sits at priority 4
//! (after WebMCP, Platform API, and HTTP Action).

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;

// Re-export discovery types for convenience.
pub use crate::acquisition::ws_discovery::{WsAuth, WsEndpoint, WsProtocol};

/// An active WebSocket session.
///
/// Wraps a `tokio-tungstenite` connection with session metadata
/// (cookies, auth tokens, protocol details).
pub struct WsSession {
    /// The WebSocket URL this session is connected to.
    pub url: String,
    /// The protocol used (Raw, Socket.IO, SockJS, SignalR).
    pub protocol: WsProtocol,
    /// Domain of the connected site.
    pub domain: String,
    /// Whether the connection is currently open.
    connected: bool,
    /// Message history (most recent messages, bounded).
    messages: Vec<WsMessage>,
    /// Maximum messages to keep in history.
    max_history: usize,
    /// Internal sink/stream — wrapped in Mutex for interior mutability.
    _inner: Mutex<Option<WsInner>>,
}

/// Internal WebSocket connection state.
struct WsInner {
    /// The underlying tungstenite write half.
    sink: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    /// The underlying tungstenite read half.
    stream: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
}

/// A WebSocket message received or sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// Direction of the message.
    pub direction: WsDirection,
    /// Message payload (text or JSON string).
    pub payload: String,
    /// Timestamp (milliseconds since session start).
    pub timestamp_ms: u64,
}

/// Direction of a WebSocket message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WsDirection {
    /// Sent from client to server.
    Sent,
    /// Received from server.
    Received,
}

impl WsSession {
    /// Connect to a WebSocket endpoint.
    ///
    /// Builds the connection URL and optional headers (cookies, auth tokens),
    /// then opens the WebSocket connection.
    pub async fn connect(endpoint: &WsEndpoint, cookies: &HashMap<String, String>) -> Result<Self> {
        use futures::StreamExt;
        use tokio_tungstenite::tungstenite::http::Request;

        // Build the WebSocket URL.
        let ws_url = &endpoint.url;

        // Build the HTTP request with cookies/auth headers.
        let mut request_builder = Request::builder().uri(ws_url);

        // Add cookies as a Cookie header.
        if !cookies.is_empty() {
            let cookie_str: String = cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");
            request_builder = request_builder.header("Cookie", cookie_str);
        }

        // Add origin header (many WS servers require it).
        let origin = if let Ok(parsed) = url::Url::parse(ws_url) {
            format!(
                "{}://{}",
                if parsed.scheme() == "wss" {
                    "https"
                } else {
                    "http"
                },
                parsed.host_str().unwrap_or("localhost")
            )
        } else {
            "https://localhost".to_string()
        };
        request_builder = request_builder.header("Origin", &origin);

        let request = request_builder
            .body(())
            .map_err(|e| anyhow::anyhow!("failed to build WS request: {e}"))?;

        // Connect.
        let (ws_stream, _response) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {e}"))?;

        let (sink, stream) = ws_stream.split();

        let domain = url::Url::parse(ws_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_default();

        Ok(WsSession {
            url: ws_url.clone(),
            protocol: endpoint.protocol.clone(),
            domain,
            connected: true,
            messages: Vec::new(),
            max_history: 1000,
            _inner: Mutex::new(Some(WsInner { sink, stream })),
        })
    }

    /// Send a JSON-serializable message over the WebSocket.
    ///
    /// For Socket.IO, wraps the message in the appropriate frame format.
    /// For raw WebSocket, sends as-is.
    pub async fn send_json<T: Serialize>(&mut self, msg: &T) -> Result<()> {
        use futures::SinkExt;
        use tokio_tungstenite::tungstenite::Message;

        if !self.connected {
            bail!("WebSocket is not connected");
        }

        let payload = serde_json::to_string(msg)?;

        // Wrap for Socket.IO if needed.
        let wire_payload = match &self.protocol {
            WsProtocol::SocketIO => format!("42{payload}"),
            _ => payload.clone(),
        };

        let mut inner_guard = self._inner.lock().await;
        if let Some(inner) = inner_guard.as_mut() {
            inner
                .sink
                .send(Message::Text(wire_payload))
                .await
                .map_err(|e| anyhow::anyhow!("failed to send WS message: {e}"))?;
        } else {
            bail!("WebSocket connection not available");
        }
        drop(inner_guard);

        self.messages.push(WsMessage {
            direction: WsDirection::Sent,
            payload,
            timestamp_ms: 0, // Caller can set real timestamps.
        });

        // Trim history.
        if self.messages.len() > self.max_history {
            let drain = self.messages.len() - self.max_history;
            self.messages.drain(..drain);
        }

        Ok(())
    }

    /// Receive the next message from the WebSocket.
    ///
    /// Returns `None` if the connection is closed. Automatically skips
    /// ping/pong control frames and returns the next data message.
    pub async fn receive(&mut self) -> Result<Option<String>> {
        use futures::StreamExt;
        use tokio_tungstenite::tungstenite::Message;

        loop {
            if !self.connected {
                return Ok(None);
            }

            let mut inner_guard = self._inner.lock().await;
            let inner = match inner_guard.as_mut() {
                Some(i) => i,
                None => return Ok(None),
            };

            match inner.stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    // Unwrap Socket.IO frame if needed.
                    let payload = match &self.protocol {
                        WsProtocol::SocketIO => text
                            .strip_prefix("42")
                            .map(|s| s.to_string())
                            .unwrap_or(text),
                        _ => text,
                    };

                    drop(inner_guard);

                    self.messages.push(WsMessage {
                        direction: WsDirection::Received,
                        payload: payload.clone(),
                        timestamp_ms: 0,
                    });

                    if self.messages.len() > self.max_history {
                        let drain = self.messages.len() - self.max_history;
                        self.messages.drain(..drain);
                    }

                    return Ok(Some(payload));
                }
                Some(Ok(Message::Binary(data))) => {
                    drop(inner_guard);
                    return Ok(Some(format!("[binary: {} bytes]", data.len())));
                }
                Some(Ok(Message::Close(_))) => {
                    drop(inner_guard);
                    self.connected = false;
                    return Ok(None);
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {
                    // Control frames — skip and loop for next real message.
                    drop(inner_guard);
                    continue;
                }
                Some(Err(e)) => {
                    drop(inner_guard);
                    self.connected = false;
                    bail!("WebSocket error: {e}");
                }
                None => {
                    drop(inner_guard);
                    self.connected = false;
                    return Ok(None);
                }
            }
        }
    }

    /// Receive messages for a given duration, returning all collected messages.
    pub async fn watch(&mut self, duration_ms: u64) -> Result<Vec<WsMessage>> {
        let mut collected = Vec::new();
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_millis(duration_ms);

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, self.receive()).await {
                Ok(Ok(Some(payload))) => {
                    collected.push(WsMessage {
                        direction: WsDirection::Received,
                        payload,
                        timestamp_ms: 0,
                    });
                }
                Ok(Ok(None)) => break, // Connection closed.
                Ok(Err(_)) => break,   // Error.
                Err(_) => break,       // Timeout reached.
            }
        }

        Ok(collected)
    }

    /// Close the WebSocket connection gracefully.
    pub async fn close(&mut self) -> Result<()> {
        use futures::SinkExt;
        use tokio_tungstenite::tungstenite::Message;

        if !self.connected {
            return Ok(());
        }

        let mut inner_guard = self._inner.lock().await;
        if let Some(inner) = inner_guard.as_mut() {
            inner.sink.send(Message::Close(None)).await.ok();
        }
        *inner_guard = None;
        drop(inner_guard);

        self.connected = false;
        Ok(())
    }

    /// Whether the connection is currently open.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get the message history.
    pub fn history(&self) -> &[WsMessage] {
        &self.messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_serde() {
        let msg = WsMessage {
            direction: WsDirection::Received,
            payload: r#"{"type":"update","data":42}"#.to_string(),
            timestamp_ms: 12345,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.direction, WsDirection::Received);
        assert_eq!(parsed.timestamp_ms, 12345);
        assert!(parsed.payload.contains("update"));
    }

    #[test]
    fn test_ws_direction_eq() {
        assert_eq!(WsDirection::Sent, WsDirection::Sent);
        assert_ne!(WsDirection::Sent, WsDirection::Received);
    }
}
