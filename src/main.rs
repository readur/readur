use axum::{
    extract::DefaultBodyLimit,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};
use tracing::{info, error, warn};
use anyhow;
use sqlx::{Row, Column};

use readur::{config::Config, db::Database, AppState, *};

#[cfg(test)]
mod tests;

/// Determines the correct path for static files based on the environment
/// Checks multiple possible locations in order of preference
fn determine_static_files_path() -> std::path::PathBuf {
    use std::path::PathBuf;
    
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // List of possible static file locations in order of preference
    let possible_paths = vec![
        // Docker/production environment - frontend build copied to /app/frontend/dist
        current_dir.join("frontend/dist"),
        // Development environment - frontend build in local frontend/dist
        PathBuf::from("frontend/dist"),
        // Alternative development setup
        current_dir.join("../frontend/dist"),
        // Fallback to current directory if somehow the build is there
        current_dir.join("dist"),
        // Last resort fallback
        PathBuf::from("dist"),
    ];
    
    for path in possible_paths {
        let index_path = path.join("index.html");
        if index_path.exists() && index_path.is_file() {
            info!("Found static files at: {}", path.display());
            return path;
        } else {
            info!("Static files not found at: {} (index.html exists: {})", 
                path.display(), index_path.exists());
        }
    }
    
    // If no valid path found, default to frontend/dist and let it fail gracefully
    warn!("No valid static files directory found, defaulting to 'frontend/dist'");
    PathBuf::from("frontend/dist")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging with custom filters to reduce spam from noisy crates
    // Users can override with RUST_LOG environment variable, e.g.:
    // RUST_LOG=debug cargo run                                          (enable debug for all)
    // RUST_LOG=readur=debug,pdf_extract=error,sqlx::postgres::notice=off (debug for readur, suppress spam)
    // RUST_LOG=sqlx::postgres::notice=debug                             (show PostgreSQL notices for debugging)
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // Default filter when RUST_LOG is not set
            tracing_subscriber::EnvFilter::new("info")
                .add_directive("pdf_extract=error".parse().unwrap())           // Suppress pdf_extract WARN spam
                .add_directive("sqlx::postgres::notice=warn".parse().unwrap()) // Suppress PostgreSQL NOTICE spam  
                .add_directive("readur=info".parse().unwrap())                 // Keep our app logs at info
        });
    
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();
    
    println!("\n🚀 READUR APPLICATION STARTUP");
    println!("{}", "=".repeat(60));
    
    // Load and validate configuration with comprehensive logging
    let config = match Config::from_env() {
        Ok(cfg) => {
            println!("✅ Configuration loaded and validated successfully");
            cfg
        }
        Err(e) => {
            println!("❌ CRITICAL: Configuration loading failed!");
            println!("Error: {}", e);
            println!("\n🔧 Please check your environment variables and fix the configuration issues above.");
            return Err(e);
        }
    };
    
    // Log critical configuration values that affect startup
    println!("\n🔗 STARTUP CONFIGURATION:");
    println!("{}", "=".repeat(50));
    println!("🌐 Server will start on: {}", config.server_address);
    // Parse database URL safely without exposing credentials
    let db_info = if let Some(at_pos) = config.database_url.find('@') {
        let host_part = &config.database_url[at_pos + 1..];
        let protocol = if config.database_url.starts_with("postgresql://") { "postgresql" } else { "postgres" };
        
        // Extract just username from credentials part (before @)
        let creds_part = &config.database_url[..at_pos];
        let username = if let Some(proto_end) = creds_part.find("://") {
            let after_proto = &creds_part[proto_end + 3..];
            if let Some(colon_pos) = after_proto.find(':') {
                &after_proto[..colon_pos]
            } else {
                after_proto
            }
        } else {
            "unknown"
        };
        // if we get the username, let's now mask it to get just the first and last character
        let masked_username = format!("{}{}", &username[..1], &username[username.len() - 1..]);
        
        format!("{}://{}:***@{}", protocol, masked_username, host_part)
    } else {
        "Invalid database URL format".to_string()
    };
    
    println!("🗄️  Database connection: {}", db_info);
    println!("📁 Upload directory: {}", config.upload_path);
    println!("👁️  Watch directory: {}", config.watch_folder);
    
    // Initialize upload directory structure
    info!("Initializing upload directory structure...");
    let file_service = readur::services::file_service::FileService::new(config.upload_path.clone());
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
    println!("\n🗄️  DATABASE CONNECTION:");
    println!("{}", "=".repeat(50));
    
    let web_db = match Database::new_with_pool_config(&config.database_url, 20, 2).await {
        Ok(db) => {
            println!("✅ Web database pool created (max: 20 connections, min idle: 2)");
            db
        }
        Err(e) => {
            println!("❌ CRITICAL: Failed to connect to database for web operations!");
            println!("Database URL: {}", db_info);  // Use the already-masked URL
            println!("Error: {}", e);
            println!("\n🔧 Please verify:");
            println!("   - Database server is running");
            println!("   - DATABASE_URL is correct");
            println!("   - Database credentials are valid");
            println!("   - Network connectivity to database");
            return Err(e.into());
        }
    };
    
    let background_db = match Database::new_with_pool_config(&config.database_url, 30, 3).await {
        Ok(db) => {
            println!("✅ Background database pool created (max: 30 connections, min idle: 3)");
            db
        }
        Err(e) => {
            println!("❌ CRITICAL: Failed to connect to database for background operations!");
            println!("Error: {}", e);
            return Err(e.into());
        }
    };
    
    // Don't run the old migration system - let SQLx handle everything
    // db.migrate().await?;
    
    // Run SQLx migrations
    info!("Running SQLx migrations...");
    let migrations = sqlx::migrate!("./migrations");
    let total_migrations = migrations.migrations.len();
    
    if total_migrations > 0 {
        // Verify migrations are in correct chronological order
        let mut is_ordered = true;
        let mut prev_version = 0i64;
        
        for migration in migrations.migrations.iter() {
            if migration.version <= prev_version {
                error!("❌ Migration {} is out of order (previous: {})", migration.version, prev_version);
                is_ordered = false;
            }
            prev_version = migration.version;
        }
        
        if is_ordered {
            info!("✅ {} migrations found in correct chronological order", total_migrations);
        } else {
            error!("❌ Migrations are not in chronological order - this may cause issues");
        }
        
        // Log first and last migration for reference
        let first_migration = &migrations.migrations[0];
        let last_migration = &migrations.migrations[total_migrations - 1];
        
        info!("Migration range: {} ({}) → {} ({})", 
              first_migration.version, first_migration.description,
              last_migration.version, last_migration.description);
    } else {
        info!("No migrations found");
    }
    
    // Enhanced migration execution with detailed logging
    info!("🔄 Starting migration execution...");
    
    // Check current database migration state
    let applied_migrations = sqlx::query_scalar::<_, i64>(
        "SELECT version FROM _sqlx_migrations ORDER BY version"
    )
    .fetch_all(web_db.get_pool())
    .await
    .unwrap_or_default();
    
    if !applied_migrations.is_empty() {
        info!("📋 {} migrations already applied in database", applied_migrations.len());
        info!("📋 Latest applied migration: {}", applied_migrations.last().unwrap_or(&0));
    } else {
        info!("📋 No migrations previously applied - fresh database");
    }
    
    // List all migrations that will be processed
    info!("📝 Migrations to process:");
    for (i, migration) in migrations.migrations.iter().enumerate() {
        let status = if applied_migrations.contains(&migration.version) {
            "✅ APPLIED"
        } else {
            "⏳ PENDING"
        };
        info!("  {}: {} ({}) [{}]", 
              i + 1, migration.version, migration.description, status);
    }
    
    let result = migrations.run(web_db.get_pool()).await;
    match result {
        Ok(_) => {
            info!("✅ SQLx migrations completed successfully");
            
            // Verify final migration state
            let final_applied = sqlx::query_scalar::<_, i64>(
                "SELECT version FROM _sqlx_migrations ORDER BY version"
            )
            .fetch_all(web_db.get_pool())
            .await
            .unwrap_or_default();
            
            info!("📊 Final migration state: {} total applied", final_applied.len());
            if let Some(latest) = final_applied.last() {
                info!("📊 Latest migration now: {}", latest);
            }
            
        }
        Err(e) => {
            error!("❌ CRITICAL: SQLx migrations failed!");
            error!("Migration error: {}", e);
            
            // Get detailed error information
            error!("🔍 Migration failure details:");
            error!("  Error type: {}", std::any::type_name_of_val(&e));
            error!("  Error message: {}", e);
            
            // Try to get the current migration state even after failure
            match sqlx::query_scalar::<_, i64>(
                "SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1"
            )
            .fetch_optional(web_db.get_pool())
            .await {
                Ok(Some(latest)) => error!("  Last successful migration: {}", latest),
                Ok(None) => error!("  No migrations were applied successfully"),
                Err(table_err) => error!("  Could not read migration table: {}", table_err),
            }
            
            return Err(e.into());
        }
    }
    
    // Seed admin user  
    seed::seed_admin_user(&background_db).await?;
    
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
    let shared_queue_service = Arc::new(readur::ocr::queue::OcrQueueService::new(
        background_db.clone(), 
        background_db.get_pool().clone(), 
        concurrent_jobs
    ));
    
    // Initialize OIDC client if enabled
    let oidc_client = if config.oidc_enabled {
        match readur::oidc::OidcClient::new(&config).await {
            Ok(client) => {
                println!("✅ OIDC client initialized successfully");
                Some(Arc::new(client))
            }
            Err(e) => {
                error!("❌ Failed to initialize OIDC client: {}", e);
                println!("❌ OIDC authentication will be disabled");
                None
            }
        }
    } else {
        println!("ℹ️  OIDC authentication is disabled");
        None
    };
    
    // Create shared progress tracker
    let sync_progress_tracker = Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new());
    
    // Create web-facing state with shared queue service
    let web_state = AppState { 
        db: web_db, 
        config: config.clone(),
        webdav_scheduler: None, // Will be set after creating scheduler
        source_scheduler: None, // Will be set after creating scheduler
        queue_service: shared_queue_service.clone(),
        oidc_client: oidc_client.clone(),
        sync_progress_tracker: sync_progress_tracker.clone(),
    };
    let web_state = Arc::new(web_state);
    
    // Create background state with shared queue service
    let background_state = AppState {
        db: background_db,
        config: config.clone(),
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service: shared_queue_service.clone(),
        oidc_client: oidc_client.clone(),
        sync_progress_tracker: sync_progress_tracker.clone(),
    };
    let background_state = Arc::new(background_state);
    
    let watcher_config = config.clone();
    let watcher_db = background_state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = readur::scheduling::watcher::start_folder_watcher(watcher_config, watcher_db).await {
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
        info!("🚀 Starting OCR queue worker...");
        if let Err(e) = queue_worker.start_worker().await {
            error!("❌ OCR queue worker error: {}", e);
        } else {
            info!("✅ OCR queue worker started successfully");
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
    println!("\n📅 SCHEDULER INITIALIZATION:");
    println!("{}", "=".repeat(50));
    
    let source_scheduler = Arc::new(readur::scheduling::source_scheduler::SourceScheduler::new(background_state.clone()));
    println!("✅ Universal source scheduler created (handles WebDAV, Local, S3)");
    
    // Keep WebDAV scheduler for backward compatibility with existing WebDAV endpoints
    let webdav_scheduler = Arc::new(readur::scheduling::webdav_scheduler::WebDAVScheduler::new(background_state.clone()));
    println!("✅ Legacy WebDAV scheduler created (backward compatibility)");
    
    // Update the web state to include scheduler references
    let updated_web_state = AppState {
        db: web_state.db.clone(),
        config: web_state.config.clone(),
        webdav_scheduler: Some(webdav_scheduler.clone()),
        source_scheduler: Some(source_scheduler.clone()),
        queue_service: shared_queue_service.clone(),
        oidc_client: oidc_client.clone(),
        sync_progress_tracker: sync_progress_tracker.clone(),
    };
    let web_state = Arc::new(updated_web_state);
    
    // Start universal source scheduler on background runtime
    println!("⏰ Scheduling background source sync to start in 30 seconds");
    let scheduler_for_background = source_scheduler.clone();
    background_runtime.spawn(async move {
        info!("Starting universal source sync scheduler with 30-second startup delay");
        // Wait 30 seconds before starting scheduler to allow server to fully initialize
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        info!("🔄 Universal source sync scheduler starting after startup delay - this will check for WebDAV sources!");
        scheduler_for_background.start().await;
    });
    
    // Determine the correct static files path for SPA serving
    let static_dir = determine_static_files_path();
    let index_file = static_dir.join("index.html");
    
    info!("Using static files directory: {}", static_dir.display());
    info!("Using index.html file: {}", index_file.display());
    
    // Create the router with the updated state
    let app = Router::new()
        .route("/api/health", get(readur::health_check))
        .nest("/api/auth", readur::routes::auth::router())
        .nest("/api/documents", readur::routes::documents::router())
        .nest("/api/ignored-files", readur::routes::ignored_files::ignored_files_routes())
        .nest("/api/labels", readur::routes::labels::router())
        .nest("/api/metrics", readur::routes::metrics::router())
        .nest("/metrics", readur::routes::prometheus_metrics::router())
        .nest("/api/notifications", readur::routes::notifications::router())
        .nest("/api/ocr", readur::routes::ocr::router())
        .nest("/api/queue", readur::routes::queue::router())
        .nest("/api/search", readur::routes::search::router())
        .nest("/api/settings", readur::routes::settings::router())
        .nest("/api/sources", readur::routes::sources::router())
        .nest("/api/users", readur::routes::users::router())
        .nest("/api/webdav", readur::routes::webdav::router())
        .merge(readur::swagger::create_swagger_router())
        .fallback_service(
            ServeDir::new(&static_dir)
                .precompressed_gzip()
                .precompressed_br()
                .fallback(ServeFile::new(&index_file))
        )
        .layer(DefaultBodyLimit::max(config.max_file_size_mb as usize * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(web_state.clone());

    println!("\n🌐 STARTING HTTP SERVER:");
    println!("{}", "=".repeat(50));
    
    let listener = match tokio::net::TcpListener::bind(&config.server_address).await {
        Ok(listener) => {
            println!("✅ HTTP server bound to: {}", config.server_address);
            listener
        }
        Err(e) => {
            println!("❌ CRITICAL: Failed to bind to address: {}", config.server_address);
            println!("Error: {}", e);
            println!("\n🔧 Please check:");
            println!("   - Address {} is not already in use", config.server_address);
            println!("   - SERVER_HOST and SERVER_PORT environment variables are correct");
            println!("   - You have permission to bind to this address");
            return Err(e.into());
        }
    };
    
    println!("\n🎉 READUR APPLICATION READY!");
    println!("{}", "=".repeat(60));
    println!("🌐 Server: http://{}", config.server_address);
    println!("📁 Upload Directory: {}", config.upload_path);
    println!("👁️  Watch Directory: {}", config.watch_folder);
    println!("🔄 Source Scheduler: Will start in 30 seconds");
    println!("📋 Check logs above for any configuration warnings");
    println!("{}", "=".repeat(60));
    
    info!("🚀 Readur server is now running and accepting connections");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}


