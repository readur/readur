//! Document tools: browse, read OCR text, upload, delete and re-run OCR.
//! All of them go through `services::documents`, the same code the REST
//! document routes use.

use base64ct::{Base64, Encoding};
use http::request::Parts;
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    schemars, tool, tool_router, ErrorData as McpError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    mcp::{format, ReadurMcp},
    models::{DocumentFilters, DocumentResponse, User},
    routes::documents::DocumentError,
    services::documents::{self as service, UploadRequest},
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
    #[schemars(with = "String", extend("format" = "uuid"))]
    pub id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OcrTextParams {
    /// Document ID (UUID).
    #[schemars(with = "String", extend("format" = "uuid"))]
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
    #[schemars(with = "Option<Vec<String>>")]
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
            .map_err(|e| internal("list documents", e))?;

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
                doc.ocr_status.as_deref().unwrap_or("unknown"),
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
            .map_err(|e| internal("get document", e))?
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
            .map_err(|e| internal("get document OCR text", e))?
        else {
            return Ok(not_found(p.id));
        };

        let status = ocr.ocr_status.clone().unwrap_or_else(|| "unknown".into());
        let full = ocr.ocr_text.as_deref().unwrap_or_default();
        let total_chars = full.chars().count();
        let offset = p.offset.unwrap_or(0) as usize;
        if offset > total_chars {
            return Ok(format::tool_error(format!(
                "offset {offset} is past the end of the OCR text ({total_chars} characters)"
            )));
        }
        let max_chars = p
            .max_chars
            .unwrap_or(DEFAULT_OCR_CHARS)
            .clamp(1, MAX_OCR_CHARS) as usize;
        let text: String = full.chars().skip(offset).take(max_chars).collect();
        let end = offset + text.chars().count();
        let next_offset = (end < total_chars).then_some(end);

        let summary = if !ocr.has_ocr_text {
            let mut msg = format!(
                "No OCR text is available yet for \"{}\" ({}). OCR status: {}.",
                ocr.filename, ocr.id, status
            );
            match status.as_str() {
                "failed" => msg.push_str(" Use retry_document_ocr to try again."),
                "processing" => {
                    if let Some(pages) = ocr.pages_processed {
                        msg.push_str(&format!(" {pages} page(s) processed so far."));
                    }
                    msg.push_str(" Check back shortly.");
                }
                _ => msg.push_str(" The document is waiting in the OCR queue."),
            }
            msg
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
            total_chars,
            offset,
            text,
            next_offset,
        };
        format::text_and_json(summary, &page)
    }

    #[tool(
        description = "Upload a new document to Readur. Provide the file as a base64-encoded string; it is stored and queued for OCR automatically. Supports PDFs, images (PNG, JPEG, TIFF, ...), plain text and Office documents. Optionally attach some of your labels. If an identical file already exists, the existing document is returned.",
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
        let filename = p.filename.trim().to_string();
        if filename.is_empty() {
            return Ok(format::tool_error("filename must not be empty"));
        }

        // Reject oversized payloads before decoding them.
        let encoded: String = p
            .content_base64
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect();
        let padding = encoded.bytes().rev().take_while(|&b| b == b'=').count();
        let decoded_len = (encoded.len() / 4 * 3).saturating_sub(padding);
        if let Err(e) = service::check_upload_size(&self.state, &filename, decoded_len) {
            return upload_error(e);
        }
        let data = match Base64::decode_vec(&encoded) {
            Ok(data) => data,
            Err(_) => {
                return Ok(format::tool_error(
                    "content_base64 is not valid base64 (standard alphabet with padding)",
                ))
            }
        };
        if data.is_empty() {
            return Ok(format::tool_error("the file is empty"));
        }

        let mime_type = p
            .mime_type
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
            label_ids: p.label_ids.unwrap_or_default(),
            ..Default::default()
        };
        let uploaded = match service::upload_document(&self.state, user, request).await {
            Ok(uploaded) => uploaded,
            Err(e) => return upload_error(e),
        };

        let text = if uploaded.status == "duplicate" {
            format!(
                "An identical file already exists: \"{}\" ({}). No new document was created.",
                uploaded.filename, uploaded.id
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
        format::text_and_json(text, &uploaded)
    }

    #[tool(
        description = "Permanently delete a document and its stored files by ID. This cannot be undone.",
        annotations(
            title = "Delete Document",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
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
            .map_err(|e| internal("delete document", e))?;
        if !deleted {
            return Ok(not_found(p.id));
        }
        format::text_and_json(
            format!("Document {} was deleted.", p.id),
            &serde_json::json!({ "id": p.id, "deleted": true }),
        )
    }

    #[tool(
        description = "Queue a document for another OCR pass, e.g. after OCR failed or produced poor text. Does nothing if OCR is already running for the document.",
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
            Err(e) => Err(internal("retry OCR", format!("{e:?}"))),
        }
    }
}

fn describe_document(doc: &DocumentResponse) -> String {
    let mut lines = vec![
        format!("Document {}", doc.id),
        format!("  Filename:   {}", doc.original_filename),
        format!("  MIME type:  {}", doc.mime_type),
        format!("  Size:       {}", format::file_size(doc.file_size)),
        format!(
            "  OCR status: {}",
            doc.ocr_status.as_deref().unwrap_or("unknown")
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

/// Upload failures the caller can fix become tool errors; the rest are
/// server faults, already logged by the service with their details.
fn upload_error(error: DocumentError) -> Result<CallToolResult, McpError> {
    match error {
        DocumentError::BadRequest(msg)
        | DocumentError::InvalidQuery(msg)
        | DocumentError::Conflict(msg)
        | DocumentError::PayloadTooLarge(msg) => Ok(format::tool_error(msg)),
        DocumentError::NotFound => Ok(format::tool_error("Document not found")),
        _ => Err(McpError::internal_error("failed to upload document", None)),
    }
}

fn internal(action: &str, error: impl std::fmt::Display) -> McpError {
    tracing::error!("MCP failed to {}: {}", action, error);
    McpError::internal_error(format!("failed to {action}"), None)
}
