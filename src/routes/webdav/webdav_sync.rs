use std::sync::Arc;
use std::path::Path;
use tracing::{error, info, warn};
use chrono::Utc;

use crate::{
    AppState,
    models::{CreateWebDAVFile, UpdateWebDAVSyncState},
    ocr_queue::OcrQueueService,
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
                
                let mut folder_files_processed = 0;
                let files_to_process = files.len();
                
                for (idx, file_info) in files.into_iter().enumerate() {
                    if file_info.is_directory {
                        continue; // Skip directories
                    }
                    
                    // Check if file extension is supported
                    let file_extension = Path::new(&file_info.name)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    
                    if !config.file_extensions.contains(&file_extension) {
                        continue; // Skip unsupported file types
                    }
                    
                    // Check if we've already processed this file
                    match state.db.get_webdav_file_by_path(user_id, &file_info.path).await {
                        Ok(Some(existing_file)) => {
                            // Check if file has changed (compare ETags)
                            if existing_file.etag == file_info.etag {
                                info!("Skipping unchanged file: {} (ETag: {})", file_info.path, file_info.etag);
                                continue;
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
                    
                    // Update progress
                    let progress_update = UpdateWebDAVSyncState {
                        last_sync_at: Some(Utc::now()),
                        sync_cursor: None,
                        is_running: true,
                        files_processed: (total_files_processed + folder_files_processed) as i64,
                        files_remaining: (files_to_process - idx - 1) as i64,
                        current_folder: Some(folder_path.clone()),
                        errors: sync_errors.clone(),
                    };
                    
                    if let Err(e) = state.db.update_webdav_sync_state(user_id, &progress_update).await {
                        warn!("Failed to update sync progress: {}", e);
                    }
                    
                    // Download the file
                    match webdav_service.download_file(&file_info.path).await {
                        Ok(file_data) => {
                            info!("Downloaded file: {} ({} bytes)", file_info.name, file_data.len());
                            
                            // Create file service and save file to disk
                            let file_service = FileService::new(state.config.upload_path.clone());
                            
                            let saved_file_path = match file_service.save_file(&file_info.name, &file_data).await {
                                Ok(path) => path,
                                Err(e) => {
                                    error!("Failed to save file {}: {}", file_info.name, e);
                                    sync_errors.push(format!("Failed to save {}: {}", file_info.name, e));
                                    
                                    // Record failed file in database
                                    let failed_file = CreateWebDAVFile {
                                        user_id,
                                        webdav_path: file_info.path.clone(),
                                        etag: file_info.etag.clone(),
                                        last_modified: file_info.last_modified,
                                        file_size: file_info.size,
                                        mime_type: file_info.mime_type.clone(),
                                        document_id: None,
                                        sync_status: "failed".to_string(),
                                        sync_error: Some(e.to_string()),
                                    };
                                    
                                    if let Err(db_err) = state.db.create_or_update_webdav_file(&failed_file).await {
                                        error!("Failed to record failed file: {}", db_err);
                                    }
                                    
                                    continue;
                                }
                            };
                            
                            // Create document record
                            let document = file_service.create_document(
                                &file_info.name,
                                &file_info.name, // original filename same as name
                                &saved_file_path,
                                file_info.size,
                                &file_info.mime_type,
                                user_id,
                            );
                            
                            // Save document to database
                            match state.db.create_document(document).await {
                                Ok(saved_document) => {
                                    info!("Created document record: {} (ID: {})", file_info.name, saved_document.id);
                                    
                                    // Record successful file in WebDAV tracking
                                    let webdav_file = CreateWebDAVFile {
                                        user_id,
                                        webdav_path: file_info.path.clone(),
                                        etag: file_info.etag.clone(),
                                        last_modified: file_info.last_modified,
                                        file_size: file_info.size,
                                        mime_type: file_info.mime_type.clone(),
                                        document_id: Some(saved_document.id),
                                        sync_status: "completed".to_string(),
                                        sync_error: None,
                                    };
                                    
                                    if let Err(e) = state.db.create_or_update_webdav_file(&webdav_file).await {
                                        error!("Failed to record WebDAV file: {}", e);
                                    }
                                    
                                    // Add to OCR queue if enabled
                                    if enable_background_ocr {
                                        match sqlx::PgPool::connect(&state.config.database_url).await {
                                            Ok(pool) => {
                                                let queue_service = OcrQueueService::new(state.db.clone(), pool, 1);
                                                
                                                // Calculate priority based on file size
                                                let priority = match file_info.size {
                                                    0..=1048576 => 10,          // <= 1MB: highest priority
                                                    ..=5242880 => 8,            // 1-5MB: high priority
                                                    ..=10485760 => 6,           // 5-10MB: medium priority  
                                                    ..=52428800 => 4,           // 10-50MB: low priority
                                                    _ => 2,                     // > 50MB: lowest priority
                                                };
                                                
                                                if let Err(e) = queue_service.enqueue_document(saved_document.id, priority, file_info.size).await {
                                                    error!("Failed to enqueue document for OCR: {}", e);
                                                } else {
                                                    info!("Enqueued document {} for OCR processing", saved_document.id);
                                                }
                                            }
                                            Err(e) => {
                                                error!("Failed to connect to database for OCR queueing: {}", e);
                                            }
                                        }
                                    }
                                    
                                    folder_files_processed += 1;
                                }
                                Err(e) => {
                                    error!("Failed to create document record for {}: {}", file_info.name, e);
                                    sync_errors.push(format!("Failed to create document {}: {}", file_info.name, e));
                                    
                                    // Update WebDAV file status to failed
                                    let failed_file = CreateWebDAVFile {
                                        user_id,
                                        webdav_path: file_info.path.clone(),
                                        etag: file_info.etag.clone(),
                                        last_modified: file_info.last_modified,
                                        file_size: file_info.size,
                                        mime_type: file_info.mime_type.clone(),
                                        document_id: None,
                                        sync_status: "failed".to_string(),
                                        sync_error: Some(e.to_string()),
                                    };
                                    
                                    if let Err(db_err) = state.db.create_or_update_webdav_file(&failed_file).await {
                                        error!("Failed to record failed file: {}", db_err);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to download file {}: {}", file_info.path, e);
                            sync_errors.push(format!("Failed to download {}: {}", file_info.path, e));
                            
                            // Record download failure
                            let failed_file = CreateWebDAVFile {
                                user_id,
                                webdav_path: file_info.path.clone(),
                                etag: file_info.etag.clone(),
                                last_modified: file_info.last_modified,
                                file_size: file_info.size,
                                mime_type: file_info.mime_type.clone(),
                                document_id: None,
                                sync_status: "failed".to_string(),
                                sync_error: Some(format!("Download failed: {}", e)),
                            };
                            
                            if let Err(db_err) = state.db.create_or_update_webdav_file(&failed_file).await {
                                error!("Failed to record failed file: {}", db_err);
                            }
                        }
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