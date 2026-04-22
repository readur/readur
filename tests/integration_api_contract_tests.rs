/*!
 * API contract tests — drift detection between frontend and backend.
 *
 * Maintains an authoritative list of every API endpoint the frontend calls.
 * Two tests enforce invariants:
 *
 * 1. `no_dashed_paths_in_frontend_contract` — no hyphens in path segments
 *    (style rule: use slashes instead of dashes).
 *
 * 2. `every_frontend_endpoint_is_registered` — mounts the full backend
 *    router and sends an authenticated request to each path; asserts the
 *    response is neither 404 nor 405. Catches backend renames that
 *    forgot to update the list, and new frontend calls that forgot to
 *    register a matching route.
 *
 * When adding or renaming a frontend API call, add or edit the matching
 * entry here.
 */

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use tower::ServiceExt;
use uuid::Uuid;

use readur::test_utils::{TestAuthHelper, TestContext};
use readur::AppState;

/// Every `(method, path)` pair the frontend calls. Keep in sync with
/// `frontend/src/services/api.ts` and any direct `fetch`/`axios` calls
/// in pages.
const FRONTEND_ENDPOINTS: &[(&str, &str)] = &[
    // Auth & API keys
    ("POST",   "/api/auth/login"),
    ("GET",    "/api/auth/me"),
    ("POST",   "/api/auth/keys"),
    ("GET",    "/api/auth/keys"),
    ("DELETE", "/api/auth/keys/{id}"),
    // Documents
    ("GET",    "/api/documents"),
    ("GET",    "/api/documents/{id}"),
    ("GET",    "/api/documents/{id}/download"),
    ("GET",    "/api/documents/{id}/processed/image"),
    ("GET",    "/api/documents/failed/ocr"),
    ("DELETE", "/api/documents/cleanup/low/confidence"),
    ("DELETE", "/api/documents/cleanup/failed/ocr"),
    // OCR retry
    ("POST",   "/api/documents/ocr/retry/bulk"),
    ("GET",    "/api/documents/ocr/retry/stats"),
    ("GET",    "/api/documents/ocr/retry/recommendations"),
    ("GET",    "/api/documents/{id}/ocr/retry/history"),
    // Ignored files
    ("GET",    "/api/ignored/files"),
    ("GET",    "/api/ignored/files/stats"),
    ("DELETE", "/api/ignored/files/{id}"),
    ("DELETE", "/api/ignored/files/bulk/delete"),
    // Shared links
    ("POST",   "/api/shared/links"),
    ("GET",    "/api/shared/links"),
    ("DELETE", "/api/shared/links/{id}"),
    // Users watch directory
    ("GET",    "/api/users/{id}/watch/directory"),
    ("POST",   "/api/users/{id}/watch/directory"),
    ("DELETE", "/api/users/{id}/watch/directory"),
    // Sources
    ("POST",   "/api/sources/test/connection"),
    ("POST",   "/api/sources/{id}/scan/deep"),
    // WebDAV
    ("POST",   "/api/webdav/test/connection"),
    ("POST",   "/api/webdav/crawl/estimate"),
    ("GET",    "/api/webdav/sync/status"),
    ("POST",   "/api/webdav/sync/start"),
    ("POST",   "/api/webdav/sync/cancel"),
    // Scan failures (nested separately in main.rs)
    ("GET",    "/api/webdav/scan/failures"),
    ("GET",    "/api/webdav/scan/failures/retry/candidates"),
    ("GET",    "/api/webdav/scan/failures/{id}"),
    ("POST",   "/api/webdav/scan/failures/{id}/retry"),
    ("POST",   "/api/webdav/scan/failures/{id}/exclude"),
    // Queue
    ("POST",   "/api/queue/enqueue/pending"),
    // Notifications
    ("POST",   "/api/notifications/read/all"),
    // Source errors
    ("GET",    "/api/source/errors/retry/candidates"),
];

#[test]
fn no_dashed_paths_in_frontend_contract() {
    let dashed: Vec<String> = FRONTEND_ENDPOINTS
        .iter()
        .filter(|(_, p)| p.contains('-'))
        .map(|(m, p)| format!("{m} {p}"))
        .collect();
    assert!(
        dashed.is_empty(),
        "dashed paths found in contract (use slashes instead):\n{}",
        dashed.join("\n"),
    );
}

/// Build the full production router from a TestContext's state. Kept in sync
/// with `src/main.rs` — if you add a new `.nest(...)` there, mirror it here.
fn build_full_router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api/auth", readur::routes::auth::router())
        .nest("/api/documents", readur::routes::documents::router())
        .nest("/api/ignored/files", readur::routes::ignored_files::ignored_files_routes())
        .nest("/api/labels", readur::routes::labels::router())
        .nest("/api/metrics", readur::routes::metrics::router())
        .nest("/api/notifications", readur::routes::notifications::router())
        .nest("/api/ocr", readur::routes::ocr::router())
        .nest("/api/queue", readur::routes::queue::router())
        .nest("/api/search", readur::routes::search::router())
        .nest("/api/settings", readur::routes::settings::router())
        .nest("/api/source/errors", readur::routes::source_errors::router())
        .nest("/api/sources", readur::routes::sources::router())
        .nest("/api/users", readur::routes::users::router())
        .nest("/api/webdav", readur::routes::webdav::router())
        .nest("/api/webdav/scan/failures", readur::routes::webdav_scan_failures::router())
        .nest("/api/shared/links", readur::routes::shared_links::authenticated_router())
        .nest("/api/comments", readur::routes::comments::router())
        .with_state(state)
}

/// Probe each path with an obscure method (TRACE). Axum's router returns:
/// - `405 METHOD_NOT_ALLOWED` if the path is registered with any method
/// - `404 NOT_FOUND` if no route matches the path at all
///
/// This cleanly separates "route exists" from "handler returned 404 because
/// the resource is missing," which a naive same-method probe can't do.
#[tokio::test]
async fn every_frontend_endpoint_is_registered() {
    let ctx = TestContext::new().await;
    let app = build_full_router(ctx.state().clone());

    let mut missing = Vec::new();
    for (method, path) in FRONTEND_ENDPOINTS {
        let concrete = path.replace("{id}", &Uuid::new_v4().to_string());

        let probe = Request::builder()
            .method("TRACE")
            .uri(&concrete)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(probe).await.unwrap();
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            missing.push(format!("{} {} (TRACE probe: {})", method, path, status));
        }
    }

    assert!(
        missing.is_empty(),
        "The following frontend endpoints are not registered on the backend:\n{}",
        missing.join("\n"),
    );
}

/// Complement to `every_frontend_endpoint_is_registered`: also asserts the
/// *method* is one the route accepts. Sends the declared method with a valid
/// JWT and expects anything other than 405 Method Not Allowed. Handler-level
/// 404s (resource not found) are accepted as "route + method both OK."
#[tokio::test]
async fn every_frontend_endpoint_accepts_declared_method() {
    let ctx = TestContext::new().await;
    let auth = TestAuthHelper::new(ctx.app().clone());
    let user = auth.create_test_user().await;
    let jwt = auth.login_user(&user.username, &user.password).await;

    let app = build_full_router(ctx.state().clone());

    let mut wrong_method = Vec::new();
    for (method, path) in FRONTEND_ENDPOINTS {
        let concrete = path.replace("{id}", &Uuid::new_v4().to_string());

        let mut builder = Request::builder()
            .method(*method)
            .uri(&concrete)
            .header("Authorization", format!("Bearer {}", jwt));

        let body = if matches!(*method, "POST" | "PUT" | "PATCH" | "DELETE") {
            builder = builder.header("Content-Type", "application/json");
            Body::from("{}")
        } else {
            Body::empty()
        };

        let response = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
        let status = response.status();

        if status == StatusCode::METHOD_NOT_ALLOWED {
            wrong_method.push(format!("{} {}", method, path));
        }
    }

    assert!(
        wrong_method.is_empty(),
        "The following endpoints don't accept the stated HTTP method:\n{}",
        wrong_method.join("\n"),
    );
}
