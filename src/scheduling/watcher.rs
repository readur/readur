use anyhow::Result;
use chrono::{DateTime, Utc};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn, Level};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    config::Config,
    db::Database,
    ingestion::document_ingestion::{
        DeduplicationPolicy, DocumentIngestionService, IngestionResult,
    },
    models::FileIngestionInfo,
    ocr::queue::{EnqueueOutcome, OcrQueueService},
    scheduling::{
        user_watch_manager::UserWatchManager,
        watch_filter::{
            age_since, calculate_priority, header_preview, is_system_or_temp_file,
            is_text_extractable_file, is_too_old, is_unchanged, is_valid_pdf, size_skip,
            FileLocation, SizeSkip, WatchRoots,
        },
        watch_owner::WatchFolderOwner,
    },
    services::{file_service::FileService, user_watch_service::UserWatchService},
};

pub async fn start_folder_watcher(
    config: Config,
    db: Database,
    file_service: std::sync::Arc<FileService>,
) -> Result<()> {
    info!("Starting hybrid folder watcher on: {}", config.watch_folder);
    info!("Upload path configured as: {}", config.upload_path);

    if config.enable_per_user_watch {
        info!(
            "Per-user watch directories enabled. Base directory: {}",
            config.user_watch_base_dir
        );
    }

    // Debug: Check if paths resolve correctly
    let watch_canonical = std::path::Path::new(&config.watch_folder)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&config.watch_folder));
    let upload_canonical = std::path::Path::new(&config.upload_path)
        .canonicalize()
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
        config.ocr_timeout_seconds,
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

    let owner = WatchFolderOwner::from_env();
    info!(
        "Global watch folder files are assigned to the admin account '{}' (ADMIN_USERNAME)",
        owner.username()
    );
    let ctx = WatchContext {
        config,
        db,
        file_service,
        queue_service,
        user_watch_manager,
        owner,
    };

    match watch_strategy {
        WatchStrategy::PollingBased => start_polling_watcher(ctx).await,
        WatchStrategy::Hybrid => {
            // Start both methods concurrently
            let notify_ctx = ctx.clone();
            let notify_handle = tokio::spawn(async move {
                if let Err(e) = start_notify_watcher(notify_ctx).await {
                    warn!("Notify watcher failed, continuing with polling: {}", e);
                }
            });

            let polling_result = start_polling_watcher(ctx).await;

            // Cancel notify watcher if polling completes
            notify_handle.abort();

            polling_result
        }
    }
}

/// Everything [`process_file`] needs, built once per watcher.
#[derive(Clone)]
pub struct WatchContext {
    pub config: Config,
    pub db: Database,
    pub file_service: std::sync::Arc<FileService>,
    pub queue_service: OcrQueueService,
    /// Set when per-user watch directories are enabled.
    pub user_watch_manager: Option<UserWatchManager>,
    /// Owner of files from the global watch folder.
    pub owner: WatchFolderOwner,
}

#[derive(Debug, Clone)]
enum WatchStrategy {
    PollingBased, // For network filesystems (NFS, SMB, S3, etc.)
    Hybrid,       // Local filesystems: notify, with polling as a backup
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
    if path_str.starts_with("//")
        || path_str.contains("nfs")
        || path_str.contains("smb")
        || path_str.contains("cifs")
        || std::env::var("FORCE_POLLING_WATCH").is_ok()
    {
        return Ok(WatchStrategy::PollingBased);
    }

    // For local filesystems, use hybrid approach (notify with polling backup)
    Ok(WatchStrategy::Hybrid)
}

async fn start_notify_watcher(ctx: WatchContext) -> Result<()> {
    let config = &ctx.config;
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
    info!(
        "Started notify-based watcher on global folder: {}",
        config.watch_folder
    );

    // Also watch user watch directories if enabled
    if config.enable_per_user_watch {
        let user_watch_path = Path::new(&config.user_watch_base_dir);
        if user_watch_path.exists() {
            watcher.watch(user_watch_path, RecursiveMode::Recursive)?;
            info!(
                "Started notify-based watcher on user watch folder: {}",
                config.user_watch_base_dir
            );
        } else {
            info!(
                "User watch base directory does not exist yet: {}",
                config.user_watch_base_dir
            );
        }
    }

    let watch_interval = Duration::from_secs(config.effective_watch_interval_seconds());
    let mut known_files: HashMap<PathBuf, SystemTime> = HashMap::new();
    let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                if !is_relevant_event(&event.kind) {
                    debug!("Ignoring irrelevant event: {:?}", event.kind);
                    continue;
                }

                for path in event.paths {
                    // Debounce: skip if processed within the watch interval
                    if let Some(&last) = last_processed.get(&path) {
                        let elapsed = last.elapsed();
                        if elapsed < watch_interval {
                            debug!(
                                "Debouncing file {:?} (processed {:.1}s ago)",
                                path,
                                elapsed.as_secs_f64()
                            );
                            continue;
                        }
                    }

                    // Mtime dedup: skip if mtime hasn't changed
                    let current_mtime = tokio::fs::metadata(&path)
                        .await
                        .ok()
                        .and_then(|m| m.modified().ok());

                    if let Some(mtime) = current_mtime {
                        if known_files.get(&path) == Some(&mtime) {
                            debug!("Skipping unchanged file {:?} (mtime unchanged)", path);
                            continue;
                        }
                    }

                    let stability_ms = config.effective_file_stability_check_ms();
                    if is_file_stable(&path, stability_ms).await {
                        if let Err(e) = process_file(&path, &ctx).await {
                            error!("Failed to process file {:?}: {}", path, e);
                        } else {
                            // Update tracking on successful processing
                            if let Some(mtime) = current_mtime {
                                known_files.insert(path.clone(), mtime);
                            }
                            last_processed.insert(path, Instant::now());
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

async fn start_polling_watcher(ctx: WatchContext) -> Result<()> {
    let config = &ctx.config;
    info!("Started polling-based watcher on: {}", config.watch_folder);

    let mut known_files: HashSet<(PathBuf, SystemTime)> = HashSet::new();
    let mut interval = interval(Duration::from_secs(
        config.effective_watch_interval_seconds(),
    ));

    // Initial scan of global watch directory
    info!(
        "Starting initial scan of global watch directory: {}",
        config.watch_folder
    );
    scan_directory(&config.watch_folder, &mut known_files, &ctx).await?;

    // Initial scan of user watch directories if enabled
    if config.enable_per_user_watch {
        info!(
            "Starting initial scan of user watch directories: {}",
            config.user_watch_base_dir
        );
        scan_directory(&config.user_watch_base_dir, &mut known_files, &ctx).await?;
    }

    info!(
        "Initial scan completed. Found {} files to track",
        known_files.len()
    );

    loop {
        interval.tick().await;

        // Scan global watch directory
        if let Err(e) = scan_directory(&config.watch_folder, &mut known_files, &ctx).await {
            error!("Error during global watch directory scan: {}", e);
            // Continue polling even if one scan fails
        }

        // Scan user watch directories if enabled
        if config.enable_per_user_watch {
            if let Err(e) =
                scan_directory(&config.user_watch_base_dir, &mut known_files, &ctx).await
            {
                error!("Error during user watch directory scan: {}", e);
                // Continue polling even if one scan fails
            }
        }
    }
}

async fn scan_directory(
    watch_folder: &str,
    known_files: &mut HashSet<(PathBuf, SystemTime)>,
    ctx: &WatchContext,
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
                        let stability_ms = ctx.config.effective_file_stability_check_ms();
                        if is_file_stable(&path, stability_ms).await {
                            debug!("Found new/modified file: {:?}", path);
                            if let Err(e) = process_file(&path, ctx).await {
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
    let Some(before) = size_and_mtime(path).await else {
        // If we can't read metadata, assume it's not stable
        return false;
    };

    // Wait for the configured stability check duration
    sleep(Duration::from_millis(stability_check_ms)).await;

    size_and_mtime(path)
        .await
        .is_some_and(|after| is_unchanged(before, after))
}

async fn size_and_mtime(path: &Path) -> Option<(u64, Option<SystemTime>)> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    Some((metadata.len(), metadata.modified().ok()))
}

/// Filter notify events to only process relevant filesystem changes.
/// Allows: file creation, content modification, renames, and ambiguous events.
/// Rejects: access-only, removal, and metadata-only changes.
fn is_relevant_event(kind: &EventKind) -> bool {
    use notify::event::ModifyKind;

    matches!(
        kind,
        EventKind::Create(_)
            | EventKind::Modify(ModifyKind::Data(_))
            | EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Modify(ModifyKind::Any)
            | EventKind::Any
    )
}

/// Ingest one file from a watch directory and queue it for OCR. Files that
/// should not be ingested (hidden, disallowed type, too large, no owner,
/// ...) are skipped with `Ok(())`.
pub async fn process_file(path: &std::path::Path, ctx: &WatchContext) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if !is_eligible_before_read(path, &filename, ctx).await {
        return Ok(());
    }

    info!(
        "Processing file: {:?} (from watch directory: {})",
        path, ctx.config.watch_folder
    );

    let file_data = tokio::fs::read(path).await?;
    let Some(mime_type) = extractable_mime_type(&filename, &file_data) else {
        return Ok(());
    };

    let Some(target_user_id) = file_owner(path, &filename, ctx).await? else {
        warn!("Skipping watch folder file {} - it has no owner", filename);
        return Ok(());
    };

    // Validate PDF files before processing
    if mime_type == "application/pdf" && !is_valid_pdf(&file_data) {
        warn!(
            "Skipping invalid PDF file: {} (size: {} bytes, header: {:?})",
            filename,
            file_data.len(),
            header_preview(&file_data)
        );
        return Ok(());
    }

    ingest_and_queue(path, file_data, target_user_id, ctx).await
}

/// The checks that need no file contents: name, type, location and age.
/// `false` (with the reason logged) when the file is to be skipped.
async fn is_eligible_before_read(path: &Path, filename: &str, ctx: &WatchContext) -> bool {
    let config = &ctx.config;

    // Skip hidden files, temporary files, and system files
    if is_system_or_temp_file(filename) {
        debug!("Skipping system/temporary file: {}", filename);
        return false;
    }

    if !ctx
        .file_service
        .is_allowed_file_type(filename, &config.allowed_file_types)
    {
        debug!("Skipping file with disallowed type: {}", filename);
        return false;
    }

    is_in_watched_location(path, filename, config) && !is_past_max_age(path, filename, config).await
}

/// Files in the upload directory are managed by WebDAV/manual uploads, and
/// files outside every watch directory are not ours. A file whose path
/// cannot be canonicalised is given the benefit of the doubt.
fn is_in_watched_location(path: &Path, filename: &str, config: &Config) -> bool {
    let Ok(file_canonical) = path.canonicalize() else {
        return true;
    };
    match WatchRoots::from_config(config).classify(&file_canonical) {
        FileLocation::UploadDir => {
            debug!(
                "Skipping file in upload directory (managed by WebDAV/manual upload): {}",
                filename
            );
            false
        }
        FileLocation::OutsideWatchDirs => {
            debug!("Skipping file outside of watch directories: {}", filename);
            false
        }
        FileLocation::WatchDir => true,
    }
}

/// Whether the file is older than `max_file_age_hours`, when that is set and
/// the file's creation time is readable.
async fn is_past_max_age(path: &Path, filename: &str, config: &Config) -> bool {
    let Some(max_age_hours) = config.max_file_age_hours else {
        return false;
    };
    let Ok(created) = tokio::fs::metadata(path).await.and_then(|m| m.created()) else {
        return false;
    };
    let age = age_since(created, SystemTime::now());
    if !is_too_old(age, max_age_hours) {
        return false;
    }
    debug!(
        "Skipping old file: {} (age: {}h)",
        filename,
        age.as_secs() / 3600
    );
    true
}

/// The file's MIME type, when its size and type let it be ingested; `None`
/// (with the reason logged) for empty, oversized or non-extractable files.
fn extractable_mime_type(filename: &str, file_data: &[u8]) -> Option<String> {
    let file_size = file_data.len() as i64;
    match size_skip(file_size) {
        // Skip very large files (> 500MB by default)
        Some(SizeSkip::TooLarge) => {
            warn!(
                "Skipping large file: {} ({} MB)",
                filename,
                file_size / 1024 / 1024
            );
            return None;
        }
        Some(SizeSkip::Empty) => {
            debug!("Skipping empty file: {}", filename);
            return None;
        }
        None => {}
    }

    let mime_type = mime_guess::from_path(filename)
        .first_or_octet_stream()
        .to_string();

    // Check if file can have text extracted (OCR or Office document text extraction)
    if !is_text_extractable_file(&mime_type) {
        debug!(
            "Skipping non-text-extractable file: {} ({})",
            filename, mime_type
        );
        return None;
    }
    Some(mime_type)
}

/// Ingest the file for `target_user_id` and queue a newly created document
/// for OCR.
async fn ingest_and_queue(
    path: &Path,
    file_data: Vec<u8>,
    target_user_id: Uuid,
    ctx: &WatchContext,
) -> Result<()> {
    // Extract basic file info first
    let mut file_info = extract_file_info_from_path(path).await?;

    // Extract content-based metadata
    if let Ok(Some(content_metadata)) = crate::metadata_extraction::extract_content_metadata(
        &file_data,
        &file_info.mime_type,
        &file_info.name,
    )
    .await
    {
        file_info.metadata = Some(content_metadata);
    }

    // Use the unified ingestion service for consistent deduplication
    let ingestion_service =
        DocumentIngestionService::new(ctx.db.clone(), ctx.file_service.as_ref().clone());

    let result = ingestion_service
        .ingest_from_file_info(
            &file_info,
            file_data,
            target_user_id,
            DeduplicationPolicy::Skip,
            "watch_folder",
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    handle_ingestion_result(result, &file_info, ctx).await
}

/// Log what ingestion did with the file and queue a new document for OCR.
async fn handle_ingestion_result(
    result: IngestionResult,
    file_info: &FileIngestionInfo,
    ctx: &WatchContext,
) -> Result<()> {
    if let IngestionResult::Created(doc) = &result {
        return queue_new_document(doc.id, file_info, ctx).await;
    }
    match not_created_log(&result, &file_info.name) {
        Some((Level::DEBUG, message)) => debug!("{}", message),
        Some((_, message)) => info!("{}", message),
        None => {}
    }
    Ok(())
}

/// Queue a newly created document for OCR, prioritised by size and type.
async fn queue_new_document(
    document_id: Uuid,
    file_info: &FileIngestionInfo,
    ctx: &WatchContext,
) -> Result<()> {
    info!(
        "Created new document for watch folder file {}: {}",
        file_info.name, document_id
    );

    // Enqueue for OCR processing with priority based on file size and type
    let priority = calculate_priority(file_info.size, &file_info.mime_type);
    // A new document has no job yet; if one exists, OCR runs anyway.
    let _queued: EnqueueOutcome = ctx
        .queue_service
        .enqueue_document(document_id, priority, file_info.size)
        .await?;

    info!(
        "Successfully queued file for OCR: {} (size: {} bytes)",
        file_info.name, file_info.size
    );
    Ok(())
}

/// What to log, and at which level, when ingestion created no new document
/// for `filename` (nothing is queued for OCR). `None` for a created one.
fn not_created_log(result: &IngestionResult, filename: &str) -> Option<(Level, String)> {
    let line = match result {
        IngestionResult::Created(_) => return None,
        IngestionResult::Skipped {
            existing_document_id,
            reason,
        } => (
            Level::DEBUG,
            format!(
                "Skipped duplicate watch folder file {}: {} (existing: {})",
                filename, reason, existing_document_id
            ),
        ),
        IngestionResult::ExistingDocument(doc) => (
            Level::INFO,
            format!(
                "Found existing document for watch folder file {}: {} (not re-queuing for OCR)",
                filename, doc.id
            ),
        ),
        IngestionResult::TrackedAsDuplicate {
            existing_document_id,
        } => (
            Level::INFO,
            format!(
                "Tracked watch folder file {} as duplicate of existing document: {}",
                filename, existing_document_id
            ),
        ),
        IngestionResult::Ignored { ignored_file_id } => (
            Level::INFO,
            format!(
                "Skipped watch folder file {}: on the ignored-files list ({})",
                filename, ignored_file_id
            ),
        ),
    };
    Some(line)
}

/// The user a watched file belongs to: the user of the per-user watch
/// directory it sits in, otherwise the global watch folder's owner. `None`
/// when that owner cannot be resolved (already logged).
async fn file_owner(path: &Path, filename: &str, ctx: &WatchContext) -> Result<Option<Uuid>> {
    if let Some(manager) = ctx
        .user_watch_manager
        .as_ref()
        .filter(|m| m.is_user_watch_path(path))
    {
        if let Some(user) = manager.get_user_by_file_path(path).await? {
            info!(
                "File {} belongs to user: {} ({})",
                filename, user.username, user.id
            );
            return Ok(Some(user.id));
        }
        warn!(
            "File {} is in a user watch directory but no user owns it - assigning to the admin '{}'",
            filename,
            ctx.owner.username()
        );
    } else {
        debug!(
            "File {} is in the global watch directory - assigning to the admin '{}'",
            filename,
            ctx.owner.username()
        );
    }
    ctx.owner.resolve(&ctx.db).await
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
    let last_modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| {
            DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_else(Utc::now)
        });

    let created_at = metadata
        .created()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| {
            DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_else(Utc::now)
        });

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
        etag: format!(
            "{}-{}",
            file_size,
            last_modified.map_or(0, |t| t.timestamp())
        ),
        is_directory: metadata.is_dir(),
        created_at,
        permissions,
        owner,
        group,
        metadata: None, // Could extract EXIF/other metadata in the future
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{
        AccessKind, CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode,
    };

    #[test]
    fn relevant_event_accepts_create() {
        assert!(is_relevant_event(&EventKind::Create(CreateKind::File)));
        assert!(is_relevant_event(&EventKind::Create(CreateKind::Any)));
    }

    #[test]
    fn relevant_event_accepts_data_modification() {
        assert!(is_relevant_event(&EventKind::Modify(ModifyKind::Data(
            DataChange::Content
        ))));
        assert!(is_relevant_event(&EventKind::Modify(ModifyKind::Data(
            DataChange::Any
        ))));
    }

    #[test]
    fn relevant_event_accepts_rename() {
        assert!(is_relevant_event(&EventKind::Modify(ModifyKind::Name(
            RenameMode::To
        ))));
        assert!(is_relevant_event(&EventKind::Modify(ModifyKind::Name(
            RenameMode::Any
        ))));
    }

    #[test]
    fn relevant_event_accepts_ambiguous() {
        assert!(is_relevant_event(&EventKind::Modify(ModifyKind::Any)));
        assert!(is_relevant_event(&EventKind::Any));
    }

    #[test]
    fn relevant_event_rejects_access() {
        assert!(!is_relevant_event(&EventKind::Access(AccessKind::Read)));
        assert!(!is_relevant_event(&EventKind::Access(AccessKind::Any)));
    }

    #[test]
    fn relevant_event_rejects_remove() {
        assert!(!is_relevant_event(&EventKind::Remove(RemoveKind::File)));
        assert!(!is_relevant_event(&EventKind::Remove(RemoveKind::Any)));
    }

    #[test]
    fn relevant_event_rejects_metadata_only() {
        assert!(!is_relevant_event(&EventKind::Modify(
            ModifyKind::Metadata(MetadataKind::Any)
        )));
        assert!(!is_relevant_event(&EventKind::Modify(
            ModifyKind::Metadata(MetadataKind::WriteTime)
        )));
    }

    #[test]
    fn relevant_event_rejects_other() {
        assert!(!is_relevant_event(&EventKind::Other));
    }

    #[test]
    fn created_documents_are_queued_not_logged_as_skipped() {
        let doc = crate::test_utils::document_helpers::create_test_document(Uuid::new_v4());
        assert_eq!(
            not_created_log(&IngestionResult::Created(doc), "a.pdf"),
            None
        );
    }

    #[test]
    fn duplicate_skips_log_at_debug() {
        let id = Uuid::new_v4();
        let result = IngestionResult::Skipped {
            existing_document_id: id,
            reason: "same hash".to_string(),
        };
        assert_eq!(
            not_created_log(&result, "a.pdf"),
            Some((
                Level::DEBUG,
                format!("Skipped duplicate watch folder file a.pdf: same hash (existing: {id})")
            ))
        );
    }

    #[test]
    fn existing_tracked_and_ignored_files_log_at_info() {
        let doc = crate::test_utils::document_helpers::create_test_document(Uuid::new_v4());
        let doc_id = doc.id;
        let id = Uuid::new_v4();
        let cases = [
            (
                IngestionResult::ExistingDocument(doc),
                format!("Found existing document for watch folder file a.pdf: {doc_id} (not re-queuing for OCR)"),
            ),
            (
                IngestionResult::TrackedAsDuplicate {
                    existing_document_id: id,
                },
                format!("Tracked watch folder file a.pdf as duplicate of existing document: {id}"),
            ),
            (
                IngestionResult::Ignored {
                    ignored_file_id: id,
                },
                format!("Skipped watch folder file a.pdf: on the ignored-files list ({id})"),
            ),
        ];
        for (result, message) in cases {
            assert_eq!(
                not_created_log(&result, "a.pdf"),
                Some((Level::INFO, message))
            );
        }
    }
}
