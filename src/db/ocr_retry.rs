use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OcrRetryHistory {
    pub id: Uuid,
    pub document_id: Uuid,
    pub user_id: Uuid,
    pub retry_reason: Option<String>,
    pub previous_status: Option<String>,
    pub previous_failure_reason: Option<String>,
    pub previous_error: Option<String>,
    pub priority: i32,
    pub queue_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Record an OCR retry attempt
pub async fn record_ocr_retry(
    pool: &PgPool,
    document_id: Uuid,
    user_id: Uuid,
    retry_reason: &str,
    priority: i32,
    queue_id: Option<Uuid>,
) -> Result<Uuid> {
    // First get the current OCR status
    let current_status = sqlx::query(
        r#"
        SELECT ocr_status, ocr_failure_reason, ocr_error
        FROM documents
        WHERE id = $1
        "#
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    
    let (previous_status, previous_failure_reason, previous_error) = if let Some(row) = current_status {
        (
            row.get::<Option<String>, _>("ocr_status"),
            row.get::<Option<String>, _>("ocr_failure_reason"),
            row.get::<Option<String>, _>("ocr_error"),
        )
    } else {
        (None, None, None)
    };
    
    // Insert retry history record
    let retry_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO ocr_retry_history (
            document_id, user_id, retry_reason, previous_status, 
            previous_failure_reason, previous_error, priority, queue_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#
    )
    .bind(document_id)
    .bind(user_id)
    .bind(retry_reason)
    .bind(previous_status)
    .bind(previous_failure_reason)
    .bind(previous_error)
    .bind(priority)
    .bind(queue_id)
    .fetch_one(pool)
    .await?;
    
    // Increment retry count
    sqlx::query(
        r#"
        UPDATE documents
        SET ocr_retry_count = COALESCE(ocr_retry_count, 0) + 1,
            updated_at = NOW()
        WHERE id = $1
        "#
    )
    .bind(document_id)
    .execute(pool)
    .await?;
    
    Ok(retry_id)
}

/// Get retry history for a document
pub async fn get_document_retry_history(
    pool: &PgPool,
    document_id: Uuid,
) -> Result<Vec<OcrRetryHistory>> {
    let history = sqlx::query_as::<_, OcrRetryHistory>(
        r#"
        SELECT id, document_id, user_id, retry_reason, previous_status,
               previous_failure_reason, previous_error, priority, queue_id, created_at
        FROM ocr_retry_history
        WHERE document_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    
    Ok(history)
}

/// Get documents eligible for OCR retry based on criteria
pub async fn get_eligible_documents_for_retry(
    pool: &PgPool,
    user_id: Option<Uuid>,
    mime_types: Option<&[String]>,
    failure_reasons: Option<&[String]>,
    max_retry_count: Option<i32>,
    limit: Option<i64>,
) -> Result<Vec<EligibleDocument>> {
    let mut query = sqlx::QueryBuilder::new(
        r#"
        SELECT d.id, d.filename, d.file_size, d.mime_type, 
               d.ocr_failure_reason, d.ocr_retry_count,
               d.created_at, d.updated_at
        FROM documents d
        WHERE d.ocr_status = 'failed'
        "#
    );
    
    // Add user filter
    if let Some(uid) = user_id {
        query.push(" AND d.user_id = ");
        query.push_bind(uid);
    }
    
    // Add MIME type filter
    if let Some(types) = mime_types {
        if !types.is_empty() {
            query.push(" AND d.mime_type = ANY(");
            query.push_bind(types);
            query.push(")");
        }
    }
    
    // Add failure reason filter
    if let Some(reasons) = failure_reasons {
        if !reasons.is_empty() {
            query.push(" AND d.ocr_failure_reason = ANY(");
            query.push_bind(reasons);
            query.push(")");
        }
    }
    
    // Add retry count filter
    if let Some(max_retries) = max_retry_count {
        query.push(" AND COALESCE(d.ocr_retry_count, 0) < ");
        query.push_bind(max_retries);
    }
    
    query.push(" ORDER BY d.created_at DESC");
    
    if let Some(lim) = limit {
        query.push(" LIMIT ");
        query.push_bind(lim);
    }
    
    let documents = query.build_query_as::<EligibleDocument>()
        .fetch_all(pool)
        .await?;
    
    Ok(documents)
}

/// Get OCR retry statistics
pub async fn get_ocr_retry_statistics(
    pool: &PgPool,
    user_id: Option<Uuid>,
) -> Result<OcrRetryStats> {
    let user_filter = if let Some(uid) = user_id {
        format!("AND user_id = '{}'", uid)
    } else {
        String::new()
    };
    
    let stats = sqlx::query(&format!(
        r#"
        SELECT 
            COUNT(DISTINCT document_id) as documents_with_retries,
            COUNT(*) as total_retry_attempts,
            AVG(priority) as avg_priority,
            MAX(created_at) as last_retry_at
        FROM ocr_retry_history
        WHERE 1=1 {}
        "#,
        user_filter
    ))
    .fetch_one(pool)
    .await?;
    
    let retry_counts = sqlx::query(&format!(
        r#"
        SELECT 
            COALESCE(ocr_retry_count, 0) as retry_count,
            COUNT(*) as document_count
        FROM documents
        WHERE ocr_status = 'failed'
          {}
        GROUP BY ocr_retry_count
        ORDER BY retry_count
        "#,
        if user_id.is_some() { "AND user_id = $1" } else { "" }
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    
    let retry_distribution: Vec<(i32, i64)> = retry_counts.into_iter()
        .map(|row| {
            (
                row.get::<i32, _>("retry_count"),
                row.get::<i64, _>("document_count"),
            )
        })
        .collect();
    
    Ok(OcrRetryStats {
        documents_with_retries: stats.get::<i64, _>("documents_with_retries"),
        total_retry_attempts: stats.get::<i64, _>("total_retry_attempts"),
        avg_priority: stats.get::<Option<f64>, _>("avg_priority").unwrap_or(0.0),
        last_retry_at: stats.get::<Option<DateTime<Utc>>, _>("last_retry_at"),
        retry_distribution,
    })
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EligibleDocument {
    pub id: Uuid,
    pub filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub ocr_failure_reason: Option<String>,
    pub ocr_retry_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OcrRetryStats {
    pub documents_with_retries: i64,
    pub total_retry_attempts: i64,
    pub avg_priority: f64,
    pub last_retry_at: Option<DateTime<Utc>>,
    pub retry_distribution: Vec<(i32, i64)>, // (retry_count, document_count)
}