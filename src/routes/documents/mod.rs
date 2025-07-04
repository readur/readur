use axum::{routing::{get, post, delete}, Router};
use std::sync::Arc;
use crate::AppState;

pub mod types;
pub mod crud;
pub mod ocr;
pub mod bulk;
pub mod debug;
pub mod failed;

// Re-export commonly used types and functions for backward compatibility
pub use types::*;
pub use crud::*;
pub use ocr::*;
pub use bulk::*;
pub use debug::*;
pub use failed::*;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // CRUD operations
        .route("/", post(upload_document))
        .route("/", get(list_documents))
        .route("/{id}", get(get_document_by_id))
        .route("/{id}", delete(delete_document))
        .route("/{id}/download", get(download_document))
        .route("/{id}/view", get(view_document))
        
        // OCR operations
        .route("/{id}/ocr", get(get_document_ocr))
        .route("/{id}/ocr/retry", post(retry_ocr))
        .route("/ocr/stats", get(get_ocr_stats))
        .route("/{id}/ocr/stop", post(cancel_ocr))
        
        // Bulk operations
        .route("/bulk/delete", post(bulk_delete_documents))
        .route("/cleanup/low-confidence", delete(delete_low_confidence_documents))
        .route("/cleanup/failed-ocr", delete(delete_failed_ocr_documents))
        
        // Debug operations
        .route("/{id}/debug", get(get_document_debug_info))
        .route("/{id}/thumbnail", get(get_document_thumbnail))
        .route("/{id}/processed", get(get_processed_image))
        .route("/{id}/validate", get(validate_document_integrity))
        .route("/duplicates", get(get_user_duplicates))
        
        // Failed documents
        .route("/failed", get(get_failed_documents))
        .route("/failed/{id}", get(view_failed_document))
        .route("/failed/ocr", get(get_failed_ocr_documents))
}