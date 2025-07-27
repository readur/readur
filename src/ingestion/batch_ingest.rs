use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;
use chrono::{DateTime, Utc};

use crate::{
    config::Config,
    db::Database,
    services::file_service::FileService,
    ingestion::document_ingestion::{DocumentIngestionService, IngestionResult, DeduplicationPolicy},
    ocr::queue::OcrQueueService,
    models::FileIngestionInfo,
};

pub struct BatchIngester {
    db: Database,
    queue_service: OcrQueueService,
    file_service: FileService,
    config: Config,
    batch_size: usize,
    max_concurrent_io: usize,
}

impl BatchIngester {
    pub fn new(
        db: Database,
        queue_service: OcrQueueService,
        file_service: FileService,
        config: Config,
    ) -> Self {
        Self {
            db,
            queue_service,
            file_service,
            config,
            batch_size: 1000, // Process files in batches of 1000
            max_concurrent_io: 50, // Limit concurrent file I/O operations
        }
    }

    /// Ingest all files from a directory recursively
    pub async fn ingest_directory(&self, dir_path: &Path, user_id: Uuid) -> Result<()> {
        info!("Starting batch ingestion from directory: {:?}", dir_path);
        
        // Collect all file paths first
        let mut file_paths = Vec::new();
        for entry in WalkDir::new(dir_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path().to_path_buf();
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                
                if self.file_service.is_allowed_file_type(&filename, &self.config.allowed_file_types) {
                    file_paths.push(path);
                }
            }
        }
        
        info!("Found {} files to ingest", file_paths.len());
        
        // Process files in batches
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_io));
        let mut batch = Vec::new();
        let mut queue_items = Vec::new();
        
        for (idx, path) in file_paths.iter().enumerate() {
            let semaphore_clone = semaphore.clone();
            let path_clone = path.clone();
            let file_service = self.file_service.clone();
            let user_id_clone = user_id;
            
            // Process file asynchronously
            let db_clone = self.db.clone();
            let handle = tokio::spawn(async move {
                let permit = semaphore_clone.acquire().await.unwrap();
                let _permit = permit;
                process_single_file(path_clone, file_service, user_id_clone, db_clone).await
            });
            
            batch.push(handle);
            
            // When batch is full or we're at the end, process it
            if batch.len() >= self.batch_size || idx == file_paths.len() - 1 {
                info!("Processing batch of {} files", batch.len());
                
                // Wait for all files in batch to be processed
                for handle in batch.drain(..) {
                    match handle.await {
                        Ok(Ok(Some((doc_id, file_size)))) => {
                            let priority = calculate_priority(file_size);
                            queue_items.push((doc_id, priority, file_size));
                        }
                        Ok(Ok(None)) => {
                            // File was skipped
                        }
                        Ok(Err(e)) => {
                            error!("Error processing file: {}", e);
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                        }
                    }
                }
                
                // Batch insert documents into queue
                if !queue_items.is_empty() {
                    info!("Enqueueing {} documents for OCR", queue_items.len());
                    self.queue_service.enqueue_documents_batch(queue_items.clone()).await?;
                    queue_items.clear();
                }
                
                // Log progress
                info!("Progress: {}/{} files processed", idx + 1, file_paths.len());
            }
        }
        
        info!("Batch ingestion completed");
        Ok(())
    }

    /// Monitor ingestion progress
    pub async fn monitor_progress(&self) -> Result<()> {
        loop {
            let stats = self.queue_service.get_stats().await?;
            
            info!(
                "Queue Status - Pending: {}, Processing: {}, Failed: {}, Completed Today: {}",
                stats.pending_count,
                stats.processing_count,
                stats.failed_count,
                stats.completed_today
            );
            
            if let Some(avg_wait) = stats.avg_wait_time_minutes {
                info!("Average wait time: {:.2} minutes", avg_wait);
            }
            
            if let Some(oldest) = stats.oldest_pending_minutes {
                if oldest > 60.0 {
                    warn!("Oldest pending item: {:.2} hours", oldest / 60.0);
                } else {
                    info!("Oldest pending item: {:.2} minutes", oldest);
                }
            }
            
            if stats.pending_count == 0 && stats.processing_count == 0 {
                info!("All items processed!");
                break;
            }
            
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
        
        Ok(())
    }
}

/// Extract FileIngestionInfo from filesystem path and metadata
async fn extract_file_info_from_path(path: &Path) -> Result<FileIngestionInfo> {
    let metadata = fs::metadata(path).await?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    
    let file_size = metadata.len() as i64;
    let mime_type = mime_guess::from_path(&filename)
        .first_or_octet_stream()
        .to_string();
    
    // Extract timestamps
    let last_modified = metadata.modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_else(Utc::now));
    
    let created_at = metadata.created()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_else(Utc::now));
    
    // Extract Unix permissions (if on Unix-like system)
    #[cfg(unix)]
    let (permissions, owner, group) = {
        use std::os::unix::fs::MetadataExt;
        let permissions = Some(metadata.mode());
        
        // For now, just use uid/gid as strings (could be enhanced to resolve names later)
        let owner = Some(metadata.uid().to_string());
        let group = Some(metadata.gid().to_string());
            
        (permissions, owner, group)
    };
    
    // On non-Unix systems, permissions/owner/group are not available
    #[cfg(not(unix))]
    let (permissions, owner, group) = (None, None, None);
    
    Ok(FileIngestionInfo {
        relative_path: path.to_string_lossy().to_string(),
        full_path: path.to_string_lossy().to_string(), // For filesystem, relative and full are the same
        #[allow(deprecated)]
        path: path.to_string_lossy().to_string(),
        name: filename,
        size: file_size,
        mime_type,
        last_modified,
        etag: format!("{}-{}", file_size, last_modified.map_or(0, |t| t.timestamp())),
        is_directory: metadata.is_dir(),
        created_at,
        permissions,
        owner,
        group,
        metadata: None, // Could extract EXIF/other metadata in the future
    })
}

async fn process_single_file(
    path: PathBuf,
    file_service: FileService,
    user_id: Uuid,
    db: Database,
) -> Result<Option<(Uuid, i64)>> {
    // Extract basic file info first
    let mut file_info = extract_file_info_from_path(&path).await?;
    
    // Skip very large files (> 100MB)
    if file_info.size > 100 * 1024 * 1024 {
        warn!("Skipping large file: {} ({} MB)", file_info.name, file_info.size / 1024 / 1024);
        return Ok(None);
    }
    
    // Read file data
    let file_data = fs::read(&path).await?;
    
    // Extract content-based metadata
    if let Ok(Some(content_metadata)) = crate::metadata_extraction::extract_content_metadata(&file_data, &file_info.mime_type, &file_info.name).await {
        file_info.metadata = Some(content_metadata);
    }
    
    // Use the unified ingestion service with full metadata support
    let ingestion_service = DocumentIngestionService::new(db, file_service);
    
    let result = ingestion_service
        .ingest_from_file_info(&file_info, file_data, user_id, DeduplicationPolicy::Skip, "batch_ingest", None)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    match result {
        IngestionResult::Created(doc) => {
            info!("Created new document for batch file {}: {}", file_info.name, doc.id);
            Ok(Some((doc.id, file_info.size)))
        }
        IngestionResult::Skipped { existing_document_id, reason } => {
            info!("Skipped duplicate batch file {}: {} (existing: {})", file_info.name, reason, existing_document_id);
            Ok(None) // File was skipped due to deduplication
        }
        IngestionResult::ExistingDocument(doc) => {
            info!("Found existing document for batch file {}: {}", file_info.name, doc.id);
            Ok(None) // Don't re-queue for OCR
        }
        IngestionResult::TrackedAsDuplicate { existing_document_id } => {
            info!("Tracked batch file {} as duplicate of existing document: {}", file_info.name, existing_document_id);
            Ok(None) // File was tracked as duplicate
        }
    }
}

fn calculate_priority(file_size: i64) -> i32 {
    const MB: i64 = 1024 * 1024;
    const MB5: i64 = 5 * 1024 * 1024;
    const MB10: i64 = 10 * 1024 * 1024;
    const MB50: i64 = 50 * 1024 * 1024;
    
    match file_size {
        0..=MB => 10,           // <= 1MB: highest priority
        ..=MB5 => 8,            // 1-5MB: high priority
        ..=MB10 => 6,           // 5-10MB: medium priority
        ..=MB50 => 4,           // 10-50MB: low priority
        _ => 2,                 // > 50MB: lowest priority
    }
}

