use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;
use chrono::{DateTime, Utc};

use crate::{
    config::Config, 
    db::Database, 
    services::{file_service::FileService, user_watch_service::UserWatchService}, 
    scheduling::user_watch_manager::UserWatchManager,
    ingestion::document_ingestion::{DocumentIngestionService, IngestionResult, DeduplicationPolicy},
    ocr::queue::OcrQueueService,
    models::FileIngestionInfo,
};

pub async fn start_folder_watcher(config: Config, db: Database, file_service: std::sync::Arc<FileService>) -> Result<()> {
    info!("Starting hybrid folder watcher on: {}", config.watch_folder);
    info!("Upload path configured as: {}", config.upload_path);
    
    if config.enable_per_user_watch {
        info!("Per-user watch directories enabled. Base directory: {}", config.user_watch_base_dir);
    }
    
    // Debug: Check if paths resolve correctly
    let watch_canonical = std::path::Path::new(&config.watch_folder).canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&config.watch_folder));
    let upload_canonical = std::path::Path::new(&config.upload_path).canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&config.upload_path));
    
    info!("Watch folder canonical path: {:?}", watch_canonical);
    info!("Upload folder canonical path: {:?}", upload_canonical);
    
    // Use the provided file service
    let queue_service = OcrQueueService::new(
        db.clone(),
        db.get_pool().clone(),
        1,
        file_service.clone(),
        config.max_pdf_size_mb,
        config.max_office_document_size_mb,
    );
    
    // Initialize user watch components if enabled
    let user_watch_manager = if config.enable_per_user_watch {
        let user_watch_service = UserWatchService::new(&config.user_watch_base_dir);
        let manager = UserWatchManager::new(db.clone(), user_watch_service);
        
        if let Err(e) = manager.initialize().await {
            error!("Failed to initialize user watch manager: {}", e);
            return Err(e);
        }
        
        Some(manager)
    } else {
        None
    };
    
    // Determine watch strategy based on filesystem type
    let watch_path = Path::new(&config.watch_folder);
    let watch_strategy = determine_watch_strategy(watch_path).await?;
    
    info!("Using watch strategy: {:?}", watch_strategy);
    
    match watch_strategy {
        WatchStrategy::NotifyBased => {
            start_notify_watcher(config, db, file_service, queue_service, user_watch_manager).await
        }
        WatchStrategy::PollingBased => {
            start_polling_watcher(config, db, file_service, queue_service, user_watch_manager).await
        }
        WatchStrategy::Hybrid => {
            // Start both methods concurrently
            let config_clone = config.clone();
            let db_clone = db.clone();
            let file_service_clone = file_service.clone();
            let queue_service_clone = queue_service.clone();
            let user_watch_manager_clone = user_watch_manager.clone();
            
            let notify_handle = tokio::spawn(async move {
                if let Err(e) = start_notify_watcher(config_clone, db_clone, file_service_clone, queue_service_clone, user_watch_manager_clone).await {
                    warn!("Notify watcher failed, continuing with polling: {}", e);
                }
            });
            
            let polling_result = start_polling_watcher(config, db, file_service, queue_service, user_watch_manager).await;
            
            // Cancel notify watcher if polling completes
            notify_handle.abort();
            
            polling_result
        }
    }
}

#[derive(Debug, Clone)]
enum WatchStrategy {
    NotifyBased,    // For local filesystems
    PollingBased,   // For network filesystems (NFS, SMB, S3, etc.)
    Hybrid,         // Try notify first, fall back to polling
}

async fn determine_watch_strategy(path: &Path) -> Result<WatchStrategy> {
    // Try to determine filesystem type
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If canonicalize fails, assume network filesystem
            return Ok(WatchStrategy::PollingBased);
        }
    };
    
    let path_str = canonical_path.to_string_lossy();
    
    // Check for common network filesystem patterns
    if path_str.starts_with("//") || 
       path_str.contains("nfs") || 
       path_str.contains("smb") || 
       path_str.contains("cifs") ||
       std::env::var("FORCE_POLLING_WATCH").is_ok() {
        return Ok(WatchStrategy::PollingBased);
    }
    
    // For local filesystems, use hybrid approach (notify with polling backup)
    Ok(WatchStrategy::Hybrid)
}

async fn start_notify_watcher(
    config: Config,
    db: Database,
    file_service: std::sync::Arc<FileService>,
    queue_service: OcrQueueService,
    user_watch_manager: Option<UserWatchManager>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("Failed to send file event: {}", e);
            }
        },
        notify::Config::default(),
    )?;

    // Watch the global watch folder
    watcher.watch(Path::new(&config.watch_folder), RecursiveMode::Recursive)?;
    info!("Started notify-based watcher on global folder: {}", config.watch_folder);
    
    // Also watch user watch directories if enabled
    if config.enable_per_user_watch {
        let user_watch_path = Path::new(&config.user_watch_base_dir);
        if user_watch_path.exists() {
            watcher.watch(user_watch_path, RecursiveMode::Recursive)?;
            info!("Started notify-based watcher on user watch folder: {}", config.user_watch_base_dir);
        } else {
            info!("User watch base directory does not exist yet: {}", config.user_watch_base_dir);
        }
    }
    
    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                for path in event.paths {
                    let stability_ms = config.file_stability_check_ms.unwrap_or(1000);
                    if is_file_stable(&path, stability_ms).await {
                        if let Err(e) = process_file(&path, &db, &file_service, &queue_service, &config, &user_watch_manager).await {
                            error!("Failed to process file {:?}: {}", path, e);
                        }
                    } else {
                        debug!("File {:?} not stable yet, skipping", path);
                    }
                }
            }
            Err(e) => error!("Watch error: {:?}", e),
        }
    }
    
    Ok(())
}

async fn start_polling_watcher(
    config: Config,
    db: Database,
    file_service: std::sync::Arc<FileService>,
    queue_service: OcrQueueService,
    user_watch_manager: Option<UserWatchManager>,
) -> Result<()> {
    info!("Started polling-based watcher on: {}", config.watch_folder);
    
    let mut known_files: HashSet<(PathBuf, SystemTime)> = HashSet::new();
    let mut interval = interval(Duration::from_secs(config.watch_interval_seconds.unwrap_or(30)));
    
    // Initial scan of global watch directory
    info!("Starting initial scan of global watch directory: {}", config.watch_folder);
    scan_directory(&config.watch_folder, &mut known_files, &db, &file_service, &queue_service, &config, &user_watch_manager).await?;
    
    // Initial scan of user watch directories if enabled
    if config.enable_per_user_watch {
        info!("Starting initial scan of user watch directories: {}", config.user_watch_base_dir);
        scan_directory(&config.user_watch_base_dir, &mut known_files, &db, &file_service, &queue_service, &config, &user_watch_manager).await?;
    }
    
    info!("Initial scan completed. Found {} files to track", known_files.len());
    
    loop {
        interval.tick().await;
        
        // Scan global watch directory
        if let Err(e) = scan_directory(&config.watch_folder, &mut known_files, &db, &file_service, &queue_service, &config, &user_watch_manager).await {
            error!("Error during global watch directory scan: {}", e);
            // Continue polling even if one scan fails
        }
        
        // Scan user watch directories if enabled
        if config.enable_per_user_watch {
            if let Err(e) = scan_directory(&config.user_watch_base_dir, &mut known_files, &db, &file_service, &queue_service, &config, &user_watch_manager).await {
                error!("Error during user watch directory scan: {}", e);
                // Continue polling even if one scan fails
            }
        }
    }
}

async fn scan_directory(
    watch_folder: &str,
    known_files: &mut HashSet<(PathBuf, SystemTime)>,
    db: &Database,
    file_service: &FileService,
    queue_service: &OcrQueueService,
    config: &Config,
    user_watch_manager: &Option<UserWatchManager>,
) -> Result<()> {
    let mut current_files: HashSet<(PathBuf, SystemTime)> = HashSet::new();
    
    // Walk directory and collect all files with their modification times
    for entry in WalkDir::new(watch_folder)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();
            debug!("Found file during scan: {:?}", path);
            
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let file_info = (path.clone(), modified);
                    current_files.insert(file_info.clone());
                    
                    // Check if this is a new file or modified file
                    if !known_files.contains(&file_info) {
                        // Wait a bit to ensure file is fully written
                        let stability_ms = config.file_stability_check_ms.unwrap_or(1000);
                        if is_file_stable(&path, stability_ms).await {
                            debug!("Found new/modified file: {:?}", path);
                            if let Err(e) = process_file(&path, db, file_service, queue_service, config, user_watch_manager).await {
                                error!("Failed to process file {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Update known files
    *known_files = current_files;
    
    Ok(())
}

async fn is_file_stable(path: &Path, stability_check_ms: u64) -> bool {
    // Check if file size AND modification time are stable (not currently being written)
    if let Ok(metadata1) = tokio::fs::metadata(path).await {
        let size1 = metadata1.len();
        let mtime1 = metadata1.modified().ok();

        // Wait for the configured stability check duration
        sleep(Duration::from_millis(stability_check_ms)).await;

        if let Ok(metadata2) = tokio::fs::metadata(path).await {
            let size2 = metadata2.len();
            let mtime2 = metadata2.modified().ok();
            // File is stable only if both size AND mtime are unchanged
            return size1 == size2 && mtime1 == mtime2;
        }
    }

    // If we can't read metadata, assume it's not stable
    false
}

async fn process_file(
    path: &std::path::Path,
    db: &Database,
    file_service: &FileService,
    queue_service: &OcrQueueService,
    config: &Config,
    user_watch_manager: &Option<UserWatchManager>,
) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    
    // Skip hidden files, temporary files, and system files
    if filename.starts_with('.') || 
       filename.starts_with('~') || 
       filename.ends_with(".tmp") ||
       filename.ends_with(".temp") ||
       filename.contains("$RECYCLE.BIN") ||
       filename.contains("System Volume Information") {
        debug!("Skipping system/temporary file: {}", filename);
        return Ok(());
    }
    
    if !file_service.is_allowed_file_type(&filename, &config.allowed_file_types) {
        debug!("Skipping file with disallowed type: {}", filename); 
        return Ok(());
    }
    
    // CRITICAL: Skip files that are in the upload directory - these are managed by WebDAV/manual uploads
    let path_str = path.to_string_lossy();
    let upload_path_normalized = std::path::Path::new(&config.upload_path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&config.upload_path));
    
    if let Ok(file_canonical) = path.canonicalize() {
        if file_canonical.starts_with(&upload_path_normalized) {
            debug!("Skipping file in upload directory (managed by WebDAV/manual upload): {}", filename);
            return Ok(());
        }
    }
    
    // Skip files that are not in either global watch directory or user watch directories
    let global_watch_canonical = std::path::Path::new(&config.watch_folder)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&config.watch_folder));
    let user_watch_canonical = if config.enable_per_user_watch {
        Some(std::path::Path::new(&config.user_watch_base_dir)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(&config.user_watch_base_dir)))
    } else {
        None
    };
    
    if let Ok(file_canonical) = path.canonicalize() {
        let in_global_watch = file_canonical.starts_with(&global_watch_canonical);
        let in_user_watch = user_watch_canonical
            .as_ref()
            .map(|user_watch| file_canonical.starts_with(user_watch))
            .unwrap_or(false);
        
        if !in_global_watch && !in_user_watch {
            debug!("Skipping file outside of watch directories: {}", filename);
            return Ok(());
        }
    }
    
    // Check file age if configured
    if let Some(max_age_hours) = config.max_file_age_hours {
        if let Ok(metadata) = tokio::fs::metadata(path).await {
            if let Ok(created) = metadata.created() {
                let age = SystemTime::now().duration_since(created).unwrap_or_default();
                if age.as_secs() > max_age_hours * 3600 {
                    debug!("Skipping old file: {} (age: {}h)", filename, age.as_secs() / 3600);
                    return Ok(());
                }
            }
        }
    }
    
    info!("Processing new file: {:?} (from watch directory: {})", path, config.watch_folder);
    
    let file_data = tokio::fs::read(path).await?;
    let file_size = file_data.len() as i64;
    
    // Skip very large files (> 500MB by default)
    const MAX_FILE_SIZE: i64 = 500 * 1024 * 1024;
    if file_size > MAX_FILE_SIZE {
        warn!("Skipping large file: {} ({} MB)", filename, file_size / 1024 / 1024);
        return Ok(());
    }
    
    // Skip empty files
    if file_size == 0 {
        debug!("Skipping empty file: {}", filename);
        return Ok(());
    }
    
    let mime_type = mime_guess::from_path(&filename)
        .first_or_octet_stream()
        .to_string();
    
    // Check if file can have text extracted (OCR or Office document text extraction)
    if !is_text_extractable_file(&mime_type) {
        debug!("Skipping non-text-extractable file: {} ({})", filename, mime_type);
        return Ok(());  
    }
    
    // Determine which user this file belongs to
    let target_user_id = if let Some(ref manager) = user_watch_manager {
        // Check if file is in user watch directory
        if manager.is_user_watch_path(path) {
            // Extract user from file path
            match manager.get_user_by_file_path(path).await? {
                Some(user) => {
                    info!("File {} belongs to user: {} ({})", filename, user.username, user.id);
                    user.id
                }
                None => {
                    warn!("File {} is in user watch directory but no user found - assigning to admin", filename);
                    // Fallback to admin
                    let admin_user = db.get_user_by_username("admin").await?
                        .ok_or_else(|| anyhow::anyhow!("Admin user not found. Please ensure the admin user is created."))?;
                    admin_user.id
                }
            }
        } else {
            // File is in global watch directory, assign to admin
            debug!("File {} is in global watch directory - assigning to admin", filename);
            let admin_user = db.get_user_by_username("admin").await?
                .ok_or_else(|| anyhow::anyhow!("Admin user not found. Please ensure the admin user is created."))?;
            admin_user.id
        }
    } else {
        // Per-user watch is disabled, always use admin
        debug!("Per-user watch disabled - assigning file {} to admin", filename);
        let admin_user = db.get_user_by_username("admin").await?
            .ok_or_else(|| anyhow::anyhow!("Admin user not found. Please ensure the admin user is created."))?;
        admin_user.id
    };
    
    // Validate PDF files before processing
    if mime_type == "application/pdf" {
        if !is_valid_pdf(&file_data) {
            warn!(
                "Skipping invalid PDF file: {} (size: {} bytes, header: {:?})",
                filename,
                file_data.len(),
                file_data.get(0..50).unwrap_or(&[]).iter().map(|&b| {
                    if b >= 32 && b <= 126 { b as char } else { '.' }
                }).collect::<String>()
            );
            return Ok(());
        }
    }
    
    // Extract basic file info first
    let mut file_info = extract_file_info_from_path(path).await?;
    
    // Extract content-based metadata
    if let Ok(Some(content_metadata)) = crate::metadata_extraction::extract_content_metadata(&file_data, &file_info.mime_type, &file_info.name).await {
        file_info.metadata = Some(content_metadata);
    }
    
    // Use the unified ingestion service for consistent deduplication
    let ingestion_service = DocumentIngestionService::new(db.clone(), file_service.clone());
    
    let result = ingestion_service
        .ingest_from_file_info(&file_info, file_data, target_user_id, DeduplicationPolicy::Skip, "watch_folder", None)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    match result {
        IngestionResult::Created(doc) => {
            info!("Created new document for watch folder file {}: {}", file_info.name, doc.id);
            
            // Enqueue for OCR processing with priority based on file size and type
            let priority = calculate_priority(file_info.size, &file_info.mime_type);
            queue_service.enqueue_document(doc.id, priority, file_info.size).await?;
            
            info!("Successfully queued file for OCR: {} (size: {} bytes)", file_info.name, file_info.size);
        }
        IngestionResult::Skipped { existing_document_id, reason } => {
            info!("Skipped duplicate watch folder file {}: {} (existing: {})", file_info.name, reason, existing_document_id);
        }
        IngestionResult::ExistingDocument(doc) => {
            info!("Found existing document for watch folder file {}: {} (not re-queuing for OCR)", file_info.name, doc.id);
        }
        IngestionResult::TrackedAsDuplicate { existing_document_id } => {
            info!("Tracked watch folder file {} as duplicate of existing document: {}", file_info.name, existing_document_id);
        }
    }
    
    Ok(())
}

/// Extract FileIngestionInfo from filesystem path and metadata (for watcher)
async fn extract_file_info_from_path(path: &Path) -> Result<FileIngestionInfo> {
    let metadata = tokio::fs::metadata(path).await?;
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
        let permissions = Some(metadata.mode() as u32);
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

fn is_ocr_able_file(mime_type: &str) -> bool {
    // Check mime types that are suitable for OCR processing (images and PDFs)
    matches!(mime_type,
        "application/pdf" | 
        "image/png" | "image/jpeg" | "image/jpg" | 
        "image/tiff" | "image/bmp" | "image/gif"
    )
}

fn is_text_extractable_file(mime_type: &str) -> bool {
    // Check mime types that support text extraction (OCR + Office documents + plain text)
    matches!(mime_type,
        // OCR-able files
        "application/pdf" | 
        "image/png" | "image/jpeg" | "image/jpg" | 
        "image/tiff" | "image/bmp" | "image/gif" |
        // Plain text
        "text/plain" |
        // Office document formats
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" | // DOCX
        "application/msword" |                                                      // DOC
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" |      // XLSX
        "application/vnd.ms-excel" |                                                // XLS  
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" // PPTX (for future)
    )
}

/// Calculate priority based on file size and type (smaller files and images get higher priority)
fn calculate_priority(file_size: i64, mime_type: &str) -> i32 {
    const MB: i64 = 1024 * 1024;
    const MB5: i64 = 5 * 1024 * 1024;
    const MB10: i64 = 10 * 1024 * 1024;
    const MB50: i64 = 50 * 1024 * 1024;
    
    let base_priority = match file_size {
        0..=MB => 10,           // <= 1MB: highest priority
        ..=MB5 => 8,            // 1-5MB: high priority  
        ..=MB10 => 6,           // 5-10MB: medium priority
        ..=MB50 => 4,           // 10-50MB: low priority
        _ => 2,                 // > 50MB: lowest priority
    };
    
    // Boost priority for images (usually faster to OCR)
    let type_boost = if mime_type.starts_with("image/") {
        2
    } else if mime_type == "text/plain" {
        1
    } else {
        0
    };
    
    (base_priority + type_boost).min(10)
}

/// Check if the given bytes represent a valid PDF file
/// Handles PDFs with leading null bytes or whitespace
fn is_valid_pdf(data: &[u8]) -> bool {
    if data.len() < 5 {
        return false;
    }
    
    // Find the first occurrence of "%PDF-" in the first 1KB of the file
    // Some PDFs have leading null bytes or other metadata
    let search_limit = data.len().min(1024);
    let search_data = &data[0..search_limit];
    
    for i in 0..=search_limit.saturating_sub(5) {
        if &search_data[i..i+5] == b"%PDF-" {
            return true;
        }
    }
    
    false
}

/// Remove leading null bytes and return clean PDF data
/// Returns the original data if no PDF header is found
fn clean_pdf_data(data: &[u8]) -> &[u8] {
    if data.len() < 5 {
        return data;
    }
    
    // Find the first occurrence of "%PDF-" in the first 1KB
    let search_limit = data.len().min(1024);
    
    for i in 0..=search_limit.saturating_sub(5) {
        if &data[i..i+5] == b"%PDF-" {
            return &data[i..];
        }
    }
    
    // If no PDF header found, return original data
    data
}

