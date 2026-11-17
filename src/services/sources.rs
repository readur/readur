//! Source listings as the API reports them.

use uuid::Uuid;

use crate::models::SourceResponse;

/// A user's sources with document counts, credentials redacted. Backs
/// `GET /api/sources` and the MCP `list_sources` tool.
pub async fn source_responses_for_user(
    db: &crate::db::Database,
    user_id: Uuid,
) -> anyhow::Result<Vec<SourceResponse>> {
    let sources = db.get_sources(user_id).await?;

    // Get source IDs for batch counting
    let source_ids: Vec<Uuid> = sources.iter().map(|s| s.id).collect();

    // Get document counts for all sources in one query
    let counts = db.count_documents_for_sources(user_id, &source_ids).await?;

    // Create a map for quick lookup
    let count_map: std::collections::HashMap<Uuid, (i64, i64)> = counts
        .into_iter()
        .map(|(id, total, ocr)| (id, (total, ocr)))
        .collect();

    Ok(sources
        .into_iter()
        .map(|s| {
            let (total_docs, total_ocr) = count_map.get(&s.id).copied().unwrap_or((0, 0));
            let mut response: SourceResponse = s.into();
            response.total_documents = total_docs;
            response.total_documents_ocr = total_ocr;
            response
        })
        .collect())
}
