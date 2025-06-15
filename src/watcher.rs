use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::{config::Config, db::Database, file_service::FileService, ocr_queue::OcrQueueService};

pub async fn start_folder_watcher(config: Config, db: Database) -> Result<()> {
    info!("Starting hybrid folder watcher on: {}", config.watch_folder);
    
    // Initialize services with shared database
    let file_service = FileService::new(config.upload_path.clone());
    let queue_service = OcrQueueService::new(db.clone(), db.get_pool().clone(), 1);
    
    // Determine watch strategy based on filesystem type
    let watch_path = Path::new(&config.watch_folder);
    let watch_strategy = determine_watch_strategy(watch_path).await?;
    
    info!("Using watch strategy: {:?}", watch_strategy);
    
    match watch_strategy {
        WatchStrategy::NotifyBased => {
            start_notify_watcher(config, db, file_service, queue_service).await
        }
        WatchStrategy::PollingBased => {
            start_polling_watcher(config, db, file_service, queue_service).await
        }
        WatchStrategy::Hybrid => {
            // Start both methods concurrently
            let config_clone = config.clone();
            let db_clone = db.clone();
            let file_service_clone = file_service.clone();
            let queue_service_clone = queue_service.clone();
            
            let notify_handle = tokio::spawn(async move {
                if let Err(e) = start_notify_watcher(config_clone, db_clone, file_service_clone, queue_service_clone).await {
                    warn!("Notify watcher failed, continuing with polling: {}", e);
                }
            });
            
            let polling_result = start_polling_watcher(config, db, file_service, queue_service).await;
            
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
    file_service: FileService,
    queue_service: OcrQueueService,
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

    watcher.watch(Path::new(&config.watch_folder), RecursiveMode::Recursive)?;
    
    info!("Started notify-based watcher on: {}", config.watch_folder);
    
    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                for path in event.paths {
                    if let Err(e) = process_file(&path, &db, &file_service, &queue_service, &config).await {
                        error!("Failed to process file {:?}: {}", path, e);
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
    file_service: FileService,
    queue_service: OcrQueueService,
) -> Result<()> {
    info!("Started polling-based watcher on: {}", config.watch_folder);
    
    let mut known_files: HashSet<(PathBuf, SystemTime)> = HashSet::new();
    let mut interval = interval(Duration::from_secs(config.watch_interval_seconds.unwrap_or(30)));
    
    // Initial scan
    scan_directory(&config.watch_folder, &mut known_files, &db, &file_service, &queue_service, &config).await?;
    
    loop {
        interval.tick().await;
        
        if let Err(e) = scan_directory(&config.watch_folder, &mut known_files, &db, &file_service, &queue_service, &config).await {
            error!("Error during directory scan: {}", e);
            // Continue polling even if one scan fails
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
            
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let file_info = (path.clone(), modified);
                    current_files.insert(file_info.clone());
                    
                    // Check if this is a new file or modified file
                    if !known_files.contains(&file_info) {
                        // Wait a bit to ensure file is fully written
                        if is_file_stable(&path).await {
                            debug!("Found new/modified file: {:?}", path);
                            if let Err(e) = process_file(&path, db, file_service, queue_service, config).await {
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

async fn is_file_stable(path: &Path) -> bool {
    // Check if file size is stable (not currently being written)
    if let Ok(metadata1) = tokio::fs::metadata(path).await {
        let size1 = metadata1.len();
        
        // Wait a short time
        sleep(Duration::from_millis(500)).await;
        
        if let Ok(metadata2) = tokio::fs::metadata(path).await {
            let size2 = metadata2.len();
            return size1 == size2;
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
    
    info!("Processing new file: {:?}", path);
    
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
    
    // Check if file is OCR-able
    if !is_ocr_able_file(&mime_type) {
        debug!("Skipping non-OCR-able file: {} ({})", filename, mime_type);
        return Ok(());  
    }
    
    // Check for duplicate files (same filename and size)
    if let Ok(existing_docs) = db.find_documents_by_filename(&filename).await {
        for doc in existing_docs {
            if doc.file_size == file_size {
                info!("Skipping duplicate file: {} (already exists with same size)", filename);
                return Ok(());
            }
        }
    }
    
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
    
    let saved_file_path = file_service.save_file(&filename, &file_data).await?;
    
    // Fetch admin user ID from database for watch folder documents
    let admin_user = db.get_user_by_username("admin").await?
        .ok_or_else(|| anyhow::anyhow!("Admin user not found. Please ensure the admin user is created."))?;
    let admin_user_id = admin_user.id;
    
    let document = file_service.create_document(
        &filename,
        &filename,
        &saved_file_path,
        file_size,
        &mime_type,
        admin_user_id,
    );
    
    let created_doc = db.create_document(document).await?;
    
    // Enqueue for OCR processing with priority based on file size and type
    let priority = calculate_priority(file_size, &mime_type);
    queue_service.enqueue_document(created_doc.id, priority, file_size).await?;
    
    info!("Successfully queued file for OCR: {} (size: {} bytes)", filename, file_size);
    
    Ok(())
}

fn is_ocr_able_file(mime_type: &str) -> bool {
    matches!(mime_type,
        "application/pdf" |
        "text/plain" |
        "image/png" | "image/jpeg" | "image/jpg" | "image/tiff" | "image/bmp" | "image/gif" |
        "application/msword" | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
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