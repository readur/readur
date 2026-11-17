use axum::http::StatusCode;
use thiserror::Error;
use uuid::Uuid;

use super::{impl_into_response, AppError, ErrorCategory, ErrorSeverity};

/// Errors related to settings management operations
#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("Settings not found for user")]
    NotFound,

    #[error("Settings not found for user {user_id}")]
    NotFoundForUser { user_id: Uuid },

    #[error("Invalid language '{language}'. Available languages: {available_languages}")]
    InvalidLanguage {
        language: String,
        available_languages: String,
    },

    #[error("Invalid value for setting '{setting_name}': {value}. {constraint}")]
    InvalidValue {
        setting_name: String,
        value: String,
        constraint: String,
    },

    #[error("Setting '{setting_name}' is read-only and cannot be modified")]
    ReadOnlySetting { setting_name: String },

    #[error("Validation failed for setting '{setting_name}': {reason}")]
    ValidationFailed {
        setting_name: String,
        reason: String,
    },

    #[error("Invalid OCR configuration: {details}")]
    InvalidOcrConfiguration { details: String },

    #[error("Invalid file type '{file_type}'. Supported types: {supported_types}")]
    InvalidFileType {
        file_type: String,
        supported_types: String,
    },

    #[error("Value {value} is out of range for '{setting_name}'. Valid range: {min} - {max}")]
    ValueOutOfRange {
        setting_name: String,
        value: i32,
        min: i32,
        max: i32,
    },

    #[error(
        "Invalid timeout value {timeout_seconds}s. Valid range: {min_seconds}s - {max_seconds}s"
    )]
    InvalidTimeout {
        timeout_seconds: i32,
        min_seconds: i32,
        max_seconds: i32,
    },

    #[error("DPI value {dpi} is invalid. Valid range: {min_dpi} - {max_dpi}")]
    InvalidDpi {
        dpi: i32,
        min_dpi: i32,
        max_dpi: i32,
    },

    #[error("Confidence threshold {confidence} is invalid. Valid range: 0.0 - 1.0")]
    InvalidConfidenceThreshold { confidence: f32 },

    #[error("Invalid character list for '{list_type}': {details}")]
    InvalidCharacterList { list_type: String, details: String },

    #[error("Conflicting settings: {setting1} and {setting2} cannot both be enabled")]
    ConflictingSettings { setting1: String, setting2: String },

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Cannot reset system-wide settings")]
    SystemSettingsReset,

    #[error("Invalid search configuration: {details}")]
    InvalidSearchConfiguration { details: String },
}

impl AppError for SettingsError {
    fn status_code(&self) -> StatusCode {
        match self {
            SettingsError::NotFound | SettingsError::NotFoundForUser { .. } => {
                StatusCode::NOT_FOUND
            }
            SettingsError::InvalidLanguage { .. } => StatusCode::BAD_REQUEST,
            SettingsError::InvalidValue { .. } => StatusCode::BAD_REQUEST,
            SettingsError::ReadOnlySetting { .. } => StatusCode::FORBIDDEN,
            SettingsError::ValidationFailed { .. } => StatusCode::BAD_REQUEST,
            SettingsError::InvalidOcrConfiguration { .. } => StatusCode::BAD_REQUEST,
            SettingsError::InvalidFileType { .. } => StatusCode::BAD_REQUEST,
            SettingsError::ValueOutOfRange { .. } => StatusCode::BAD_REQUEST,
            SettingsError::InvalidTimeout { .. } => StatusCode::BAD_REQUEST,
            SettingsError::InvalidDpi { .. } => StatusCode::BAD_REQUEST,
            SettingsError::InvalidConfidenceThreshold { .. } => StatusCode::BAD_REQUEST,
            SettingsError::InvalidCharacterList { .. } => StatusCode::BAD_REQUEST,
            SettingsError::ConflictingSettings { .. } => StatusCode::CONFLICT,
            SettingsError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            SettingsError::SystemSettingsReset => StatusCode::FORBIDDEN,
            SettingsError::InvalidSearchConfiguration { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn user_message(&self) -> String {
        match self {
            SettingsError::NotFound | SettingsError::NotFoundForUser { .. } => {
                "Settings not found".to_string()
            }
            SettingsError::InvalidLanguage { .. } => "Invalid language specified".to_string(),
            SettingsError::InvalidValue { setting_name, .. } => {
                format!("Invalid value for {}", setting_name)
            }
            SettingsError::ReadOnlySetting { setting_name } => {
                format!("Setting '{}' cannot be modified", setting_name)
            }
            SettingsError::ValidationFailed {
                setting_name,
                reason,
            } => format!("Validation failed for {}: {}", setting_name, reason),
            SettingsError::InvalidOcrConfiguration { .. } => {
                "Invalid OCR configuration".to_string()
            }
            SettingsError::InvalidFileType { .. } => "Invalid file type specified".to_string(),
            SettingsError::ValueOutOfRange {
                setting_name,
                min,
                max,
                ..
            } => format!("{} must be between {} and {}", setting_name, min, max),
            SettingsError::InvalidTimeout {
                min_seconds,
                max_seconds,
                ..
            } => format!(
                "Timeout must be between {}s and {}s",
                min_seconds, max_seconds
            ),
            SettingsError::InvalidDpi {
                min_dpi, max_dpi, ..
            } => format!("DPI must be between {} and {}", min_dpi, max_dpi),
            SettingsError::InvalidConfidenceThreshold { .. } => {
                "Confidence threshold must be between 0.0 and 1.0".to_string()
            }
            SettingsError::InvalidCharacterList { list_type, .. } => {
                format!("Invalid {} character list", list_type)
            }
            SettingsError::ConflictingSettings { setting1, setting2 } => {
                format!("Settings '{}' and '{}' conflict", setting1, setting2)
            }
            SettingsError::PermissionDenied { reason } => format!("Permission denied: {}", reason),
            SettingsError::SystemSettingsReset => "System settings cannot be reset".to_string(),
            SettingsError::InvalidSearchConfiguration { .. } => {
                "Invalid search configuration".to_string()
            }
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            SettingsError::NotFound => "SETTINGS_NOT_FOUND",
            SettingsError::NotFoundForUser { .. } => "SETTINGS_NOT_FOUND_FOR_USER",
            SettingsError::InvalidLanguage { .. } => "SETTINGS_INVALID_LANGUAGE",
            SettingsError::InvalidValue { .. } => "SETTINGS_INVALID_VALUE",
            SettingsError::ReadOnlySetting { .. } => "SETTINGS_READ_ONLY",
            SettingsError::ValidationFailed { .. } => "SETTINGS_VALIDATION_FAILED",
            SettingsError::InvalidOcrConfiguration { .. } => "SETTINGS_INVALID_OCR_CONFIG",
            SettingsError::InvalidFileType { .. } => "SETTINGS_INVALID_FILE_TYPE",
            SettingsError::ValueOutOfRange { .. } => "SETTINGS_VALUE_OUT_OF_RANGE",
            SettingsError::InvalidTimeout { .. } => "SETTINGS_INVALID_TIMEOUT",
            SettingsError::InvalidDpi { .. } => "SETTINGS_INVALID_DPI",
            SettingsError::InvalidConfidenceThreshold { .. } => "SETTINGS_INVALID_CONFIDENCE",
            SettingsError::InvalidCharacterList { .. } => "SETTINGS_INVALID_CHARACTER_LIST",
            SettingsError::ConflictingSettings { .. } => "SETTINGS_CONFLICTING",
            SettingsError::PermissionDenied { .. } => "SETTINGS_PERMISSION_DENIED",
            SettingsError::SystemSettingsReset => "SETTINGS_SYSTEM_RESET_DENIED",
            SettingsError::InvalidSearchConfiguration { .. } => "SETTINGS_INVALID_SEARCH_CONFIG",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            SettingsError::PermissionDenied { .. } | SettingsError::SystemSettingsReset => {
                ErrorCategory::Auth
            }
            SettingsError::InvalidOcrConfiguration { .. } => ErrorCategory::OcrProcessing,
            _ => ErrorCategory::Config,
        }
    }

    fn error_severity(&self) -> ErrorSeverity {
        match self {
            SettingsError::ReadOnlySetting { .. }
            | SettingsError::PermissionDenied { .. }
            | SettingsError::SystemSettingsReset => ErrorSeverity::Important,
            SettingsError::InvalidOcrConfiguration { .. }
            | SettingsError::ConflictingSettings { .. } => ErrorSeverity::Important,
            SettingsError::NotFound | SettingsError::NotFoundForUser { .. } => {
                ErrorSeverity::Expected
            }
            _ => ErrorSeverity::Minor,
        }
    }

    fn suppression_key(&self) -> Option<String> {
        match self {
            SettingsError::NotFound => Some("settings_not_found".to_string()),
            SettingsError::InvalidLanguage { language, .. } => {
                Some(format!("settings_invalid_language_{}", language))
            }
            _ => None,
        }
    }

    fn suggested_action(&self) -> Option<String> {
        match self {
            SettingsError::InvalidLanguage {
                available_languages,
                ..
            } => Some(format!(
                "Choose from available languages: {}",
                available_languages
            )),
            SettingsError::InvalidFileType {
                supported_types, ..
            } => Some(format!("Use supported file types: {}", supported_types)),
            SettingsError::ValueOutOfRange { min, max, .. } => {
                Some(format!("Enter a value between {} and {}", min, max))
            }
            SettingsError::InvalidTimeout {
                min_seconds,
                max_seconds,
                ..
            } => Some(format!(
                "Set timeout between {}s and {}s",
                min_seconds, max_seconds
            )),
            SettingsError::InvalidDpi {
                min_dpi, max_dpi, ..
            } => Some(format!("Set DPI between {} and {}", min_dpi, max_dpi)),
            SettingsError::InvalidConfidenceThreshold { .. } => {
                Some("Set confidence threshold between 0.0 and 1.0".to_string())
            }
            SettingsError::ConflictingSettings { setting1, setting2 } => Some(format!(
                "Disable either '{}' or '{}' to resolve the conflict",
                setting1, setting2
            )),
            SettingsError::ReadOnlySetting { .. } => {
                Some("This setting cannot be modified through the API".to_string())
            }
            _ => None,
        }
    }
}

impl_into_response!(SettingsError);

/// Convenience methods for creating common settings errors
impl SettingsError {
    pub fn not_found_for_user(user_id: Uuid) -> Self {
        Self::NotFoundForUser { user_id }
    }

    pub fn invalid_language<S: Into<String>>(language: S, available_languages: S) -> Self {
        Self::InvalidLanguage {
            language: language.into(),
            available_languages: available_languages.into(),
        }
    }

    pub fn invalid_value<S: Into<String>>(setting_name: S, value: S, constraint: S) -> Self {
        Self::InvalidValue {
            setting_name: setting_name.into(),
            value: value.into(),
            constraint: constraint.into(),
        }
    }

    pub fn read_only_setting<S: Into<String>>(setting_name: S) -> Self {
        Self::ReadOnlySetting {
            setting_name: setting_name.into(),
        }
    }

    pub fn validation_failed<S: Into<String>>(setting_name: S, reason: S) -> Self {
        Self::ValidationFailed {
            setting_name: setting_name.into(),
            reason: reason.into(),
        }
    }

    pub fn invalid_ocr_configuration<S: Into<String>>(details: S) -> Self {
        Self::InvalidOcrConfiguration {
            details: details.into(),
        }
    }

    pub fn invalid_file_type<S: Into<String>>(file_type: S, supported_types: S) -> Self {
        Self::InvalidFileType {
            file_type: file_type.into(),
            supported_types: supported_types.into(),
        }
    }

    pub fn value_out_of_range<S: Into<String>>(
        setting_name: S,
        value: i32,
        min: i32,
        max: i32,
    ) -> Self {
        Self::ValueOutOfRange {
            setting_name: setting_name.into(),
            value,
            min,
            max,
        }
    }

    pub fn invalid_timeout(timeout_seconds: i32, min_seconds: i32, max_seconds: i32) -> Self {
        Self::InvalidTimeout {
            timeout_seconds,
            min_seconds,
            max_seconds,
        }
    }

    pub fn invalid_dpi(dpi: i32, min_dpi: i32, max_dpi: i32) -> Self {
        Self::InvalidDpi {
            dpi,
            min_dpi,
            max_dpi,
        }
    }

    pub fn invalid_confidence_threshold(confidence: f32) -> Self {
        Self::InvalidConfidenceThreshold { confidence }
    }

    pub fn invalid_character_list<S: Into<String>>(list_type: S, details: S) -> Self {
        Self::InvalidCharacterList {
            list_type: list_type.into(),
            details: details.into(),
        }
    }

    pub fn conflicting_settings<S: Into<String>>(setting1: S, setting2: S) -> Self {
        Self::ConflictingSettings {
            setting1: setting1.into(),
            setting2: setting2.into(),
        }
    }

    pub fn permission_denied<S: Into<String>>(reason: S) -> Self {
        Self::PermissionDenied {
            reason: reason.into(),
        }
    }

    pub fn invalid_search_configuration<S: Into<String>>(details: S) -> Self {
        Self::InvalidSearchConfiguration {
            details: details.into(),
        }
    }
}
