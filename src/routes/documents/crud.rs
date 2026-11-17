use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{Json, Response},
};
use std::sync::Arc;
use tracing::{debug, error, warn};

use super::types::{DocumentUploadResponse, PaginatedDocumentsResponse, PaginationQuery};
use crate::{
    auth::AuthUser, errors::document::DocumentError, models::DocumentResponse,
    services::documents as documents_service, AppState,
};

/// Upload a new document
#[utoipa::path(
    post,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    request_body(content = String, description = "Document file", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Document uploaded successfully", body = DocumentUploadResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<DocumentUploadResponse>, DocumentError> {
    let mut form = UploadForm::default();
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        let error_msg = format!("Failed to get multipart field: {}", e);
        error!("{}", error_msg);
        DocumentError::BadRequest(error_msg)
    })? {
        form.add_field(field).await?;
    }
    let response =
        documents_service::upload_document(&state, &auth_user.user, form.into_request()?).await?;
    Ok(Json(response))
}

/// The multipart fields an upload understands.
#[derive(Debug, PartialEq, Eq)]
enum UploadField {
    File,
    OcrLanguage,
    OcrLanguages,
    LabelIds,
    Unknown,
}

impl UploadField {
    /// `ocr_languages` and `label_ids` may repeat, with or without a `[]`
    /// or `[n]` suffix.
    fn from_name(name: &str) -> Self {
        let base = name.split_once('[').map_or(name, |(base, _)| base);
        match base {
            "file" if base == name => Self::File,
            "ocr_language" if base == name => Self::OcrLanguage,
            "ocr_languages" => Self::OcrLanguages,
            "label_ids" => Self::LabelIds,
            _ => Self::Unknown,
        }
    }
}

/// An upload form as its fields arrive.
#[derive(Default)]
struct UploadForm {
    file: Option<(String, String, Vec<u8>)>,
    ocr_language: Option<String>,
    ocr_languages: Vec<String>,
    label_ids: Vec<uuid::Uuid>,
}

impl UploadForm {
    async fn add_field(
        &mut self,
        field: axum::extract::multipart::Field<'_>,
    ) -> Result<(), DocumentError> {
        match UploadField::from_name(field.name().unwrap_or("")) {
            UploadField::File => self.file = Some(read_file_field(field).await?),
            UploadField::OcrLanguage => {
                if let Some(language) =
                    validated_ocr_language(&read_text_field(field, "language").await?)?
                {
                    self.ocr_language = Some(language);
                }
            }
            UploadField::OcrLanguages => {
                if let Some(language) =
                    validated_ocr_language(&read_text_field(field, "language").await?)?
                {
                    self.ocr_languages.push(language);
                }
            }
            UploadField::LabelIds => {
                self.label_ids.extend(parse_label_ids(
                    &read_text_field(field, "label_ids").await?,
                )?);
            }
            UploadField::Unknown => {}
        }
        Ok(())
    }

    fn into_request(self) -> Result<documents_service::UploadRequest, DocumentError> {
        let (filename, mime_type, data) = self.file.ok_or_else(|| {
            let error_msg = "No file found in upload".to_string();
            error!("{}", error_msg);
            DocumentError::BadRequest(error_msg)
        })?;
        Ok(documents_service::UploadRequest {
            filename,
            mime_type,
            data,
            ocr_languages: self.ocr_languages,
            ocr_language: self.ocr_language,
            label_ids: self.label_ids,
        })
    }
}

async fn read_text_field(
    field: axum::extract::multipart::Field<'_>,
    what: &str,
) -> Result<String, DocumentError> {
    field
        .text()
        .await
        .map_err(|_| DocumentError::BadRequest(format!("Failed to read {} field", what)))
}

/// The uploaded file: (filename, content type, bytes).
async fn read_file_field(
    field: axum::extract::multipart::Field<'_>,
) -> Result<(String, String, Vec<u8>), DocumentError> {
    let filename = field
        .file_name()
        .ok_or_else(|| {
            let error_msg = "No filename provided in upload".to_string();
            error!("{}", error_msg);
            DocumentError::BadRequest(error_msg)
        })?
        .to_string();
    let content_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();
    let data = field.bytes().await.map_err(|e| {
        let error_msg = format!("Failed to read file data: {}", e);
        error!("{}", error_msg);
        DocumentError::BadRequest(error_msg)
    })?;
    Ok((filename, content_type, data.to_vec()))
}

/// A trimmed OCR language that tesseract has data for; `None` for a blank
/// value.
fn validated_ocr_language(raw: &str) -> Result<Option<String>, DocumentError> {
    let language = raw.trim();
    if language.is_empty() {
        return Ok(None);
    }
    let health_checker = crate::ocr::health::OcrHealthChecker::new();
    match health_checker.validate_language(language) {
        Ok(_) => {
            debug!("OCR language validated: {}", language);
            Ok(Some(language.to_string()))
        }
        Err(e) => {
            let available_languages = health_checker.get_available_languages().unwrap_or_default();
            let error_msg = format!(
                "Invalid OCR language '{}': {}. Available languages: {}",
                language,
                e,
                available_languages.join(", ")
            );
            warn!("{}", error_msg);
            Err(DocumentError::BadRequest(error_msg))
        }
    }
}

/// Label UUIDs from one field value: a single id or a comma-separated list.
fn parse_label_ids(value: &str) -> Result<Vec<uuid::Uuid>, DocumentError> {
    value
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(|id| {
            id.parse()
                .map_err(|_| DocumentError::BadRequest(format!("Invalid label id '{}'", id)))
        })
        .collect()
}

/// Get a specific document by ID
#[utoipa::path(
    get,
    path = "/api/documents/{id}",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document details", body = DocumentResponse),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_document_by_id(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<DocumentResponse>, StatusCode> {
    documents_service::get_document(&state, &auth_user.user, document_id)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// List documents with pagination and filtering
#[utoipa::path(
    get,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Paginated list of documents", body = PaginatedDocumentsResponse),
        (status = 400, description = "Invalid sort or filter parameter"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<PaginatedDocumentsResponse>, DocumentError> {
    let filters = query.filters();
    filters.validate().map_err(DocumentError::InvalidQuery)?;

    let response = documents_service::list_documents(
        &state,
        &auth_user.user,
        &filters,
        query.sort_by,
        query.sort_order,
        query.limit.unwrap_or(25),
        query.offset.unwrap_or(0),
    )
    .await
    .map_err(|e| {
        error!("Database error listing documents: {}", e);
        DocumentError::InternalServerError("Failed to list documents".to_string())
    })?;
    Ok(Json(response))
}

/// Delete a specific document
#[utoipa::path(
    delete,
    path = "/api/documents/{id}",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 204, description = "Document deleted successfully"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<StatusCode, StatusCode> {
    let deleted = documents_service::delete_document(&state, &auth_user.user, document_id)
        .await
        .map_err(|e| {
            error!("Database error deleting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Download a document file
#[utoipa::path(
    get,
    path = "/api/documents/{id}/download",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document file", content_type = "application/octet-stream"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn download_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Response<Body>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_service = &state.file_service;
    let file_data = file_service
        .read_file(&document.file_path)
        .await
        .map_err(|e| {
            error!("Failed to read document file {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = crate::http_security::user_content_response(
        &document.mime_type,
        &document.original_filename,
        false,
        file_data,
    )
    .map_err(|e| {
        error!("Failed to build response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    debug!("Document downloaded: {}", document_id);
    Ok(response)
}

/// View a document in the browser
#[utoipa::path(
    get,
    path = "/api/documents/{id}/view",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document file for viewing", content_type = "application/octet-stream"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn view_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Response<Body>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_service = &state.file_service;
    let file_data = file_service
        .read_file(&document.file_path)
        .await
        .map_err(|e| {
            error!("Failed to read document file {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = crate::http_security::user_content_response(
        &document.mime_type,
        &document.original_filename,
        true,
        file_data,
    )
    .map_err(|e| {
        error!("Failed to build response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    debug!("Document viewed: {}", document_id);
    Ok(response)
}

/// Get user's duplicate documents
#[utoipa::path(
    get,
    path = "/api/documents/duplicates",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of duplicate groups to return per page"),
        ("offset" = Option<i64>, Query, description = "Number of duplicate groups to skip")
    ),
    responses(
        (status = 200, description = "User's duplicate documents grouped by hash", body = serde_json::Value),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_user_duplicates(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);

    let duplicates = state
        .db
        .get_user_duplicates(auth_user.user.id, auth_user.user.role, limit, offset)
        .await
        .map_err(|e| {
            error!("Failed to get user duplicates: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_count = duplicates.len() as i64;

    let response = serde_json::json!({
        "duplicates": duplicates,
        "pagination": {
            "total": total_count,
            "limit": limit,
            "offset": offset,
            "has_more": offset + limit < total_count
        },
        "statistics": {
            "total_duplicate_groups": total_count
        }
    });

    Ok(Json(response))
}

#[cfg(test)]
mod upload_form_tests {
    use super::*;

    #[test]
    fn field_names_are_classified() {
        assert_eq!(UploadField::from_name("file"), UploadField::File);
        assert_eq!(
            UploadField::from_name("ocr_language"),
            UploadField::OcrLanguage
        );
        for name in ["ocr_languages", "ocr_languages[]", "ocr_languages[2]"] {
            assert_eq!(
                UploadField::from_name(name),
                UploadField::OcrLanguages,
                "{name}"
            );
        }
        for name in ["label_ids", "label_ids[]", "label_ids[0]"] {
            assert_eq!(
                UploadField::from_name(name),
                UploadField::LabelIds,
                "{name}"
            );
        }
        for name in [
            "",
            "files",
            "file[0]",
            "ocr_language[]",
            "ocr_languagesx",
            "label_idsx",
            "other",
        ] {
            assert_eq!(UploadField::from_name(name), UploadField::Unknown, "{name}");
        }
    }

    #[test]
    fn label_ids_accept_one_id_or_a_comma_separated_list() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        assert_eq!(parse_label_ids(&a.to_string()).unwrap(), vec![a]);
        assert_eq!(
            parse_label_ids(&format!(" {a} , {b} ,")).unwrap(),
            vec![a, b]
        );
        assert!(parse_label_ids("").unwrap().is_empty());
        assert!(matches!(
            parse_label_ids("not-a-uuid"),
            Err(DocumentError::BadRequest(_))
        ));
    }

    #[test]
    fn blank_ocr_language_is_ignored() {
        assert!(validated_ocr_language("   ").unwrap().is_none());
    }

    #[test]
    fn upload_without_a_file_is_a_bad_request() {
        let form = UploadForm {
            ocr_language: Some("eng".into()),
            ..Default::default()
        };
        assert!(matches!(
            form.into_request(),
            Err(DocumentError::BadRequest(_))
        ));
    }

    #[test]
    fn upload_form_carries_every_field_into_the_request() {
        let label = uuid::Uuid::new_v4();
        let form = UploadForm {
            file: Some(("a.pdf".into(), "application/pdf".into(), vec![1, 2, 3])),
            ocr_language: Some("eng".into()),
            ocr_languages: vec!["eng".into(), "deu".into()],
            label_ids: vec![label],
        };
        let request = form.into_request().unwrap();
        assert_eq!(request.filename, "a.pdf");
        assert_eq!(request.mime_type, "application/pdf");
        assert_eq!(request.data, vec![1, 2, 3]);
        assert_eq!(request.ocr_language.as_deref(), Some("eng"));
        assert_eq!(
            request.ocr_languages,
            vec!["eng".to_string(), "deu".to_string()]
        );
        assert_eq!(request.label_ids, vec![label]);
    }
}
