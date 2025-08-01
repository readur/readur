/*!
 * CLI tool to enqueue documents with pending OCR status
 * 
 * This addresses the issue where documents marked as pending by migrations
 * are not automatically added to the OCR processing queue.
 * 
 * Usage: cargo run --bin enqueue_pending_ocr
 */

use anyhow::Result;
use sqlx::Row;
use tracing::{info, warn, error};
use uuid::Uuid;

use readur::{
    config::Config,
    db::Database,
    ocr::queue::OcrQueueService,
    services::file_service::FileService,
    storage::factory::create_storage_backend,
    storage::StorageConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🔍 Scanning for documents with pending OCR status...");
    
    // Load configuration
    let config = Config::from_env()?;
    
    // Connect to database
    let db = Database::new(&config.database_url).await?;
    
    // Create file service
    let storage_config = if config.s3_enabled {
        #[cfg(feature = "s3")]
        {
            StorageConfig::S3 { 
                s3_config: config.s3_config.as_ref().unwrap().clone(),
                fallback_path: Some(config.upload_path.clone()),
            }
        }
        #[cfg(not(feature = "s3"))]
        {
            StorageConfig::Local { upload_path: config.upload_path.clone() }
        }
    } else {
        StorageConfig::Local { upload_path: config.upload_path.clone() }
    };
    let storage_backend = create_storage_backend(storage_config).await?;
    let file_service = std::sync::Arc::new(FileService::with_storage(config.upload_path.clone(), storage_backend));
    
    let queue_service = OcrQueueService::new(db.clone(), db.get_pool().clone(), 1, file_service);
    
    // Find documents with pending OCR status that aren't in the queue
    let pending_documents = sqlx::query(
        r#"
        SELECT d.id, d.filename, d.file_size, d.mime_type, d.created_at
        FROM documents d
        LEFT JOIN ocr_queue oq ON d.id = oq.document_id
        WHERE d.ocr_status = 'pending'
          AND oq.document_id IS NULL
          AND d.file_path IS NOT NULL
          AND (d.mime_type LIKE 'image/%' OR d.mime_type = 'application/pdf' OR d.mime_type = 'text/plain')
        ORDER BY d.created_at ASC
        "#
    )
    .fetch_all(db.get_pool())
    .await?;
    
    if pending_documents.is_empty() {
        info!("✅ No pending documents found that need to be queued");
        return Ok(());
    }
    
    info!("📋 Found {} documents with pending OCR status", pending_documents.len());
    
    // Prepare batch insert data
    let mut documents_to_queue = Vec::new();
    
    for row in &pending_documents {
        let document_id: Uuid = row.get("id");
        let filename: String = row.get("filename");
        let file_size: i64 = row.get("file_size");
        let mime_type: String = row.get("mime_type");
        
        // Calculate priority based on file size
        let priority = match file_size {
            0..=1048576 => 10,          // <= 1MB: highest priority
            ..=5242880 => 8,            // 1-5MB: high priority  
            ..=10485760 => 6,           // 5-10MB: medium priority
            ..=52428800 => 4,           // 10-50MB: low priority
            _ => 2,                     // > 50MB: lowest priority
        };
        
        let size_mb = file_size as f64 / (1024.0 * 1024.0);
        info!("  📄 {} ({}) - {:.2} MB - Priority {}", 
              filename, mime_type, size_mb, priority);
        
        documents_to_queue.push((document_id, priority, file_size));
    }
    
    // Batch enqueue documents
    info!("🚀 Enqueuing {} documents for OCR processing...", documents_to_queue.len());
    
    match queue_service.enqueue_documents_batch(documents_to_queue).await {
        Ok(queue_ids) => {
            info!("✅ Successfully queued {} documents for OCR processing", queue_ids.len());
            info!("🔄 OCR worker should start processing these documents automatically");
            
            // Show queue statistics
            match queue_service.get_stats().await {
                Ok(stats) => {
                    info!("📊 Queue Statistics:");
                    info!("   • Pending: {}", stats.pending_count);
                    info!("   • Processing: {}", stats.processing_count);
                    info!("   • Failed: {}", stats.failed_count);
                    info!("   • Completed today: {}", stats.completed_today);
                    if let Some(wait_time) = stats.avg_wait_time_minutes {
                        info!("   • Average wait time: {:.1} minutes", wait_time);
                    }
                    if let Some(oldest) = stats.oldest_pending_minutes {
                        info!("   • Oldest pending: {:.1} minutes", oldest);
                    }
                }
                Err(e) => {
                    warn!("Failed to get queue statistics: {}", e);
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to enqueue documents: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}