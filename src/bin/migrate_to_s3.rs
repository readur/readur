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

use anyhow::Result;
use clap::Parser;
use std::path::Path;
use uuid::Uuid;
use tracing::{info, warn, error};

use readur::{
    config::Config,
    db::Database,
    services::{s3_service::S3Service, file_service::FileService},
};

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
    
    // Perform migration
    let mut migrated_count = 0;
    let mut failed_count = 0;
    
    for doc in local_documents {
        info!("📦 Migrating: {} ({})", doc.original_filename, doc.id);
        
        match migrate_document(&db, &s3_service, &file_service, &doc, args.delete_local).await {
            Ok(_) => {
                migrated_count += 1;
                info!("✅ Successfully migrated: {}", doc.original_filename);
            }
            Err(e) => {
                failed_count += 1;
                error!("❌ Failed to migrate {}: {}", doc.original_filename, e);
            }
        }
    }
    
    info!("🎉 Migration completed!");
    info!("✅ Successfully migrated: {} files", migrated_count);
    if failed_count > 0 {
        warn!("❌ Failed to migrate: {} files", failed_count);
    }
    
    Ok(())
}

async fn migrate_document(
    db: &Database,
    s3_service: &S3Service,
    file_service: &FileService,
    document: &readur::models::Document,
    delete_local: bool,
) -> Result<()> {
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
    migrate_associated_files(s3_service, file_service, document, delete_local).await?;
    
    // Delete local file if requested
    if delete_local {
        if let Err(e) = tokio::fs::remove_file(&local_path).await {
            warn!("Failed to delete local file {}: {}", document.file_path, e);
        } else {
            info!("🗑️  Deleted local file: {}", document.file_path);
        }
    }
    
    Ok(())
}

async fn migrate_associated_files(
    s3_service: &S3Service,
    file_service: &FileService,
    document: &readur::models::Document,
    delete_local: bool,
) -> Result<()> {
    
    // Migrate thumbnail
    let thumbnail_path = file_service.get_thumbnails_path().join(format!("{}_thumb.jpg", document.id));
    if thumbnail_path.exists() {
        match tokio::fs::read(&thumbnail_path).await {
            Ok(thumbnail_data) => {
                if let Err(e) = s3_service.store_thumbnail(document.user_id, document.id, &thumbnail_data).await {
                    warn!("Failed to migrate thumbnail for {}: {}", document.id, e);
                } else {
                    info!("📸 Migrated thumbnail for: {}", document.original_filename);
                    if delete_local {
                        let _ = tokio::fs::remove_file(&thumbnail_path).await;
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
                if let Err(e) = s3_service.store_processed_image(document.user_id, document.id, &processed_data).await {
                    warn!("Failed to migrate processed image for {}: {}", document.id, e);
                } else {
                    info!("🖼️  Migrated processed image for: {}", document.original_filename);
                    if delete_local {
                        let _ = tokio::fs::remove_file(&processed_path).await;
                    }
                }
            }
            Err(e) => warn!("Failed to read processed image {}: {}", processed_path.display(), e),
        }
    }
    
    Ok(())
}