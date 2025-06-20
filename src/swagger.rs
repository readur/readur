use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{SecurityScheme, HttpAuthScheme, Http};
use utoipa_swagger_ui::SwaggerUi;
use axum::Router;
use std::sync::Arc;

use crate::{
    models::{
        CreateUser, LoginRequest, LoginResponse, UserResponse, UpdateUser,
        DocumentResponse, SearchRequest, SearchResponse, EnhancedDocumentResponse,
        SettingsResponse, UpdateSettings, SearchMode, SearchSnippet, HighlightRange,
        FacetItem, SearchFacetsResponse, Notification, NotificationSummary, CreateNotification,
        Source, SourceResponse, CreateSource, UpdateSource, SourceWithStats,
        WebDAVSourceConfig, LocalFolderSourceConfig, S3SourceConfig,
        WebDAVCrawlEstimate, WebDAVTestConnection, WebDAVConnectionResult, WebDAVSyncStatus,
        ProcessedImage, CreateProcessedImage
    },
    routes::metrics::{
        SystemMetrics, DatabaseMetrics, OcrMetrics, DocumentMetrics, UserMetrics, GeneralSystemMetrics
    },
    AppState,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Auth endpoints
        crate::routes::auth::register,
        crate::routes::auth::login,
        crate::routes::auth::me,
        // Document endpoints
        crate::routes::documents::upload_document,
        crate::routes::documents::list_documents,
        crate::routes::documents::get_document_by_id,
        crate::routes::documents::download_document,
        crate::routes::documents::view_document,
        crate::routes::documents::get_document_thumbnail,
        crate::routes::documents::get_document_ocr,
        crate::routes::documents::get_processed_image,
        crate::routes::documents::retry_ocr,
        crate::routes::documents::get_failed_ocr_documents,
        crate::routes::documents::get_user_duplicates,
        // Search endpoints
        crate::routes::search::search_documents,
        crate::routes::search::enhanced_search_documents,
        crate::routes::search::get_search_facets,
        // Settings endpoints
        crate::routes::settings::get_settings,
        crate::routes::settings::update_settings,
        // User endpoints
        crate::routes::users::list_users,
        crate::routes::users::create_user,
        crate::routes::users::get_user,
        crate::routes::users::update_user,
        crate::routes::users::delete_user,
        // Queue endpoints
        crate::routes::queue::get_queue_stats,
        crate::routes::queue::requeue_failed,
        crate::routes::queue::get_ocr_status,
        crate::routes::queue::pause_ocr_processing,
        crate::routes::queue::resume_ocr_processing,
        // Metrics endpoints
        crate::routes::metrics::get_system_metrics,
        // Notifications endpoints
        crate::routes::notifications::get_notifications,
        crate::routes::notifications::get_notification_summary,
        crate::routes::notifications::mark_notification_read,
        crate::routes::notifications::mark_all_notifications_read,
        crate::routes::notifications::delete_notification,
        // Sources endpoints
        crate::routes::sources::list_sources,
        crate::routes::sources::create_source,
        crate::routes::sources::get_source,
        crate::routes::sources::update_source,
        crate::routes::sources::delete_source,
        crate::routes::sources::trigger_sync,
        crate::routes::sources::stop_sync,
        crate::routes::sources::test_connection,
        crate::routes::sources::estimate_crawl,
        crate::routes::sources::estimate_crawl_with_config,
        crate::routes::sources::test_connection_with_config,
        // WebDAV endpoints
        crate::routes::webdav::start_webdav_sync,
        crate::routes::webdav::cancel_webdav_sync,
        crate::routes::webdav::get_webdav_sync_status,
        crate::routes::webdav::test_webdav_connection,
        crate::routes::webdav::estimate_webdav_crawl,
    ),
    components(
        schemas(
            CreateUser, LoginRequest, LoginResponse, UserResponse, UpdateUser,
            DocumentResponse, SearchRequest, SearchResponse, EnhancedDocumentResponse,
            SettingsResponse, UpdateSettings, SearchMode, SearchSnippet, HighlightRange,
            FacetItem, SearchFacetsResponse, Notification, NotificationSummary, CreateNotification,
            Source, SourceResponse, CreateSource, UpdateSource, SourceWithStats,
            WebDAVSourceConfig, LocalFolderSourceConfig, S3SourceConfig,
            WebDAVCrawlEstimate, WebDAVTestConnection, WebDAVConnectionResult, WebDAVSyncStatus,
            ProcessedImage, CreateProcessedImage,
            SystemMetrics, DatabaseMetrics, OcrMetrics, DocumentMetrics, UserMetrics, GeneralSystemMetrics
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "documents", description = "Document management endpoints"),
        (name = "search", description = "Document search endpoints"),
        (name = "settings", description = "User settings endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "queue", description = "OCR queue management endpoints"),
        (name = "metrics", description = "System metrics and monitoring endpoints"),
        (name = "notifications", description = "User notification endpoints"),
        (name = "sources", description = "Document source management endpoints"),
        (name = "webdav", description = "WebDAV synchronization endpoints"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "Readur API",
        version = "0.1.0",
        description = "Document management and OCR processing API",
        contact(
            name = "Readur Team",
            email = "support@readur.dev"
        )
    ),
    servers(
        (url = "/api", description = "API base path")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer))
            )
        }
    }
}

pub fn create_swagger_router() -> Router<Arc<AppState>> {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}