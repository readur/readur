use anyhow::Result;
use sqlx::PgConnection;
use uuid::Uuid;

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
