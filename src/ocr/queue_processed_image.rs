//! Saving the preprocessed OCR input image for user review.

use anyhow::Result;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::OcrQueueService;

impl OcrQueueService {
    /// Save processed image for review when the setting is enabled
    pub(super) async fn save_processed_image_for_review(
        &self,
        document_id: Uuid,
        user_id: Uuid,
        original_image_path: &str,
        processed_image_path: &str,
        processing_steps: &[String],
    ) -> Result<()> {
        use std::path::Path;

        // Use the FileService to get the proper processed images directory
        let processed_images_dir = self.file_service.get_processed_images_path();

        // Ensure the directory exists with proper error handling
        if let Err(e) = tokio::fs::create_dir_all(&processed_images_dir).await {
            error!(
                "Failed to create processed images directory {:?}: {}",
                processed_images_dir, e
            );
            return Err(anyhow::anyhow!(
                "Failed to create processed images directory: {}",
                e
            ));
        }

        info!(
            "Ensured processed images directory exists: {:?}",
            processed_images_dir
        );

        // Generate a unique filename for the processed image
        let extension = Path::new(processed_image_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg");

        let permanent_filename = format!(
            "{}_processed_{}.{}",
            document_id,
            chrono::Utc::now().timestamp(),
            extension
        );
        let permanent_path = processed_images_dir.join(&permanent_filename);

        // Verify source file exists before copying
        if !Path::new(processed_image_path).exists() {
            return Err(anyhow::anyhow!(
                "Source processed image file does not exist: {}",
                processed_image_path
            ));
        }

        // Copy the processed image to permanent location with error handling
        if let Err(e) = tokio::fs::copy(processed_image_path, &permanent_path).await {
            error!(
                "Failed to copy processed image from {} to {:?}: {}",
                processed_image_path, permanent_path, e
            );
            return Err(anyhow::anyhow!("Failed to copy processed image: {}", e));
        }

        info!(
            "Successfully copied processed image to: {:?}",
            permanent_path
        );

        // Get actual image dimensions and file size
        let image_metadata = tokio::fs::metadata(&permanent_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get processed image metadata: {}", e))?;
        let file_size = image_metadata.len() as i64;

        // Get image dimensions using image crate
        let (image_width, image_height) = tokio::task::spawn_blocking({
            let path = permanent_path.clone();
            move || -> Result<(u32, u32), anyhow::Error> {
                let img = image::open(&path).map_err(|e| {
                    anyhow::anyhow!("Failed to open processed image for dimensions: {}", e)
                })?;
                Ok((img.width(), img.height()))
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get image dimensions: {}", e))??;

        // Save to database
        let processing_parameters = serde_json::json!({
            "steps": processing_steps,
            "timestamp": chrono::Utc::now(),
            "original_path": original_image_path,
        });

        // Save metadata to database with error handling
        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO processed_images (document_id, user_id, original_image_path, processed_image_path, processing_parameters, processing_steps, image_width, image_height, file_size, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#
        )
        .bind(document_id)
        .bind(user_id)
        .bind(crate::db::sanitize::DbText(original_image_path))
        .bind(permanent_path.to_string_lossy().to_string())
        .bind(crate::db::sanitize::DbJson(&processing_parameters))
        .bind(processing_steps)
        .bind(image_width as i32)
        .bind(image_height as i32)
        .bind(file_size)
        .execute(&self.pool)
        .await {
            error!("Failed to save processed image metadata to database for document {}: {}", document_id, e);

            // Clean up the copied file if database save fails
            if let Err(cleanup_err) = tokio::fs::remove_file(&permanent_path).await {
                warn!("Failed to clean up processed image file after database error: {}", cleanup_err);
            }

            return Err(anyhow::anyhow!("Failed to save processed image metadata: {}", e));
        }

        info!(
            "Successfully saved processed image metadata for document {} to database",
            document_id
        );

        Ok(())
    }
}
