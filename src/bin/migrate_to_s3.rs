//! Migration utility to move existing local files to S3 storage
//! 
//! Usage: cargo run --bin migrate_to_s3 --features s3
//! 
//! This utility will:
//! 1. Connect to the database
//! 2. Find all documents with local file paths
//! 3. Upload files to S3 with proper structure
//! 4. Update database records with S3 paths
//! 5. Optionally delete local files after successful upload
//! 6. Support rollback on failure with transaction-like behavior

use anyhow::Result;
use clap::Parser;
use std::path::Path;
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};

use readur::{
    config::Config,
    db::Database,
    services::{s3_service::S3Service, file_service::FileService},
};

/// Migration state tracking for rollback functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationState {
    started_at: chrono::DateTime<chrono::Utc>,
    completed_migrations: Vec<MigrationRecord>,
    failed_migrations: Vec<FailedMigration>,
    total_files: usize,
    processed_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationRecord {
    document_id: Uuid,
    user_id: Uuid,
    original_path: String,
    s3_key: String,
    migrated_at: chrono::DateTime<chrono::Utc>,
    associated_files: Vec<AssociatedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssociatedFile {
    file_type: String, // "thumbnail" or "processed_image"
    original_path: String,
    s3_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FailedMigration {
    document_id: Uuid,
    original_path: String,
    error: String,
    failed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Parser)]
#[command(name = "migrate_to_s3")]
#[command(about = "Migrate existing local files to S3 storage")]
struct Args {
    /// Dry run - only show what would be migrated
    #[arg(short, long)]
    dry_run: bool,
    
    /// Delete local files after successful S3 upload
    #[arg(long)]
    delete_local: bool,
    
    /// Limit number of files to migrate (for testing)
    #[arg(short, long)]
    limit: Option<usize>,
    
    /// Only migrate files for specific user ID
    #[arg(short, long)]
    user_id: Option<String>,
    
    /// Enable rollback on failure - will revert any successful migrations if overall process fails
    #[arg(long)]
    enable_rollback: bool,
    
    /// Resume from a specific document ID (for partial recovery)
    #[arg(long)]
    resume_from: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let args = Args::parse();
    
    info!("🚀 Starting S3 migration utility");
    
    // Load configuration
    let config = Config::from_env()?;
    
    if !config.s3_enabled {
        error!("S3 is not enabled in configuration. Set S3_ENABLED=true and provide S3 configuration.");
        std::process::exit(1);
    }
    
    let s3_config = config.s3_config.as_ref()
        .ok_or_else(|| anyhow::anyhow!("S3 configuration not found"))?;
    
    // Connect to database
    info!("📊 Connecting to database...");
    let db = Database::new(&config.database_url).await?;
    
    // Initialize S3 service
    info!("☁️  Initializing S3 service...");
    let s3_service = S3Service::new(s3_config.clone()).await?;
    
    // Initialize FileService with proper storage configuration
    info!("📁 Initializing file service...");
    let storage_config = readur::storage::factory::storage_config_from_env(&config)?;
    let file_service = FileService::from_config(storage_config, config.upload_path.clone()).await?;
    
    // Test S3 connection
    match s3_service.test_connection().await {
        Ok(_) => info!("✅ S3 connection successful"),
        Err(e) => {
            error!("❌ S3 connection failed: {}", e);
            std::process::exit(1);
        }
    }
    
    // Get documents to migrate
    info!("🔍 Finding documents to migrate...");
    let mut documents = if let Some(user_id_str) = &args.user_id {
        let user_id = Uuid::parse_str(user_id_str)?;
        db.get_documents_by_user(user_id, args.limit.unwrap_or(1000) as i64, 0).await?
    } else {
        // Get all documents (this might need pagination for large datasets)
        let all_users = db.get_all_users().await?;
        let mut all_docs = Vec::new();
        
        for user in all_users {
            let user_docs = db.get_documents_by_user(user.id, 500, 0).await?;
            all_docs.extend(user_docs);
            
            if let Some(limit) = args.limit {
                if all_docs.len() >= limit {
                    all_docs.truncate(limit);
                    break;
                }
            }
        }
        all_docs
    };
    
    // Filter documents that are not already in S3
    let local_documents: Vec<_> = documents.into_iter()
        .filter(|doc| !doc.file_path.starts_with("s3://"))
        .collect();
    
    info!("📋 Found {} documents with local file paths", local_documents.len());
    
    if local_documents.is_empty() {
        info!("✅ No local documents found to migrate");
        return Ok(());
    }
    
    if args.dry_run {
        info!("🔍 DRY RUN - Would migrate the following files:");
        for doc in &local_documents {
            info!("  - {} (User: {}, Size: {} bytes)", 
                  doc.original_filename, doc.user_id, doc.file_size);
        }
        info!("💡 Run without --dry-run to perform actual migration");
        return Ok(());
    }
    
    // Initialize migration state
    let mut migration_state = MigrationState {
        started_at: chrono::Utc::now(),
        completed_migrations: Vec::new(),
        failed_migrations: Vec::new(),
        total_files: local_documents.len(),
        processed_files: 0,
    };
    
    // Resume from specific document if requested
    let start_index = if let Some(resume_from_str) = &args.resume_from {
        let resume_doc_id = Uuid::parse_str(resume_from_str)?;
        local_documents.iter().position(|doc| doc.id == resume_doc_id)
            .unwrap_or(0)
    } else {
        0
    };
    
    info!("📊 Migration plan: {} files to process (starting from index {})", 
          local_documents.len() - start_index, start_index);
    
    // Perform migration with progress tracking
    let mut migrated_count = 0;
    let mut failed_count = 0;
    
    for (index, doc) in local_documents.iter().enumerate().skip(start_index) {
        info!("📦 Migrating: {} ({}) [{}/{}]", 
              doc.original_filename, doc.id, index + 1, local_documents.len());
        
        match migrate_document_with_tracking(&db, &s3_service, &file_service, doc, args.delete_local).await {
            Ok(migration_record) => {
                migrated_count += 1;
                migration_state.completed_migrations.push(migration_record);
                migration_state.processed_files += 1;
                info!("✅ Successfully migrated: {} [{}/{}]", 
                      doc.original_filename, migrated_count, local_documents.len());
                
                // Save progress periodically (every 10 files)
                if migrated_count % 10 == 0 {
                    save_migration_state(&migration_state).await?;
                }
            }
            Err(e) => {
                failed_count += 1;
                let failed_migration = FailedMigration {
                    document_id: doc.id,
                    original_path: doc.file_path.clone(),
                    error: e.to_string(),
                    failed_at: chrono::Utc::now(),
                };
                migration_state.failed_migrations.push(failed_migration);
                migration_state.processed_files += 1;
                error!("❌ Failed to migrate {}: {}", doc.original_filename, e);
                
                // If rollback is enabled and we have failures, offer to rollback
                if args.enable_rollback && failed_count > 0 {
                    error!("💥 Migration failure detected with rollback enabled!");
                    error!("Do you want to rollback all {} successful migrations? (y/N)", migrated_count);
                    
                    // For automation, we'll automatically rollback on any failure
                    // In interactive mode, you could read from stdin here
                    warn!("🔄 Automatically initiating rollback due to failure...");
                    match rollback_migrations(&db, &s3_service, &migration_state).await {
                        Ok(rolled_back) => {
                            error!("🔄 Successfully rolled back {} migrations", rolled_back);
                            return Err(anyhow::anyhow!("Migration failed and was rolled back"));
                        }
                        Err(rollback_err) => {
                            error!("💥 CRITICAL: Rollback failed: {}", rollback_err);
                            error!("💾 Migration state saved for manual recovery");
                            save_migration_state(&migration_state).await?;
                            return Err(anyhow::anyhow!(
                                "Migration failed and rollback also failed. Check migration state file for manual recovery."
                            ));
                        }
                    }
                }
            }
        }
    }
    
    // Save final migration state
    save_migration_state(&migration_state).await?;
    
    info!("🎉 Migration completed!");
    info!("✅ Successfully migrated: {} files", migrated_count);
    if failed_count > 0 {
        warn!("❌ Failed to migrate: {} files", failed_count);
        warn!("💾 Check migration_state.json for details on failures");
    }
    
    Ok(())
}

async fn migrate_document_with_tracking(
    db: &Database,
    s3_service: &S3Service,
    file_service: &FileService,
    document: &readur::models::Document,
    delete_local: bool,
) -> Result<MigrationRecord> {
    // Read local file
    let local_path = Path::new(&document.file_path);
    if !local_path.exists() {
        return Err(anyhow::anyhow!("Local file not found: {}", document.file_path));
    }
    
    let file_data = tokio::fs::read(&local_path).await?;
    
    // Upload to S3
    let s3_key = s3_service.store_document(
        document.user_id,
        document.id,
        &document.filename,
        &file_data,
    ).await?;
    
    let s3_path = format!("s3://{}", s3_key);
    
    // Update database record
    db.update_document_file_path(document.id, &s3_path).await?;
    
    // Migrate associated files (thumbnails, processed images)
    let associated_files = migrate_associated_files_with_tracking(s3_service, file_service, document, delete_local).await?;
    
    // Delete local file if requested
    if delete_local {
        if let Err(e) = tokio::fs::remove_file(&local_path).await {
            warn!("Failed to delete local file {}: {}", document.file_path, e);
        } else {
            info!("🗑️  Deleted local file: {}", document.file_path);
        }
    }
    
    // Create migration record for tracking
    let migration_record = MigrationRecord {
        document_id: document.id,
        user_id: document.user_id,
        original_path: document.file_path.clone(),
        s3_key,
        migrated_at: chrono::Utc::now(),
        associated_files,
    };
    
    Ok(migration_record)
}

async fn migrate_associated_files_with_tracking(
    s3_service: &S3Service,
    file_service: &FileService,
    document: &readur::models::Document,
    delete_local: bool,
) -> Result<Vec<AssociatedFile>> {
    let mut associated_files = Vec::new();
    
    // Migrate thumbnail
    let thumbnail_path = file_service.get_thumbnails_path().join(format!("{}_thumb.jpg", document.id));
    if thumbnail_path.exists() {
        match tokio::fs::read(&thumbnail_path).await {
            Ok(thumbnail_data) => {
                match s3_service.store_thumbnail(document.user_id, document.id, &thumbnail_data).await {
                    Ok(s3_key) => {
                        info!("📸 Migrated thumbnail for: {}", document.original_filename);
                        associated_files.push(AssociatedFile {
                            file_type: "thumbnail".to_string(),
                            original_path: thumbnail_path.to_string_lossy().to_string(),
                            s3_key,
                        });
                        if delete_local {
                            let _ = tokio::fs::remove_file(&thumbnail_path).await;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to migrate thumbnail for {}: {}", document.id, e);
                    }
                }
            }
            Err(e) => warn!("Failed to read thumbnail {}: {}", thumbnail_path.display(), e),
        }
    }
    
    // Migrate processed image
    let processed_path = file_service.get_processed_images_path().join(format!("{}_processed.png", document.id));
    if processed_path.exists() {
        match tokio::fs::read(&processed_path).await {
            Ok(processed_data) => {
                match s3_service.store_processed_image(document.user_id, document.id, &processed_data).await {
                    Ok(s3_key) => {
                        info!("🖼️  Migrated processed image for: {}", document.original_filename);
                        associated_files.push(AssociatedFile {
                            file_type: "processed_image".to_string(),
                            original_path: processed_path.to_string_lossy().to_string(),
                            s3_key,
                        });
                        if delete_local {
                            let _ = tokio::fs::remove_file(&processed_path).await;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to migrate processed image for {}: {}", document.id, e);
                    }
                }
            }
            Err(e) => warn!("Failed to read processed image {}: {}", processed_path.display(), e),
        }
    }
    
    Ok(associated_files)
}

/// Save migration state to disk for recovery purposes
async fn save_migration_state(state: &MigrationState) -> Result<()> {
    let state_json = serde_json::to_string_pretty(state)?;
    tokio::fs::write("migration_state.json", state_json).await?;
    info!("💾 Migration state saved to migration_state.json");
    Ok(())
}

/// Rollback migrations by restoring database paths and deleting S3 objects
async fn rollback_migrations(
    db: &Database,
    s3_service: &S3Service,
    state: &MigrationState,
) -> Result<usize> {
    info!("🔄 Starting rollback of {} migrations...", state.completed_migrations.len());
    
    let mut rolled_back = 0;
    let mut rollback_errors = Vec::new();
    
    // Process migrations in reverse order (most recent first)
    for migration in state.completed_migrations.iter().rev() {
        info!("🔄 Rolling back migration for document {}", migration.document_id);
        
        // Restore original database path
        match db.update_document_file_path(migration.document_id, &migration.original_path).await {
            Ok(_) => {
                info!("✅ Restored database path for document {}", migration.document_id);
            }
            Err(e) => {
                let error_msg = format!("Failed to restore DB path for {}: {}", migration.document_id, e);
                error!("❌ {}", error_msg);
                rollback_errors.push(error_msg);
                continue; // Skip S3 cleanup if DB restore failed
            }
        }
        
        // Delete S3 object (main document)
        match s3_service.delete_file(&migration.s3_key).await {
            Ok(_) => {
                info!("🗑️  Deleted S3 object: {}", migration.s3_key);
            }
            Err(e) => {
                let error_msg = format!("Failed to delete S3 object {}: {}", migration.s3_key, e);
                warn!("⚠️  {}", error_msg);
                rollback_errors.push(error_msg);
                // Continue with associated files even if main file deletion failed
            }
        }
        
        // Delete associated S3 objects (thumbnails, processed images)
        for associated in &migration.associated_files {
            match s3_service.delete_file(&associated.s3_key).await {
                Ok(_) => {
                    info!("🗑️  Deleted associated S3 object: {} ({})", associated.s3_key, associated.file_type);
                }
                Err(e) => {
                    let error_msg = format!("Failed to delete associated S3 object {}: {}", associated.s3_key, e);
                    warn!("⚠️  {}", error_msg);
                    rollback_errors.push(error_msg);
                }
            }
        }
        
        rolled_back += 1;
    }
    
    if !rollback_errors.is_empty() {
        warn!("⚠️  Rollback completed with {} errors:", rollback_errors.len());
        for error in &rollback_errors {
            warn!("  - {}", error);
        }
        
        // Save error details for manual cleanup
        let error_state = serde_json::json!({
            "rollback_completed_at": chrono::Utc::now(),
            "rolled_back_count": rolled_back,
            "rollback_errors": rollback_errors,
            "original_migration_state": state
        });
        
        let error_json = serde_json::to_string_pretty(&error_state)?;
        tokio::fs::write("rollback_errors.json", error_json).await?;
        warn!("💾 Rollback errors saved to rollback_errors.json for manual cleanup");
    }
    
    info!("✅ Rollback completed: {} migrations processed", rolled_back);
    Ok(rolled_back)
}