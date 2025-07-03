use anyhow::Result;
use sqlx::{QueryBuilder, Postgres, Row};
use uuid::Uuid;

use crate::models::{Document, UserRole, FailedDocument};
use super::helpers::{map_row_to_document, apply_role_based_filter, DOCUMENT_FIELDS};
use crate::db::Database;

impl Database {
    /// Deletes a single document with role-based access control
    pub async fn delete_document(&self, document_id: Uuid, user_id: Uuid, user_role: UserRole) -> Result<bool> {
        let mut query = QueryBuilder::<Postgres>::new("DELETE FROM documents WHERE id = ");
        query.push_bind(document_id);
        
        apply_role_based_filter(&mut query, user_id, user_role);

        let result = query.build().execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    /// Bulk deletes multiple documents with role-based access control
    pub async fn bulk_delete_documents(&self, document_ids: &[Uuid], user_id: Uuid, user_role: UserRole) -> Result<(Vec<Uuid>, Vec<Uuid>)> {
        if document_ids.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut tx = self.pool.begin().await?;
        let mut deleted_ids = Vec::new();
        let mut failed_ids = Vec::new();

        for &doc_id in document_ids {
            let mut query = QueryBuilder::<Postgres>::new("DELETE FROM documents WHERE id = ");
            query.push_bind(doc_id);
            
            apply_role_based_filter(&mut query, user_id, user_role);
            query.push(" RETURNING id");

            match query.build().fetch_optional(&mut *tx).await {
                Ok(Some(row)) => {
                    let deleted_id: Uuid = row.get("id");
                    deleted_ids.push(deleted_id);
                }
                Ok(None) => {
                    failed_ids.push(doc_id);
                }
                Err(_) => {
                    failed_ids.push(doc_id);
                }
            }
        }

        tx.commit().await?;
        Ok((deleted_ids, failed_ids))
    }

    /// Finds documents with OCR confidence below threshold
    pub async fn find_documents_by_confidence_threshold(&self, user_id: Uuid, user_role: UserRole, max_confidence: f32, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(" FROM documents WHERE ocr_confidence IS NOT NULL AND ocr_confidence <= ");
        query.push_bind(max_confidence);

        apply_role_based_filter(&mut query, user_id, user_role);
        query.push(" ORDER BY ocr_confidence ASC, created_at DESC");
        query.push(" LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Finds documents with failed OCR processing
    pub async fn find_failed_ocr_documents(&self, user_id: Uuid, user_role: UserRole, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(" FROM documents WHERE ocr_status = 'failed'");

        apply_role_based_filter(&mut query, user_id, user_role);
        query.push(" ORDER BY created_at DESC");
        query.push(" LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Finds both low confidence and failed OCR documents
    pub async fn find_low_confidence_and_failed_documents(&self, user_id: Uuid, user_role: UserRole, max_confidence: f32, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(" FROM documents WHERE (ocr_status = 'failed' OR (ocr_confidence IS NOT NULL AND ocr_confidence <= ");
        query.push_bind(max_confidence);
        query.push("))");

        apply_role_based_filter(&mut query, user_id, user_role);
        query.push(" ORDER BY CASE WHEN ocr_status = 'failed' THEN 0 ELSE 1 END, ocr_confidence ASC, created_at DESC");
        query.push(" LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Creates a failed document record
    pub async fn create_failed_document(&self, failed_document: FailedDocument) -> Result<FailedDocument> {
        let row = sqlx::query(
            r#"
            INSERT INTO failed_documents (
                id, user_id, filename, original_filename, original_path, file_path, 
                file_size, file_hash, mime_type, content, tags, ocr_text, ocr_confidence, 
                ocr_word_count, ocr_processing_time_ms, failure_reason, failure_stage, 
                existing_document_id, ingestion_source, error_message, retry_count, 
                last_retry_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING *
            "#
        )
        .bind(failed_document.id)
        .bind(failed_document.user_id)
        .bind(&failed_document.filename)
        .bind(&failed_document.original_filename)
        .bind(&failed_document.original_path)
        .bind(&failed_document.file_path)
        .bind(failed_document.file_size)
        .bind(&failed_document.file_hash)
        .bind(&failed_document.mime_type)
        .bind(&failed_document.content)
        .bind(&failed_document.tags)
        .bind(&failed_document.ocr_text)
        .bind(failed_document.ocr_confidence)
        .bind(failed_document.ocr_word_count)
        .bind(failed_document.ocr_processing_time_ms)
        .bind(&failed_document.failure_reason)
        .bind(&failed_document.failure_stage)
        .bind(failed_document.existing_document_id)
        .bind(&failed_document.ingestion_source)
        .bind(&failed_document.error_message)
        .bind(failed_document.retry_count)
        .bind(failed_document.last_retry_at)
        .bind(failed_document.created_at)
        .bind(failed_document.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(FailedDocument {
            id: row.get("id"),
            user_id: row.get("user_id"),
            filename: row.get("filename"),
            original_filename: row.get("original_filename"),
            original_path: row.get("original_path"),
            file_path: row.get("file_path"),
            file_size: row.get("file_size"),
            file_hash: row.get("file_hash"),
            mime_type: row.get("mime_type"),
            content: row.get("content"),
            tags: row.get("tags"),
            ocr_text: row.get("ocr_text"),
            ocr_confidence: row.get("ocr_confidence"),
            ocr_word_count: row.get("ocr_word_count"),
            ocr_processing_time_ms: row.get("ocr_processing_time_ms"),
            failure_reason: row.get("failure_reason"),
            failure_stage: row.get("failure_stage"),
            existing_document_id: row.get("existing_document_id"),
            ingestion_source: row.get("ingestion_source"),
            error_message: row.get("error_message"),
            retry_count: row.get("retry_count"),
            last_retry_at: row.get("last_retry_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Creates a failed document record from an existing document
    pub async fn create_failed_document_from_document(&self, document: &Document, failure_reason: &str, failure_stage: &str, error_message: Option<&str>) -> Result<FailedDocument> {
        let failed_doc = FailedDocument {
            id: Uuid::new_v4(),
            user_id: document.user_id,
            filename: document.filename.clone(),
            original_filename: Some(document.original_filename.clone()),
            original_path: Some(document.file_path.clone()),
            file_path: Some(document.file_path.clone()),
            file_size: Some(document.file_size),
            file_hash: document.file_hash.clone(),
            mime_type: Some(document.mime_type.clone()),
            content: document.content.clone(),
            tags: document.tags.clone(),
            ocr_text: document.ocr_text.clone(),
            ocr_confidence: document.ocr_confidence,
            ocr_word_count: document.ocr_word_count,
            ocr_processing_time_ms: document.ocr_processing_time_ms,
            failure_reason: failure_reason.to_string(),
            failure_stage: failure_stage.to_string(),
            existing_document_id: Some(document.id),
            ingestion_source: "document_processing".to_string(),
            error_message: error_message.map(|s| s.to_string()),
            retry_count: Some(0),
            last_retry_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.create_failed_document(failed_doc).await
    }

    /// Updates OCR retry information for a document
    pub async fn update_document_ocr_retry(&self, document_id: Uuid, retry_count: i32, failure_reason: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE documents 
            SET ocr_retry_count = $2, ocr_failure_reason = $3, updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(document_id)
        .bind(retry_count)
        .bind(failure_reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Marks documents as completed OCR processing
    pub async fn mark_documents_ocr_completed(&self, document_ids: &[Uuid]) -> Result<u64> {
        if document_ids.is_empty() {
            return Ok(0);
        }

        let result = sqlx::query(
            r#"
            UPDATE documents 
            SET ocr_status = 'completed', ocr_completed_at = NOW(), updated_at = NOW()
            WHERE id = ANY($1)
            "#
        )
        .bind(document_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Counts documents by OCR status
    pub async fn count_documents_by_ocr_status(&self, user_id: Uuid, user_role: UserRole) -> Result<(i64, i64, i64, i64)> {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(CASE WHEN ocr_status IS NULL OR ocr_status = 'pending' THEN 1 END) as pending,
                COUNT(CASE WHEN ocr_status = 'completed' THEN 1 END) as completed,
                COUNT(CASE WHEN ocr_status = 'failed' THEN 1 END) as failed
            FROM documents WHERE 1=1
            "#
        );

        apply_role_based_filter(&mut query, user_id, user_role);

        let row = query.build().fetch_one(&self.pool).await?;

        Ok((
            row.get("total"),
            row.get("pending"),
            row.get("completed"),
            row.get("failed"),
        ))
    }
}