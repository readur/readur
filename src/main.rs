use axum::{
    http::StatusCode,
    response::Html,
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
    let db = Database::new(&config.database_url).await?;
    
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
        .fetch_all(&db.pool)
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
        .fetch_optional(&db.pool)
        .await;
    
    match check_column {
        Ok(Some(_)) => info!("✅ ocr_error column exists"),
        Ok(None) => {
            error!("❌ ocr_error column is missing! Migration 006 may not have been applied.");
            // Try to add the column manually as a fallback
            info!("Attempting to add missing columns...");
            if let Err(e) = sqlx::query("ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_error TEXT")
                .execute(&db.pool)
                .await {
                error!("Failed to add ocr_error column: {}", e);
            }
            if let Err(e) = sqlx::query("ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_completed_at TIMESTAMPTZ")
                .execute(&db.pool)
                .await {
                error!("Failed to add ocr_completed_at column: {}", e);
            }
            info!("Fallback column addition completed");
        }
        Err(e) => error!("Failed to check for ocr_error column: {}", e),
    }
    
    let result = migrations.run(&db.pool).await;
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
    .fetch_all(&db.pool)
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
    seed::seed_admin_user(&db).await?;
    
    // Seed system user for watcher
    seed::seed_system_user(&db).await?;
    
    // Reset any running WebDAV syncs from previous server instance
    match db.reset_running_webdav_syncs().await {
        Ok(count) => {
            if count > 0 {
                info!("Reset {} orphaned WebDAV sync states from server restart", count);
            }
        }
        Err(e) => {
            warn!("Failed to reset running WebDAV syncs: {}", e);
        }
    }
    
    let state = AppState { 
        db, 
        config: config.clone(),
        webdav_scheduler: None, // Will be set after creating scheduler
    };
    let state = Arc::new(state);
    
    let watcher_config = config.clone();
    let watcher_db = state.db.clone();
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
    
    // Start OCR queue worker on dedicated OCR runtime
    let concurrent_jobs = 4; // TODO: Get from config/settings  
    let queue_service = Arc::new(readur::ocr_queue::OcrQueueService::new(
        state.db.clone(), 
        state.db.get_pool().clone(), 
        concurrent_jobs
    ));
    
    let queue_worker = queue_service.clone();
    ocr_runtime.spawn(async move {
        if let Err(e) = queue_worker.start_worker().await {
            error!("OCR queue worker error: {}", e);
        }
    });
    
    // Start OCR maintenance tasks on dedicated OCR runtime
    let queue_maintenance = queue_service.clone();
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
    
    // Create WebDAV scheduler and update AppState
    let webdav_scheduler = Arc::new(readur::webdav_scheduler::WebDAVScheduler::new(state.clone()));
    
    // Update the state to include the scheduler
    let updated_state = AppState {
        db: state.db.clone(),
        config: state.config.clone(),
        webdav_scheduler: Some(webdav_scheduler.clone()),
    };
    let state = Arc::new(updated_state);
    
    // Start WebDAV background sync scheduler on background runtime
    let scheduler_for_background = webdav_scheduler.clone();
    background_runtime.spawn(async move {
        info!("Starting WebDAV background sync scheduler with 30-second startup delay");
        // Wait 30 seconds before starting scheduler to allow server to fully initialize
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        info!("WebDAV background sync scheduler starting after startup delay");
        scheduler_for_background.start().await;
    });
    
    // Create the router with the updated state
    let app = Router::new()
        .route("/api/health", get(readur::health_check))
        .nest("/api/auth", readur::routes::auth::router())
        .nest("/api/documents", readur::routes::documents::router())
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
        .fallback_service(ServeDir::new("frontend/dist").fallback(ServeFile::new("frontend/dist/index.html")))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());
    
    let listener = tokio::net::TcpListener::bind(&config.server_address).await?;
    info!("Server starting on {}", config.server_address);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}


async fn serve_spa() -> Result<Html<String>, StatusCode> {
    match tokio::fs::read_to_string("frontend/dist/index.html").await {
        Ok(html) => Ok(Html(html)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}