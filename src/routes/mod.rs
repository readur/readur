pub mod auth;
pub mod documents;
pub mod documents_ocr_retry;
pub mod ignored_files;
pub mod labels;
pub mod metrics;
pub mod notifications;
pub mod ocr;
pub mod prometheus_metrics;
pub mod queue;
pub mod search;
pub mod settings;
pub mod source_errors;
pub mod sources;
pub mod users;
pub mod webdav;
pub mod webdav_scan_failures;
pub mod shared_links;
pub mod comments;
pub mod api_keys;

use std::sync::Arc;

use axum::{routing::get, Router};

use crate::AppState;

/// Every router the server mounts, with its mount point. `api_router` is
/// built from this list, and the default-deny test checks it against a
/// reviewed copy, so a new mount cannot be added without that review.
pub fn mounted_routers() -> Vec<(&'static str, Router<Arc<AppState>>)> {
    vec![
        ("/api/auth", auth::router()),
        ("/api/documents", documents::router()),
        ("/api/ignored/files", ignored_files::ignored_files_routes()),
        ("/api/labels", labels::router()),
        ("/api/metrics", metrics::router()),
        ("/metrics", prometheus_metrics::router()),
        ("/api/notifications", notifications::router()),
        ("/api/ocr", ocr::router()),
        ("/api/queue", queue::router()),
        ("/api/search", search::router()),
        ("/api/settings", settings::router()),
        ("/api/source/errors", source_errors::router()),
        ("/api/sources", sources::router()),
        ("/api/users", users::router()),
        ("/api/webdav", webdav::router()),
        ("/api/webdav/scan/failures", webdav_scan_failures::router()),
        ("/api/shared/links", shared_links::authenticated_router()),
        ("/api/public/shared", shared_links::public_router()),
        ("/api/comments", comments::router()),
    ]
}

/// The complete HTTP API (without API docs and static frontend files).
pub fn api_router() -> Router<Arc<AppState>> {
    mounted_routers()
        .into_iter()
        .fold(Router::new().route("/api/health", get(crate::health_check)), |app, (path, router)| {
            app.nest(path, router)
        })
}
