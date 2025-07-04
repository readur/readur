use anyhow::Result;
use sqlx::{QueryBuilder, Postgres};
use uuid::Uuid;

use crate::models::{Document, UserRole};
use super::helpers::{map_row_to_document, apply_role_based_filter, apply_pagination, DOCUMENT_FIELDS};
use crate::db::Database;

impl Database {
    /// Creates a new document in the database
    pub async fn create_document(&self, document: Document) -> Result<Document> {
        let query_str = format!(
            r#"
            INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING {}
            "#,
            DOCUMENT_FIELDS
        );

        let row = sqlx::query(&query_str)
            .bind(document.id)
            .bind(&document.filename)
            .bind(&document.original_filename)
            .bind(&document.file_path)
            .bind(document.file_size)
            .bind(&document.mime_type)
            .bind(&document.content)
            .bind(&document.ocr_text)
            .bind(document.ocr_confidence)
            .bind(document.ocr_word_count)
            .bind(document.ocr_processing_time_ms)
            .bind(&document.ocr_status)
            .bind(&document.ocr_error)
            .bind(document.ocr_completed_at)
            .bind(document.ocr_retry_count)
            .bind(&document.ocr_failure_reason)
            .bind(&document.tags)
            .bind(document.created_at)
            .bind(document.updated_at)
            .bind(document.user_id)
            .bind(&document.file_hash)
            .bind(document.original_created_at)
            .bind(document.original_modified_at)
            .bind(&document.source_metadata)
            .fetch_one(&self.pool)
            .await?;

        Ok(map_row_to_document(&row))
    }

    /// Retrieves a document by ID with role-based access control
    pub async fn get_document_by_id(&self, document_id: Uuid, user_id: Uuid, user_role: UserRole) -> Result<Option<Document>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(" FROM documents WHERE id = ");
        query.push_bind(document_id);
        
        apply_role_based_filter(&mut query, user_id, user_role);

        let row = query
            .build()
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| map_row_to_document(&r)))
    }

    /// Gets documents for a user with role-based access and pagination
    pub async fn get_documents_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let query_str = format!(
            r#"
            SELECT {}
            FROM documents 
            WHERE user_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
            "#,
            DOCUMENT_FIELDS
        );

        let rows = sqlx::query(&query_str)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Gets documents with role-based access control
    pub async fn get_documents_by_user_with_role(&self, user_id: Uuid, user_role: UserRole, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(" FROM documents WHERE 1=1");
        
        apply_role_based_filter(&mut query, user_id, user_role);
        query.push(" ORDER BY created_at DESC");
        apply_pagination(&mut query, limit, offset);

        let rows = query
            .build()
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Finds a document by user and file hash (for duplicate detection)
    pub async fn get_document_by_user_and_hash(&self, user_id: Uuid, file_hash: &str) -> Result<Option<Document>> {
        let query_str = format!(
            r#"
            SELECT {}
            FROM documents 
            WHERE user_id = $1 AND file_hash = $2
            "#,
            DOCUMENT_FIELDS
        );

        let row = sqlx::query(&query_str)
            .bind(user_id)
            .bind(file_hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| map_row_to_document(&r)))
    }

    /// Finds documents by filename or original filename
    pub async fn find_documents_by_filename(&self, user_id: Uuid, filename: &str, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let query_str = format!(
            r#"
            SELECT {}
            FROM documents 
            WHERE user_id = $1 AND (filename ILIKE $2 OR original_filename ILIKE $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            DOCUMENT_FIELDS
        );

        let search_pattern = format!("%{}%", filename);
        let rows = sqlx::query(&query_str)
            .bind(user_id)
            .bind(search_pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Updates the OCR text for a document
    pub async fn update_document_ocr(&self, document_id: Uuid, ocr_text: Option<String>, ocr_confidence: Option<f32>, ocr_word_count: Option<i32>, ocr_processing_time_ms: Option<i32>, ocr_status: Option<String>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE documents 
            SET ocr_text = $2, ocr_confidence = $3, ocr_word_count = $4, ocr_processing_time_ms = $5, ocr_status = $6, updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(document_id)
        .bind(ocr_text)
        .bind(ocr_confidence)
        .bind(ocr_word_count)
        .bind(ocr_processing_time_ms)
        .bind(ocr_status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets recent documents for a specific source
    pub async fn get_recent_documents_for_source(&self, user_id: Uuid, source_id: Uuid, limit: i64) -> Result<Vec<Document>> {
        let query_str = format!(
            r#"
            SELECT {}
            FROM documents 
            WHERE user_id = $1 AND source_metadata->>'source_id' = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
            DOCUMENT_FIELDS
        );

        let rows = sqlx::query(&query_str)
            .bind(user_id)
            .bind(source_id.to_string())
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(map_row_to_document).collect())
    }
}