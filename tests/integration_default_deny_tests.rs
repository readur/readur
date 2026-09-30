//! Every API route must reject unauthenticated requests unless it is on the
//! explicit public allowlist below.
//!
//! Routes come from the OpenAPI document plus a hand-maintained list of
//! routes that have no OpenAPI annotation. The set of mounted routers is
//! compared against a reviewed copy so adding a new mount fails this test
//! until its routes are accounted for here.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use readur::test_utils::TestContext;
use tower::ServiceExt;
use utoipa::OpenApi;

/// Routes reachable without credentials.
const PUBLIC_ROUTES: &[&str] = &[
    "/api/health",
    "/api/auth/login",
    "/api/auth/register",
    "/api/auth/config",
    "/api/auth/oidc/login",
    "/api/auth/oidc/callback",
    "/api/auth/oidc/exchange",
];
const PUBLIC_PREFIXES: &[&str] = &["/api/public/shared/"];

/// Reviewed copy of `readur::routes::mounted_routers()`.
const REVIEWED_MOUNTS: &[&str] = &[
    "/api/auth",
    "/api/documents",
    "/api/ignored/files",
    "/api/labels",
    "/api/metrics",
    "/metrics",
    "/api/notifications",
    "/api/ocr",
    "/api/queue",
    "/api/search",
    "/api/settings",
    "/api/source/errors",
    "/api/sources",
    "/api/users",
    "/api/webdav",
    "/api/webdav/scan/failures",
    "/api/shared/links",
    "/api/public/shared",
    "/api/comments",
];

/// Mounted routes without an OpenAPI annotation.
const UNDOCUMENTED_ROUTES: &[(&str, &str)] = &[
    ("POST", "/api/auth/keys"),
    ("GET", "/api/auth/keys"),
    ("DELETE", "/api/auth/keys/{id}"),
    ("GET", "/api/documents/ocr/stats"),
    ("POST", "/api/documents/{id}/ocr/stop"),
    ("GET", "/api/documents/{id}/validate"),
    ("GET", "/api/comments/documents/{document_id}/comments"),
    ("POST", "/api/comments/documents/{document_id}/comments"),
    ("GET", "/api/comments/documents/{document_id}/comments/{comment_id}/replies"),
    ("PUT", "/api/comments/documents/{document_id}/comments/{comment_id}"),
    ("DELETE", "/api/comments/documents/{document_id}/comments/{comment_id}"),
    ("GET", "/api/comments/documents/{document_id}/comments/count"),
    ("POST", "/api/shared/links"),
    ("GET", "/api/shared/links"),
    ("GET", "/api/shared/links/document/{document_id}"),
    ("DELETE", "/api/shared/links/{id}"),
    ("GET", "/api/public/shared/{token}"),
    ("POST", "/api/public/shared/{token}/verify"),
    ("POST", "/api/public/shared/{token}/download"),
    ("POST", "/api/public/shared/{token}/view"),
];

/// The WebSocket upgrade extractor rejects a request that cannot be upgraded
/// (as with an in-process `oneshot`) before the handler runs, so these return
/// 400/426 here. The handler authenticates before upgrading.
const WEBSOCKET_ROUTES: &[&str] = &["/api/sources/{id}/sync/progress/ws"];

fn is_public(path: &str) -> bool {
    PUBLIC_ROUTES.contains(&path) || PUBLIC_PREFIXES.iter().any(|p| path.starts_with(p))
}

/// Replace `{param}` segments with a random UUID, or a dummy token.
fn concrete_path(template: &str) -> String {
    template
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                if segment.contains("token") {
                    "not-a-real-token".to_string()
                } else {
                    uuid::Uuid::new_v4().to_string()
                }
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn documented_routes() -> Vec<(String, String)> {
    let api = readur::swagger::ApiDoc::openapi();
    let mut routes = Vec::new();
    for (path, item) in api.paths.paths.iter() {
        let ops = [
            ("GET", item.get.is_some()),
            ("POST", item.post.is_some()),
            ("PUT", item.put.is_some()),
            ("DELETE", item.delete.is_some()),
            ("PATCH", item.patch.is_some()),
        ];
        for (method, present) in ops {
            if present {
                routes.push((method.to_string(), path.clone()));
            }
        }
    }
    routes
}

fn all_routes() -> Vec<(String, String)> {
    let mut routes = documented_routes();
    routes.extend(UNDOCUMENTED_ROUTES.iter().map(|(m, p)| (m.to_string(), p.to_string())));
    routes.sort();
    routes.dedup();
    routes
}

#[test]
fn mounted_routers_match_reviewed_list() {
    let mounted: Vec<&str> = readur::routes::mounted_routers().into_iter().map(|(path, _)| path).collect();
    assert_eq!(
        mounted, REVIEWED_MOUNTS,
        "Mounted routers changed. Add the new routes to this test (public allowlist or \
         UNDOCUMENTED_ROUTES as appropriate) and update REVIEWED_MOUNTS."
    );

    let routes = all_routes();
    for mount in REVIEWED_MOUNTS {
        assert!(
            routes.iter().any(|(_, path)| path == mount || path.starts_with(&format!("{}/", mount))),
            "No route under {} is checked by this test",
            mount
        );
    }
}

#[tokio::test]
async fn every_route_requires_authentication_unless_public() {
    let ctx = TestContext::new().await;
    let app = ctx.app().clone();

    let mut failures = Vec::new();
    let mut checked = 0;

    for (method, template) in all_routes() {
        if is_public(&template) {
            continue;
        }
        let uri = concrete_path(&template);
        let request = Request::builder()
            .method(Method::from_bytes(method.as_bytes()).unwrap())
            .uri(&uri)
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let status = app.clone().oneshot(request).await.unwrap().status();
        checked += 1;

        let allowed = if WEBSOCKET_ROUTES.contains(&template.as_str()) {
            matches!(status, StatusCode::UNAUTHORIZED | StatusCode::BAD_REQUEST | StatusCode::UPGRADE_REQUIRED)
        } else {
            status == StatusCode::UNAUTHORIZED
        };
        if !allowed {
            failures.push(format!("{} {} -> {}", method, template, status));
        }
    }

    let _ = ctx.cleanup_and_close().await;
    assert!(checked > 100, "Expected to check the whole API, only checked {}", checked);
    assert!(failures.is_empty(), "Routes not rejecting unauthenticated requests with 401:\n{}", failures.join("\n"));
}

#[tokio::test]
async fn invalid_credentials_are_rejected() {
    let ctx = TestContext::new().await;
    let app = ctx.app().clone();

    for header in ["Bearer not-a-jwt", "Bearer readur_pat_notarealkey", "Basic YWRtaW46YWRtaW4="] {
        for uri in ["/api/auth/me", "/api/documents", "/metrics"] {
            let request = Request::builder()
                .uri(uri)
                .header("authorization", header)
                .body(Body::empty())
                .unwrap();
            let status = app.clone().oneshot(request).await.unwrap().status();
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{} with '{}'", uri, header);
        }
    }

    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn metrics_requires_authentication() {
    let ctx = TestContext::new().await;
    let request = Request::builder().uri("/metrics").body(Body::empty()).unwrap();
    let status = ctx.app().clone().oneshot(request).await.unwrap().status();
    let _ = ctx.cleanup_and_close().await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn removed_ocr_endpoint_is_gone() {
    let ctx = TestContext::new().await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/ocr/perform")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"file_path":"/etc/hostname"}"#))
        .unwrap();
    let status = ctx.app().clone().oneshot(request).await.unwrap().status();
    let _ = ctx.cleanup_and_close().await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED,
        "unexpected status {}",
        status
    );
}
