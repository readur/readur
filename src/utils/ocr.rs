/*!
 * OCR Utility Functions
 * 
 * Helper functions to determine which file types require OCR processing
 * versus text extraction.
 */

/// Determine if a file requires OCR processing based on its filename/extension
pub fn file_needs_ocr(filename: &str) -> bool {
    // File extensions that should go through OCR pipeline (images and scanned PDFs)
    let ocr_extensions = vec![".pdf", ".jpg", ".jpeg", ".png", ".tiff", ".tiff", ".bmp", ".gif", ".webp"];
    let extension = extract_extension(filename);
    ocr_extensions.contains(&extension.as_str())
}

/// Determine if a file should use text extraction (Office docs, plain text)
pub fn file_needs_text_extraction(filename: &str) -> bool {
    // File extensions that should use text extraction instead of OCR
    let text_extensions = vec![".doc", ".docx", ".txt", ".rtf", ".odt", ".html", ".htm"];
    let extension = extract_extension(filename);
    text_extensions.contains(&extension.as_str())
}

/// Extract file extension from filename (lowercased)
fn extract_extension(filename: &str) -> String {
    if let Some(pos) = filename.rfind('.') {
        filename[pos..].to_lowercase()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_files_need_ocr() {
        assert!(file_needs_ocr("document.pdf"));
        assert!(file_needs_ocr("image.jpg"));
        assert!(file_needs_ocr("scan.png"));
        assert!(file_needs_ocr("photo.JPEG")); // Test case insensitive
    }

    #[test]
    fn test_office_files_need_text_extraction() {
        assert!(file_needs_text_extraction("document.docx"));
        assert!(file_needs_text_extraction("document.DOC")); // Test case insensitive
        assert!(file_needs_text_extraction("document.txt"));
        assert!(file_needs_text_extraction("document.html"));
    }

    #[test]
    fn test_office_files_dont_need_ocr() {
        assert!(!file_needs_ocr("document.docx"));
        assert!(!file_needs_ocr("document.doc"));
        assert!(!file_needs_ocr("document.txt"));
        assert!(!file_needs_ocr("document.html"));
    }

    #[test]
    fn test_image_files_dont_need_text_extraction() {
        assert!(!file_needs_text_extraction("document.pdf"));
        assert!(!file_needs_text_extraction("image.jpg"));
        assert!(!file_needs_text_extraction("scan.png"));
    }
}