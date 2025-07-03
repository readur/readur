use anyhow::Result;
use sqlx::{QueryBuilder, Postgres, Row};
use uuid::Uuid;

use crate::models::{Document, UserRole, FacetItem};
use crate::routes::labels::Label;
use super::helpers::{map_row_to_document, apply_role_based_filter, DOCUMENT_FIELDS};
use crate::db::Database;

impl Database {
    /// Gets labels for a specific document
    pub async fn get_document_labels(&self, document_id: Uuid) -> Result<Vec<Label>> {
        let rows = sqlx::query_as::<_, Label>(
            r#"
            SELECT l.id, l.user_id, l.name, l.color, l.created_at, l.updated_at
            FROM labels l
            JOIN document_labels dl ON l.id = dl.label_id
            WHERE dl.document_id = $1
            ORDER BY l.name
            "#
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Gets labels for multiple documents in batch
    pub async fn get_labels_for_documents(&self, document_ids: &[Uuid]) -> Result<Vec<(Uuid, Vec<Label>)>> {
        if document_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT dl.document_id, l.id as label_id, l.user_id, l.name, l.color, l.created_at, l.updated_at
            FROM labels l
            JOIN document_labels dl ON l.id = dl.label_id
            WHERE dl.document_id = ANY($1)
            ORDER BY dl.document_id, l.name
            "#
        )
        .bind(document_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        let mut current_doc_id: Option<Uuid> = None;
        let mut current_labels = Vec::new();

        for row in rows {
            let doc_id: Uuid = row.get("document_id");
            let label = Label {
                id: row.get("label_id"),
                user_id: Some(row.get("user_id")),
                name: row.get("name"),
                description: None,
                color: row.get("color"),
                background_color: None,
                icon: None,
                is_system: false,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                document_count: 0,
            };

            if Some(doc_id) != current_doc_id {
                if let Some(prev_doc_id) = current_doc_id {
                    result.push((prev_doc_id, std::mem::take(&mut current_labels)));
                }
                current_doc_id = Some(doc_id);
            }

            current_labels.push(label);
        }

        if let Some(doc_id) = current_doc_id {
            result.push((doc_id, current_labels));
        }

        Ok(result)
    }

    /// Finds duplicate documents by file hash for a user
    pub async fn get_user_duplicates(&self, user_id: Uuid, user_role: UserRole, limit: i64, offset: i64) -> Result<Vec<Vec<Document>>> {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"
            WITH duplicate_hashes AS (
                SELECT file_hash, COUNT(*) as count
                FROM documents 
                WHERE file_hash IS NOT NULL
            "#
        );

        if user_role != UserRole::Admin {
            query.push(" AND user_id = ");
            query.push_bind(user_id);
        }

        query.push(
            r#"
                GROUP BY file_hash
                HAVING COUNT(*) > 1
            )
            SELECT d.*
            FROM documents d
            JOIN duplicate_hashes dh ON d.file_hash = dh.file_hash
            WHERE d.file_hash IS NOT NULL
            "#
        );

        if user_role != UserRole::Admin {
            query.push(" AND d.user_id = ");
            query.push_bind(user_id);
        }

        query.push(" ORDER BY d.file_hash, d.created_at");

        let rows = query.build().fetch_all(&self.pool).await?;
        let documents: Vec<Document> = rows.iter().map(map_row_to_document).collect();

        // Group documents by file hash
        let mut duplicate_groups = Vec::new();
        let mut current_group = Vec::new();
        let mut current_hash: Option<String> = None;

        for document in documents {
            if document.file_hash != current_hash {
                if !current_group.is_empty() {
                    duplicate_groups.push(std::mem::take(&mut current_group));
                }
                current_hash = document.file_hash.clone();
            }
            current_group.push(document);
        }

        if !current_group.is_empty() {
            duplicate_groups.push(current_group);
        }

        // Apply pagination to groups
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(duplicate_groups.into_iter().skip(start).take(end - start).collect())
    }

    /// Gets MIME type facets (aggregated counts by MIME type)
    pub async fn get_mime_type_facets(&self, user_id: Uuid, user_role: UserRole) -> Result<Vec<FacetItem>> {
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT mime_type as value, COUNT(*) as count FROM documents WHERE 1=1"
        );

        apply_role_based_filter(&mut query, user_id, user_role);
        query.push(" GROUP BY mime_type ORDER BY count DESC, mime_type");

        let rows = query.build().fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|row| FacetItem {
            value: row.get("value"),
            count: row.get("count"),
        }).collect())
    }

    /// Gets tag facets (aggregated counts by tag)
    pub async fn get_tag_facets(&self, user_id: Uuid, user_role: UserRole) -> Result<Vec<FacetItem>> {
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT unnest(tags) as value, COUNT(*) as count FROM documents WHERE 1=1"
        );

        apply_role_based_filter(&mut query, user_id, user_role);
        query.push(" GROUP BY unnest(tags) ORDER BY count DESC, value");

        let rows = query.build().fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|row| FacetItem {
            value: row.get("value"),
            count: row.get("count"),
        }).collect())
    }

    /// Counts documents for a specific source
    pub async fn count_documents_for_source(&self, user_id: Uuid, source_id: Uuid) -> Result<(i64, i64)> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_documents,
                COUNT(CASE WHEN ocr_text IS NOT NULL THEN 1 END) as total_documents_ocr
            FROM documents 
            WHERE user_id = $1 AND source_metadata->>'source_id' = $2
            "#
        )
        .bind(user_id)
        .bind(source_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok((row.get("total_documents"), row.get("total_documents_ocr")))
    }

    /// Counts documents for multiple sources in batch
    pub async fn count_documents_for_sources(&self, user_id: Uuid, source_ids: &[Uuid]) -> Result<Vec<(Uuid, i64, i64)>> {
        if source_ids.is_empty() {
            return Ok(Vec::new());
        }

        let source_id_strings: Vec<String> = source_ids.iter().map(|id| id.to_string()).collect();
        
        let rows = sqlx::query(
            r#"
            SELECT 
                source_metadata->>'source_id' as source_id_str,
                COUNT(*) as total_documents,
                COUNT(CASE WHEN ocr_text IS NOT NULL THEN 1 END) as total_documents_ocr
            FROM documents 
            WHERE user_id = $1 AND source_metadata->>'source_id' = ANY($2)
            GROUP BY source_metadata->>'source_id'
            "#
        )
        .bind(user_id)
        .bind(&source_id_strings)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| {
            let source_id_str: String = row.get("source_id_str");
            let source_id = Uuid::parse_str(&source_id_str).unwrap_or_default();
            let total_documents: i64 = row.get("total_documents");
            let total_documents_ocr: i64 = row.get("total_documents_ocr");
            (source_id, total_documents, total_documents_ocr)
        }).collect())
    }

    /// Gets documents by user with role-based access and OCR status filtering
    pub async fn get_documents_by_user_with_role_and_filter(
        &self, 
        user_id: Uuid, 
        user_role: UserRole, 
        ocr_status: Option<&str>, 
        limit: i64, 
        offset: i64
    ) -> Result<Vec<Document>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(" FROM documents WHERE 1=1");

        apply_role_based_filter(&mut query, user_id, user_role);

        if let Some(status) = ocr_status {
            match status {
                "pending" => {
                    query.push(" AND (ocr_status IS NULL OR ocr_status = 'pending')");
                }
                "completed" => {
                    query.push(" AND ocr_status = 'completed'");
                }
                "failed" => {
                    query.push(" AND ocr_status = 'failed'");
                }
                _ => {
                    query.push(" AND ocr_status = ");
                    query.push_bind(status);
                }
            }
        }

        query.push(" ORDER BY created_at DESC");
        query.push(" LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Counts documents with role-based access and OCR status filtering
    pub async fn get_documents_count_with_role_and_filter(
        &self, 
        user_id: Uuid, 
        user_role: UserRole, 
        ocr_status: Option<&str>
    ) -> Result<i64> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM documents WHERE 1=1");

        apply_role_based_filter(&mut query, user_id, user_role);

        if let Some(status) = ocr_status {
            match status {
                "pending" => {
                    query.push(" AND (ocr_status IS NULL OR ocr_status = 'pending')");
                }
                "completed" => {
                    query.push(" AND ocr_status = 'completed'");
                }
                "failed" => {
                    query.push(" AND ocr_status = 'failed'");
                }
                _ => {
                    query.push(" AND ocr_status = ");
                    query.push_bind(status);
                }
            }
        }

        let row = query.build().fetch_one(&self.pool).await?;
        Ok(row.get(0))
    }
}