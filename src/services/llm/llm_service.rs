use std::sync::Arc;
use uuid::Uuid;
use sqlx::PgPool;
use serde::{Deserialize, Serialize};
use crate::models::document::Document;
use sqlx::Row;

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub label: String,
    pub name: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String, // name of source node
    pub target: String, // name of target node
    pub relationship: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone)]
pub struct LLMService {
    pool: PgPool,
    // client: reqwest::Client, // For calling LLM API
    // api_key: String,
}

impl LLMService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
        }
    }

    pub async fn analyze_document(&self, document_id: Uuid) -> Result<GraphData, String> {
        // 1. Fetch document content
        let doc: Document = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE id = $1"
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Document not found")?;

        let content = doc.ocr_text.or(doc.content).ok_or("No content to analyze")?;

        // 2. Call LLM (Stub for now)
        // TODO: Implement actual LLM call here using reqwest or similar client.
        // It should call an endpoint like OpenAI /v1/chat/completions or Ollama.
        // The prompt should instruct the LLM to extract entities and relationships in the GraphData JSON format.
        let graph_data = self.mock_llm_response(&content);

        // 3. Store graph data
        self.store_graph_data(document_id, &graph_data).await?;

        Ok(graph_data)
    }

    fn mock_llm_response(&self, _content: &str) -> GraphData {
        // Mock response
        GraphData {
            nodes: vec![
                GraphNode {
                    label: "Person".to_string(),
                    name: "John Doe".to_string(),
                    properties: serde_json::json!({"role": "Engineer"}),
                },
                GraphNode {
                    label: "Company".to_string(),
                    name: "Readur Corp".to_string(),
                    properties: serde_json::json!({"industry": "Tech"}),
                },
            ],
            edges: vec![
                GraphEdge {
                    source: "John Doe".to_string(),
                    target: "Readur Corp".to_string(),
                    relationship: "WORKS_FOR".to_string(),
                    properties: serde_json::json!({}),
                },
            ],
        }
    }

    async fn store_graph_data(&self, document_id: Uuid, data: &GraphData) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        // Clear existing graph data for this document
        sqlx::query("DELETE FROM document_nodes WHERE document_id = $1")
            .bind(document_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        // Insert nodes
        let mut node_map = std::collections::HashMap::new();

        for node in &data.nodes {
            let row = sqlx::query(
                "INSERT INTO document_nodes (document_id, label, name, properties) VALUES ($1, $2, $3, $4) RETURNING id"
            )
            .bind(document_id)
            .bind(&node.label)
            .bind(&node.name)
            .bind(&node.properties)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

            let id: Uuid = row.get("id");
            // Use label + name as key to reduce collision chance for same-named nodes of different types.
            // In a real LLM scenario, the LLM should ideally provide a unique ID or we handle duplicates better.
            let key = format!("{}:{}", node.label, node.name);
            node_map.insert(key.clone(), id);
            // Also insert just name as fallback if label is not strictly used in edge definition by the mock/LLM
            node_map.entry(node.name.clone()).or_insert(id);
        }

        // Insert edges
        for edge in &data.edges {
            // Try to find by name first (as per current struct), but could be improved to use label too if available in Edge struct
            let source_id = node_map.get(&edge.source).ok_or(format!("Source node {} not found", edge.source))?;
            let target_id = node_map.get(&edge.target).ok_or(format!("Target node {} not found", edge.target))?;

            sqlx::query(
                "INSERT INTO document_edges (document_id, source_node_id, target_node_id, relationship, properties) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(document_id)
            .bind(source_id)
            .bind(target_id)
            .bind(&edge.relationship)
            .bind(&edge.properties)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
