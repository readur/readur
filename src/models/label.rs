//! Label types shared by the REST label routes, the MCP label tools and the label queries in
//! `db::labels`, plus the input validation every label write goes through.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::errors::label::LabelError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema, TS)]
#[ts(export)]
pub struct Label {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub background_color: Option<String>,
    pub icon: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub document_count: i64,
    #[serde(default)]
    pub source_count: i64,
}

#[derive(Debug, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct CreateLabel {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_color")]
    #[ts(optional = nullable)]
    pub color: String,
    pub background_color: Option<String>,
    pub icon: Option<String>,
}

/// Colour a label gets when the caller does not pick one.
pub const DEFAULT_LABEL_COLOR: &str = "#0969da";

fn default_color() -> String {
    DEFAULT_LABEL_COLOR.to_string()
}

#[derive(Debug, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct UpdateLabel {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub background_color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct LabelAssignment {
    pub label_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema, TS, IntoParams)]
#[ts(export, optional_fields)]
pub struct LabelQuery {
    #[serde(default)]
    #[ts(optional = nullable)]
    pub include_counts: bool,
}

#[derive(Debug, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct BulkUpdateRequest {
    pub document_ids: Vec<Uuid>,
    pub label_ids: Vec<Uuid>,
    #[serde(default = "default_bulk_mode")]
    #[ts(optional = nullable)]
    pub mode: String, // "replace", "add", or "remove"
}

fn default_bulk_mode() -> String {
    "replace".to_string()
}

/// Rejects names that are blank or contain commas (commas break the comma-separated label
/// filters in search).
pub fn validate_label_name(name: &str) -> Result<(), LabelError> {
    if name.trim().is_empty() {
        return Err(LabelError::invalid_name(
            name.to_string(),
            "Name cannot be empty".to_string(),
        ));
    }
    // URL-encoded values (%2c) are decoded by serde before reaching here
    if name.contains(',') {
        return Err(LabelError::invalid_name(
            name.to_string(),
            "Name cannot contain commas".to_string(),
        ));
    }
    Ok(())
}

/// Colours are `#rrggbb`.
pub fn validate_label_color(color: &str) -> Result<(), LabelError> {
    if !color.starts_with('#') || color.len() != 7 {
        return Err(LabelError::invalid_color(color));
    }
    Ok(())
}

impl CreateLabel {
    pub fn validate(&self) -> Result<(), LabelError> {
        validate_label_name(&self.name)?;
        validate_label_color(&self.color)?;
        if let Some(bg_color) = &self.background_color {
            validate_label_color(bg_color)?;
        }
        Ok(())
    }
}

impl UpdateLabel {
    pub fn validate(&self) -> Result<(), LabelError> {
        if let Some(name) = &self.name {
            validate_label_name(name)?;
        }
        if let Some(color) = &self.color {
            validate_label_color(color)?;
        }
        if let Some(bg_color) = &self.background_color {
            validate_label_color(bg_color)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create(name: &str, color: &str) -> CreateLabel {
        CreateLabel {
            name: name.to_string(),
            description: None,
            color: color.to_string(),
            background_color: None,
            icon: None,
        }
    }

    #[test]
    fn create_label_rejects_blank_and_comma_names() {
        assert!(create("Invoices", "#0969da").validate().is_ok());
        assert!(create("  ", "#0969da").validate().is_err());
        assert!(create("a,b", "#0969da").validate().is_err());
    }

    #[test]
    fn colors_must_be_hex_triplets() {
        assert!(create("Invoices", "0969da").validate().is_err());
        assert!(create("Invoices", "#0969d").validate().is_err());
        let update = UpdateLabel {
            name: None,
            description: None,
            color: None,
            background_color: Some("blue".to_string()),
            icon: None,
        };
        assert!(update.validate().is_err());
    }
}
