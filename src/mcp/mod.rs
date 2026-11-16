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

fn service_config(state: &AppState) -> StreamableHttpServerConfig {
    // Uploads arrive base64-encoded inside the JSON-RPC body (4/3 the file
    // size), plus headroom for the envelope.
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
        .with_max_request_body_bytes(max_upload / 3 * 4 + 1024 * 1024)
}
