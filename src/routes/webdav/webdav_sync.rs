use std::sync::Arc;
use std::path::Path;
use tracing::{error, info, warn};
use chrono::Utc;
use tokio::sync::Semaphore;
use futures::stream::{FuturesUnordered, StreamExt};

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
    
    info!("Processing file: {}", file_info.path);
    
    // Check if we've already processed this file
    match state.db.get_webdav_file_by_path(user_id, &file_info.path).await {
        Ok(Some(existing_file)) => {
            // Check if file has changed (compare ETags)
            if existing_file.etag == file_info.etag {
                info!("Skipping unchanged file: {} (ETag: {})", file_info.path, file_info.etag);
                return Ok(false); // Not processed (no change)
            }
            info!("File has changed: {} (old ETag: {}, new ETag: {})", 
                file_info.path, existing_file.etag, file_info.etag);
        }
        Ok(None) => {
            info!("New file found: {}", file_info.path);
        }
        Err(e) => {
            warn!("Error checking existing file {}: {}", file_info.path, e);
        }
    }
    
    // Download the file
    let file_data = webdav_service.download_file(&file_info.path).await
        .map_err(|e| format!("Failed to download {}: {}", file_info.path, e))?;
    
    info!("Downloaded file: {} ({} bytes)", file_info.name, file_data.len());
    
    // Create file service and save file to disk
    let file_service = FileService::new(state.config.upload_path.clone());
    
    let saved_file_path = file_service.save_file(&file_info.name, &file_data).await
        .map_err(|e| format!("Failed to save {}: {}", file_info.name, e))?;
    
    // Create document record
    let file_service = FileService::new(state.config.upload_path.clone());
    let document = file_service.create_document(
        &file_info.name,
        &file_info.name, // original filename same as name
        &saved_file_path,
        file_data.len() as i64,
        &file_info.mime_type,
        user_id,
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
        match state.db.pool.acquire().await {
            Ok(conn) => {
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
    }
    
    Ok(true) // Successfully processed
}