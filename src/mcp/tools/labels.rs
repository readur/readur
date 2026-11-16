//! Label tools. Labels belong to the user who created them, and only the
//! owner of a document can change its labels, whatever their role, exactly
//! as in the REST API (`routes/labels.rs`). All queries go through
//! `db/labels.rs`.

use http::request::Parts;
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router, ErrorData as McpError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::labels::{is_duplicate_key, LabelAssignOutcome, LabelRemoveOutcome},
    errors::{label::LabelError, AppError},
    mcp::{format, ReadurMcp},
    routes::labels::{CreateLabel, Label, UpdateLabel},
};

const DEFAULT_LABEL_COLOR: &str = "#0969da";

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ListLabelsParams {
    /// Also report how many documents and sources use each label (slower). Defaults to false.
    #[serde(default)]
    pub include_counts: bool,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreateLabelParams {
    /// Name of the label. Must not be blank or contain commas, and must be unique among your labels.
    pub name: String,
    /// Hex colour like "#ff0000". Defaults to "#0969da".
    pub color: Option<String>,
    /// Optional description of what the label is for.
    pub description: Option<String>,
    /// Optional icon name shown next to the label in the UI.
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct UpdateLabelParams {
    /// ID of the label to update (from list_labels).
    #[schemars(with = "String", extend("format" = "uuid"))]
    pub id: Uuid,
    /// New name. Must not be blank or contain commas.
    pub name: Option<String>,
    /// New hex colour like "#ff0000".
    pub color: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New icon name.
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct DocumentLabelsParams {
    /// ID of the document.
    #[schemars(with = "String", extend("format" = "uuid"))]
    pub document_id: Uuid,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct DocumentLabelParams {
    /// ID of the document.
    #[schemars(with = "String", extend("format" = "uuid"))]
    pub document_id: Uuid,
    /// ID of the label (from list_labels).
    #[schemars(with = "String", extend("format" = "uuid"))]
    pub label_id: Uuid,
}

#[derive(Serialize)]
struct LabelList<'a> {
    labels: &'a [Label],
}

#[derive(Serialize)]
struct DocumentLabels<'a> {
    document_id: Uuid,
    labels: &'a [Label],
}

#[derive(Serialize)]
struct LabelChange {
    document_id: Uuid,
    label_id: Uuid,
    status: &'static str,
}

fn internal(context: &str, error: impl std::fmt::Display) -> McpError {
    tracing::error!("MCP {context} failed: {error}");
    McpError::internal_error(format!("{context} failed"), None)
}

/// Maps a failed label insert/update: a duplicate name is the caller's to fix.
fn write_error(
    context: &str,
    name: Option<&str>,
    error: sqlx::Error,
) -> Result<CallToolResult, McpError> {
    if is_duplicate_key(&error) {
        let name = name.unwrap_or_default().to_string();
        return Ok(format::tool_error(
            LabelError::duplicate_name(name).user_message(),
        ));
    }
    Err(internal(context, error))
}

/// `- Name (ID: ...)` plus colour, description and, when known, usage.
fn describe(label: &Label, with_counts: bool) -> String {
    let mut line = format!(
        "- {} (ID: {})\n  Color: {}",
        label.name, label.id, label.color
    );
    if let Some(description) = label.description.as_deref().filter(|d| !d.is_empty()) {
        line.push_str(&format!("\n  Description: {description}"));
    }
    if with_counts {
        line.push_str(&format!(
            "\n  Used by: {} documents, {} sources",
            label.document_count, label.source_count
        ));
    }
    line
}

#[tool_router(router = labels_router, vis = "pub(crate)")]
impl ReadurMcp {
    #[tool(
        description = "List your labels, sorted by name. Returns each label's ID, name, color, and description; set include_counts to also see how many documents and sources use each one. Use the IDs with add_document_label, remove_document_label and update_label.",
        annotations(title = "List Labels", read_only_hint = true, open_world_hint = false)
    )]
    async fn list_labels(
        &self,
        Parameters(p): Parameters<ListLabelsParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let labels = self
            .state
            .db
            .list_labels(user.id, p.include_counts)
            .await
            .map_err(|e| internal("list_labels", e))?;

        let text = if labels.is_empty() {
            "No labels found.".to_string()
        } else {
            let lines: Vec<String> = labels
                .iter()
                .map(|label| describe(label, p.include_counts))
                .collect();
            format!("Labels ({} total)\n\n{}", labels.len(), lines.join("\n"))
        };
        format::text_and_json(text, &LabelList { labels: &labels })
    }

    #[tool(
        description = "Create a new label with a name and optional color, description and icon. Names must be unique among your labels and cannot contain commas; colors are hex like \"#ff0000\". The label can then be assigned to documents with add_document_label.",
        annotations(
            title = "Create Label",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_label(
        &self,
        Parameters(p): Parameters<CreateLabelParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let new_label = CreateLabel {
            name: p.name,
            description: p.description,
            color: p.color.unwrap_or_else(|| DEFAULT_LABEL_COLOR.to_string()),
            background_color: None,
            icon: p.icon,
        };
        if let Err(e) = new_label.validate() {
            return Ok(format::tool_error(e.user_message()));
        }

        let label = match self.state.db.create_label(user.id, &new_label).await {
            Ok(label) => label,
            Err(e) => return write_error("create_label", Some(&new_label.name), e),
        };
        let text = format!("Label created.\n\n{}", describe(&label, false));
        format::text_and_json(text, &label)
    }

    #[tool(
        description = "Update one of your labels. Only the fields you pass change; the rest keep their current values. Use it to rename a label or change its color, description or icon.",
        annotations(
            title = "Update Label",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn update_label(
        &self,
        Parameters(p): Parameters<UpdateLabelParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let changes = UpdateLabel {
            name: p.name,
            description: p.description,
            color: p.color,
            background_color: None,
            icon: p.icon,
        };
        if let Err(e) = changes.validate() {
            return Ok(format::tool_error(e.user_message()));
        }

        let label = match self.state.db.update_label(p.id, user.id, &changes).await {
            Ok(Some(label)) => label,
            Ok(None) => {
                return Ok(format::tool_error(format!("Label {} not found.", p.id)));
            }
            Err(e) => return write_error("update_label", changes.name.as_deref(), e),
        };
        let text = format!("Label updated.\n\n{}", describe(&label, false));
        format::text_and_json(text, &label)
    }

    #[tool(
        description = "Get the labels assigned to one of your documents, with each label's ID, name, color, and description.",
        annotations(
            title = "Get Document Labels",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_document_labels(
        &self,
        Parameters(p): Parameters<DocumentLabelsParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let labels = match self
            .state
            .db
            .get_document_labels_for_user(p.document_id, user.id)
            .await
            .map_err(|e| internal("get_document_labels", e))?
        {
            Some(labels) => labels,
            None => {
                return Ok(format::tool_error(format!(
                    "Document {} not found.",
                    p.document_id
                )))
            }
        };

        let text = if labels.is_empty() {
            format!("No labels assigned to document {}.", p.document_id)
        } else {
            let lines: Vec<String> = labels.iter().map(|l| describe(l, false)).collect();
            format!(
                "Labels for document {} ({} total)\n\n{}",
                p.document_id,
                labels.len(),
                lines.join("\n")
            )
        };
        format::text_and_json(
            text,
            &DocumentLabels {
                document_id: p.document_id,
                labels: &labels,
            },
        )
    }

    #[tool(
        description = "Assign one of your labels to one of your documents. Assigning a label the document already has is a no-op.",
        annotations(
            title = "Add Document Label",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn add_document_label(
        &self,
        Parameters(p): Parameters<DocumentLabelParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let outcome = self
            .state
            .db
            .add_document_label_for_user(p.document_id, p.label_id, user.id)
            .await
            .map_err(|e| internal("add_document_label", e))?;

        let (status, text) = match outcome {
            LabelAssignOutcome::Added => (
                "added",
                format!("Label {} added to document {}.", p.label_id, p.document_id),
            ),
            LabelAssignOutcome::AlreadyAssigned => (
                "already_assigned",
                format!(
                    "Document {} already has label {}.",
                    p.document_id, p.label_id
                ),
            ),
            LabelAssignOutcome::DocumentNotFound => {
                return Ok(format::tool_error(format!(
                    "Document {} not found.",
                    p.document_id
                )))
            }
            LabelAssignOutcome::LabelNotFound => {
                return Ok(format::tool_error(format!(
                    "Label {} not found.",
                    p.label_id
                )))
            }
        };
        format::text_and_json(
            text,
            &LabelChange {
                document_id: p.document_id,
                label_id: p.label_id,
                status,
            },
        )
    }

    #[tool(
        description = "Remove a label from one of your documents. The label itself is kept; only the assignment is removed.",
        annotations(
            title = "Remove Document Label",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn remove_document_label(
        &self,
        Parameters(p): Parameters<DocumentLabelParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let outcome = self
            .state
            .db
            .remove_document_label_for_user(p.document_id, p.label_id, user.id)
            .await
            .map_err(|e| internal("remove_document_label", e))?;

        match outcome {
            LabelRemoveOutcome::Removed => format::text_and_json(
                format!(
                    "Label {} removed from document {}.",
                    p.label_id, p.document_id
                ),
                &LabelChange {
                    document_id: p.document_id,
                    label_id: p.label_id,
                    status: "removed",
                },
            ),
            LabelRemoveOutcome::NotAssigned => Ok(format::tool_error(format!(
                "Document {} does not have label {}.",
                p.document_id, p.label_id
            ))),
            LabelRemoveOutcome::DocumentNotFound => Ok(format::tool_error(format!(
                "Document {} not found.",
                p.document_id
            ))),
        }
    }
}
