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
        ProcessedImage, CreateProcessedImage, IgnoredFileResponse, IgnoredFilesQuery,
        DocumentListResponse, DocumentOcrResponse, DocumentOperationResponse,
        BulkDeleteResponse, PaginationInfo, DocumentDuplicatesResponse
    },
    routes::{
        metrics::{
            SystemMetrics, DatabaseMetrics, OcrMetrics, DocumentMetrics, UserMetrics, GeneralSystemMetrics
        },
        labels::{
            Label, CreateLabel, UpdateLabel, LabelAssignment, LabelQuery, BulkUpdateRequest as LabelBulkUpdateRequest
        },
        documents::BulkDeleteRequest
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
        crate::routes::auth::oidc_login,
        crate::routes::auth::oidc_callback,
        // Document endpoints
        crate::routes::documents::crud::upload_document,
        crate::routes::documents::crud::list_documents,
        crate::routes::documents::crud::get_document_by_id,
        crate::routes::documents::crud::delete_document,
        crate::routes::documents::bulk::bulk_delete_documents,
        crate::routes::documents::crud::download_document,
        crate::routes::documents::crud::view_document,
        crate::routes::documents::debug::get_document_thumbnail,
        crate::routes::documents::ocr::get_document_ocr,
        crate::routes::documents::debug::get_processed_image,
        crate::routes::documents::ocr::retry_ocr,
        crate::routes::documents::debug::get_document_debug_info,
        crate::routes::documents::failed::get_failed_ocr_documents,
        crate::routes::documents::failed::view_failed_document,
        crate::routes::documents::bulk::delete_low_confidence_documents,
        crate::routes::documents::bulk::delete_failed_ocr_documents,
        crate::routes::documents::crud::get_user_duplicates,
        // Labels endpoints
        crate::routes::labels::get_labels,
        crate::routes::labels::create_label,
        crate::routes::labels::get_label,
        crate::routes::labels::update_label,
        crate::routes::labels::delete_label,
        crate::routes::labels::get_document_labels,
        crate::routes::labels::update_document_labels,
        crate::routes::labels::add_document_label,
        crate::routes::labels::remove_document_label,
        crate::routes::labels::bulk_update_document_labels,
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
        crate::routes::queue::enqueue_pending_documents,
        crate::routes::queue::get_ocr_status,
        crate::routes::queue::pause_ocr_processing,
        crate::routes::queue::resume_ocr_processing,
        // Metrics endpoints
        crate::routes::metrics::get_system_metrics,
        crate::routes::prometheus_metrics::get_prometheus_metrics,
        // Notifications endpoints
        crate::routes::notifications::get_notifications,
        crate::routes::notifications::get_notification_summary,
        crate::routes::notifications::mark_notification_read,
        crate::routes::notifications::mark_all_notifications_read,
        crate::routes::notifications::delete_notification,
        // Sources endpoints
        crate::routes::sources::crud::list_sources,
        crate::routes::sources::crud::create_source,
        crate::routes::sources::crud::get_source,
        crate::routes::sources::crud::update_source,
        crate::routes::sources::crud::delete_source,
        crate::routes::sources::sync::trigger_sync,
        crate::routes::sources::sync::stop_sync,
        crate::routes::sources::sync::trigger_deep_scan,
        crate::routes::sources::validation::test_connection,
        crate::routes::sources::validation::validate_source,
        crate::routes::sources::estimation::estimate_crawl,
        crate::routes::sources::estimation::estimate_crawl_with_config,
        crate::routes::sources::validation::test_connection_with_config,
        // WebDAV endpoints
        crate::routes::webdav::start_webdav_sync,
        crate::routes::webdav::cancel_webdav_sync,
        crate::routes::webdav::get_webdav_sync_status,
        crate::routes::webdav::test_webdav_connection,
        crate::routes::webdav::estimate_webdav_crawl,
        // OCR endpoints
        crate::routes::ocr::get_available_languages,
        crate::ocr::api::health_check,
        crate::ocr::api::perform_ocr,
        // Ignored files endpoints
        crate::routes::ignored_files::list_ignored_files,
        crate::routes::ignored_files::get_ignored_file,
        crate::routes::ignored_files::delete_ignored_file,
        crate::routes::ignored_files::bulk_delete_ignored_files,
        crate::routes::ignored_files::get_ignored_files_stats,
        // Health check
        crate::health_check,
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
            ProcessedImage, CreateProcessedImage, IgnoredFileResponse, IgnoredFilesQuery,
            crate::routes::ignored_files::BulkDeleteIgnoredFilesRequest,
            crate::routes::ignored_files::IgnoredFilesStats,
            crate::routes::ignored_files::SourceTypeCount,
            SystemMetrics, DatabaseMetrics, OcrMetrics, DocumentMetrics, UserMetrics, GeneralSystemMetrics,
            // Labels schemas
            Label, CreateLabel, UpdateLabel, LabelAssignment, LabelQuery, LabelBulkUpdateRequest,
            // Document schemas
            BulkDeleteRequest, DocumentListResponse, DocumentOcrResponse, DocumentOperationResponse,
            BulkDeleteResponse, PaginationInfo, DocumentDuplicatesResponse, crate::routes::documents::RetryOcrRequest,
            // OCR schemas
            crate::routes::ocr::AvailableLanguagesResponse, crate::routes::ocr::LanguageInfo,
            crate::ocr::api::OcrHealthResponse, crate::ocr::api::OcrErrorResponse, crate::ocr::api::OcrRequest
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "documents", description = "Document management endpoints"),
        (name = "labels", description = "Document labeling and categorization endpoints"),
        (name = "search", description = "Document search endpoints"),
        (name = "settings", description = "User settings endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "queue", description = "OCR queue management endpoints"),
        (name = "metrics", description = "System metrics and monitoring endpoints"),
        (name = "notifications", description = "User notification endpoints"),
        (name = "sources", description = "Document source management endpoints"),
        (name = "webdav", description = "WebDAV synchronization endpoints"),
        (name = "ignored_files", description = "Ignored files management endpoints"),
        (name = "ocr", description = "OCR service management endpoints"),
        (name = "health", description = "Health check endpoint"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "Readur API",
        version = "2.5.3",
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