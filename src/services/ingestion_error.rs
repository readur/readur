//! Classifies a failed document ingestion by the *type* of what failed, never
//! by the wording of its message: the error chain is walked and each cause is
//! downcast to the error types the ingestion pipeline produces (database,
//! timeout, storage I/O, OCR).
//!
//! Ingestion returns `anyhow::Error`, whose `chain()` yields the original
//! typed errors. (Converting it into a `Box<dyn Error>` would hide them
//! behind anyhow's private wrapper, so keep it an `anyhow::Error`.)

use std::error::Error as StdError;

use tracing::error;

use crate::errors::document::DocumentError;

/// What kind of failure stopped an ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestionFailureKind {
    /// The database rejected the row (unique, foreign key, not-null, check or
    /// exclusion constraint).
    Constraint,
    /// Something took too long: a `tokio::time::timeout`, a connection-pool
    /// wait, or an I/O operation that timed out.
    Timeout,
    /// OCR rejected the document.
    Ocr,
    /// The file's bytes could not be read or decoded.
    File,
    /// Anything else, including other database failures.
    Internal,
}

impl IngestionFailureKind {
    /// Classifies `error` by the first cause in its chain whose type is
    /// recognised. An unrecognised chain is `Internal`.
    pub fn of(error: &anyhow::Error) -> Self {
        error
            .chain()
            .find_map(Self::of_cause)
            .unwrap_or(Self::Internal)
    }

    /// The kind of one cause, or `None` to keep walking the chain.
    fn of_cause(cause: &(dyn StdError + 'static)) -> Option<Self> {
        if let Some(db) = cause.downcast_ref::<sqlx::Error>() {
            // Every other database failure (connection, protocol, decoding)
            // is the server's problem, even when its source is an I/O error.
            return Some(match db {
                sqlx::Error::Database(e) if !matches!(e.kind(), sqlx::error::ErrorKind::Other) => {
                    Self::Constraint
                }
                sqlx::Error::PoolTimedOut => Self::Timeout,
                _ => Self::Internal,
            });
        }
        if cause.is::<tokio::time::error::Elapsed>() {
            return Some(Self::Timeout);
        }
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            use std::io::ErrorKind;
            return Some(match io.kind() {
                ErrorKind::TimedOut => Self::Timeout,
                // The bytes themselves are unusable.
                ErrorKind::InvalidData | ErrorKind::InvalidInput | ErrorKind::UnexpectedEof => {
                    Self::File
                }
                // Storage failures (disk full, missing or unwritable
                // directories, permissions) are the server's problem.
                _ => Self::Internal,
            });
        }
        if cause.is::<crate::ocr::error::OcrError>() {
            return Some(Self::Ocr);
        }
        #[cfg(feature = "ocr")]
        if cause.is::<image::ImageError>() {
            return Some(Self::File);
        }
        None
    }

    /// The client-facing error for this kind. The raw error can name tables,
    /// constraints and storage paths, so the message is fixed per kind.
    pub fn document_error(self) -> DocumentError {
        match self {
            Self::Constraint => DocumentError::DatabaseConstraintViolation(
                "The document conflicts with existing data and was not stored",
            ),
            Self::Timeout => DocumentError::UploadTimeout("Upload processing timed out"),
            Self::Ocr => DocumentError::OcrProcessingError("The document could not be processed"),
            Self::File => DocumentError::FileProcessingError("The file could not be processed"),
            Self::Internal => DocumentError::InternalServerError("Failed to ingest document"),
        }
    }
}

/// Maps an ingestion failure onto the closest client-facing error, logging
/// the raw error (which never reaches the client).
pub fn categorize_ingestion_error(raw: &anyhow::Error) -> DocumentError {
    let error = IngestionFailureKind::of(raw).document_error();
    error!(
        "Upload ingestion failed ({}): {:#}",
        error.status_and_code().1,
        raw
    );
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use std::borrow::Cow;

    /// A database error as Postgres reports it, with the given kind
    /// (`ErrorKind` is not `Copy`, hence the constructor).
    #[derive(Debug)]
    struct PgError {
        kind: fn() -> sqlx::error::ErrorKind,
        message: &'static str,
    }

    impl std::fmt::Display for PgError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.message)
        }
    }

    impl StdError for PgError {}

    impl sqlx::error::DatabaseError for PgError {
        fn message(&self) -> &str {
            self.message
        }
        fn code(&self) -> Option<Cow<'_, str>> {
            None
        }
        fn as_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn StdError + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn StdError + Send + Sync + 'static> {
            self
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            (self.kind)()
        }
    }

    fn database_error(kind: fn() -> sqlx::error::ErrorKind) -> sqlx::Error {
        sqlx::Error::Database(Box::new(PgError {
            kind,
            message:
                "duplicate key value violates unique constraint \"idx_documents_user_file_hash\"",
        }))
    }

    async fn elapsed() -> tokio::time::error::Elapsed {
        tokio::time::timeout(
            std::time::Duration::from_millis(1),
            std::future::pending::<()>(),
        )
        .await
        .unwrap_err()
    }

    async fn cases() -> Vec<(&'static str, anyhow::Error, IngestionFailureKind)> {
        use sqlx::error::ErrorKind;
        use IngestionFailureKind::*;
        vec![
            (
                "unique violation",
                database_error(|| ErrorKind::UniqueViolation).into(),
                Constraint,
            ),
            (
                "foreign key violation",
                database_error(|| ErrorKind::ForeignKeyViolation).into(),
                Constraint,
            ),
            (
                "not-null violation",
                database_error(|| ErrorKind::NotNullViolation).into(),
                Constraint,
            ),
            (
                "check violation",
                database_error(|| ErrorKind::CheckViolation).into(),
                Constraint,
            ),
            (
                "constraint behind context",
                anyhow::Error::from(database_error(|| ErrorKind::UniqueViolation))
                    .context("Failed to create document record"),
                Constraint,
            ),
            (
                "other database error",
                database_error(|| ErrorKind::Other).into(),
                Internal,
            ),
            ("pool timeout", sqlx::Error::PoolTimedOut.into(), Timeout),
            (
                "database connection I/O",
                sqlx::Error::Io(std::io::Error::from(std::io::ErrorKind::ConnectionRefused)).into(),
                Internal,
            ),
            (
                "retried query timeout",
                anyhow::Error::new(elapsed().await)
                    .context("Database operation timed out after 3 retries"),
                Timeout,
            ),
            (
                "I/O timeout",
                std::io::Error::from(std::io::ErrorKind::TimedOut).into(),
                Timeout,
            ),
            (
                "storage write",
                anyhow::Error::new(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    "No space left on device",
                ))
                .context("failed to write /var/lib/readur/uploads/7f3a/secret.pdf"),
                Internal,
            ),
            (
                "undecodable bytes",
                std::io::Error::new(std::io::ErrorKind::InvalidData, "truncated stream").into(),
                File,
            ),
            (
                "OCR rejection",
                crate::ocr::error::OcrError::InvalidImageFormat {
                    details: "truncated".into(),
                }
                .into(),
                Ocr,
            ),
            // Message text is ignored: these words used to pick a category.
            (
                "plain message",
                anyhow::anyhow!("constraint timeout ocr processing file read duplicate"),
                Internal,
            ),
        ]
    }

    #[tokio::test]
    async fn classifies_by_error_type() {
        for (name, error, expected) in cases().await {
            assert_eq!(IngestionFailureKind::of(&error), expected, "{name}");
        }
    }

    async fn body_of(error: DocumentError) -> (u16, serde_json::Value) {
        let response = error.into_response();
        let status = response.status().as_u16();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    /// Each kind keeps its status and code; the message never carries the
    /// raw error's detail.
    #[tokio::test]
    async fn ingestion_errors_keep_their_category_but_not_their_detail() {
        let expected = [
            (
                IngestionFailureKind::Constraint,
                409,
                "UPLOAD_DB_CONSTRAINT",
            ),
            (IngestionFailureKind::Timeout, 408, "UPLOAD_TIMEOUT"),
            (IngestionFailureKind::Ocr, 422, "UPLOAD_OCR_ERROR"),
            (
                IngestionFailureKind::File,
                422,
                "UPLOAD_FILE_PROCESSING_ERROR",
            ),
            (IngestionFailureKind::Internal, 500, "UPLOAD_INTERNAL_ERROR"),
        ];
        for (name, error, kind) in cases().await {
            let raw = format!("{error:#}");
            let (status, body) = body_of(categorize_ingestion_error(&error)).await;
            let (_, want_status, want_code) = expected.iter().find(|(k, ..)| *k == kind).unwrap();
            assert_eq!(status, *want_status, "{name}");
            assert_eq!(body["status"], *want_status, "{name}");
            assert_eq!(body["code"], *want_code, "{name}");
            let shown = body["error"].as_str().unwrap().to_lowercase();
            assert!(!shown.is_empty(), "{name}");
            for leak in [
                "constraint",
                "idx_",
                "/var/lib",
                "no space left",
                "truncated",
            ] {
                assert!(
                    !shown.contains(leak),
                    "{leak:?} leaked from {raw:?}: {body}"
                );
            }
        }
    }
}
