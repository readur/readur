//! What source sync needs to know about a document it stored earlier, to
//! decide whether a listed file has to be downloaded again.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::db::Database;
use crate::db::sanitize::ToDb;

/// The stored document for one (source, path) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedDocumentIdentity {
    pub id: Uuid,
    pub file_size: i64,
    pub original_modified_at: Option<DateTime<Utc>>,
}

impl SyncedDocumentIdentity {
    /// Whether a listing with this size and modification time describes the
    /// stored document unchanged. Unknown (`None`) modification times on
    /// either side never match, so such files are downloaded and compared by
    /// content as before. Times are compared to the second because sources
    /// report them at that precision while the database keeps microseconds.
    pub fn matches(&self, size: i64, modified: Option<DateTime<Utc>>) -> bool {
        if self.file_size != size {
            return false;
        }
        match (self.original_modified_at, modified) {
            (Some(stored), Some(listed)) => (stored - listed).num_seconds().abs() < 1,
            _ => false,
        }
    }
}

impl Database {
    /// The document source sync stored for `source_path` of `source_id`, if
    /// any. `source_path` is the listing's relative path, exactly as
    /// `ingest_from_file_info` stores it.
    pub async fn find_synced_document(
        &self,
        user_id: Uuid,
        source_id: Uuid,
        source_path: &str,
    ) -> Result<Option<SyncedDocumentIdentity>> {
        let row = sqlx::query(
            r#"SELECT id, file_size, original_modified_at
               FROM documents
               WHERE user_id = $1 AND source_id = $2 AND source_path = $3
               ORDER BY created_at DESC
               LIMIT 1"#,
        )
        .bind(user_id)
        .bind(source_id)
        .bind(source_path.for_db())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| SyncedDocumentIdentity {
            id: row.get("id"),
            file_size: row.get("file_size"),
            original_modified_at: row.get("original_modified_at"),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn stored(modified: Option<DateTime<Utc>>) -> SyncedDocumentIdentity {
        SyncedDocumentIdentity { id: Uuid::new_v4(), file_size: 10, original_modified_at: modified }
    }

    #[test]
    fn matches_only_with_same_size_and_known_equal_time() {
        let t = Utc::now();
        assert!(stored(Some(t)).matches(10, Some(t)));
        assert!(stored(Some(t)).matches(10, Some(t + Duration::milliseconds(400))));
        assert!(!stored(Some(t)).matches(10, Some(t + Duration::seconds(2))));
        assert!(!stored(Some(t)).matches(11, Some(t)));
        assert!(!stored(Some(t)).matches(10, None));
        assert!(!stored(None).matches(10, Some(t)));
        assert!(!stored(None).matches(10, None));
    }
}
