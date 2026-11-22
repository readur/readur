//! Migration utility to move existing local files to S3 storage
//!
//! Usage: cargo run --bin migrate_to_s3 --features s3
//!
//! This utility will:
//! 1. Connect to the database
//! 2. Page through every document with a local file path, oldest first
//! 3. Upload files to S3 with proper structure
//! 4. Update database records with S3 paths
//! 5. Optionally delete local files after successful upload
//! 6. Support rollback on failure with transaction-like behavior

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{error, info, warn};
use uuid::Uuid;

use readur::{
    config::Config,
    db::Database,
    services::{
        file_service::FileService,
        s3_service::{
            migration::{LocalDocumentPager, MigrationCandidate, DEFAULT_PAGE_SIZE},
            S3Service,
        },
    },
};

/// Migration state tracking for rollback functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationState {
    started_at: chrono::DateTime<chrono::Utc>,
    completed_migrations: Vec<MigrationRecord>,
    failed_migrations: Vec<FailedMigration>,
    total_files: usize,
    processed_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationRecord {
    document_id: Uuid,
    user_id: Uuid,
    original_path: String,
    s3_key: String,
    migrated_at: chrono::DateTime<chrono::Utc>,
    associated_files: Vec<AssociatedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssociatedFile {
    file_type: String, // "thumbnail" or "processed_image"
    original_path: String,
    s3_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FailedMigration {
    document_id: Uuid,
    original_path: String,
    error: String,
    failed_at: chrono::DateTime<chrono::Utc>,
}

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

    /// Enable rollback on failure - will revert any successful migrations if overall process fails
    #[arg(long)]
    enable_rollback: bool,

    /// Resume from a specific document ID (for partial recovery)
    #[arg(long)]
    resume_from: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();

    info!("🚀 Starting S3 migration utility");

    // Load configuration
    let config = Config::from_env()?;

    if !config.s3_enabled {
        error!(
            "S3 is not enabled in configuration. Set S3_ENABLED=true and provide S3 configuration."
        );
        std::process::exit(1);
    }

    let s3_config = config
        .s3_config
        .as_ref()
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

    let user_id = args.user_id.as_deref().map(Uuid::parse_str).transpose()?;
    let mut pager = LocalDocumentPager::new(db.get_pool().clone(), user_id, DEFAULT_PAGE_SIZE);
    if let Some(resume_from) = &args.resume_from {
        pager = pager.resume_from(Uuid::parse_str(resume_from)?).await?;
    }

    // Documents already on S3 are never selected, so a re-run only sees
    // what is still local (new documents, and earlier failures).
    let remaining = pager.count_remaining().await? as usize;
    let planned = args.limit.map_or(remaining, |limit| limit.min(remaining));
    info!("📋 Found {} documents with local file paths", remaining);
    if planned < remaining {
        info!(
            "📋 --limit {}: only the oldest {} will be processed",
            planned, planned
        );
    }

    if planned == 0 {
        info!("✅ No local documents found to migrate");
        return Ok(());
    }

    if args.dry_run {
        info!("🔍 DRY RUN - Would migrate the following files:");
        for _ in 0..planned {
            let Some(doc) = pager.next_document().await? else {
                break;
            };
            info!(
                "  - {} (User: {}, Size: {} bytes)",
                doc.original_filename, doc.user_id, doc.file_size
            );
        }
        info!("💡 Run without --dry-run to perform actual migration");
        return Ok(());
    }

    let mut run = MigrationRun {
        db: &db,
        s3_service: &s3_service,
        file_service: &file_service,
        args: &args,
        state: MigrationState {
            started_at: chrono::Utc::now(),
            completed_migrations: Vec::new(),
            failed_migrations: Vec::new(),
            total_files: planned,
            processed_files: 0,
        },
    };
    info!("📊 Migration plan: {} files to process", planned);
    run.migrate_all(&mut pager).await
}

/// One migration run: what it migrates with, and the progress it records.
struct MigrationRun<'a> {
    db: &'a Database,
    s3_service: &'a S3Service,
    file_service: &'a FileService,
    args: &'a Args,
    state: MigrationState,
}

impl MigrationRun<'_> {
    async fn migrate_all(&mut self, pager: &mut LocalDocumentPager) -> Result<()> {
        while self.state.processed_files < self.state.total_files {
            let Some(doc) = pager.next_document().await? else {
                break;
            };
            self.migrate_one(&doc).await?;
        }
        save_migration_state(&self.state).await?;
        self.log_summary();
        Ok(())
    }

    fn log_summary(&self) {
        info!(
            "🎉 Migration completed! ✅ Successfully migrated: {} files",
            self.state.completed_migrations.len()
        );
        let failed = self.state.failed_migrations.len();
        if failed > 0 {
            warn!("❌ Failed to migrate: {failed} files; check migration_state.json for details on failures");
        }
    }

    /// Migrate one document and record the outcome. Errors only when a
    /// failure ends the run (rollback enabled) or progress cannot be saved.
    async fn migrate_one(&mut self, doc: &MigrationCandidate) -> Result<()> {
        self.state.processed_files += 1;
        info!(
            "📦 Migrating: {} ({}) [{}/{}]",
            doc.original_filename, doc.id, self.state.processed_files, self.state.total_files
        );
        match migrate_document_with_tracking(
            self.db,
            self.s3_service,
            self.file_service,
            doc,
            self.args.delete_local,
        )
        .await
        {
            Ok(migration_record) => {
                self.state.completed_migrations.push(migration_record);
                let migrated = self.state.completed_migrations.len();
                info!(
                    "✅ Successfully migrated: {} [{}/{}]",
                    doc.original_filename, migrated, self.state.total_files
                );
                // Save progress periodically (every 10 files)
                if migrated.is_multiple_of(10) {
                    save_migration_state(&self.state).await?;
                }
                Ok(())
            }
            Err(e) => {
                self.record_failure(doc, e);
                if self.args.enable_rollback {
                    return self.roll_back_after_failure().await;
                }
                Ok(())
            }
        }
    }

    fn record_failure(&mut self, doc: &MigrationCandidate, e: anyhow::Error) {
        error!("❌ Failed to migrate {}: {}", doc.original_filename, e);
        self.state.failed_migrations.push(FailedMigration {
            document_id: doc.id,
            original_path: doc.file_path.clone(),
            error: e.to_string(),
            failed_at: chrono::Utc::now(),
        });
    }

    /// Undo every migration of this run; always ends the run with an error.
    async fn roll_back_after_failure(&self) -> Result<()> {
        error!(
            "💥 Migration failure detected with rollback enabled! 🔄 Rolling back {} successful migrations...",
            self.state.completed_migrations.len()
        );
        match rollback_migrations(self.db, self.s3_service, &self.state).await {
            Ok(rolled_back) => {
                error!("🔄 Successfully rolled back {} migrations", rolled_back);
                Err(anyhow::anyhow!("Migration failed and was rolled back"))
            }
            Err(rollback_err) => {
                error!("💥 CRITICAL: Rollback failed: {rollback_err}; 💾 migration state saved for manual recovery");
                save_migration_state(&self.state).await?;
                Err(anyhow::anyhow!(
                    "Migration failed and rollback also failed. Check migration state file for manual recovery."
                ))
            }
        }
    }
}

async fn migrate_document_with_tracking(
    db: &Database,
    s3_service: &S3Service,
    file_service: &FileService,
    document: &MigrationCandidate,
    delete_local: bool,
) -> Result<MigrationRecord> {
    // Read local file
    let local_path = Path::new(&document.file_path);
    if !local_path.exists() {
        return Err(anyhow::anyhow!(
            "Local file not found: {}",
            document.file_path
        ));
    }

    let file_data = tokio::fs::read(&local_path).await?;

    // Upload to S3
    let s3_key = s3_service
        .store_document(
            document.user_id,
            document.id,
            &document.filename,
            &file_data,
        )
        .await?;

    let s3_path = format!("s3://{}", s3_key);

    // Update database record
    db.update_document_file_path(document.id, &s3_path).await?;

    // Migrate associated files (thumbnails, processed images)
    let associated_files =
        migrate_associated_files_with_tracking(s3_service, file_service, document, delete_local)
            .await?;

    // Delete local file if requested
    if delete_local {
        if let Err(e) = tokio::fs::remove_file(&local_path).await {
            warn!("Failed to delete local file {}: {}", document.file_path, e);
        } else {
            info!("🗑️  Deleted local file: {}", document.file_path);
        }
    }

    // Create migration record for tracking
    let migration_record = MigrationRecord {
        document_id: document.id,
        user_id: document.user_id,
        original_path: document.file_path.clone(),
        s3_key,
        migrated_at: chrono::Utc::now(),
        associated_files,
    };

    Ok(migration_record)
}

async fn migrate_associated_files_with_tracking(
    s3_service: &S3Service,
    file_service: &FileService,
    document: &MigrationCandidate,
    delete_local: bool,
) -> Result<Vec<AssociatedFile>> {
    let candidates = [
        (
            AssociatedKind::Thumbnail,
            file_service
                .get_thumbnails_path()
                .join(format!("{}_thumb.jpg", document.id)),
        ),
        (
            AssociatedKind::ProcessedImage,
            file_service
                .get_processed_images_path()
                .join(format!("{}_processed.png", document.id)),
        ),
    ];
    let mut associated_files = Vec::new();
    for (kind, path) in candidates {
        if path.exists() {
            associated_files.extend(
                migrate_associated_file(s3_service, document, kind, &path, delete_local).await,
            );
        }
    }
    Ok(associated_files)
}

/// The derived files stored next to a document.
#[derive(Clone, Copy)]
enum AssociatedKind {
    Thumbnail,
    ProcessedImage,
}

impl AssociatedKind {
    /// The `file_type` recorded in the migration state.
    fn file_type(self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail",
            Self::ProcessedImage => "processed_image",
        }
    }
}

/// Upload one derived file. A failure is logged and yields `None`: it does
/// not fail the document.
async fn migrate_associated_file(
    s3_service: &S3Service,
    document: &MigrationCandidate,
    kind: AssociatedKind,
    path: &Path,
    delete_local: bool,
) -> Option<AssociatedFile> {
    let stored = match tokio::fs::read(path).await {
        Ok(data) => match kind {
            AssociatedKind::Thumbnail => {
                s3_service
                    .store_thumbnail(document.user_id, document.id, &data)
                    .await
            }
            AssociatedKind::ProcessedImage => {
                s3_service
                    .store_processed_image(document.user_id, document.id, &data)
                    .await
            }
        },
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {}", path.display(), e)),
    };
    let s3_key = match stored {
        Ok(key) => key,
        Err(e) => {
            warn!(
                "Failed to migrate {} for {}: {}",
                kind.file_type(),
                document.id,
                e
            );
            return None;
        }
    };
    info!(
        "Migrated {} for: {}",
        kind.file_type(),
        document.original_filename
    );
    if delete_local {
        let _ = tokio::fs::remove_file(path).await;
    }
    Some(AssociatedFile {
        file_type: kind.file_type().to_string(),
        original_path: path.to_string_lossy().to_string(),
        s3_key,
    })
}

/// Save migration state to disk for recovery purposes
async fn save_migration_state(state: &MigrationState) -> Result<()> {
    let state_json = serde_json::to_string_pretty(state)?;
    tokio::fs::write("migration_state.json", state_json).await?;
    info!("💾 Migration state saved to migration_state.json");
    Ok(())
}

/// Rollback migrations by restoring database paths and deleting S3 objects
async fn rollback_migrations(
    db: &Database,
    s3_service: &S3Service,
    state: &MigrationState,
) -> Result<usize> {
    info!(
        "🔄 Starting rollback of {} migrations...",
        state.completed_migrations.len()
    );

    let mut rolled_back = 0;
    let mut rollback_errors = Vec::new();

    for migration in rollback_order(state) {
        if roll_back_one(db, s3_service, migration, &mut rollback_errors).await {
            rolled_back += 1;
        }
    }

    if !rollback_errors.is_empty() {
        save_rollback_errors(rolled_back, &rollback_errors, state).await?;
    }

    info!(
        "✅ Rollback completed: {} migrations processed",
        rolled_back
    );
    Ok(rolled_back)
}

/// Migrations in the order they are undone: most recent first.
fn rollback_order(state: &MigrationState) -> impl Iterator<Item = &MigrationRecord> {
    state.completed_migrations.iter().rev()
}

/// Undo one migration: restore its database path, then delete its S3
/// objects. Failures are logged and pushed onto `errors`. Returns whether
/// the migration counts as rolled back: its S3 objects are left alone (and
/// it does not count) when the database path could not be restored.
async fn roll_back_one(
    db: &Database,
    s3_service: &S3Service,
    migration: &MigrationRecord,
    errors: &mut Vec<String>,
) -> bool {
    info!(
        "🔄 Rolling back migration for document {}",
        migration.document_id
    );

    if !restore_db_path(db, migration, errors).await {
        return false; // Skip S3 cleanup if DB restore failed
    }
    delete_migrated_objects(s3_service, migration, errors).await;
    true
}

/// Point the document back at its original local path.
async fn restore_db_path(
    db: &Database,
    migration: &MigrationRecord,
    errors: &mut Vec<String>,
) -> bool {
    match db
        .update_document_file_path(migration.document_id, &migration.original_path)
        .await
    {
        Ok(_) => {
            info!(
                "✅ Restored database path for document {}",
                migration.document_id
            );
            true
        }
        Err(e) => {
            let error_msg = RollbackError::RestoreDbPath(migration.document_id).describe(e);
            error!("❌ {}", error_msg);
            errors.push(error_msg);
            false
        }
    }
}

/// Delete the document's S3 object and its associated objects. A failed
/// delete is recorded and does not stop the others.
async fn delete_migrated_objects(
    s3_service: &S3Service,
    migration: &MigrationRecord,
    errors: &mut Vec<String>,
) {
    // Delete S3 object (main document); continue with associated files even
    // if main file deletion failed
    match s3_service.delete_file(&migration.s3_key).await {
        Ok(_) => info!("🗑️  Deleted S3 object: {}", migration.s3_key),
        Err(e) => record_delete_error(RollbackError::DeleteObject(&migration.s3_key), e, errors),
    }

    // Delete associated S3 objects (thumbnails, processed images)
    for associated in &migration.associated_files {
        match s3_service.delete_file(&associated.s3_key).await {
            Ok(_) => info!(
                "🗑️  Deleted associated S3 object: {} ({})",
                associated.s3_key, associated.file_type
            ),
            Err(e) => record_delete_error(
                RollbackError::DeleteAssociatedObject(&associated.s3_key),
                e,
                errors,
            ),
        }
    }
}

fn record_delete_error(
    failed: RollbackError<'_>,
    e: impl std::fmt::Display,
    errors: &mut Vec<String>,
) {
    let error_msg = failed.describe(e);
    warn!("⚠️  {}", error_msg);
    errors.push(error_msg);
}

/// A rollback step that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RollbackError<'a> {
    RestoreDbPath(Uuid),
    DeleteObject(&'a str),
    DeleteAssociatedObject(&'a str),
}

impl RollbackError<'_> {
    /// The message recorded for this failure, caused by `e`.
    fn describe(self, e: impl std::fmt::Display) -> String {
        match self {
            Self::RestoreDbPath(document_id) => {
                format!("Failed to restore DB path for {}: {}", document_id, e)
            }
            Self::DeleteObject(key) => format!("Failed to delete S3 object {}: {}", key, e),
            Self::DeleteAssociatedObject(key) => {
                format!("Failed to delete associated S3 object {}: {}", key, e)
            }
        }
    }
}

/// Log the rollback's errors and save them, with the migration state, to
/// rollback_errors.json for manual cleanup.
async fn save_rollback_errors(
    rolled_back: usize,
    rollback_errors: &[String],
    state: &MigrationState,
) -> Result<()> {
    warn!(
        "⚠️  Rollback completed with {} errors:",
        rollback_errors.len()
    );
    for error in rollback_errors {
        warn!("  - {}", error);
    }

    // Save error details for manual cleanup
    let error_state =
        rollback_error_report(rolled_back, rollback_errors, state, chrono::Utc::now());
    let error_json = serde_json::to_string_pretty(&error_state)?;
    tokio::fs::write("rollback_errors.json", error_json).await?;
    warn!("💾 Rollback errors saved to rollback_errors.json for manual cleanup");
    Ok(())
}

/// The contents of rollback_errors.json.
fn rollback_error_report(
    rolled_back: usize,
    rollback_errors: &[String],
    state: &MigrationState,
    completed_at: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    serde_json::json!({
        "rollback_completed_at": completed_at,
        "rolled_back_count": rolled_back,
        "rollback_errors": rollback_errors,
        "original_migration_state": state
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(document_id: Uuid, associated: &[(&str, &str)]) -> MigrationRecord {
        MigrationRecord {
            document_id,
            user_id: Uuid::nil(),
            original_path: format!("./uploads/{document_id}.pdf"),
            s3_key: format!("documents/{document_id}.pdf"),
            migrated_at: chrono::Utc::now(),
            associated_files: associated
                .iter()
                .map(|(file_type, s3_key)| AssociatedFile {
                    file_type: file_type.to_string(),
                    original_path: String::new(),
                    s3_key: s3_key.to_string(),
                })
                .collect(),
        }
    }

    fn state(completed_migrations: Vec<MigrationRecord>) -> MigrationState {
        MigrationState {
            started_at: chrono::Utc::now(),
            total_files: completed_migrations.len(),
            processed_files: completed_migrations.len(),
            completed_migrations,
            failed_migrations: Vec::new(),
        }
    }

    #[test]
    fn rollback_undoes_the_most_recent_migration_first() {
        let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let migrations = state(ids.iter().map(|&id| record(id, &[])).collect());
        let order: Vec<Uuid> = rollback_order(&migrations).map(|m| m.document_id).collect();
        assert_eq!(order, vec![ids[2], ids[1], ids[0]]);
        assert_eq!(rollback_order(&state(Vec::new())).count(), 0);
    }

    #[test]
    fn rollback_errors_name_the_failed_step() {
        let id = Uuid::nil();
        assert_eq!(
            RollbackError::RestoreDbPath(id).describe("db down"),
            format!("Failed to restore DB path for {id}: db down")
        );
        assert_eq!(
            RollbackError::DeleteObject("documents/a.pdf").describe("403"),
            "Failed to delete S3 object documents/a.pdf: 403"
        );
        assert_eq!(
            RollbackError::DeleteAssociatedObject("thumbnails/a.jpg").describe("404"),
            "Failed to delete associated S3 object thumbnails/a.jpg: 404"
        );
    }

    #[test]
    fn rollback_error_report_keeps_errors_and_original_state() {
        let id = Uuid::new_v4();
        let state = state(vec![record(id, &[("thumbnail", "thumbnails/a.jpg")])]);
        let at = chrono::Utc::now();
        let errors = vec!["first".to_string(), "second".to_string()];
        let report = rollback_error_report(1, &errors, &state, at);

        assert_eq!(report["rolled_back_count"], 1);
        assert_eq!(
            report["rollback_errors"],
            serde_json::json!(["first", "second"])
        );
        assert_eq!(report["rollback_completed_at"], serde_json::json!(at));
        let original = &report["original_migration_state"];
        assert_eq!(
            original["completed_migrations"][0]["document_id"],
            serde_json::json!(id)
        );
        assert_eq!(
            original["completed_migrations"][0]["associated_files"][0]["s3_key"],
            "thumbnails/a.jpg"
        );
    }
}
