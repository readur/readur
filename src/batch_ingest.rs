use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;
use sha2::{Sha256, Digest};

use crate::{
    config::Config,
    db::Database,
    file_service::FileService,
    ocr_queue::OcrQueueService,
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

async fn process_single_file(
    path: PathBuf,
    file_service: FileService,
    user_id: Uuid,
    db: Database,
) -> Result<Option<(Uuid, i64)>> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    
    // Read file metadata
    let metadata = fs::metadata(&path).await?;
    let file_size = metadata.len() as i64;
    
    // Skip very large files (> 100MB)
    if file_size > 100 * 1024 * 1024 {
        warn!("Skipping large file: {} ({} MB)", filename, file_size / 1024 / 1024);
        return Ok(None);
    }
    
    // Read file data
    let file_data = fs::read(&path).await?;
    
    // Calculate file hash for deduplication
    let file_hash = calculate_file_hash(&file_data);
    
    // Check for duplicate content using efficient hash lookup
    match db.get_document_by_user_and_hash(user_id, &file_hash).await {
        Ok(Some(existing_doc)) => {
            info!("Skipping duplicate file: {} matches existing document {} (hash: {})", 
                filename, existing_doc.original_filename, &file_hash[..8]);
            return Ok(None); // Skip processing duplicate
        }
        Ok(None) => {
            info!("No duplicate content found for hash {}, proceeding with file processing", &file_hash[..8]);
        }
        Err(e) => {
            warn!("Error checking for duplicate hash {}: {}", &file_hash[..8], e);
            // Continue processing even if duplicate check fails
        }
    }
    
    let mime_type = mime_guess::from_path(&filename)
        .first_or_octet_stream()
        .to_string();
    
    // Save file
    let file_path = file_service.save_file(&filename, &file_data).await?;
    
    // Create document with hash
    let document = file_service.create_document(
        &filename,
        &filename,
        &file_path,
        file_size,
        &mime_type,
        user_id,
        Some(file_hash),
    );
    
    // Save to database (without OCR)
    let created_doc = db.create_document(document).await?;
    
    Ok(Some((created_doc.id, file_size)))
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

fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}