use axum::{
    routing::get,
    Router,
};
use sqlx::Row;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};
use tracing::{info, error, warn};

use readur::{config::Config, db::Database, AppState, *};

#[cfg(test)]
mod tests;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    let config = Config::from_env()?;
    
    // Initialize upload directory structure
    info!("Initializing upload directory structure...");
    let file_service = readur::file_service::FileService::new(config.upload_path.clone());
    if let Err(e) = file_service.initialize_directory_structure().await {
        error!("Failed to initialize directory structure: {}", e);
        return Err(e.into());
    }
    info!("✅ Upload directory structure initialized");
    
    // Migrate existing files to new structure (one-time operation)
    info!("Migrating existing files to structured directories...");
    if let Err(e) = file_service.migrate_existing_files().await {
        warn!("Failed to migrate some existing files: {}", e);
        // Don't fail startup for migration issues
    }
    
    // Create separate database pools for different workloads
    let web_db = Database::new_with_pool_config(&config.database_url, 20, 2).await?;  // Web UI pool
    let background_db = Database::new_with_pool_config(&config.database_url, 30, 3).await?;  // Background operations pool
    
    // Don't run the old migration system - let SQLx handle everything
    // db.migrate().await?;
    
    // Run SQLx migrations
    info!("Running SQLx migrations...");
    let migrations = sqlx::migrate!("./migrations");
    info!("Found {} migrations", migrations.migrations.len());
    
    for migration in migrations.migrations.iter() {
        info!("Migration available: {} - {}", migration.version, migration.description);
    }
    
    // Check current migration status
    let applied_result = sqlx::query("SELECT version, description FROM _sqlx_migrations ORDER BY version")
        .fetch_all(web_db.get_pool())
        .await;
    
    match applied_result {
        Ok(rows) => {
            info!("Currently applied migrations:");
            for row in rows {
                let version: i64 = row.get("version");
                let description: String = row.get("description");
                info!("  - {} {}", version, description);
            }
        }
        Err(e) => {
            info!("No existing migrations found (this is normal for first run): {}", e);
        }
    }
    
    // Check if ocr_error column exists
    let check_column = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_name = 'documents' AND column_name = 'ocr_error'")
        .fetch_optional(web_db.get_pool())
        .await;
    
    match check_column {
        Ok(Some(_)) => info!("✅ ocr_error column exists"),
        Ok(None) => {
            error!("❌ ocr_error column is missing! Migration 006 may not have been applied.");
            // Try to add the column manually as a fallback
            info!("Attempting to add missing columns...");
            if let Err(e) = sqlx::query("ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_error TEXT")
                .execute(web_db.get_pool())
                .await {
                error!("Failed to add ocr_error column: {}", e);
            }
            if let Err(e) = sqlx::query("ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_completed_at TIMESTAMPTZ")
                .execute(web_db.get_pool())
                .await {
                error!("Failed to add ocr_completed_at column: {}", e);
            }
            info!("Fallback column addition completed");
        }
        Err(e) => error!("Failed to check for ocr_error column: {}", e),
    }
    
    let result = migrations.run(web_db.get_pool()).await;
    match result {
        Ok(_) => info!("SQLx migrations completed successfully"),
        Err(e) => {
            error!("Failed to run SQLx migrations: {}", e);
            return Err(e.into());
        }
    }
    
    // Debug: Check what columns exist in documents table
    let columns_result = sqlx::query(
        "SELECT column_name FROM information_schema.columns 
         WHERE table_name = 'documents' AND table_schema = 'public'
         ORDER BY ordinal_position"
    )
    .fetch_all(web_db.get_pool())
    .await;
    
    match columns_result {
        Ok(rows) => {
            info!("Columns in documents table:");
            for row in rows {
                let column_name: String = row.get("column_name");
                info!("  - {}", column_name);
            }
        }
        Err(e) => {
            error!("Failed to check columns: {}", e);
        }
    }
    
    // Seed admin user  
    seed::seed_admin_user(&background_db).await?;
    
    // Seed system user for watcher
    seed::seed_system_user(&background_db).await?;
    
    // Reset any running WebDAV syncs from previous server instance using background DB
    match background_db.reset_running_webdav_syncs().await {
        Ok(count) => {
            if count > 0 {
                info!("Reset {} orphaned WebDAV sync states from server restart", count);
            }
        }
        Err(e) => {
            warn!("Failed to reset running WebDAV syncs: {}", e);
        }
    }
    
    // Reset any running universal source syncs from previous server instance
    match background_db.reset_running_source_syncs().await {
        Ok(count) => {
            if count > 0 {
                info!("Reset {} orphaned source sync states from server restart", count);
            }
        }
        Err(e) => {
            warn!("Failed to reset running source syncs: {}", e);
        }
    }
    
    // Create shared OCR queue service for both web and background operations
    let concurrent_jobs = 15; // Limit concurrent OCR jobs to prevent DB pool exhaustion
    let shared_queue_service = Arc::new(readur::ocr_queue::OcrQueueService::new(
        background_db.clone(), 
        background_db.get_pool().clone(), 
        concurrent_jobs
    ));
    
    // Create web-facing state with shared queue service
    let web_state = AppState { 
        db: web_db, 
        config: config.clone(),
        webdav_scheduler: None, // Will be set after creating scheduler
        source_scheduler: None, // Will be set after creating scheduler
        queue_service: shared_queue_service.clone(),
    };
    let web_state = Arc::new(web_state);
    
    // Create background state with shared queue service
    let background_state = AppState {
        db: background_db,
        config: config.clone(),
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service: shared_queue_service.clone(),
    };
    let background_state = Arc::new(background_state);
    
    let watcher_config = config.clone();
    let watcher_db = background_state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = readur::watcher::start_folder_watcher(watcher_config, watcher_db).await {
            error!("Folder watcher error: {}", e);
        }
    });
    
    // Create dedicated runtime for OCR processing to prevent interference with WebDAV
    let ocr_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(3)  // Dedicated threads for OCR work
        .thread_name("readur-ocr")
        .enable_all()
        .build()?;
    
    // Create separate runtime for other background tasks (WebDAV, maintenance)
    let background_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)  // Dedicated threads for WebDAV and maintenance
        .thread_name("readur-background")
        .enable_all()
        .build()?;
        
    // Create dedicated runtime for database-heavy operations
    let db_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)  // Dedicated threads for intensive DB operations
        .thread_name("readur-db")
        .enable_all()
        .build()?;
    
    // Start OCR queue worker on dedicated OCR runtime using shared queue service
    let queue_worker = shared_queue_service.clone();
    ocr_runtime.spawn(async move {
        if let Err(e) = queue_worker.start_worker().await {
            error!("OCR queue worker error: {}", e);
        }
    });
    
    // Start OCR maintenance tasks on dedicated OCR runtime
    let queue_maintenance = shared_queue_service.clone();
    ocr_runtime.spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            
            // Recover stale items (older than 10 minutes)
            if let Err(e) = queue_maintenance.recover_stale_items(10).await {
                error!("Error recovering stale items: {}", e);
            }
            
            // Clean up old completed items (older than 7 days)
            if let Err(e) = queue_maintenance.cleanup_completed(7).await {
                error!("Error cleaning up completed items: {}", e);
            }
        }
    });
    
    // Create universal source scheduler with background state (handles WebDAV, Local, S3)
    let source_scheduler = Arc::new(readur::source_scheduler::SourceScheduler::new(background_state.clone()));
    
    // Keep WebDAV scheduler for backward compatibility with existing WebDAV endpoints
    let webdav_scheduler = Arc::new(readur::webdav_scheduler::WebDAVScheduler::new(background_state.clone()));
    
    // Update the web state to include scheduler references
    let updated_web_state = AppState {
        db: web_state.db.clone(),
        config: web_state.config.clone(),
        webdav_scheduler: Some(webdav_scheduler.clone()),
        source_scheduler: Some(source_scheduler.clone()),
        queue_service: shared_queue_service.clone(),
    };
    let web_state = Arc::new(updated_web_state);
    
    // Start universal source scheduler on background runtime
    let scheduler_for_background = source_scheduler.clone();
    background_runtime.spawn(async move {
        info!("Starting universal source sync scheduler with 30-second startup delay");
        // Wait 30 seconds before starting scheduler to allow server to fully initialize
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        info!("Universal source sync scheduler starting after startup delay");
        scheduler_for_background.start().await;
    });
    
    // Create the router with the updated state
    let app = Router::new()
        .route("/api/health", get(readur::health_check))
        .nest("/api/auth", readur::routes::auth::router())
        .nest("/api/documents", readur::routes::documents::router())
        .nest("/api/labels", readur::routes::labels::router())
        .nest("/api/metrics", readur::routes::metrics::router())
        .nest("/metrics", readur::routes::prometheus_metrics::router())
        .nest("/api/notifications", readur::routes::notifications::router())
        .nest("/api/queue", readur::routes::queue::router())
        .nest("/api/search", readur::routes::search::router())
        .nest("/api/settings", readur::routes::settings::router())
        .nest("/api/sources", readur::routes::sources::router())
        .nest("/api/users", readur::routes::users::router())
        .nest("/api/webdav", readur::routes::webdav::router())
        .merge(readur::swagger::create_swagger_router())
        .fallback_service(
            ServeDir::new("frontend/dist")
                .precompressed_gzip()
                .precompressed_br()
                .fallback(ServeFile::new("dist/index.html"))
        )
        .layer(CorsLayer::permissive())
        .with_state(web_state.clone());
    
    // Debug static file serving setup
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    info!("Server working directory: {}", current_dir.display());
    
    let dist_path = current_dir.join("frontend/dist");
    info!("Looking for static files at: {}", dist_path.display());
    info!("dist directory exists: {}", dist_path.exists());
    
    if dist_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&dist_path) {
            info!("Contents of dist directory:");
            for entry in entries.flatten() {
                info!("  - {}", entry.file_name().to_string_lossy());
            }
        }
        
        let index_path = dist_path.join("index.html");
        info!("index.html exists: {}", index_path.exists());
        if index_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&index_path) {
                info!("index.html size: {} bytes", metadata.len());
            }
        }
    }

    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;
    info!("Server starting on {}", config.server_address);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}


