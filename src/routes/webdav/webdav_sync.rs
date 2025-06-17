use std::sync::Arc;
use std::path::Path;
use tracing::{error, info, warn};
use chrono::Utc;
use tokio::sync::Semaphore;
use futures::stream::{FuturesUnordered, StreamExt};
use sha2::{Sha256, Digest};

use crate::{
    AppState,
    models::{CreateWebDAVFile, UpdateWebDAVSyncState},
    file_service::FileService,
    webdav_service::{WebDAVConfig, WebDAVService},
};

pub async fn perform_webdav_sync_with_tracking(
    state: Arc<AppState>,
    user_id: uuid::Uuid,
    webdav_service: WebDAVService,
    config: WebDAVConfig,
    enable_background_ocr: bool,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    info!("Performing WebDAV sync for user {} on {} folders", user_id, config.watch_folders.len());
    
    // Update sync state to running
    let sync_state_update = UpdateWebDAVSyncState {
        last_sync_at: Some(Utc::now()),
        sync_cursor: None,
        is_running: true,
        files_processed: 0,
        files_remaining: 0,
        current_folder: None,
        errors: Vec::new(),
    };
    
    if let Err(e) = state.db.update_webdav_sync_state(user_id, &sync_state_update).await {
        error!("Failed to update sync state: {}", e);
    }

    // Ensure sync state is cleared on any exit path
    let cleanup_sync_state = |errors: Vec<String>, files_processed: usize| {
        let state_clone = state.clone();
        let user_id_clone = user_id;
        tokio::spawn(async move {
            let final_state = UpdateWebDAVSyncState {
                last_sync_at: Some(Utc::now()),
                sync_cursor: None,
                is_running: false,
                files_processed: files_processed as i64,
                files_remaining: 0,
                current_folder: None,
                errors,
            };
            
            if let Err(e) = state_clone.db.update_webdav_sync_state(user_id_clone, &final_state).await {
                error!("Failed to cleanup sync state: {}", e);
            }
        });
    };

    // Perform sync with proper cleanup
    let sync_result = perform_sync_internal(state.clone(), user_id, webdav_service, config, enable_background_ocr).await;
    
    match &sync_result {
        Ok(files_processed) => {
            cleanup_sync_state(Vec::new(), *files_processed);
        }
        Err(e) => {
            let error_msg = format!("Sync failed: {}", e);
            cleanup_sync_state(vec![error_msg], 0);
        }
    }
    
    sync_result
}

async fn perform_sync_internal(
    state: Arc<AppState>,
    user_id: uuid::Uuid,
    webdav_service: WebDAVService,
    config: WebDAVConfig,
    enable_background_ocr: bool,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    
    let mut total_files_processed = 0;
    let mut sync_errors = Vec::new();
    
    // Process each watch folder
    for folder_path in &config.watch_folders {
        // Check if sync has been cancelled before processing each folder
        if let Ok(Some(sync_state)) = state.db.get_webdav_sync_state(user_id).await {
            if !sync_state.is_running {
                info!("WebDAV sync cancelled, stopping folder processing");
                return Err("Sync cancelled by user".into());
            }
        }
        
        info!("Syncing folder: {}", folder_path);
        
        // Update current folder in sync state
        let folder_update = UpdateWebDAVSyncState {
            last_sync_at: Some(Utc::now()),
            sync_cursor: None,
            is_running: true,
            files_processed: total_files_processed as i64,
            files_remaining: 0,
            current_folder: Some(folder_path.clone()),
            errors: sync_errors.clone(),
        };
        
        if let Err(e) = state.db.update_webdav_sync_state(user_id, &folder_update).await {
            warn!("Failed to update sync folder state: {}", e);
        }
        
        // Discover files in the folder
        match webdav_service.discover_files_in_folder(folder_path).await {
            Ok(files) => {
                info!("Found {} files in folder {}", files.len(), folder_path);
                
                // Filter files for processing
                let files_to_process: Vec<_> = files.into_iter()
                    .filter(|file_info| {
                        // Skip directories
                        if file_info.is_directory {
                            return false;
                        }
                        
                        // Check if file extension is supported
                        let file_extension = Path::new(&file_info.name)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        
                        config.file_extensions.contains(&file_extension)
                    })
                    .collect();
                
                info!("Processing {} files from folder {}", files_to_process.len(), folder_path);
                
                // Process files concurrently with a limit to avoid overwhelming the system
                let concurrent_limit = 5; // Max 5 concurrent downloads
                let semaphore = Arc::new(Semaphore::new(concurrent_limit));
                let mut folder_files_processed = 0;
                
                // Create futures for processing each file concurrently
                let mut file_futures = FuturesUnordered::new();
                
                for file_info in files_to_process.iter() {
                    let state_clone = state.clone();
                    let webdav_service_clone = webdav_service.clone();
                    let file_info_clone = file_info.clone();
                    let semaphore_clone = semaphore.clone();
                    
                    // Create a future for processing this file
                    let future = async move {
                        process_single_file(
                            state_clone,
                            user_id,
                            &webdav_service_clone,
                            &file_info_clone,
                            enable_background_ocr,
                            semaphore_clone,
                        ).await
                    };
                    
                    file_futures.push(future);
                }
                
                // Process files concurrently and collect results
                while let Some(result) = file_futures.next().await {
                    // Check if sync has been cancelled
                    if let Ok(Some(sync_state)) = state.db.get_webdav_sync_state(user_id).await {
                        if !sync_state.is_running {
                            info!("WebDAV sync cancelled during file processing, stopping");
                            // Cancel remaining futures
                            file_futures.clear();
                            sync_errors.push("Sync cancelled by user during file processing".to_string());
                            break;
                        }
                    }
                    
                    match result {
                        Ok(processed) => {
                            if processed {
                                folder_files_processed += 1;
                                info!("Successfully processed file ({} completed in this folder)", folder_files_processed);
                            }
                        }
                        Err(error) => {
                            error!("File processing error: {}", error);
                            sync_errors.push(error);
                        }
                    }
                    
                    // Update progress periodically
                    let progress_update = UpdateWebDAVSyncState {
                        last_sync_at: Some(Utc::now()),
                        sync_cursor: None,
                        is_running: true,
                        files_processed: (total_files_processed + folder_files_processed) as i64,
                        files_remaining: (files_to_process.len() - folder_files_processed) as i64,
                        current_folder: Some(folder_path.clone()),
                        errors: sync_errors.clone(),
                    };
                    
                    if let Err(e) = state.db.update_webdav_sync_state(user_id, &progress_update).await {
                        warn!("Failed to update sync progress: {}", e);
                    }
                }
                
                total_files_processed += folder_files_processed;
            }
            Err(e) => {
                error!("Failed to discover files in folder {}: {}", folder_path, e);
                sync_errors.push(format!("Failed to list folder {}: {}", folder_path, e));
            }
        }
    }
    
    info!("WebDAV sync completed for user {}: {} files processed", user_id, total_files_processed);
    Ok(total_files_processed)
}

// Helper function to process a single file asynchronously
async fn process_single_file(
    state: Arc<AppState>,
    user_id: uuid::Uuid,
    webdav_service: &WebDAVService,
    file_info: &crate::models::FileInfo,
    enable_background_ocr: bool,
    semaphore: Arc<Semaphore>,
) -> Result<bool, String> {
    // Acquire semaphore permit to limit concurrent downloads
    let _permit = semaphore.acquire().await.map_err(|e| format!("Semaphore error: {}", e))?;
    
    // Check if sync has been cancelled before processing this file
    if let Ok(Some(sync_state)) = state.db.get_webdav_sync_state(user_id).await {
        if !sync_state.is_running {
            info!("Sync cancelled, skipping file: {}", file_info.path);
            return Err("Sync cancelled by user".to_string());
        }
    }
    
    info!("Processing file: {}", file_info.path);
    
    // Check if we've already processed this file
    info!("Checking WebDAV tracking for: {}", file_info.path);
    match state.db.get_webdav_file_by_path(user_id, &file_info.path).await {
        Ok(Some(existing_file)) => {
            info!("Found existing WebDAV file record: {} (current ETag: {}, remote ETag: {})", 
                file_info.path, existing_file.etag, file_info.etag);
            
            // Check if file has changed (compare ETags)
            if existing_file.etag == file_info.etag {
                info!("Skipping unchanged WebDAV file: {} (ETag: {})", file_info.path, file_info.etag);
                return Ok(false); // Not processed (no change)
            }
            info!("WebDAV file has changed: {} (old ETag: {}, new ETag: {})", 
                file_info.path, existing_file.etag, file_info.etag);
        }
        Ok(None) => {
            info!("New WebDAV file detected: {}", file_info.path);
        }
        Err(e) => {
            warn!("Error checking existing WebDAV file {}: {}", file_info.path, e);
        }
    }
    
    // Download the file
    let file_data = webdav_service.download_file(&file_info.path).await
        .map_err(|e| format!("Failed to download {}: {}", file_info.path, e))?;
    
    info!("Downloaded file: {} ({} bytes)", file_info.name, file_data.len());
    
    // Calculate file hash for deduplication 
    let file_hash = calculate_file_hash(&file_data);
    
    // Check if this exact file content already exists for this user using efficient hash lookup
    info!("Checking for duplicate content for user {}: {} (hash: {}, size: {} bytes)", 
        user_id, file_info.name, &file_hash[..8], file_data.len());
    
    // Use efficient database hash lookup instead of reading all documents
    match state.db.get_document_by_user_and_hash(user_id, &file_hash).await {
        Ok(Some(existing_doc)) => {
            info!("Found duplicate content for user {}: {} matches existing document {} (hash: {})", 
                user_id, file_info.name, existing_doc.original_filename, &file_hash[..8]);
            
            // Record this WebDAV file as a duplicate but link to existing document
            let webdav_file = CreateWebDAVFile {
                user_id,
                webdav_path: file_info.path.clone(),
                etag: file_info.etag.clone(),
                last_modified: file_info.last_modified,
                file_size: file_info.size,
                mime_type: file_info.mime_type.clone(),
                document_id: Some(existing_doc.id), // Link to existing document
                sync_status: "duplicate_content".to_string(),
                sync_error: None,
            };
            
            if let Err(e) = state.db.create_or_update_webdav_file(&webdav_file).await {
                error!("Failed to record duplicate WebDAV file: {}", e);
            }
            
            info!("WebDAV file marked as duplicate_content, skipping processing");
            return Ok(false); // Not processed (duplicate)
        }
        Ok(None) => {
            info!("No duplicate content found for hash {}, proceeding with file processing", &file_hash[..8]);
        }
        Err(e) => {
            warn!("Error checking for duplicate hash {}: {}", &file_hash[..8], e);
            // Continue processing even if duplicate check fails
        }
    }
    
    // Create file service and save file to disk
    let file_service = FileService::new(state.config.upload_path.clone());
    
    let saved_file_path = file_service.save_file(&file_info.name, &file_data).await
        .map_err(|e| format!("Failed to save {}: {}", file_info.name, e))?;
    
    // Create document record with hash
    let file_service = FileService::new(state.config.upload_path.clone());
    let document = file_service.create_document(
        &file_info.name,
        &file_info.name, // original filename same as name
        &saved_file_path,
        file_data.len() as i64,
        &file_info.mime_type,
        user_id,
        Some(file_hash.clone()), // Store the calculated hash
    );
    
    // Save document to database
    let created_document = state.db.create_document(document)
        .await
        .map_err(|e| format!("Failed to create document {}: {}", file_info.name, e))?;
    
    info!("Created document record for {}: {}", file_info.name, created_document.id);
    
    // Record successful file in WebDAV files table
    let webdav_file = CreateWebDAVFile {
        user_id,
        webdav_path: file_info.path.clone(),
        etag: file_info.etag.clone(),
        last_modified: file_info.last_modified,
        file_size: file_info.size,
        mime_type: file_info.mime_type.clone(),
        document_id: Some(created_document.id),
        sync_status: "synced".to_string(),
        sync_error: None,
    };
    
    if let Err(e) = state.db.create_or_update_webdav_file(&webdav_file).await {
        error!("Failed to record WebDAV file: {}", e);
    }
    
    // Queue for OCR processing if enabled
    if enable_background_ocr {
        info!("Background OCR is enabled, queueing document {} for processing", created_document.id);
        
        match state.db.pool.acquire().await {
            Ok(_conn) => {
                let queue_service = crate::ocr_queue::OcrQueueService::new(
                    state.db.clone(), 
                    state.db.pool.clone(), 
                    4
                );
                
                // Determine priority based on file size
                let priority = if file_info.size <= 1024 * 1024 { 10 } // ≤ 1MB: High priority
                else if file_info.size <= 5 * 1024 * 1024 { 8 } // ≤ 5MB: Medium priority  
                else if file_info.size <= 10 * 1024 * 1024 { 6 } // ≤ 10MB: Normal priority
                else if file_info.size <= 50 * 1024 * 1024 { 4 } // ≤ 50MB: Low priority
                else { 2 }; // > 50MB: Lowest priority
                
                if let Err(e) = queue_service.enqueue_document(created_document.id, priority, file_info.size).await {
                    error!("Failed to enqueue document for OCR: {}", e);
                } else {
                    info!("Enqueued document {} for OCR processing", created_document.id);
                }
            }
            Err(e) => {
                error!("Failed to connect to database for OCR queueing: {}", e);
            }
        }
    } else {
        info!("Background OCR is disabled, skipping OCR queue for document {}", created_document.id);
    }
    
    Ok(true) // Successfully processed
}

fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}