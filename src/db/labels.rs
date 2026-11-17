use anyhow::Result;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::db::{sanitize::ToDb, Database};
use crate::errors::label::LabelError;
use crate::routes::labels::{CreateLabel, Label, UpdateLabel};

/// The labels every new account starts with: name, description, colour. They are ordinary labels
/// the user owns, so they can be renamed, recoloured or deleted.
pub const DEFAULT_LABELS: [(&str, &str, &str); 5] = [
    ("Important", "High priority documents", "#d73a49"),
    ("To Review", "Documents that need review", "#f66a0a"),
    ("Archive", "Archived documents", "#6f42c1"),
    ("Work", "Work-related documents", "#0969da"),
    ("Personal", "Personal documents", "#1a7f37"),
];

/// Gives `user_id` its own copy of the default labels, skipping any name it already has.
pub async fn seed_default_labels(conn: &mut PgConnection, user_id: Uuid) -> Result<()> {
    for (name, description, color) in DEFAULT_LABELS {
        sqlx::query(
            "INSERT INTO labels (user_id, name, description, color) VALUES ($1, $2, $3, $4)
             ON CONFLICT (user_id, name) DO NOTHING",
        )
        .bind(user_id)
        .bind(name)
        .bind(description)
        .bind(color)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Rejects names that are blank or contain commas (commas break the comma-separated label
/// filters in search).
pub fn validate_label_name(name: &str) -> Result<(), LabelError> {
    if name.trim().is_empty() {
        return Err(LabelError::invalid_name(
            name.to_string(),
            "Name cannot be empty".to_string(),
        ));
    }
    // URL-encoded values (%2c) are decoded by serde before reaching here
    if name.contains(',') {
        return Err(LabelError::invalid_name(
            name.to_string(),
            "Name cannot contain commas".to_string(),
        ));
    }
    Ok(())
}

/// Colours are `#rrggbb`.
pub fn validate_label_color(color: &str) -> Result<(), LabelError> {
    if !color.starts_with('#') || color.len() != 7 {
        return Err(LabelError::invalid_color(color));
    }
    Ok(())
}

impl CreateLabel {
    pub fn validate(&self) -> Result<(), LabelError> {
        validate_label_name(&self.name)?;
        validate_label_color(&self.color)?;
        if let Some(bg_color) = &self.background_color {
            validate_label_color(bg_color)?;
        }
        Ok(())
    }
}

impl UpdateLabel {
    pub fn validate(&self) -> Result<(), LabelError> {
        if let Some(name) = &self.name {
            validate_label_name(name)?;
        }
        if let Some(color) = &self.color {
            validate_label_color(color)?;
        }
        if let Some(bg_color) = &self.background_color {
            validate_label_color(bg_color)?;
        }
        Ok(())
    }
}

/// The constraint that keeps each user's label names unique
/// (`migrations/20250620100017_add_labels_system.sql`).
pub const LABEL_NAME_UNIQUE_CONSTRAINT: &str = "unique_user_label_name";

/// Whether a failed label insert/update collided with a name the user already has.
///
/// Decided from the structured error Postgres sends, never from its message text: the server
/// translates messages per `lc_messages`, but the SQLSTATE (23505, `unique_violation`) and the
/// constraint name are fixed. Only the label-name constraint counts; any other unique violation
/// is a genuine failure and must not be reported to the user as a name clash.
pub fn is_label_name_taken(error: &sqlx::Error) -> bool {
    error.as_database_error().is_some_and(|db| {
        db.is_unique_violation() && db.constraint() == Some(LABEL_NAME_UNIQUE_CONSTRAINT)
    })
}

/// Outcome of assigning a label to a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelAssignOutcome {
    Added,
    AlreadyAssigned,
    DocumentNotFound,
    LabelNotFound,
}

/// Outcome of removing a label from a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRemoveOutcome {
    Removed,
    NotAssigned,
    DocumentNotFound,
}

/// Label queries for one user. Labels and label assignments belong to the user who owns them:
/// every function here scopes by `user_id`, whatever that user's role.
impl Database {
    /// The user's labels by name, optionally with how many documents and sources use each.
    pub async fn list_labels(&self, user_id: Uuid, include_counts: bool) -> Result<Vec<Label>> {
        let labels = if include_counts {
            sqlx::query_as::<_, Label>(
                r#"
                SELECT
                    l.id, l.user_id, l.name, l.description, l.color,
                    l.background_color, l.icon, l.created_at, l.updated_at,
                    COUNT(DISTINCT dl.document_id) as document_count,
                    COUNT(DISTINCT sl.source_id) as source_count
                FROM labels l
                LEFT JOIN document_labels dl ON l.id = dl.label_id
                LEFT JOIN source_labels sl ON l.id = sl.label_id
                WHERE l.user_id = $1
                GROUP BY l.id, l.user_id, l.name, l.description, l.color,
                         l.background_color, l.icon, l.created_at, l.updated_at
                ORDER BY l.name
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Label>(
                r#"
                SELECT
                    id, user_id, name, description, color, background_color, icon,
                    created_at, updated_at, 0::bigint as document_count, 0::bigint as source_count
                FROM labels
                WHERE user_id = $1
                ORDER BY name
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(labels)
    }

    /// One of the user's labels with its usage counts.
    pub async fn get_label_for_user(&self, label_id: Uuid, user_id: Uuid) -> Result<Option<Label>> {
        let label = sqlx::query_as::<_, Label>(
            r#"
            SELECT
                l.id, l.user_id, l.name, l.description, l.color,
                l.background_color, l.icon, l.created_at, l.updated_at,
                COUNT(DISTINCT dl.document_id) as document_count,
                COUNT(DISTINCT sl.source_id) as source_count
            FROM labels l
            LEFT JOIN document_labels dl ON l.id = dl.label_id
            LEFT JOIN source_labels sl ON l.id = sl.label_id
            WHERE l.id = $1 AND l.user_id = $2
            GROUP BY l.id, l.user_id, l.name, l.description, l.color,
                     l.background_color, l.icon, l.created_at, l.updated_at
            "#,
        )
        .bind(label_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(label)
    }

    /// Inserts a label for the user. Call [`CreateLabel::validate`] first; a name the user
    /// already has fails with an error [`is_label_name_taken`] recognises.
    pub async fn create_label(
        &self,
        user_id: Uuid,
        label: &CreateLabel,
    ) -> std::result::Result<Label, sqlx::Error> {
        sqlx::query_as::<_, Label>(
            r#"
            INSERT INTO labels (user_id, name, description, color, background_color, icon)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, user_id, name, description, color, background_color, icon,
                created_at, updated_at, 0::bigint as document_count, 0::bigint as source_count
            "#,
        )
        .bind(user_id)
        .bind(label.name.for_db())
        .bind(label.description.for_db())
        .bind(label.color.for_db())
        .bind(label.background_color.for_db())
        .bind(label.icon.for_db())
        .fetch_one(&self.pool)
        .await
    }

    /// Updates the fields `changes` sets on one of the user's labels, leaving the rest as they
    /// are. `None` when the user has no such label. Call [`UpdateLabel::validate`] first.
    pub async fn update_label(
        &self,
        label_id: Uuid,
        user_id: Uuid,
        changes: &UpdateLabel,
    ) -> std::result::Result<Option<Label>, sqlx::Error> {
        sqlx::query_as::<_, Label>(
            r#"
            UPDATE labels
            SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                color = COALESCE($5, color),
                background_color = COALESCE($6, background_color),
                icon = COALESCE($7, icon),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND user_id = $2
            RETURNING
                id, user_id, name, description, color, background_color, icon,
                created_at, updated_at, 0::bigint as document_count, 0::bigint as source_count
            "#,
        )
        .bind(label_id)
        .bind(user_id)
        .bind(changes.name.for_db())
        .bind(changes.description.for_db())
        .bind(changes.color.for_db())
        .bind(changes.background_color.for_db())
        .bind(changes.icon.for_db())
        .fetch_optional(&self.pool)
        .await
    }

    /// Whether `user_id` owns the document. Label assignments follow document ownership, so
    /// admins cannot relabel other users' documents either.
    pub async fn user_owns_document(&self, document_id: Uuid, user_id: Uuid) -> Result<bool> {
        let owned = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND user_id = $2)",
        )
        .bind(document_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(owned)
    }

    /// True when every id in `label_ids` is a label owned by `user_id`.
    pub async fn user_owns_labels(&self, label_ids: &[Uuid], user_id: Uuid) -> Result<bool> {
        if label_ids.is_empty() {
            return Ok(true);
        }
        let mut unique = label_ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM labels WHERE id = ANY($1) AND user_id = $2")
                .bind(&unique)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(count as usize == unique.len())
    }

    /// Attaches labels to a document, skipping ones it already has. Callers
    /// check label ownership first (`user_owns_labels`).
    pub async fn add_labels_to_document(
        &self,
        document_id: Uuid,
        label_ids: &[Uuid],
        assigned_by: Uuid,
    ) -> Result<()> {
        if label_ids.is_empty() {
            return Ok(());
        }
        sqlx::query(
            r#"
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            SELECT $1, label_id, $3 FROM UNNEST($2::uuid[]) AS label_id
            ON CONFLICT (document_id, label_id) DO NOTHING
            "#,
        )
        .bind(document_id)
        .bind(label_ids)
        .bind(assigned_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The labels on one of the user's documents; `None` when the user has no such document.
    pub async fn get_document_labels_for_user(
        &self,
        document_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Vec<Label>>> {
        if !self.user_owns_document(document_id, user_id).await? {
            return Ok(None);
        }
        Ok(Some(self.get_document_labels(document_id).await?))
    }

    /// Puts one of the user's labels on one of the user's documents.
    ///
    /// One statement checks both ownerships and inserts, so there is no gap between the check
    /// and the write and only one round trip. Concurrent calls for the same pair are settled by
    /// the `(document_id, label_id)` unique constraint: exactly one reports `Added`, the rest
    /// `AlreadyAssigned`.
    pub async fn add_document_label_for_user(
        &self,
        document_id: Uuid,
        label_id: Uuid,
        user_id: Uuid,
    ) -> Result<LabelAssignOutcome> {
        let (owns_document, owns_label, inserted) = sqlx::query_as::<_, (bool, bool, bool)>(
            r#"
            WITH doc AS (
                SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND user_id = $3) AS owned
            ),
            lbl AS (
                SELECT EXISTS(SELECT 1 FROM labels WHERE id = $2 AND user_id = $3) AS owned
            ),
            ins AS (
                INSERT INTO document_labels (document_id, label_id, assigned_by)
                SELECT $1, $2, $3
                WHERE (SELECT owned FROM doc) AND (SELECT owned FROM lbl)
                ON CONFLICT (document_id, label_id) DO NOTHING
                RETURNING 1
            )
            SELECT
                (SELECT owned FROM doc),
                (SELECT owned FROM lbl),
                EXISTS(SELECT 1 FROM ins)
            "#,
        )
        .bind(document_id)
        .bind(label_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(assign_outcome(owns_document, owns_label, inserted))
    }

    /// Takes a label off one of the user's documents. Like
    /// [`Database::add_document_label_for_user`], the ownership check and the delete are one
    /// statement.
    pub async fn remove_document_label_for_user(
        &self,
        document_id: Uuid,
        label_id: Uuid,
        user_id: Uuid,
    ) -> Result<LabelRemoveOutcome> {
        let (owns_document, deleted) = sqlx::query_as::<_, (bool, bool)>(
            r#"
            WITH doc AS (
                SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND user_id = $3) AS owned
            ),
            del AS (
                DELETE FROM document_labels
                WHERE document_id = $1 AND label_id = $2 AND (SELECT owned FROM doc)
                RETURNING 1
            )
            SELECT (SELECT owned FROM doc), EXISTS(SELECT 1 FROM del)
            "#,
        )
        .bind(document_id)
        .bind(label_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(remove_outcome(owns_document, deleted))
    }
}

/// Classifies [`Database::add_document_label_for_user`]'s result. A foreign document wins over
/// a foreign label, so a caller probing someone else's document learns nothing about labels.
fn assign_outcome(owns_document: bool, owns_label: bool, inserted: bool) -> LabelAssignOutcome {
    match (owns_document, owns_label, inserted) {
        (false, _, _) => LabelAssignOutcome::DocumentNotFound,
        (true, false, _) => LabelAssignOutcome::LabelNotFound,
        (true, true, true) => LabelAssignOutcome::Added,
        (true, true, false) => LabelAssignOutcome::AlreadyAssigned,
    }
}

/// Classifies [`Database::remove_document_label_for_user`]'s result.
fn remove_outcome(owns_document: bool, deleted: bool) -> LabelRemoveOutcome {
    match (owns_document, deleted) {
        (false, _) => LabelRemoveOutcome::DocumentNotFound,
        (true, true) => LabelRemoveOutcome::Removed,
        (true, false) => LabelRemoveOutcome::NotAssigned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::error::{DatabaseError, ErrorKind};
    use std::borrow::Cow;

    /// A database error with a chosen message, SQLSTATE class and constraint, to show
    /// [`is_label_name_taken`] reads the structured fields and ignores the text.
    #[derive(Debug)]
    struct FakeDbError {
        message: &'static str,
        unique_violation: bool,
        constraint: Option<&'static str>,
    }

    impl std::fmt::Display for FakeDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.message)
        }
    }

    impl std::error::Error for FakeDbError {}

    impl DatabaseError for FakeDbError {
        fn message(&self) -> &str {
            self.message
        }
        fn code(&self) -> Option<Cow<'_, str>> {
            self.unique_violation.then_some(Cow::Borrowed("23505"))
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn constraint(&self) -> Option<&str> {
            self.constraint
        }
        fn kind(&self) -> ErrorKind {
            if self.unique_violation {
                ErrorKind::UniqueViolation
            } else {
                ErrorKind::Other
            }
        }
    }

    fn db_error(
        message: &'static str,
        unique_violation: bool,
        constraint: Option<&'static str>,
    ) -> sqlx::Error {
        sqlx::Error::Database(Box::new(FakeDbError {
            message,
            unique_violation,
            constraint,
        }))
    }

    #[test]
    fn name_clash_is_recognised_whatever_the_server_language() {
        // What a server with lc_messages = 'de_DE' sends: no English words at all.
        let german = db_error(
            "doppelter Schlüsselwert verletzt Unique-Constraint »unique_user_label_name«",
            true,
            Some(LABEL_NAME_UNIQUE_CONSTRAINT),
        );
        assert!(is_label_name_taken(&german));
    }

    #[test]
    fn english_duplicate_key_text_alone_is_not_a_name_clash() {
        // The old text match would have said yes to all of these.
        let not_unique = db_error(
            "duplicate key value violates unique constraint \"unique_user_label_name\"",
            false,
            Some(LABEL_NAME_UNIQUE_CONSTRAINT),
        );
        assert!(!is_label_name_taken(&not_unique));

        let other_constraint = db_error(
            "duplicate key value violates unique constraint \"labels_pkey\"",
            true,
            Some("labels_pkey"),
        );
        assert!(!is_label_name_taken(&other_constraint));

        let no_constraint = db_error("duplicate key value violates unique constraint", true, None);
        assert!(!is_label_name_taken(&no_constraint));

        let not_a_database_error = sqlx::Error::Protocol("duplicate key".into());
        assert!(!is_label_name_taken(&not_a_database_error));
    }

    #[test]
    fn assign_outcome_reports_a_foreign_document_before_a_foreign_label() {
        use LabelAssignOutcome::*;
        assert_eq!(assign_outcome(false, false, false), DocumentNotFound);
        assert_eq!(assign_outcome(false, true, false), DocumentNotFound);
        assert_eq!(assign_outcome(true, false, false), LabelNotFound);
        assert_eq!(assign_outcome(true, true, true), Added);
        assert_eq!(assign_outcome(true, true, false), AlreadyAssigned);
    }

    #[test]
    fn remove_outcome_reports_a_foreign_document_first() {
        use LabelRemoveOutcome::*;
        assert_eq!(remove_outcome(false, false), DocumentNotFound);
        assert_eq!(remove_outcome(true, true), Removed);
        assert_eq!(remove_outcome(true, false), NotAssigned);
    }
}
