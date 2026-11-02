//! Classification of OCR error messages into `failed_documents` failure reasons.

/// Maps an OCR error string to a standardized failure reason, and whether
/// the error is expected for some PDFs (logged at debug rather than warn).
pub(crate) fn classify_ocr_error(error_str: &str) -> (&'static str, bool) {
    if error_str.contains("font encoding") || error_str.contains("missing unicode map") {
        ("pdf_parsing_error", true)  // Font encoding issues are PDF parsing problems
    } else if error_str.contains("corrupted internal structure") || error_str.contains("corrupted") {
        ("file_corrupted", true)     // Corrupted files should use file_corrupted
    } else if error_str.contains("timeout") || error_str.contains("timed out") {
        ("ocr_timeout", false)
    } else if error_str.contains("memory") || error_str.contains("out of memory") {
        ("ocr_memory_limit", false)
    } else if error_str.contains("panic") {
        ("pdf_parsing_error", true)
    } else if error_str.contains("unsupported") {
        ("unsupported_format", false)
    } else if error_str.contains("too large") || error_str.contains("file size") {
        ("file_too_large", false)
    } else if error_str.contains("No extractable text") || error_str.contains("0 words") {
        ("low_ocr_confidence", false)  // No extractable text treated as low confidence OCR
    } else if error_str.contains("validation") || error_str.contains("document may have been modified") {
        ("other", false)  // Document validation failures use "other"
    } else {
        ("other", false)  // Fallback for any unrecognized errors
    }
}
