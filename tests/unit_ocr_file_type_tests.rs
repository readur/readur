/*!
 * Unit Tests for OCR File Type Routing
 * 
 * These tests verify the utility functions that determine whether files
 * should be routed to OCR or text extraction pipelines.
 */

#[cfg(test)]
mod tests {
    use readur::utils::ocr::{file_needs_ocr, file_needs_text_extraction};

    #[test]
    fn test_office_documents_dont_need_ocr() {
        // Office documents should NOT be routed to OCR
        assert!(!file_needs_ocr("document.docx"), "DOCX files should not need OCR");
        assert!(!file_needs_ocr("document.DOC"), "DOC files should not need OCR (case insensitive)");
        assert!(!file_needs_ocr("report.doc"), "DOC files should not need OCR");
        assert!(!file_needs_ocr("DOCUMENT.DOCX"), "DOCX files should not need OCR (case insensitive)");
    }

    #[test] 
    fn test_office_documents_need_text_extraction() {
        // Office documents SHOULD use text extraction
        assert!(file_needs_text_extraction("document.docx"), "DOCX files should need text extraction");
        assert!(file_needs_text_extraction("document.DOC"), "DOC files should need text extraction (case insensitive)");
        assert!(file_needs_text_extraction("report.doc"), "DOC files should need text extraction");
        assert!(file_needs_text_extraction("DOCUMENT.DOCX"), "DOCX files should need text extraction (case insensitive)");
    }

    #[test]
    fn test_image_files_need_ocr() {
        // Image files SHOULD be routed to OCR
        assert!(file_needs_ocr("scan.png"), "PNG files should need OCR");
        assert!(file_needs_ocr("photo.jpg"), "JPG files should need OCR");
        assert!(file_needs_ocr("image.JPEG"), "JPEG files should need OCR (case insensitive)");
        assert!(file_needs_ocr("document.tiff"), "TIFF files should need OCR");
        assert!(file_needs_ocr("bitmap.bmp"), "BMP files should need OCR");
        assert!(file_needs_ocr("graphic.gif"), "GIF files should need OCR");
        assert!(file_needs_ocr("modern.webp"), "WEBP files should need OCR");
    }

    #[test]
    fn test_image_files_dont_need_text_extraction() {
        // Image files should NOT use text extraction
        assert!(!file_needs_text_extraction("scan.png"), "PNG files should not need text extraction");
        assert!(!file_needs_text_extraction("photo.jpg"), "JPG files should not need text extraction");
        assert!(!file_needs_text_extraction("image.JPEG"), "JPEG files should not need text extraction");
        assert!(!file_needs_text_extraction("document.tiff"), "TIFF files should not need text extraction");
        assert!(!file_needs_text_extraction("bitmap.bmp"), "BMP files should not need text extraction");
    }

    #[test]
    fn test_pdf_files_need_ocr() {
        // PDF files SHOULD be routed to OCR (they might be scanned documents)
        assert!(file_needs_ocr("document.pdf"), "PDF files should need OCR");
        assert!(file_needs_ocr("SCAN.PDF"), "PDF files should need OCR (case insensitive)");
        assert!(file_needs_ocr("report.pdf"), "PDF files should need OCR");
    }

    #[test]
    fn test_pdf_files_dont_need_text_extraction() {
        // PDF files should NOT use text extraction (they go through OCR which can handle both scanned and text PDFs)
        assert!(!file_needs_text_extraction("document.pdf"), "PDF files should not need text extraction");
        assert!(!file_needs_text_extraction("SCAN.PDF"), "PDF files should not need text extraction");
    }

    #[test]
    fn test_text_files_need_text_extraction() {
        // Plain text files should use text extraction
        assert!(file_needs_text_extraction("readme.txt"), "TXT files should need text extraction");
        assert!(file_needs_text_extraction("document.html"), "HTML files should need text extraction");
        assert!(file_needs_text_extraction("page.HTM"), "HTM files should need text extraction (case insensitive)");
        assert!(file_needs_text_extraction("document.rtf"), "RTF files should need text extraction");
        assert!(file_needs_text_extraction("document.odt"), "ODT files should need text extraction");
    }

    #[test]
    fn test_text_files_dont_need_ocr() {
        // Plain text files should NOT go through OCR
        assert!(!file_needs_ocr("readme.txt"), "TXT files should not need OCR");
        assert!(!file_needs_ocr("document.html"), "HTML files should not need OCR");
        assert!(!file_needs_ocr("page.HTM"), "HTM files should not need OCR");
        assert!(!file_needs_ocr("document.rtf"), "RTF files should not need OCR");
        assert!(!file_needs_ocr("document.odt"), "ODT files should not need OCR");
    }

    #[test]
    fn test_unsupported_files_dont_need_either() {
        // Files that aren't supported should not need either OCR or text extraction
        assert!(!file_needs_ocr("video.mp4"), "MP4 files should not need OCR");
        assert!(!file_needs_ocr("audio.mp3"), "MP3 files should not need OCR");
        assert!(!file_needs_ocr("archive.zip"), "ZIP files should not need OCR");
        assert!(!file_needs_ocr("executable.exe"), "EXE files should not need OCR");
        
        assert!(!file_needs_text_extraction("video.mp4"), "MP4 files should not need text extraction");
        assert!(!file_needs_text_extraction("audio.mp3"), "MP3 files should not need text extraction");
        assert!(!file_needs_text_extraction("archive.zip"), "ZIP files should not need text extraction");
        assert!(!file_needs_text_extraction("executable.exe"), "EXE files should not need text extraction");
    }

    #[test]
    fn test_files_without_extension() {
        // Files without extensions should not need either
        assert!(!file_needs_ocr("README"), "Files without extension should not need OCR");
        assert!(!file_needs_ocr("Makefile"), "Files without extension should not need OCR");
        assert!(!file_needs_ocr(""), "Empty filename should not need OCR");
        
        assert!(!file_needs_text_extraction("README"), "Files without extension should not need text extraction");
        assert!(!file_needs_text_extraction("Makefile"), "Files without extension should not need text extraction");
        assert!(!file_needs_text_extraction(""), "Empty filename should not need text extraction");
    }

    #[test]
    fn test_edge_cases() {
        // Test various edge cases
        assert!(!file_needs_ocr(".docx"), "Hidden DOCX files should not need OCR");
        assert!(file_needs_text_extraction(".docx"), "Hidden DOCX files should need text extraction");
        
        assert!(file_needs_ocr("file.name.with.dots.png"), "Files with multiple dots should work correctly");
        assert!(!file_needs_text_extraction("file.name.with.dots.png"), "Files with multiple dots should work correctly");
        
        assert!(file_needs_text_extraction("file.name.with.dots.docx"), "Files with multiple dots should work correctly");
        assert!(!file_needs_ocr("file.name.with.dots.docx"), "Files with multiple dots should work correctly");
    }
}