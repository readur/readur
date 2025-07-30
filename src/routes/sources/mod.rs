use axum::{routing::{get, post, delete, put}, Router};
use std::sync::Arc;
use crate::AppState;

pub mod crud;
pub mod sync;
pub mod validation;
pub mod estimation;

// Re-export commonly used functions and types for backward compatibility
pub use crud::*;
pub use sync::*;
pub use validation::*;
pub use estimation::*;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // CRUD operations
        .route("/", get(list_sources))
        .route("/", post(create_source))
        .route("/{id}", get(get_source))
        .route("/{id}", put(update_source))
        .route("/{id}", delete(delete_source))
        
        // Sync operations
        .route("/{id}/sync", post(trigger_sync))
        .route("/{id}/sync/stop", post(stop_sync))
        .route("/{id}/sync/progress/ws", get(sync_progress_websocket))
        .route("/{id}/sync/status", get(get_sync_status))
        .route("/{id}/deep-scan", post(trigger_deep_scan))
        
        // Validation operations
        .route("/{id}/validate", post(validate_source))
        .route("/test", post(test_connection_with_config))
        
        // Estimation operations
        .route("/{id}/estimate", get(estimate_crawl))
        .route("/estimate", post(estimate_crawl_with_config))
}