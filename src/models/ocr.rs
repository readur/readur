//! OCR language listing as the API reports it.

use serde::Serialize;
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema, TS)]
#[ts(export)]
pub struct AvailableLanguagesResponse {
    pub available_languages: Vec<LanguageInfo>,
    pub current_user_language: String,
}

#[derive(Serialize, ToSchema, TS)]
#[ts(export)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    pub installed: bool,
}
