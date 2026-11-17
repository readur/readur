//! Paging through the documents the `migrate_to_s3` tool still has to move.
//! Included from `s3_service.rs` as `migration`.
//!
//! Every document whose `file_path` is not an `s3://` path is visited exactly
//! once, oldest first, in keyset pages on `(created_at, id)`. The cursor only
//! moves forward, so migrating a document while paging (its path turns into
//! `s3://...`) never shifts later pages the way an OFFSET would, and a
//! re-run visits only what is still local: documents already on S3 are
//! skipped by the query itself.

use std::collections::VecDeque;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::keys::STORAGE_PATH_SCHEME;

/// WHERE clause of the paging queries: local documents ($1 is the `s3://%`
/// pattern) of the user ($2, any when NULL) after the cursor ($3, $4), or at
/// it when $5. A macro so the queries stay static strings.
macro_rules! local_documents_filter {
    () => {
        "file_path NOT LIKE $1
           AND ($2::uuid IS NULL OR user_id = $2)
           AND ($3::timestamptz IS NULL
                OR (created_at, id) > ($3, $4)
                OR ($5 AND id = $4))"
    };
}

/// Rows fetched per page when the caller has no reason to pick another size.
pub const DEFAULT_PAGE_SIZE: i64 = 500;

/// The columns of a document the migration reads.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MigrationCandidate {
    pub id: Uuid,
    pub user_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

/// Where the next page starts.
#[derive(Debug, Clone, Copy)]
struct Cursor {
    created_at: DateTime<Utc>,
    id: Uuid,
    /// Whether the document at the cursor itself is included (a resume
    /// point), or only the documents after it (the last row of a page).
    inclusive: bool,
}

/// Walks every not-yet-migrated document, optionally of one user.
pub struct LocalDocumentPager {
    pool: PgPool,
    user_id: Option<Uuid>,
    page_size: i64,
    cursor: Option<Cursor>,
    /// Fetched rows not yet handed out by [`Self::next_document`].
    buffered: VecDeque<MigrationCandidate>,
}

impl LocalDocumentPager {
    /// A pager from the oldest local document. `page_size` below 1 is
    /// treated as 1.
    pub fn new(pool: PgPool, user_id: Option<Uuid>, page_size: i64) -> Self {
        Self {
            pool,
            user_id,
            page_size: page_size.max(1),
            cursor: None,
            buffered: VecDeque::new(),
        }
    }

    /// Start at `document_id` (included) instead of the oldest document.
    pub async fn resume_from(mut self, document_id: Uuid) -> Result<Self> {
        let created_at: DateTime<Utc> =
            sqlx::query_scalar("SELECT created_at FROM documents WHERE id = $1")
                .bind(document_id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to look up the resume document")?
                .with_context(|| format!("Resume document {document_id} does not exist"))?;
        self.cursor = Some(Cursor {
            created_at,
            id: document_id,
            inclusive: true,
        });
        Ok(self)
    }

    /// How many local documents are left from the cursor on.
    pub async fn count_remaining(&self) -> Result<i64> {
        let (after_time, after_id, inclusive) = self.cursor_binds();
        sqlx::query_scalar(concat!(
            "SELECT COUNT(*) FROM documents WHERE ",
            local_documents_filter!()
        ))
        .bind(format!("{STORAGE_PATH_SCHEME}%"))
        .bind(self.user_id)
        .bind(after_time)
        .bind(after_id)
        .bind(inclusive)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count the documents left to migrate")
    }

    /// The next page, oldest first; empty once every local document has
    /// been returned. Do not mix with [`Self::next_document`], which pages
    /// ahead of what it has handed out.
    pub async fn next_page(&mut self) -> Result<Vec<MigrationCandidate>> {
        let (after_time, after_id, inclusive) = self.cursor_binds();
        let page: Vec<MigrationCandidate> = sqlx::query_as(concat!(
            "SELECT id, user_id, filename, original_filename, file_path, file_size, created_at
             FROM documents WHERE ",
            local_documents_filter!(),
            " ORDER BY created_at, id LIMIT $6"
        ))
        .bind(format!("{STORAGE_PATH_SCHEME}%"))
        .bind(self.user_id)
        .bind(after_time)
        .bind(after_id)
        .bind(inclusive)
        .bind(self.page_size)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch the next page of documents to migrate")?;

        if let Some(last) = page.last() {
            self.cursor = Some(Cursor {
                created_at: last.created_at,
                id: last.id,
                inclusive: false,
            });
        }
        Ok(page)
    }

    /// The next document, fetching a page when the previous one is used up;
    /// `None` once every local document has been returned.
    pub async fn next_document(&mut self) -> Result<Option<MigrationCandidate>> {
        if self.buffered.is_empty() {
            self.buffered = self.next_page().await?.into();
        }
        Ok(self.buffered.pop_front())
    }

    fn cursor_binds(&self) -> (Option<DateTime<Utc>>, Option<Uuid>, bool) {
        match self.cursor {
            Some(c) => (Some(c.created_at), Some(c.id), c.inclusive),
            None => (None, None, false),
        }
    }
}
