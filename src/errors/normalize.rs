//! Middleware that gives every API error response the [`ApiErrorBody`]
//! shape.
//!
//! Handlers should build their errors through an `AppError` type or
//! [`super::error_response`]; this layer is the backstop for the responses
//! that do not: a bare `StatusCode`, an axum extractor rejection (plain-text
//! "Failed to deserialize the JSON body..."), the router's 405, or a JSON body
//! in some other shape. Such a body is replaced by an [`ApiErrorBody`] that
//! keeps the status, the headers (`Retry-After`, `WWW-Authenticate`, ...) and,
//! for a client error, the original message.
//!
//! Server errors (5xx) that were not built as an [`ApiErrorBody`] get the
//! status's reason phrase instead of their text, which may hold internal
//! detail. JSON-RPC bodies (the MCP endpoint) are left alone, since MCP
//! clients parse those.

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderValue, StatusCode},
    response::Response,
};
use serde_json::Value;

use super::{status_code_name, ApiErrorBody};

/// Error bodies larger than this are not inspected; they are replaced.
const MAX_INSPECTED_BODY: usize = 64 * 1024;

/// `axum::middleware::map_response` function: rewrites a non-conforming
/// error response into the shared shape.
pub async fn normalize_error_response(response: Response) -> Response {
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, MAX_INSPECTED_BODY)
        .await
        .unwrap_or_default();
    let normalized = match normalized_body(status, &bytes) {
        Some(normalized) => normalized,
        // Already conforming (or JSON-RPC): send the original bytes back.
        None => return Response::from_parts(parts, Body::from(bytes)),
    };
    let json = serde_json::to_vec(&normalized).expect("ApiErrorBody serializes");
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Response::from_parts(parts, Body::from(json))
}

/// The body to send instead of `bytes`, or `None` to keep them.
pub fn normalized_body(status: StatusCode, bytes: &Bytes) -> Option<ApiErrorBody> {
    let json: Option<Value> = serde_json::from_slice(bytes).ok();
    if let Some(Value::Object(object)) = &json {
        if object.contains_key("jsonrpc") {
            return None;
        }
        if let Ok(body) = serde_json::from_value::<ApiErrorBody>(Value::Object(object.clone())) {
            if body.status == status.as_u16() && !body.code.is_empty() {
                return None;
            }
        }
    }

    let text = |key: &str| -> Option<String> {
        json.as_ref()?
            .get(key)?
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let code = text("code")
        .or_else(|| text("error_code"))
        .unwrap_or_else(|| status_code_name(status));
    let original_message = match &json {
        Some(_) => text("error").or_else(|| text("message")),
        None => std::str::from_utf8(bytes)
            .ok()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    };
    let reason = status.canonical_reason().unwrap_or("Error").to_string();
    let error = if status.is_server_error() {
        reason
    } else {
        original_message.unwrap_or(reason)
    };
    Some(ApiErrorBody {
        error,
        code,
        status: status.as_u16(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn body(status: StatusCode, raw: &str) -> Option<ApiErrorBody> {
        normalized_body(status, &Bytes::from(raw.to_string()))
    }

    fn expect(error: &str, code: &str, status: u16) -> Option<ApiErrorBody> {
        Some(ApiErrorBody {
            error: error.into(),
            code: code.into(),
            status,
        })
    }

    #[test]
    fn conforming_bodies_are_kept() {
        let raw = r#"{"error":"User not found","code":"USER_NOT_FOUND","status":404}"#;
        assert_eq!(body(StatusCode::NOT_FOUND, raw), None);
        // Extra fields do not matter.
        let raw = r#"{"error":"x","code":"X","status":400,"extra":1}"#;
        assert_eq!(body(StatusCode::BAD_REQUEST, raw), None);
    }

    #[test]
    fn empty_and_text_bodies_get_the_shape() {
        assert_eq!(
            body(StatusCode::NOT_FOUND, ""),
            expect("Not Found", "NOT_FOUND", 404)
        );
        assert_eq!(
            body(StatusCode::UNAUTHORIZED, "Missing authorization header"),
            expect("Missing authorization header", "UNAUTHORIZED", 401)
        );
        assert_eq!(
            body(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Failed to deserialize the JSON body"
            ),
            expect(
                "Failed to deserialize the JSON body",
                "UNPROCESSABLE_ENTITY",
                422
            )
        );
    }

    #[test]
    fn other_json_shapes_are_converted() {
        assert_eq!(
            body(StatusCode::BAD_REQUEST, r#"{"error":"Bad name"}"#),
            expect("Bad name", "BAD_REQUEST", 400)
        );
        assert_eq!(
            body(
                StatusCode::CONFLICT,
                r#"{"error":"Taken","error_code":"UPLOAD_CONFLICT","status":409}"#
            ),
            expect("Taken", "UPLOAD_CONFLICT", 409)
        );
        assert_eq!(
            body(
                StatusCode::BAD_REQUEST,
                r#"{"success":false,"message":"Nope"}"#
            ),
            expect("Nope", "BAD_REQUEST", 400)
        );
        // A status that disagrees with the response is corrected.
        assert_eq!(
            body(
                StatusCode::NOT_FOUND,
                r#"{"error":"Gone","code":"GONE","status":410}"#
            ),
            expect("Gone", "GONE", 404)
        );
    }

    #[test]
    fn server_error_text_is_not_passed_on() {
        assert_eq!(
            body(
                StatusCode::INTERNAL_SERVER_ERROR,
                "relation \"users\" does not exist"
            ),
            expect("Internal Server Error", "INTERNAL_SERVER_ERROR", 500)
        );
        assert_eq!(
            body(
                StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"error":"/var/lib/readur: EACCES"}"#
            ),
            expect("Internal Server Error", "INTERNAL_SERVER_ERROR", 500)
        );
    }

    #[test]
    fn json_rpc_bodies_are_kept() {
        let raw = r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid"},"id":null}"#;
        assert_eq!(body(StatusCode::BAD_REQUEST, raw), None);
    }

    #[tokio::test]
    async fn middleware_keeps_status_and_headers() {
        let mut response = (StatusCode::TOO_MANY_REQUESTS, "slow down").into_response();
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("30"));
        let response = normalize_error_response(response).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "30");
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ApiErrorBody = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, expect("slow down", "TOO_MANY_REQUESTS", 429).unwrap());
    }

    #[tokio::test]
    async fn middleware_leaves_success_responses_alone() {
        let response = normalize_error_response("fine".into_response()).await;
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&bytes[..], b"fine");
    }

    #[test]
    fn status_code_names() {
        assert_eq!(status_code_name(StatusCode::NOT_FOUND), "NOT_FOUND");
        assert_eq!(
            status_code_name(StatusCode::PAYLOAD_TOO_LARGE),
            "PAYLOAD_TOO_LARGE"
        );
        assert_eq!(
            status_code_name(StatusCode::NOT_IMPLEMENTED),
            "NOT_IMPLEMENTED"
        );
        assert_eq!(
            status_code_name(StatusCode::from_u16(599).unwrap()),
            "HTTP_599"
        );
    }
}
