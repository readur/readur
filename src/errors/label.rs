use axum::http::StatusCode;
use thiserror::Error;
use uuid::Uuid;

use super::{AppError, ErrorCategory, ErrorSeverity, impl_into_response};

/// Errors related to label management operations
#[derive(Error, Debug)]
pub enum LabelError {
    #[error("Label not found")]
    NotFound,
    
    #[error("Label with ID {id} not found")]
    NotFoundById { id: Uuid },
    
    #[error("Label with name '{name}' already exists")]
    DuplicateName { name: String },
    
    #[error("Cannot modify system label '{name}'")]
    SystemLabelModification { name: String },
    
    #[error("Invalid color format '{color}'. Use hex format like #0969da")]
    InvalidColor { color: String },
    
    #[error("Label name '{name}' is invalid: {reason}")]
    InvalidName { name: String, reason: String },
    
    #[error("Label is in use by {document_count} documents and cannot be deleted")]
    LabelInUse { document_count: i64 },
    
    #[error("Icon '{icon}' is not supported. Supported icons: {supported_icons}")]
    InvalidIcon { icon: String, supported_icons: String },
    
    #[error("Maximum number of labels ({max_labels}) reached")]
    MaxLabelsReached { max_labels: i32 },
    
    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },
    
    #[error("Background color '{color}' conflicts with text color '{text_color}'")]
    ColorConflict { color: String, text_color: String },
    
    #[error("Label description too long: {length} characters (max: {max_length})")]
    DescriptionTooLong { length: usize, max_length: usize },
    
    #[error("Cannot delete label: {reason}")]
    DeleteRestricted { reason: String },
    
    #[error("Invalid label assignment to document {document_id}: {reason}")]
    InvalidAssignment { document_id: Uuid, reason: String },
    
    #[error("Label '{name}' is reserved and cannot be created")]
    ReservedName { name: String },
}

impl AppError for LabelError {
    fn status_code(&self) -> StatusCode {
        match self {
            LabelError::NotFound | LabelError::NotFoundById { .. } => StatusCode::NOT_FOUND,
            LabelError::DuplicateName { .. } => StatusCode::CONFLICT,
            LabelError::SystemLabelModification { .. } => StatusCode::FORBIDDEN,
            LabelError::InvalidColor { .. } => StatusCode::BAD_REQUEST,
            LabelError::InvalidName { .. } => StatusCode::BAD_REQUEST,
            LabelError::LabelInUse { .. } => StatusCode::CONFLICT,
            LabelError::InvalidIcon { .. } => StatusCode::BAD_REQUEST,
            LabelError::MaxLabelsReached { .. } => StatusCode::CONFLICT,
            LabelError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            LabelError::ColorConflict { .. } => StatusCode::BAD_REQUEST,
            LabelError::DescriptionTooLong { .. } => StatusCode::BAD_REQUEST,
            LabelError::DeleteRestricted { .. } => StatusCode::CONFLICT,
            LabelError::InvalidAssignment { .. } => StatusCode::BAD_REQUEST,
            LabelError::ReservedName { .. } => StatusCode::CONFLICT,
        }
    }
    
    fn user_message(&self) -> String {
        match self {
            LabelError::NotFound | LabelError::NotFoundById { .. } => "Label not found".to_string(),
            LabelError::DuplicateName { .. } => "A label with this name already exists".to_string(),
            LabelError::SystemLabelModification { .. } => "System labels cannot be modified".to_string(),
            LabelError::InvalidColor { .. } => "Invalid color format - use hex format like #0969da".to_string(),
            LabelError::InvalidName { reason, .. } => format!("Invalid label name: {}", reason),
            LabelError::LabelInUse { document_count } => format!("Label is in use by {} documents and cannot be deleted", document_count),
            LabelError::InvalidIcon { .. } => "Invalid icon specified".to_string(),
            LabelError::MaxLabelsReached { max_labels } => format!("Maximum number of labels ({}) reached", max_labels),
            LabelError::PermissionDenied { reason } => format!("Permission denied: {}", reason),
            LabelError::ColorConflict { .. } => "Color combination provides poor contrast".to_string(),
            LabelError::DescriptionTooLong { max_length, .. } => format!("Description too long (max {} characters)", max_length),
            LabelError::DeleteRestricted { reason } => format!("Cannot delete label: {}", reason),
            LabelError::InvalidAssignment { reason, .. } => format!("Invalid label assignment: {}", reason),
            LabelError::ReservedName { .. } => "Label name is reserved and cannot be used".to_string(),
        }
    }
    
    fn error_code(&self) -> &'static str {
        match self {
            LabelError::NotFound => "LABEL_NOT_FOUND",
            LabelError::NotFoundById { .. } => "LABEL_NOT_FOUND_BY_ID",
            LabelError::DuplicateName { .. } => "LABEL_DUPLICATE_NAME",
            LabelError::SystemLabelModification { .. } => "LABEL_SYSTEM_MODIFICATION",
            LabelError::InvalidColor { .. } => "LABEL_INVALID_COLOR",
            LabelError::InvalidName { .. } => "LABEL_INVALID_NAME",
            LabelError::LabelInUse { .. } => "LABEL_IN_USE",
            LabelError::InvalidIcon { .. } => "LABEL_INVALID_ICON",
            LabelError::MaxLabelsReached { .. } => "LABEL_MAX_REACHED",
            LabelError::PermissionDenied { .. } => "LABEL_PERMISSION_DENIED",
            LabelError::ColorConflict { .. } => "LABEL_COLOR_CONFLICT",
            LabelError::DescriptionTooLong { .. } => "LABEL_DESCRIPTION_TOO_LONG",
            LabelError::DeleteRestricted { .. } => "LABEL_DELETE_RESTRICTED",
            LabelError::InvalidAssignment { .. } => "LABEL_INVALID_ASSIGNMENT",
            LabelError::ReservedName { .. } => "LABEL_RESERVED_NAME",
        }
    }
    
    fn error_category(&self) -> ErrorCategory {
        match self {
            LabelError::PermissionDenied { .. } 
            | LabelError::SystemLabelModification { .. } => ErrorCategory::Auth,
            _ => ErrorCategory::Database, // Most label operations are database-related
        }
    }
    
    fn error_severity(&self) -> ErrorSeverity {
        match self {
            LabelError::SystemLabelModification { .. } 
            | LabelError::PermissionDenied { .. } => ErrorSeverity::Important,
            LabelError::NotFound 
            | LabelError::DuplicateName { .. } 
            | LabelError::LabelInUse { .. } => ErrorSeverity::Expected,
            _ => ErrorSeverity::Minor,
        }
    }
    
    fn suppression_key(&self) -> Option<String> {
        match self {
            LabelError::NotFound => Some("label_not_found".to_string()),
            LabelError::DuplicateName { name } => Some(format!("label_duplicate_{}", name)),
            _ => None,
        }
    }
    
    fn suggested_action(&self) -> Option<String> {
        match self {
            LabelError::DuplicateName { .. } => Some("Please choose a different name for the label".to_string()),
            LabelError::InvalidColor { .. } => Some("Use a valid hex color format like #0969da or #ff5722".to_string()),
            LabelError::LabelInUse { .. } => Some("Remove the label from all documents first, then try deleting".to_string()),
            LabelError::MaxLabelsReached { .. } => Some("Delete unused labels or contact administrator for limit increase".to_string()),
            LabelError::ColorConflict { .. } => Some("Choose colors with better contrast for readability".to_string()),
            LabelError::DescriptionTooLong { max_length, .. } => Some(format!("Shorten description to {} characters or less", max_length)),
            LabelError::ReservedName { .. } => Some("Choose a different name that is not reserved by the system".to_string()),
            LabelError::InvalidIcon { supported_icons, .. } => Some(format!("Use one of the supported icons: {}", supported_icons)),
            _ => None,
        }
    }
}

impl_into_response!(LabelError);

/// Convenience methods for creating common label errors
impl LabelError {
    pub fn not_found_by_id(id: Uuid) -> Self {
        Self::NotFoundById { id }
    }
    
    pub fn duplicate_name<S: Into<String>>(name: S) -> Self {
        Self::DuplicateName { name: name.into() }
    }
    
    pub fn system_label_modification<S: Into<String>>(name: S) -> Self {
        Self::SystemLabelModification { name: name.into() }
    }
    
    pub fn invalid_color<S: Into<String>>(color: S) -> Self {
        Self::InvalidColor { color: color.into() }
    }
    
    pub fn invalid_name<S: Into<String>>(name: S, reason: S) -> Self {
        Self::InvalidName { 
            name: name.into(), 
            reason: reason.into() 
        }
    }
    
    pub fn label_in_use(document_count: i64) -> Self {
        Self::LabelInUse { document_count }
    }
    
    pub fn invalid_icon<S: Into<String>>(icon: S, supported_icons: S) -> Self {
        Self::InvalidIcon { 
            icon: icon.into(), 
            supported_icons: supported_icons.into() 
        }
    }
    
    pub fn max_labels_reached(max_labels: i32) -> Self {
        Self::MaxLabelsReached { max_labels }
    }
    
    pub fn permission_denied<S: Into<String>>(reason: S) -> Self {
        Self::PermissionDenied { reason: reason.into() }
    }
    
    pub fn color_conflict<S: Into<String>>(color: S, text_color: S) -> Self {
        Self::ColorConflict { 
            color: color.into(), 
            text_color: text_color.into() 
        }
    }
    
    pub fn description_too_long(length: usize, max_length: usize) -> Self {
        Self::DescriptionTooLong { length, max_length }
    }
    
    pub fn delete_restricted<S: Into<String>>(reason: S) -> Self {
        Self::DeleteRestricted { reason: reason.into() }
    }
    
    pub fn invalid_assignment<S: Into<String>>(document_id: Uuid, reason: S) -> Self {
        Self::InvalidAssignment { 
            document_id, 
            reason: reason.into() 
        }
    }
    
    pub fn reserved_name<S: Into<String>>(name: S) -> Self {
        Self::ReservedName { name: name.into() }
    }
}