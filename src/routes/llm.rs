use axum::{
    extract::{Path, State},
    Json, Router, routing::{get, post},
    http::StatusCode,
};
use serde_json::json;
use uuid::Uuid;
use crate::services::llm::llm_service::LLMService;
use crate::AppState;
use std::sync::Arc;
use sqlx::Row;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/analyze", post(analyze_document))
        .route("/:id/graph", get(get_document_graph))
}

async fn analyze_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let service = LLMService::new(state.db.get_pool().clone());
    let result = service.analyze_document(id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!(result)))
}

async fn get_document_graph(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = state.db.get_pool();

    let nodes_rows = sqlx::query(
        "SELECT id, label, name, properties FROM document_nodes WHERE document_id = $1"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let nodes: Vec<serde_json::Value> = nodes_rows.iter().map(|row| {
        json!({
            "id": row.get::<Uuid, _>("id"),
            "label": row.get::<String, _>("label"),
            "name": row.get::<String, _>("name"),
            "properties": row.get::<serde_json::Value, _>("properties")
        })
    }).collect();

    let edges_rows = sqlx::query(
        "SELECT id, source_node_id, target_node_id, relationship, properties FROM document_edges WHERE document_id = $1"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let edges: Vec<serde_json::Value> = edges_rows.iter().map(|row| {
        json!({
            "id": row.get::<Uuid, _>("id"),
            "source": row.get::<Uuid, _>("source_node_id"),
            "target": row.get::<Uuid, _>("target_node_id"),
            "relationship": row.get::<String, _>("relationship"),
            "properties": row.get::<serde_json::Value, _>("properties")
        })
    }).collect();

    Ok(Json(json!({
        "nodes": nodes,
        "edges": edges
    })))
}
