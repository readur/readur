use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::models::{IgnoredFile, IgnoredFileResponse, CreateIgnoredFile, IgnoredFilesQuery};
use anyhow::{Result, Context};
use crate::db::sanitize::ToDb;

/// Ignored-files `reason` for a document its owner deleted.
pub const REASON_DELETED_BY_USER: &str = "deleted by user";
/// Ignored-files `reason` for a document removed by the low-OCR-confidence
/// clean-up.
pub const REASON_LOW_CONFIDENCE_CLEANUP: &str = "low OCR confidence clean-up";
/// Ignored-files `reason` for a document removed by the failed-OCR clean-up.
pub const REASON_FAILED_OCR_CLEANUP: &str = "failed OCR clean-up";

pub async fn create_ignored_file(
    pool: &PgPool,
    ignored_file: CreateIgnoredFile,
) -> Result<IgnoredFile> {
    let record = sqlx::query(
        r#"
        INSERT INTO ignored_files (
            file_hash, filename, original_filename, file_path, file_size, mime_type,
            source_type, source_path, source_identifier, ignored_by, reason
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id, file_hash, filename, original_filename, file_path, file_size, mime_type,
                  source_type, source_path, source_identifier, ignored_at, ignored_by, reason, created_at
        "#
    )
    .bind(&ignored_file.file_hash)
    .bind(ignored_file.filename.for_db())
    .bind(ignored_file.original_filename.for_db())
    .bind(ignored_file.file_path.for_db())
    .bind(ignored_file.file_size)
    .bind(ignored_file.mime_type.for_db())
    .bind(ignored_file.source_type.for_db())
    .bind(ignored_file.source_path.for_db())
    .bind(ignored_file.source_identifier.for_db())
    .bind(ignored_file.ignored_by)
    .bind(ignored_file.reason.for_db())
    .fetch_one(pool)
    .await
    .context("Failed to create ignored file record")?;

    Ok(IgnoredFile {
        id: record.get("id"),
        file_hash: record.get("file_hash"),
        filename: record.get("filename"),
        original_filename: record.get("original_filename"),
        file_path: record.get("file_path"),
        file_size: record.get("file_size"),
        mime_type: record.get("mime_type"),
        source_type: record.get("source_type"),
        source_path: record.get("source_path"),
        source_identifier: record.get("source_identifier"),
        ignored_at: record.get("ignored_at"),
        ignored_by: record.get("ignored_by"),
        reason: record.get("reason"),
        created_at: record.get("created_at"),
    })
}

pub async fn list_ignored_files(
    pool: &PgPool,
    user_id: Uuid,
    query: &IgnoredFilesQuery,
) -> Result<Vec<IgnoredFileResponse>> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);

    // Build query based on filters
    let rows = match (&query.source_type, &query.source_identifier, &query.filename) {
        (Some(source_type), Some(source_identifier), Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1 
                  AND ig.source_type = $2 
                  AND ig.source_identifier = $3 
                  AND (ig.filename ILIKE $4 OR ig.original_filename ILIKE $4)
                ORDER BY ig.ignored_at DESC LIMIT $5 OFFSET $6
                "#
            )
            .bind(user_id)
            .bind(source_type.for_db())
            .bind(source_identifier.for_db())
            .bind(pattern.for_db())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        },
        (Some(source_type), Some(source_identifier), None) => {
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1 AND ig.source_type = $2 AND ig.source_identifier = $3
                ORDER BY ig.ignored_at DESC LIMIT $4 OFFSET $5
                "#
            )
            .bind(user_id)
            .bind(source_type.for_db())
            .bind(source_identifier.for_db())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        },
        (Some(source_type), None, Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1 
                  AND ig.source_type = $2 
                  AND (ig.filename ILIKE $3 OR ig.original_filename ILIKE $3)
                ORDER BY ig.ignored_at DESC LIMIT $4 OFFSET $5
                "#
            )
            .bind(user_id)
            .bind(source_type.for_db())
            .bind(pattern.for_db())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        },
        (Some(source_type), None, None) => {
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1 AND ig.source_type = $2
                ORDER BY ig.ignored_at DESC LIMIT $3 OFFSET $4
                "#
            )
            .bind(user_id)
            .bind(source_type.for_db())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        },
        (None, Some(source_identifier), Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1 
                  AND ig.source_identifier = $2 
                  AND (ig.filename ILIKE $3 OR ig.original_filename ILIKE $3)
                ORDER BY ig.ignored_at DESC LIMIT $4 OFFSET $5
                "#
            )
            .bind(user_id)
            .bind(source_identifier.for_db())
            .bind(pattern.for_db())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        },
        (None, Some(source_identifier), None) => {
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1 AND ig.source_identifier = $2
                ORDER BY ig.ignored_at DESC LIMIT $3 OFFSET $4
                "#
            )
            .bind(user_id)
            .bind(source_identifier.for_db())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        },
        (None, None, Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1 AND (ig.filename ILIKE $2 OR ig.original_filename ILIKE $2)
                ORDER BY ig.ignored_at DESC LIMIT $3 OFFSET $4
                "#
            )
            .bind(user_id)
            .bind(pattern.for_db())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        },
        (None, None, None) => {
            sqlx::query(
                r#"
                SELECT 
                    ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
                    ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
                    ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
                    u.username as ignored_by_username
                FROM ignored_files ig
                LEFT JOIN users u ON ig.ignored_by = u.id
                WHERE ig.ignored_by = $1
                ORDER BY ig.ignored_at DESC LIMIT $2 OFFSET $3
                "#
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .context("Failed to fetch ignored files")?
        }
    };

    let mut ignored_files = Vec::new();
    for row in rows {
        let ignored_file = IgnoredFileResponse {
            id: row.get("id"),
            file_hash: row.get("file_hash"),
            filename: row.get("filename"),
            original_filename: row.get("original_filename"),
            file_path: row.get("file_path"),
            file_size: row.get("file_size"),
            mime_type: row.get("mime_type"),
            source_type: row.get("source_type"),
            source_path: row.get("source_path"),
            source_identifier: row.get("source_identifier"),
            ignored_at: row.get("ignored_at"),
            ignored_by: row.get("ignored_by"),
            ignored_by_username: row.get("ignored_by_username"),
            reason: row.get("reason"),
            created_at: row.get("created_at"),
        };
        ignored_files.push(ignored_file);
    }

    Ok(ignored_files)
}

pub async fn get_ignored_file_by_id(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<IgnoredFileResponse>> {
    let row = sqlx::query(
        r#"
        SELECT 
            ig.id, ig.file_hash, ig.filename, ig.original_filename, ig.file_path,
            ig.file_size, ig.mime_type, ig.source_type, ig.source_path, 
            ig.source_identifier, ig.ignored_at, ig.ignored_by, ig.reason, ig.created_at,
            u.username as ignored_by_username
        FROM ignored_files ig
        LEFT JOIN users u ON ig.ignored_by = u.id
        WHERE ig.id = $1 AND ig.ignored_by = $2
        "#
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch ignored file by ID")?;

    if let Some(row) = row {
        Ok(Some(IgnoredFileResponse {
            id: row.get("id"),
            file_hash: row.get("file_hash"),
            filename: row.get("filename"),
            original_filename: row.get("original_filename"),
            file_path: row.get("file_path"),
            file_size: row.get("file_size"),
            mime_type: row.get("mime_type"),
            source_type: row.get("source_type"),
            source_path: row.get("source_path"),
            source_identifier: row.get("source_identifier"),
            ignored_at: row.get("ignored_at"),
            ignored_by: row.get("ignored_by"),
            ignored_by_username: row.get("ignored_by_username"),
            reason: row.get("reason"),
            created_at: row.get("created_at"),
        }))
    } else {
        Ok(None)
    }
}

/// Takes a file off the user's ignored list. `false` when the user has no
/// such entry.
pub async fn delete_ignored_file(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool> {
    let removed = bulk_delete_ignored_files(pool, vec![id], user_id)
        .await
        .context("Failed to delete ignored file")?;
    Ok(removed > 0)
}

/// The id of the user's ignored-files entry for this content, if any.
/// Automatic ingestion (source sync, watch folders, batch import) skips such
/// content; uploads are never checked.
pub async fn find_ignored_file(
    pool: &PgPool,
    user_id: Uuid,
    file_hash: &str,
) -> Result<Option<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM ignored_files WHERE ignored_by = $1 AND file_hash = $2 \
         ORDER BY ignored_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(file_hash)
    .fetch_optional(pool)
    .await
    .context("Failed to look up ignored file")
}

pub async fn count_ignored_files(
    pool: &PgPool,
    user_id: Uuid,
    query: &IgnoredFilesQuery,
) -> Result<i64> {
    let row = match (&query.source_type, &query.source_identifier, &query.filename) {
        (Some(source_type), Some(source_identifier), Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1 AND source_type = $2 AND source_identifier = $3 AND (filename ILIKE $4 OR original_filename ILIKE $4)")
                .bind(user_id)
                .bind(source_type.for_db())
                .bind(source_identifier.for_db())
                .bind(pattern.for_db())
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        },
        (Some(source_type), Some(source_identifier), None) => {
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1 AND source_type = $2 AND source_identifier = $3")
                .bind(user_id)
                .bind(source_type.for_db())
                .bind(source_identifier.for_db())
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        },
        (Some(source_type), None, Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1 AND source_type = $2 AND (filename ILIKE $3 OR original_filename ILIKE $3)")
                .bind(user_id)
                .bind(source_type.for_db())
                .bind(pattern.for_db())
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        },
        (Some(source_type), None, None) => {
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1 AND source_type = $2")
                .bind(user_id)
                .bind(source_type.for_db())
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        },
        (None, Some(source_identifier), Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1 AND source_identifier = $2 AND (filename ILIKE $3 OR original_filename ILIKE $3)")
                .bind(user_id)
                .bind(source_identifier.for_db())
                .bind(pattern.for_db())
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        },
        (None, Some(source_identifier), None) => {
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1 AND source_identifier = $2")
                .bind(user_id)
                .bind(source_identifier.for_db())
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        },
        (None, None, Some(filename)) => {
            let pattern = format!("%{}%", filename);
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1 AND (filename ILIKE $2 OR original_filename ILIKE $2)")
                .bind(user_id)
                .bind(pattern.for_db())
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        },
        (None, None, None) => {
            sqlx::query("SELECT COUNT(*) as count FROM ignored_files WHERE ignored_by = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .context("Failed to count ignored files")?
        }
    };

    Ok(row.get::<i64, _>("count"))
}

/// Takes several files off the user's ignored list; returns how many were
/// removed.
pub async fn bulk_delete_ignored_files(
    pool: &PgPool,
    ids: Vec<Uuid>,
    user_id: Uuid,
) -> Result<i64> {
    // A WebDAV sync skips a path whose tracking row has an unchanged ETag,
    // and the row of a deleted document stays behind without a document.
    // Dropping that row makes the next sync download the file again, so
    // taking a file off the list really lets it be imported again.
    sqlx::query_scalar(
        r#"
        WITH removed AS (
            DELETE FROM ignored_files WHERE id = ANY($1) AND ignored_by = $2
            RETURNING ignored_by, source_path
        ),
        untracked AS (
            DELETE FROM webdav_files w
            USING removed r
            WHERE w.user_id = r.ignored_by
              AND w.webdav_path = r.source_path
              AND w.document_id IS NULL
        )
        SELECT COUNT(*) FROM removed
        "#,
    )
    .bind(&ids)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("Failed to bulk delete ignored files")
}
