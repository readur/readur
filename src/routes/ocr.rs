use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    auth::AuthUser,
    ocr::health::OcrHealthChecker,
    AppState,
};

#[derive(Serialize, ToSchema)]
pub struct AvailableLanguagesResponse {
    pub available_languages: Vec<LanguageInfo>,
    pub current_user_language: String,
}

#[derive(Serialize, ToSchema)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    pub installed: bool,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(crate::ocr::api::health_check))
        .route("/perform", axum::routing::post(crate::ocr::api::perform_ocr))
        .route("/languages", get(get_available_languages))
}

#[utoipa::path(
    get,
    path = "/api/ocr/languages",
    tag = "ocr",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Available OCR languages and user's current language", body = AvailableLanguagesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_available_languages(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<AvailableLanguagesResponse>, StatusCode> {
    // Get user's current OCR language setting
    let user_settings = state
        .db
        .get_user_settings(auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let current_language = user_settings
        .map(|s| s.ocr_language)
        .unwrap_or_else(|| "eng".to_string());

    // Get available languages from Tesseract
    let health_checker = OcrHealthChecker::new();
    let available_languages = match health_checker.get_available_languages() {
        Ok(langs) => langs,
        Err(_) => {
            // Fallback to common languages if detection fails
            vec!["eng".to_string()]
        }
    };

    // Create language info with display names
    let language_info: Vec<LanguageInfo> = available_languages
        .into_iter()
        .map(|code| LanguageInfo {
            name: get_language_display_name(&code),
            installed: true, // If it's returned by get_available_languages, it's installed
            code,
        })
        .collect();

    Ok(Json(AvailableLanguagesResponse {
        available_languages: language_info,
        current_user_language: current_language,
    }))
}

/// Convert language codes to human-readable names
fn get_language_display_name(code: &str) -> String {
    match code {
        "eng" => "English",
        "spa" => "Spanish",
        "fra" => "French",
        "deu" => "German",
        "ita" => "Italian",
        "por" => "Portuguese",
        "rus" => "Russian",
        "jpn" => "Japanese",
        "chi_sim" => "Chinese (Simplified)",
        "chi_tra" => "Chinese (Traditional)",
        "kor" => "Korean",
        "ara" => "Arabic",
        "hin" => "Hindi",
        "tha" => "Thai",
        "vie" => "Vietnamese",
        "pol" => "Polish",
        "nld" => "Dutch",
        "dan" => "Danish",
        "nor" => "Norwegian",
        "swe" => "Swedish",
        "fin" => "Finnish",
        "ces" => "Czech",
        "hun" => "Hungarian",
        "tur" => "Turkish",
        "heb" => "Hebrew",
        "ukr" => "Ukrainian",
        "bul" => "Bulgarian",
        "ron" => "Romanian",
        "hrv" => "Croatian",
        "slk" => "Slovak",
        "slv" => "Slovenian",
        "est" => "Estonian",
        "lav" => "Latvian",
        "lit" => "Lithuanian",
        "ell" => "Greek",
        "cat" => "Catalan",
        "eus" => "Basque",
        "gla" => "Scottish Gaelic",
        "gle" => "Irish",
        "cym" => "Welsh",
        "isl" => "Icelandic",
        "mlt" => "Maltese",
        "afr" => "Afrikaans",
        "sqi" => "Albanian",
        "aze" => "Azerbaijani",
        "bel" => "Belarusian",
        "ben" => "Bengali",
        "bos" => "Bosnian",
        "bre" => "Breton",
        "kan" => "Kannada",
        "kat" => "Georgian",
        "kaz" => "Kazakh",
        "kir" => "Kyrgyz",
        "lao" => "Lao",
        "lat" => "Latin",
        "ltz" => "Luxembourgish",
        "mkd" => "Macedonian",
        "msa" => "Malay",
        "mal" => "Malayalam",
        "mar" => "Marathi",
        "nep" => "Nepali",
        "ori" => "Odia",
        "pan" => "Punjabi",
        "pus" => "Pashto",
        "fas" => "Persian",
        "san" => "Sanskrit",
        "sin" => "Sinhala",
        "srp" => "Serbian",
        "tam" => "Tamil",
        "tel" => "Telugu",
        "tgk" => "Tajik",
        "uzb" => "Uzbek",
        "urd" => "Urdu",
        _ => {
            // For unknown codes, just return the code as-is
            code
        }
    }.to_string()
}