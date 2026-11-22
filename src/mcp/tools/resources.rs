//! MCP resources: original document files at `readur://documents/{id}`.

use base64ct::{Base64, Encoding};
use rmcp::{
    model::{
        ListResourceTemplatesResult, ReadResourceRequestParams, ReadResourceResponse,
        ReadResourceResult, ResourceContents, ResourceTemplate,
    },
    ErrorData as McpError,
};
use uuid::Uuid;

use crate::{
    mcp::{format, ReadurMcp},
    models::User,
    storage::LimitedRead,
};

const DOCUMENT_URI_PREFIX: &str = "readur://documents/";

/// Largest file returned inline. Resource contents travel base64-encoded in a
/// single JSON-RPC response, so anything bigger is refused.
pub const MAX_RESOURCE_BYTES: i64 = 10 * 1024 * 1024;

pub fn resource_templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::with_all_items(vec![ResourceTemplate::new(
        format!("{DOCUMENT_URI_PREFIX}{{id}}"),
        "Document file",
    )
    .with_title("Document file")
    .with_description(format!(
        "The original uploaded file of a Readur document, by document ID. Text files are returned \
         as text, everything else base64-encoded with the document's MIME type. Files over {} \
         are refused; use get_document_ocr_text to read their extracted text instead.",
        format::file_size(MAX_RESOURCE_BYTES)
    ))])
}

pub async fn read_resource(
    server: &ReadurMcp,
    user: &User,
    request: ReadResourceRequestParams,
) -> Result<ReadResourceResponse, McpError> {
    let uri = request.uri;
    let document_id = uri
        .strip_prefix(DOCUMENT_URI_PREFIX)
        .ok_or_else(|| McpError::resource_not_found(format!("unknown resource: {uri}"), None))?;
    let document_id = Uuid::parse_str(document_id).map_err(|_| {
        McpError::invalid_params(
            format!("invalid document ID in {uri}; expected readur://documents/<uuid>"),
            None,
        )
    })?;

    // Same access rule as `/api/documents/{id}/download`.
    let document = server
        .state
        .db
        .get_document_by_id(document_id, user.id, user.role)
        .await
        .map_err(|e| format::internal(&format!("load document {document_id}"), e))?
        .ok_or_else(|| McpError::resource_not_found(format!("document not found: {uri}"), None))?;

    let too_large = |size: Option<i64>| {
        let size = match size {
            Some(size) => format!("is {}, over", format::file_size(size)),
            // A bounded read stopped past the limit: the exact size is unknown.
            None => "is over".to_string(),
        };
        McpError::invalid_params(
            format!(
                "{} {size} the {} limit for resource reads. Use get_document_ocr_text to \
                 read its extracted text instead.",
                document.original_filename,
                format::file_size(MAX_RESOURCE_BYTES)
            ),
            None,
        )
    };
    // Cheap early reject from the recorded size.
    if document.file_size > MAX_RESOURCE_BYTES {
        return Err(too_large(Some(document.file_size)));
    }

    // The recorded size describes the uploaded bytes and the stored file can
    // differ, so the read itself is bounded: it stops one byte past the limit.
    let data = match server
        .state
        .file_service
        .read_file_limited(&document.file_path, MAX_RESOURCE_BYTES as u64)
        .await
        .map_err(|e| format::internal(&format!("read file of document {document_id}"), e))?
    {
        LimitedRead::Within(data) => data,
        LimitedRead::TooLarge => return Err(too_large(None)),
    };

    let contents = if document.mime_type.starts_with("text/") {
        match String::from_utf8(data) {
            Ok(text) => ResourceContents::text(text, uri.clone()),
            Err(e) => ResourceContents::blob(Base64::encode_string(e.as_bytes()), uri.clone()),
        }
    } else {
        ResourceContents::blob(Base64::encode_string(&data), uri.clone())
    }
    .with_mime_type(document.mime_type.clone());

    Ok(ReadResourceResult::new(vec![contents]).into())
}
