//! Built-in Model Context Protocol (MCP) server.
//!
//! Mounted at `/api/mcp` using the Streamable HTTP transport in stateless
//! mode, so it works behind any number of replicas without a session store.
//! Requests authenticate exactly like the REST API: a session JWT or a
//! `readur_pat_` API key in `Authorization: Bearer`. Tools call the same
//! database and service code as the REST handlers, scoped to the caller.

pub mod format;
mod server;
pub mod tools;

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{FromRequestParts, Request, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use rmcp::transport::streamable_http_server::{
    session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

use crate::{auth::AuthUser, models::User, AppState};

pub use server::ReadurMcp;

/// The authenticated caller, stored in the request extensions so tools can
/// read it from the `http::request::Parts` rmcp hands them.
#[derive(Clone, Debug)]
pub struct McpCaller(pub User);

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", any(handle))
}

async fn handle(State(state): State<Arc<AppState>>, request: Request) -> Response {
    if !state.config.mcp.enabled {
        return StatusCode::NOT_FOUND.into_response();
    }

    let (mut parts, body) = request.into_parts();
    let user = match AuthUser::from_request_parts(&mut parts, &state).await {
        Ok(AuthUser { user }) => user,
        Err(rejection) => return rejection,
    };
    parts.extensions.insert(McpCaller(user));
    // rmcp rejects requests without a Host header even with host validation
    // off. HTTP/2 carries the authority in the URI instead.
    if !parts.headers.contains_key(header::HOST) {
        let authority = parts
            .uri
            .authority()
            .map(|a| a.as_str())
            .unwrap_or("localhost");
        if let Ok(value) = HeaderValue::from_str(authority) {
            parts.headers.insert(header::HOST, value);
        }
    }

    // Built per request on purpose: in stateless mode rmcp creates a fresh
    // `ReadurMcp` per request anyway, the costly part (tool router and
    // schemas) is cached in `ReadurMcp::tools`, and this wrapper measured
    // ~80ns -- noise next to the auth lookup above. Caching it would mean
    // threading `AppState` through request extensions instead of the service.
    let service = StreamableHttpService::new(
        {
            let state = state.clone();
            move || Ok(ReadurMcp::new(state.clone()))
        },
        Arc::new(NeverSessionManager::default()),
        service_config(&state),
    );
    service
        .handle(Request::from_parts(parts, body))
        .await
        .map(Body::new)
        .into_response()
}

/// The largest JSON-RPC body to accept, given the largest allowed file.
/// Uploads arrive base64-encoded (4/3 the size) and `upload_document`
/// accepts line-wrapped base64: MIME wrapping adds a CRLF per 76
/// characters, which JSON escapes to 4 bytes (`\r\n`), another 80/76.
/// Allow twice the file size plus 1 MiB for the envelope; the tool still
/// enforces the real limit on the decoded bytes.
pub(crate) fn max_request_body_bytes(max_file_bytes: usize) -> usize {
    max_file_bytes.saturating_mul(2).saturating_add(1024 * 1024)
}

fn service_config(state: &AppState) -> StreamableHttpServerConfig {
    let max_upload = state.config.max_file_size_mb as usize * 1024 * 1024;
    StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        // Host validation guards unauthenticated loopback servers against DNS
        // rebinding. Every request here needs a bearer credential, which a
        // browser never attaches on its own, and readur is usually reached
        // through a reverse proxy under an arbitrary hostname.
        .disable_allowed_hosts()
        .with_max_request_body_bytes(max_request_body_bytes(max_upload))
}

#[cfg(test)]
mod tests {
    use base64ct::{Base64, Encoding};
    use serde_json::json;

    use super::max_request_body_bytes;

    /// The JSON-RPC body an MCP client sends to upload `size` bytes as
    /// MIME-wrapped (76 characters + CRLF per line) base64.
    fn wrapped_upload_body(size: usize) -> usize {
        let encoded = Base64::encode_string(&vec![0xAB; size]);
        let wrapped: Vec<&str> = encoded
            .as_bytes()
            .chunks(76)
            .map(|line| std::str::from_utf8(line).unwrap())
            .collect();
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "upload_document", "arguments": {
                "filename": "scan.pdf", "content_base64": wrapped.join("\r\n")
            }}
        })
        .to_string()
        .len()
    }

    #[test]
    fn a_wrapped_upload_just_under_the_file_limit_fits_in_the_body_limit() {
        let max_file = 1024 * 1024;
        let body = wrapped_upload_body(max_file - 1);
        assert!(
            body <= max_request_body_bytes(max_file),
            "{body} > {}",
            max_request_body_bytes(max_file)
        );
    }

    #[test]
    fn the_body_limit_still_bounds_requests() {
        assert!(max_request_body_bytes(50 * 1024 * 1024) < 101 * 1024 * 1024 + 1);
        assert_eq!(max_request_body_bytes(usize::MAX), usize::MAX);
    }
}
