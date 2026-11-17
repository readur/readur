//! The WebSocket that streams a source's sync progress, with the origin and
//! token checks of its handshake.

use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Path, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

use crate::{utils::client_ip::PeerIsTrustedProxy, AppState};

/// WebSocket endpoint for real-time sync progress updates
///
/// This endpoint provides real-time updates about source synchronization progress via WebSocket.
/// It sends progress messages every second during active sync operations and heartbeat messages
/// when no sync is running. This replaces the previous Server-Sent Events (SSE) implementation
/// with improved security by using query parameter authentication instead of exposing JWT tokens.
///
/// # Message Types
/// - `progress`: Real-time sync progress updates with detailed statistics
/// - `heartbeat`: Keep-alive messages when no sync is active
/// - `error`: Error messages for connection or sync issues
/// - `connection_confirmed`: Confirmation that the WebSocket connection is established
///
/// # Security
/// Authentication is handled via JWT token in the `Sec-WebSocket-Protocol` header during WebSocket handshake.
/// This secure approach prevents token exposure in logs, browser history, and referrer headers.
#[utoipa::path(
    get,
    path = "/api/sources/{id}/sync/progress/ws",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID to monitor for sync progress")
    ),
    responses(
        (status = 101, description = "WebSocket connection established - will stream real-time progress updates"),
        (status = 401, description = "Unauthorized - invalid or missing authentication token"),
        (status = 404, description = "Source not found or user does not have access"),
        (status = 500, description = "Internal server error during WebSocket upgrade")
    )
)]
pub async fn sync_progress_websocket(
    ws: WebSocketUpgrade,
    Path(source_id): Path<Uuid>,
    headers: HeaderMap,
    PeerIsTrustedProxy(via_trusted_proxy): PeerIsTrustedProxy,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    // WebSocket handshakes are not subject to CORS, so the origin is
    // checked explicitly.
    if !websocket_origin_allowed(&headers, via_trusted_proxy, &state.config) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Token travels in Sec-WebSocket-Protocol (`bearer.<token>`) because
    // browsers cannot set an Authorization header on WebSocket requests.
    let (token, auth_protocol) =
        extract_websocket_token_and_protocol(&headers).ok_or(StatusCode::UNAUTHORIZED)?;

    // Same checks as every other endpoint: API keys, revocation, account state.
    let user = crate::auth::authenticate_token(&token, &state)
        .await
        .map_err(|resp| resp.status())?;

    // Verify the source exists and the user has access
    let _source = state
        .db
        .get_source(user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Upgrade the connection to WebSocket with protocol acknowledgment
    Ok(ws
        .protocols([auth_protocol.clone()])
        .on_upgrade(move |socket| handle_websocket(socket, source_id, state)))
}

/// Handle WebSocket connection for sync progress updates
async fn handle_websocket(mut socket: WebSocket, source_id: Uuid, state: Arc<AppState>) {
    info!("WebSocket connection established for source {}", source_id);

    // Send connection confirmation
    let confirmation_msg = serde_json::json!({
        "type": "connection_confirmed",
        "data": {
            "source_id": source_id,
            "timestamp": chrono::Utc::now().timestamp()
        }
    });

    if let Err(e) = socket
        .send(Message::Text(confirmation_msg.to_string().into()))
        .await
    {
        error!(
            "Failed to send connection confirmation for source {}: {}",
            source_id, e
        );
        return;
    }

    let progress_tracker = state.sync_progress_tracker.clone();

    loop {
        // Check for progress update
        let progress_info = progress_tracker.get_progress(source_id);

        let message = match progress_info {
            Some(info) => {
                // Send current progress
                match serde_json::to_string(&serde_json::json!({
                    "type": "progress",
                    "data": info
                })) {
                    Ok(json) => Message::Text(json.into()),
                    Err(e) => {
                        error!("Failed to serialize progress info: {}", e);
                        let error_msg = serde_json::json!({
                            "type": "error",
                            "data": {
                                "message": format!("Failed to serialize progress: {}", e),
                                "error_type": "serialization_error"
                            }
                        });
                        Message::Text(error_msg.to_string().into())
                    }
                }
            }
            None => {
                // No active sync, send a heartbeat
                Message::Text(
                    serde_json::json!({
                        "type": "heartbeat",
                        "data": {
                            "source_id": source_id,
                            "is_active": false,
                            "timestamp": chrono::Utc::now().timestamp()
                        }
                    })
                    .to_string()
                    .into(),
                )
            }
        };

        // Send the message to the client
        if let Err(e) = socket.send(message).await {
            error!(
                "Failed to send WebSocket message for source {}: {}",
                source_id, e
            );

            // Try to send error notification to client before breaking
            let error_notification = serde_json::json!({
                "type": "error",
                "data": {
                    "message": "Connection error occurred, closing connection",
                    "error_type": "connection_error",
                    "details": e.to_string()
                }
            });

            // Attempt to send error message (ignore if this fails too)
            let _ = socket
                .send(Message::Text(error_notification.to_string().into()))
                .await;
            break;
        }

        // Wait before next update
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Check if the connection is still alive by trying to send a ping
        if let Err(e) = socket.send(Message::Ping(vec![].into())).await {
            info!(
                "WebSocket connection closed for source {} (ping failed: {})",
                source_id, e
            );

            // Try to send graceful closure message
            let closure_msg = serde_json::json!({
                "type": "error",
                "data": {
                    "message": "Connection lost during ping check",
                    "error_type": "ping_failed",
                    "details": e.to_string()
                }
            });

            // Attempt to send closure message (ignore if this fails)
            let _ = socket
                .send(Message::Text(closure_msg.to_string().into()))
                .await;
            break;
        }
    }

    // Send final close message if connection is still open
    let close_msg = serde_json::json!({
        "type": "connection_closing",
        "data": {
            "source_id": source_id,
            "message": "Server is closing connection",
            "timestamp": chrono::Utc::now().timestamp()
        }
    });

    // Try to send close notification (ignore failures)
    let _ = socket
        .send(Message::Text(close_msg.to_string().into()))
        .await;

    info!("WebSocket connection terminated for source {}", source_id);
}

/// Decide whether a WebSocket handshake's `Origin` is acceptable.
///
/// Non-browser clients usually send no `Origin`; those are allowed because
/// they must still present a valid token. When an `Origin` is present it must
/// match the request's own `Host`, the configured `PUBLIC_URL`, or one of
/// `CORS_ALLOWED_ORIGINS` -- or, for requests relayed by a trusted proxy, the
/// host the proxy reports in `X-Forwarded-Host` (with `X-Forwarded-Proto`).
fn websocket_origin_allowed(
    headers: &HeaderMap,
    via_trusted_proxy: bool,
    config: &crate::config::Config,
) -> bool {
    let Some(origin) = headers.get(axum::http::header::ORIGIN) else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let origin = origin.trim_end_matches('/');

    // Compare the origin's host[:port] with the Host header.
    let origin_authority = origin.split_once("://").map(|(_, rest)| rest);
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok());
    if let (Some(authority), Some(host)) = (origin_authority, host) {
        if authority.eq_ignore_ascii_case(host) {
            return true;
        }
    }

    if via_trusted_proxy && matches_forwarded_host(origin, headers) {
        return true;
    }

    let public_origin = config.public_url.as_deref().and_then(url_origin);
    if public_origin.is_some_and(|p| p.eq_ignore_ascii_case(origin)) {
        return true;
    }

    config
        .security
        .cors_allowed_origins
        .iter()
        .any(|allowed| allowed.trim_end_matches('/').eq_ignore_ascii_case(origin))
}

/// Whether `origin` is the public address a proxy reports in
/// `X-Forwarded-Host`, with the scheme from `X-Forwarded-Proto` when given.
/// Only meaningful for requests from a trusted proxy.
fn matches_forwarded_host(origin: &str, headers: &HeaderMap) -> bool {
    let first_entry = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    let Some(host) = first_entry("x-forwarded-host") else {
        return false;
    };
    match (first_entry("x-forwarded-proto"), origin.split_once("://")) {
        (Some(proto), _) => origin.eq_ignore_ascii_case(&format!("{}://{}", proto, host)),
        (None, Some((_, authority))) => authority.eq_ignore_ascii_case(host),
        (None, None) => false,
    }
}

/// `scheme://host[:port]` of a URL, without path.
fn url_origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split('/').next()?;
    if authority.is_empty() {
        return None;
    }
    Some(format!("{}://{}", scheme, authority))
}

/// Extract JWT token and protocol string from WebSocket headers. The token travels in the
/// Sec-WebSocket-Protocol header to avoid exposing it in logs/URLs.
/// Returns both the token and the original protocol for handshake acknowledgment
fn extract_websocket_token_and_protocol(headers: &HeaderMap) -> Option<(String, String)> {
    // Check for token in Sec-WebSocket-Protocol header (most secure)
    if let Some(protocol_header) = headers.get("sec-websocket-protocol") {
        if let Ok(protocols) = protocol_header.to_str() {
            // Format: "bearer.{token}" or "bearer, {token}"
            for protocol in protocols.split(',') {
                let protocol = protocol.trim();
                if protocol.starts_with("bearer.") {
                    let token = protocol.trim_start_matches("bearer.").to_string();
                    return Some((token, protocol.to_string()));
                }
                if protocol.starts_with("bearer ") {
                    let token = protocol.trim_start_matches("bearer ").to_string();
                    return Some((token, protocol.to_string()));
                }
            }
        }
    }

    // Fallback to Authorization header for backward compatibility
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = auth_str.trim_start_matches("Bearer ").to_string();
                // For Authorization header fallback, create a protocol format
                return Some((token.clone(), format!("bearer.{}", token)));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(origin: Option<&str>, host: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("host", host.parse().unwrap());
        if let Some(origin) = origin {
            h.insert("origin", origin.parse().unwrap());
        }
        h
    }

    #[test]
    fn websocket_origin_checks() {
        let mut config = crate::test_helpers::create_test_config();
        config.public_url = Some("https://docs.example.com/app".to_string());
        config.security.cors_allowed_origins = vec!["http://localhost:5173".to_string()];

        assert!(websocket_origin_allowed(
            &headers(None, "internal:8000"),
            false,
            &config
        ));
        assert!(websocket_origin_allowed(
            &headers(Some("http://internal:8000"), "internal:8000"),
            false,
            &config
        ));
        assert!(websocket_origin_allowed(
            &headers(Some("https://docs.example.com"), "internal:8000"),
            false,
            &config
        ));
        assert!(websocket_origin_allowed(
            &headers(Some("http://localhost:5173"), "internal:8000"),
            false,
            &config
        ));
        assert!(!websocket_origin_allowed(
            &headers(Some("https://other.example"), "internal:8000"),
            false,
            &config
        ));
        assert!(!websocket_origin_allowed(
            &headers(Some("null"), "internal:8000"),
            false,
            &config
        ));
    }

    fn forwarded(origin: &str, host: &str, proto: Option<&str>) -> HeaderMap {
        let mut h = headers(Some(origin), "readur:8000");
        h.insert("x-forwarded-host", host.parse().unwrap());
        if let Some(proto) = proto {
            h.insert("x-forwarded-proto", proto.parse().unwrap());
        }
        h
    }

    #[test]
    fn websocket_origin_matches_forwarded_host_only_from_trusted_proxies() {
        let config = crate::test_helpers::create_test_config();
        let behind_proxy = forwarded(
            "https://readur.example.com",
            "readur.example.com",
            Some("https"),
        );
        assert!(websocket_origin_allowed(&behind_proxy, true, &config));
        // The same headers from a peer that is not a trusted proxy are ignored.
        assert!(!websocket_origin_allowed(&behind_proxy, false, &config));

        // Without X-Forwarded-Proto the host[:port] alone is compared.
        let no_proto = forwarded(
            "https://readur.example.com:8443",
            "readur.example.com:8443",
            None,
        );
        assert!(websocket_origin_allowed(&no_proto, true, &config));
        // The scheme must match when the proxy reports one.
        let wrong_scheme = forwarded(
            "http://readur.example.com",
            "readur.example.com",
            Some("https"),
        );
        assert!(!websocket_origin_allowed(&wrong_scheme, true, &config));
        // A different host is still refused.
        let other = forwarded("https://other.example", "readur.example.com", Some("https"));
        assert!(!websocket_origin_allowed(&other, true, &config));
        // The first entry of a proxy chain is the public address.
        let chain = forwarded(
            "https://readur.example.com",
            "readur.example.com, internal:8000",
            Some("https, http"),
        );
        assert!(websocket_origin_allowed(&chain, true, &config));
        // Without X-Forwarded-Host nothing extra is accepted.
        let mut no_host = headers(Some("https://readur.example.com"), "readur:8000");
        no_host.insert("x-forwarded-proto", "https".parse().unwrap());
        assert!(!websocket_origin_allowed(&no_host, true, &config));
    }
}
