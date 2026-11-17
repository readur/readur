//! Document tools: browse, read OCR text, upload, delete and re-run OCR.
//! All of them go through `services::documents`, the same code the REST
//! document routes use.

use base64ct::{Base64, Encoding};
use http::request::Parts;
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router, ErrorData as McpError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::document::DocumentError,
    mcp::{format, ReadurMcp},
    models::{DocumentFilters, DocumentOcrResponse, DocumentResponse, User},
    services::documents::{
        self as service, UploadRequest, LABELS_NOT_APPLIED_WARNING, OCR_NOT_QUEUED_WARNING,
    },
};

const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 100;
const DEFAULT_OCR_CHARS: u32 = 20_000;
const MAX_OCR_CHARS: u32 = 100_000;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListDocumentsParams {
    /// Maximum number of documents to return (default 25, max 100).
    pub limit: Option<u32>,
    /// Number of documents to skip, for paging (default 0).
    pub offset: Option<u32>,
    /// Only documents with this exact MIME type, e.g. "application/pdf".
    pub mime_type: Option<String>,
    /// Only documents with this OCR status: pending, processing, completed or failed.
    pub ocr_status: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DocumentIdParams {
    /// Document ID (UUID).
    pub id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OcrTextParams {
    /// Document ID (UUID).
    pub id: Uuid,
    /// Character offset to start reading from (default 0). Use the
    /// `next_offset` of the previous call to continue.
    pub offset: Option<u32>,
    /// Maximum number of characters to return (default 20000, max 100000).
    pub max_chars: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadDocumentParams {
    /// File name including extension, e.g. "invoice.pdf". The extension is
    /// used to guess the MIME type when `mime_type` is omitted.
    pub filename: String,
    /// The file content, base64-encoded (standard alphabet, padded).
    pub content_base64: String,
    /// MIME type of the file, e.g. "application/pdf" or "image/png".
    pub mime_type: Option<String>,
    /// IDs of the caller's labels to attach to the new document.
    pub label_ids: Option<Vec<Uuid>>,
}

/// One page of OCR text, returned by `get_document_ocr_text`.
#[derive(Debug, Serialize)]
struct OcrTextPage {
    id: Uuid,
    filename: String,
    ocr_status: Option<String>,
    has_ocr_text: bool,
    ocr_confidence: Option<f32>,
    pages_processed: Option<i32>,
    /// The text is from an earlier run; a new OCR run is queued or running.
    reprocessing: bool,
    total_chars: usize,
    offset: usize,
    text: String,
    /// Offset to pass for the next page; absent on the last page.
    next_offset: Option<usize>,
}

#[tool_router(router = documents_router, vis = "pub(crate)")]
impl ReadurMcp {
    #[tool(
        description = "List the documents you can access, newest first, with optional MIME type and OCR status filters and pagination. Returns each document's ID, filename, MIME type, OCR status, size and upload date. Use get_document for full metadata or get_document_ocr_text to read the content.",
        annotations(
            title = "List Documents",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_documents(
        &self,
        Parameters(p): Parameters<ListDocumentsParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let limit = p.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as i64;
        let offset = p.offset.unwrap_or(0) as i64;
        let filters = DocumentFilters {
            ocr_status: p.ocr_status,
            mime_types: p.mime_type.map(|m| vec![m]),
            ..Default::default()
        };
        if let Err(msg) = filters.validate() {
            return Ok(format::tool_error(msg));
        }

        let page = service::list_documents(&self.state, user, &filters, None, None, limit, offset)
            .await
            .map_err(|e| format::internal("list documents", e))?;

        let info = &page.pagination;
        let mut text = format!(
            "Documents {}-{} of {}:\n",
            if page.documents.is_empty() {
                offset
            } else {
                offset + 1
            },
            offset + page.documents.len() as i64,
            info.total
        );
        if page.documents.is_empty() {
            text.push_str("No documents found.\n");
        }
        for doc in &page.documents {
            text.push_str(&format!(
                "- [{}] {} ({}, OCR {}, {}, uploaded {})\n",
                doc.id,
                doc.original_filename,
                doc.mime_type,
                format::ocr_status(doc.ocr_status.as_deref()),
                format::file_size(doc.file_size),
                format::date(doc.created_at),
            ));
        }
        if info.has_more {
            text.push_str(&format!(
                "More documents available: call again with offset {}.\n",
                offset + limit
            ));
        }
        format::text_and_json(text, &page)
    }

    #[tool(
        description = "Get full metadata for one document by ID: filenames, MIME type, size, file hash, OCR status/confidence/word count, labels, tags, owner, source and timestamps.",
        annotations(title = "Get Document", read_only_hint = true, open_world_hint = false)
    )]
    async fn get_document(
        &self,
        Parameters(p): Parameters<DocumentIdParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let Some(doc) = service::get_document(&self.state, user, p.id)
            .await
            .map_err(|e| format::internal("get document", e))?
        else {
            return Ok(not_found(p.id));
        };
        format::text_and_json(describe_document(&doc), &doc)
    }

    #[tool(
        description = "Read the OCR-extracted text of a document. This is the primary way to read a document's contents. Long text is returned in pages: pass `offset` (from the previous call's next_offset) and `max_chars` (default 20000) to read further. If OCR has not finished yet, reports the current OCR status instead.",
        annotations(
            title = "Get Document OCR Text",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_document_ocr_text(
        &self,
        Parameters(p): Parameters<OcrTextParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let Some(ocr) = service::get_document_ocr(&self.state, user, p.id)
            .await
            .map_err(|e| format::internal("get document OCR text", e))?
        else {
            return Ok(not_found(p.id));
        };

        let status = format::ocr_status(ocr.ocr_status.as_deref()).to_string();
        let full = ocr.ocr_text.as_deref().unwrap_or_default();
        let total_chars = full.chars().count();
        let offset = p.offset.unwrap_or(0) as usize;
        if ocr.has_ocr_text {
            if let Err(message) = check_ocr_offset(offset, total_chars) {
                return Ok(format::tool_error(message));
            }
        }
        // The document's own status cannot say whether OCR will progress:
        // 'processing' outlives a crashed worker and a retried 'failed'
        // document keeps its status until a worker claims it. The queue can.
        let active_job = self
            .state
            .queue_service
            .has_active_job(ocr.id)
            .await
            .map_err(|e| format::internal("check the OCR queue", e))?;
        // A retry keeps the previous run's text until the new run finishes;
        // say so rather than present stale text as the result.
        let reprocessing = ocr.has_ocr_text && active_job;
        let max_chars = p
            .max_chars
            .unwrap_or(DEFAULT_OCR_CHARS)
            .clamp(1, MAX_OCR_CHARS) as usize;
        let text: String = full.chars().skip(offset).take(max_chars).collect();
        let end = offset + text.chars().count();
        let next_offset = (end < total_chars).then_some(end);

        let summary = if !ocr.has_ocr_text {
            no_text_summary(&ocr, &status, active_job)
        } else if total_chars == 0 {
            format!(
                "OCR finished for \"{}\" ({}) but no text was found.",
                ocr.filename, ocr.id
            )
        } else {
            let mut header = format!(
                "OCR text for \"{}\" ({}), characters {}-{} of {}",
                ocr.filename,
                ocr.id,
                offset + 1,
                end,
                total_chars
            );
            if let Some(confidence) = ocr.ocr_confidence {
                header.push_str(&format!(", confidence {confidence:.1}%"));
            }
            if reprocessing {
                header.push_str(
                    ". This is the text from the previous OCR run; the document is queued for OCR again, so call back later for the new text",
                );
            }
            match next_offset {
                Some(next) => header.push_str(&format!(
                    ". More text remains: call again with offset {next}."
                )),
                None => header.push('.'),
            }
            format!("{header}\n\n{text}")
        };

        let page = OcrTextPage {
            id: ocr.id,
            filename: ocr.filename,
            ocr_status: ocr.ocr_status,
            has_ocr_text: ocr.has_ocr_text,
            ocr_confidence: ocr.ocr_confidence,
            pages_processed: ocr.pages_processed,
            reprocessing,
            total_chars,
            offset,
            text,
            next_offset,
        };
        format::text_and_json(summary, &page)
    }

    #[tool(
        description = "Upload a new document to Readur. Provide the file as a base64-encoded string; it is stored and queued for OCR automatically. Supports PDFs, images (PNG, JPEG, TIFF, ...), plain text and Office documents. Optionally attach some of your labels. If an identical file already exists, that existing document is returned instead (no error) and the requested labels are added to it.",
        annotations(
            title = "Upload Document",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn upload_document(
        &self,
        Parameters(p): Parameters<UploadDocumentParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let UploadDocumentParams {
            filename,
            content_base64,
            mime_type,
            label_ids,
        } = p;
        let filename = filename.trim().to_string();
        if filename.is_empty() {
            return Ok(format::tool_error("filename must not be empty"));
        }

        // Reject oversized payloads before decoding them, and decode in
        // place so the request never holds the file more than once.
        let encoded = strip_base64_whitespace(content_base64);
        if let Err(e) =
            service::check_upload_size(&self.state, &filename, base64_decoded_len(&encoded))
        {
            return upload_error(e);
        }
        let Some(data) = decode_base64_in_place(encoded) else {
            return Ok(format::tool_error(
                "content_base64 is not valid base64 (standard alphabet with padding)",
            ));
        };
        if data.is_empty() {
            return Ok(format::tool_error("the file is empty"));
        }

        let mime_type = mime_type
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| {
                mime_guess::from_path(&filename)
                    .first_or_octet_stream()
                    .essence_str()
                    .to_string()
            });

        let request = UploadRequest {
            filename,
            mime_type,
            data,
            label_ids: label_ids.unwrap_or_default(),
            ..Default::default()
        };
        let uploaded = match service::upload_document_or_existing(&self.state, user, request).await
        {
            Ok(uploaded) => uploaded,
            Err(e) => return upload_error(e),
        };

        let mut text = if uploaded.status == "duplicate" {
            format!(
                "An identical file already exists: \"{}\" ({}). No new document was created; this is the existing document.",
                uploaded.filename, uploaded.id
            )
        } else if uploaded
            .warnings
            .iter()
            .any(|w| w == OCR_NOT_QUEUED_WARNING)
        {
            format!(
                "Uploaded \"{}\" ({}, {}) as document {}, but it could not be queued for OCR. Call retry_document_ocr with this ID to process it.",
                uploaded.filename,
                uploaded.mime_type,
                format::file_size(uploaded.file_size),
                uploaded.id
            )
        } else {
            format!(
                "Uploaded \"{}\" ({}, {}) as document {}. It has been queued for OCR; use get_document_ocr_text to read it once processing completes.",
                uploaded.filename,
                uploaded.mime_type,
                format::file_size(uploaded.file_size),
                uploaded.id
            )
        };
        if uploaded
            .warnings
            .iter()
            .any(|w| w == LABELS_NOT_APPLIED_WARNING)
        {
            text.push_str(
                " The requested labels were not applied; add them with add_document_label.",
            );
        }
        format::text_and_json(text, &uploaded)
    }

    #[tool(
        description = "Permanently delete a document and its stored files by ID. This cannot be undone.",
        annotations(
            title = "Delete Document",
            read_only_hint = false,
            destructive_hint = true,
            // A repeat call reports "not found": it cannot tell an already
            // deleted document from one the caller never had.
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn delete_document(
        &self,
        Parameters(p): Parameters<DocumentIdParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let deleted = service::delete_document(&self.state, user, p.id)
            .await
            .map_err(|e| format::internal("delete document", e))?;
        if !deleted {
            return Ok(not_found(p.id));
        }
        format::text_and_json(
            format!("Document {} was deleted.", p.id),
            &serde_json::json!({ "id": p.id, "deleted": true }),
        )
    }

    #[tool(
        description = "Queue a document for another OCR pass, e.g. after OCR failed or produced poor text. Does nothing if the document is already queued or being processed; a document stuck in 'processing' with no OCR job is queued again.",
        annotations(
            title = "Retry Document OCR",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn retry_document_ocr(
        &self,
        Parameters(p): Parameters<DocumentIdParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user: &User = Self::caller(&parts)?;
        match service::retry_document_ocr(&self.state, user, p.id, None, None).await {
            Ok(outcome) => {
                let text = format!("{} (document {}).", outcome.message, p.id);
                format::text_and_json(text, &outcome)
            }
            Err(DocumentError::NotFound) => Ok(not_found(p.id)),
            Err(DocumentError::BadRequest(msg)) => Ok(format::tool_error(msg)),
            Err(e) => Err(format::internal("retry OCR", format!("{e:?}"))),
        }
    }
}

/// What to tell the model about a document that has no OCR text yet. Only
/// promise progress when a queue job exists; otherwise point it at
/// `retry_document_ocr`, or it would poll a document that never changes.
fn no_text_summary(ocr: &DocumentOcrResponse, status: &str, active_job: bool) -> String {
    let mut msg = format!(
        "No OCR text is available yet for \"{}\" ({}). OCR status: {}.",
        ocr.filename, ocr.id, status
    );
    if active_job {
        if let Some(pages) = ocr.pages_processed {
            msg.push_str(&format!(" {pages} page(s) processed so far."));
        }
        msg.push_str(if status == "processing" {
            " Check back shortly."
        } else {
            " The document is waiting in the OCR queue; check back shortly."
        });
        return msg;
    }
    msg.push_str(match status {
        "failed" => " Use retry_document_ocr to try again.",
        "completed" => {
            " OCR completed but stored no text. Use retry_document_ocr to process it again."
        }
        "processing" | "pending" => {
            " No OCR job is queued or running for it, so it will not progress on its own (OCR may have been interrupted). Use retry_document_ocr to queue it."
        }
        _ => " Use retry_document_ocr to queue it for OCR.",
    });
    msg
}

fn describe_document(doc: &DocumentResponse) -> String {
    let mut lines = vec![
        format!("Document {}", doc.id),
        format!("  Filename:   {}", doc.original_filename),
        format!("  MIME type:  {}", doc.mime_type),
        format!("  Size:       {}", format::file_size(doc.file_size)),
        format!(
            "  OCR status: {}",
            format::ocr_status(doc.ocr_status.as_deref())
        ),
    ];
    if let Some(confidence) = doc.ocr_confidence {
        lines.push(format!("  OCR confidence: {confidence:.1}%"));
    }
    if let Some(words) = doc.ocr_word_count {
        lines.push(format!("  Words:      {words}"));
    }
    if let (Some(current), Some(total)) = (doc.ocr_progress_current, doc.ocr_progress_total) {
        lines.push(format!("  OCR progress: page {current} of {total}"));
    }
    if !doc.labels.is_empty() {
        let names: Vec<&str> = doc.labels.iter().map(|l| l.name.as_str()).collect();
        lines.push(format!("  Labels:     {}", names.join(", ")));
    }
    if !doc.tags.is_empty() {
        lines.push(format!("  Tags:       {}", doc.tags.join(", ")));
    }
    if let Some(owner) = &doc.username {
        lines.push(format!("  Owner:      {owner}"));
    }
    if let Some(source) = &doc.source_type {
        lines.push(format!("  Source:     {source}"));
    }
    if let Some(path) = &doc.source_path {
        lines.push(format!("  Source path: {path}"));
    }
    if let Some(hash) = &doc.file_hash {
        lines.push(format!("  SHA-256:    {hash}"));
    }
    lines.push(format!("  Uploaded:   {}", format::date(doc.created_at)));
    lines.push(format!("  Updated:    {}", format::date(doc.updated_at)));
    lines.join("\n")
}

fn not_found(id: Uuid) -> CallToolResult {
    format::tool_error(format!(
        "Document {id} was not found or you do not have access to it."
    ))
}

/// Base64 is often line-wrapped; drop the whitespace without copying.
fn strip_base64_whitespace(mut encoded: String) -> String {
    encoded.retain(|c| !c.is_ascii_whitespace());
    encoded
}

/// Decoded size of whitespace-free base64, for the size check before
/// decoding.
fn base64_decoded_len(encoded: &str) -> usize {
    let padding = encoded.bytes().rev().take_while(|&b| b == b'=').count();
    (encoded.len() / 4 * 3).saturating_sub(padding)
}

/// Decodes whitespace-free base64 into the buffer that held it, so the
/// encoded text is gone once the bytes exist; `None` if it is not valid.
fn decode_base64_in_place(encoded: String) -> Option<Vec<u8>> {
    let mut buffer = encoded.into_bytes();
    let len = Base64::decode_in_place(&mut buffer).ok()?.len();
    buffer.truncate(len);
    Some(buffer)
}

/// An `offset` into OCR text of `total_chars` characters must point at a
/// character, except that 0 is always allowed (an empty text has one empty
/// page).
fn check_ocr_offset(offset: usize, total_chars: usize) -> Result<(), String> {
    if offset == 0 || offset < total_chars {
        Ok(())
    } else {
        Err(format!(
            "offset {offset} is past the end of the OCR text ({total_chars} characters); the last valid offset is {}",
            total_chars.saturating_sub(1)
        ))
    }
}

/// Upload failures the caller can fix (bad input, too large, a duplicate,
/// a concurrent upload of the same file) become tool errors. Everything
/// else is a server fault: the service already logged the details, and its
/// messages can carry raw database or storage errors, so the model only
/// gets a generic error.
fn upload_error(error: DocumentError) -> Result<CallToolResult, McpError> {
    match error {
        DocumentError::BadRequest(msg)
        | DocumentError::InvalidQuery(msg)
        | DocumentError::Conflict(msg)
        | DocumentError::PayloadTooLarge(msg)
        | DocumentError::ConcurrentUploadError(msg) => Ok(format::tool_error(msg)),
        DocumentError::NotFound => Ok(format::tool_error("Document not found")),
        DocumentError::InternalServerError(_)
        | DocumentError::UploadTimeout(_)
        | DocumentError::DatabaseConstraintViolation(_)
        | DocumentError::OcrProcessingError(_)
        | DocumentError::FileProcessingError(_) => {
            Err(McpError::internal_error("failed to upload document", None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_base64_is_decoded_in_place() {
        let original: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let encoded = Base64::encode_string(&original);
        let wrapped = encoded
            .as_bytes()
            .chunks(76)
            .map(|line| std::str::from_utf8(line).unwrap())
            .collect::<Vec<_>>()
            .join("\r\n");
        let stripped = strip_base64_whitespace(format!("  {wrapped}\n"));
        assert_eq!(stripped, encoded);
        assert_eq!(base64_decoded_len(&stripped), original.len());
        assert_eq!(decode_base64_in_place(stripped).unwrap(), original);
    }

    #[test]
    fn decoded_length_accounts_for_padding() {
        for size in 0..10 {
            let encoded = Base64::encode_string(&vec![7u8; size]);
            assert_eq!(base64_decoded_len(&encoded), size, "{encoded}");
        }
    }

    #[test]
    fn invalid_base64_is_rejected() {
        assert!(decode_base64_in_place("not base64!".to_string()).is_none());
        assert!(
            decode_base64_in_place("abc".to_string()).is_none(),
            "missing padding"
        );
        assert_eq!(
            decode_base64_in_place(String::new()).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn ocr_offset_must_point_inside_the_text() {
        assert!(
            check_ocr_offset(0, 0).is_ok(),
            "empty text has one empty page"
        );
        assert!(check_ocr_offset(0, 100).is_ok());
        assert!(check_ocr_offset(99, 100).is_ok());
        assert!(
            check_ocr_offset(100, 100).is_err(),
            "offset == length is past the end"
        );
        assert!(check_ocr_offset(101, 100).is_err());
        assert!(check_ocr_offset(1, 0).is_err());
    }

    fn is_tool_error(result: Result<CallToolResult, McpError>) -> bool {
        matches!(result, Ok(r) if r.is_error == Some(true))
    }

    #[test]
    fn upload_errors_the_caller_can_fix_become_tool_errors() {
        for error in [
            DocumentError::BadRequest("bad".into()),
            DocumentError::InvalidQuery("bad".into()),
            DocumentError::Conflict("duplicate".into()),
            DocumentError::PayloadTooLarge("big".into()),
            DocumentError::ConcurrentUploadError("busy".into()),
            DocumentError::NotFound,
        ] {
            assert!(is_tool_error(upload_error(error)));
        }
    }

    #[test]
    fn upload_server_faults_never_reach_the_model() {
        // These carry raw database/storage error text (see
        // services::documents::categorize_ingestion_error).
        let secret =
            "duplicate key value violates unique constraint \"documents_pkey\" at /var/lib/readur";
        for error in [
            DocumentError::InternalServerError(secret.into()),
            DocumentError::UploadTimeout(secret.into()),
            DocumentError::DatabaseConstraintViolation(secret.into()),
            DocumentError::OcrProcessingError(secret.into()),
            DocumentError::FileProcessingError(secret.into()),
        ] {
            let err = upload_error(error).expect_err("server faults must be protocol errors");
            assert!(!err.message.contains("constraint"), "{}", err.message);
            assert!(!err.message.contains("/var/lib"), "{}", err.message);
        }
    }
}
