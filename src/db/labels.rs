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

/// Whether a failed insert/update hit a unique constraint (a label name the user already has).
pub fn is_duplicate_key(error: &sqlx::Error) -> bool {
    error.to_string().contains("duplicate key")
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
    /// already has fails with an error [`is_duplicate_key`] recognises.
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

    /// Whether `user_id` owns the label.
    pub async fn user_owns_label(&self, label_id: Uuid, user_id: Uuid) -> Result<bool> {
        let owned = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM labels WHERE id = $1 AND user_id = $2)",
        )
        .bind(label_id)
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
    pub async fn add_document_label_for_user(
        &self,
        document_id: Uuid,
        label_id: Uuid,
        user_id: Uuid,
    ) -> Result<LabelAssignOutcome> {
        if !self.user_owns_document(document_id, user_id).await? {
            return Ok(LabelAssignOutcome::DocumentNotFound);
        }
        if !self.user_owns_label(label_id, user_id).await? {
            return Ok(LabelAssignOutcome::LabelNotFound);
        }
        let result = sqlx::query(
            "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3)
             ON CONFLICT (document_id, label_id) DO NOTHING",
        )
        .bind(document_id)
        .bind(label_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(if result.rows_affected() == 0 {
            LabelAssignOutcome::AlreadyAssigned
        } else {
            LabelAssignOutcome::Added
        })
    }

    /// Takes a label off one of the user's documents.
    pub async fn remove_document_label_for_user(
        &self,
        document_id: Uuid,
        label_id: Uuid,
        user_id: Uuid,
    ) -> Result<LabelRemoveOutcome> {
        if !self.user_owns_document(document_id, user_id).await? {
            return Ok(LabelRemoveOutcome::DocumentNotFound);
        }
        let result =
            sqlx::query("DELETE FROM document_labels WHERE document_id = $1 AND label_id = $2")
                .bind(document_id)
                .bind(label_id)
                .execute(&self.pool)
                .await?;
        Ok(if result.rows_affected() == 0 {
            LabelRemoveOutcome::NotAssigned
        } else {
            LabelRemoveOutcome::Removed
        })
    }
}
