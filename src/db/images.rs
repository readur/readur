use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use super::Database;

impl Database {
    pub async fn create_processed_image(&self, processed_image: &crate::models::CreateProcessedImage) -> Result<crate::models::ProcessedImage> {
        let row = sqlx::query(
            r#"INSERT INTO processed_images 
               (document_id, user_id, original_image_path, processed_image_path, 
                processing_parameters, processing_steps, image_width, image_height, file_size)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING id, document_id, user_id, original_image_path, processed_image_path,
                         processing_parameters, processing_steps, image_width, image_height, 
                         file_size, created_at"#
        )
        .bind(processed_image.document_id)
        .bind(processed_image.user_id)
        .bind(&processed_image.original_image_path)
        .bind(&processed_image.processed_image_path)
        .bind(&processed_image.processing_parameters)
        .bind(&processed_image.processing_steps)
        .bind(processed_image.image_width)
        .bind(processed_image.image_height)
        .bind(processed_image.file_size)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::ProcessedImage {
            id: row.get("id"),
            document_id: row.get("document_id"),
            user_id: row.get("user_id"),
            original_image_path: row.get("original_image_path"),
            processed_image_path: row.get("processed_image_path"),
            processing_parameters: row.get("processing_parameters"),
            processing_steps: row.get("processing_steps"),
            image_width: row.get("image_width"),
            image_height: row.get("image_height"),
            file_size: row.get("file_size"),
            created_at: row.get("created_at"),
        })
    }

    pub async fn get_processed_image_by_document_id(&self, document_id: Uuid, user_id: Uuid) -> Result<Option<crate::models::ProcessedImage>> {
        let row = sqlx::query(
            r#"SELECT id, document_id, user_id, original_image_path, processed_image_path,
                      processing_parameters, processing_steps, image_width, image_height, 
                      file_size, created_at
               FROM processed_images 
               WHERE document_id = $1 AND user_id = $2
               ORDER BY created_at DESC
               LIMIT 1"#
        )
        .bind(document_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::ProcessedImage {
                id: row.get("id"),
                document_id: row.get("document_id"),
                user_id: row.get("user_id"),
                original_image_path: row.get("original_image_path"),
                processed_image_path: row.get("processed_image_path"),
                processing_parameters: row.get("processing_parameters"),
                processing_steps: row.get("processing_steps"),
                image_width: row.get("image_width"),
                image_height: row.get("image_height"),
                file_size: row.get("file_size"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_processed_images_by_document_id(&self, document_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM processed_images WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}