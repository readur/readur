//! Which OCR languages a user can pick. Backs `GET /api/ocr/languages` and the
//! MCP `get_ocr_languages` tool.

use crate::{
    models::{AvailableLanguagesResponse, LanguageInfo},
    ocr::health::OcrHealthChecker,
};

/// The installed Tesseract languages plus the user's configured OCR language.
pub async fn available_languages_for_user(
    db: &crate::db::Database,
    user_id: uuid::Uuid,
) -> anyhow::Result<AvailableLanguagesResponse> {
    // Get user's current OCR language setting
    let user_settings = db.get_user_settings(user_id).await?;

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

    Ok(AvailableLanguagesResponse {
        available_languages: language_info,
        current_user_language: current_language,
    })
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
    }
    .to_string()
}
