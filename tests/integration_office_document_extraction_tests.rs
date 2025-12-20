use readur::ocr::enhanced::EnhancedOcrService;
use readur::models::Settings;
use readur::services::file_service::FileService;
use std::fs;
use std::io::Write;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::{ZipWriter, CompressionMethod};

/// Helper function to create a proper DOCX file for testing
/// Creates a comprehensive DOCX structure that docx-rs can parse
fn create_test_docx(content: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        
        // Add [Content_Types].xml - More comprehensive structure
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
    <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
    <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
    <Override PartName="/word/fontTable.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml"/>
</Types>"#).unwrap();
        
        // Add _rels/.rels 
        zip.add_directory("_rels/", options).unwrap();
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).unwrap();
        
        // Add word directory and its _rels subdirectory
        zip.add_directory("word/", options).unwrap();
        zip.add_directory("word/_rels/", options).unwrap();
        
        // Add word/_rels/document.xml.rels
        zip.start_file("word/_rels/document.xml.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable" Target="fontTable.xml"/>
</Relationships>"#).unwrap();
        
        // Add word/document.xml with proper structure
        zip.start_file("word/document.xml", options).unwrap();
        // Escape XML entities and remove null bytes to create valid XML
        let escaped_content = content.replace('&', "&amp;")
                                    .replace('<', "&lt;")
                                    .replace('>', "&gt;")
                                    .replace('\0', ""); // Remove null bytes as they're invalid in XML
        let document_xml = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
        <w:p>
            <w:r>
                <w:t>{}</w:t>
            </w:r>
        </w:p>
        <w:sectPr>
            <w:pgSz w:w="12240" w:h="15840"/>
            <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/>
        </w:sectPr>
    </w:body>
</w:document>"#, escaped_content);
        zip.write_all(document_xml.as_bytes()).unwrap();
        
        // Add word/styles.xml (minimal styles)
        zip.start_file("word/styles.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:docDefaults>
        <w:rPrDefault>
            <w:rPr>
                <w:rFonts w:ascii="Calibri" w:eastAsia="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/>
                <w:sz w:val="22"/>
                <w:szCs w:val="22"/>
                <w:lang w:val="en-US" w:eastAsia="en-US" w:bidi="ar-SA"/>
            </w:rPr>
        </w:rPrDefault>
    </w:docDefaults>
</w:styles>"#).unwrap();
        
        // Add word/settings.xml (minimal settings)
        zip.start_file("word/settings.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:defaultTabStop w:val="708"/>
</w:settings>"#).unwrap();
        
        // Add word/fontTable.xml (minimal font table)
        zip.start_file("word/fontTable.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:fonts xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:font w:name="Calibri">
        <w:panose1 w:val="020F0502020204030204"/>
        <w:charset w:val="00"/>
        <w:family w:val="swiss"/>
        <w:pitch w:val="variable"/>
    </w:font>
</w:fonts>"#).unwrap();
        
        zip.finish().unwrap();
    }
    buffer
}

/// Helper function to create a proper XLSX file for testing
/// Uses rust_xlsxwriter to create a real XLSX file that calamine can properly read
fn create_test_xlsx(content: &str) -> Vec<u8> {
    use rust_xlsxwriter::*;
    
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    
    // Add the test content to cell A1
    worksheet.write_string(0, 0, content).expect("Failed to write to worksheet");
    
    // Save to buffer and return bytes
    workbook.save_to_buffer().expect("Failed to save XLSX to buffer")
}

#[tokio::test]
async fn test_docx_text_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = temp_dir.path().join("test.docx");
    
    // Create a test DOCX file
    let test_content = "This is a test DOCX document with some content.";
    let docx_data = create_test_docx(test_content);
    fs::write(&docx_path, docx_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Extract text from DOCX
    let result = ocr_service.extract_text_from_office(
        docx_path.to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &settings
    ).await;
    
    assert!(result.is_ok(), "DOCX extraction should succeed");
    let ocr_result = result.unwrap();
    // The extracted text may include section breaks and other document structure
    assert!(ocr_result.text.contains(test_content), "Should contain the test content: {}", ocr_result.text);
    assert_eq!(ocr_result.confidence, 100.0);
    assert!(ocr_result.word_count > 0);
}

#[tokio::test]
async fn test_xlsx_text_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let xlsx_path = temp_dir.path().join("test.xlsx");
    
    // Create a test XLSX file
    let test_content = "Excel spreadsheet test data";
    let xlsx_data = create_test_xlsx(test_content);
    fs::write(&xlsx_path, xlsx_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Extract text from XLSX
    let result = ocr_service.extract_text_from_office(
        xlsx_path.to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        &settings
    ).await;
    
    assert!(result.is_ok(), "XLSX extraction should succeed");
    let ocr_result = result.unwrap();
    assert_eq!(ocr_result.text.trim(), test_content);
    assert_eq!(ocr_result.confidence, 100.0);
    assert!(ocr_result.word_count > 0);
}

#[tokio::test]
async fn test_null_byte_removal() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = temp_dir.path().join("test_nulls.docx");
    
    // Create a test DOCX file with null bytes embedded (shouldn't happen in real files)
    let test_content = "Test\0with\0null\0bytes";
    let docx_data = create_test_docx(test_content);
    fs::write(&docx_path, docx_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Extract text from DOCX
    let result = ocr_service.extract_text_from_office(
        docx_path.to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &settings
    ).await;
    
    assert!(result.is_ok(), "DOCX extraction should succeed even with null bytes");
    let ocr_result = result.unwrap();
    
    // Verify null bytes were removed (they were stripped during DOCX creation since they're invalid in XML)
    assert!(!ocr_result.text.contains('\0'), "Extracted text should not contain null bytes");
    // The XML extraction may add section breaks, so check if the main text is present
    assert!(ocr_result.text.contains("Testwithnullbytes"), "Extracted text should contain the expected content");
}

#[tokio::test]
async fn test_preserve_formatting() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = temp_dir.path().join("test_formatting.docx");
    
    // Create a test DOCX file with special formatting
    let test_content = "Line 1\n\nLine 2\t\tTabbed\n   Indented   ";
    let docx_data = create_test_docx(test_content);
    fs::write(&docx_path, docx_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Extract text from DOCX
    let result = ocr_service.extract_text_from_office(
        docx_path.to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &settings
    ).await;
    
    assert!(result.is_ok(), "DOCX extraction should succeed");
    let ocr_result = result.unwrap();
    
    // Verify formatting is preserved (no aggressive sanitization)
    // Note: The DOCX might not preserve exact formatting, but we shouldn't be removing it
    assert!(ocr_result.text.contains("Line 1"));
    assert!(ocr_result.text.contains("Line 2"));
    assert!(ocr_result.text.contains("Tabbed"));
    assert!(ocr_result.text.contains("Indented"));
}

#[tokio::test]
async fn test_empty_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = temp_dir.path().join("empty.docx");
    
    // Create an empty DOCX file
    let docx_data = create_test_docx("");
    fs::write(&docx_path, docx_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Extract text from empty DOCX
    let result = ocr_service.extract_text_from_office(
        docx_path.to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &settings
    ).await;
    
    // Should fail with appropriate error message
    assert!(result.is_err(), "Empty DOCX should return an error");
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("No text content found") || error_msg.contains("empty"));
}

#[tokio::test]
async fn test_corrupted_docx() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = temp_dir.path().join("corrupted.docx");
    
    // Create a corrupted DOCX file (not a valid ZIP)
    fs::write(&docx_path, b"This is not a valid DOCX file").unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Try to extract text from corrupted DOCX
    let result = ocr_service.extract_text_from_office(
        docx_path.to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &settings
    ).await;
    
    // Should fail with appropriate error message
    assert!(result.is_err(), "Corrupted DOCX should return an error");
    let error_msg = result.unwrap_err().to_string();
    // Check for various error messages that indicate a corrupted file
    assert!(
        error_msg.contains("invalid Zip archive") ||  // Actual error from zip crate
        error_msg.contains("Invalid ZIP") || 
        error_msg.contains("corrupted") ||
        error_msg.contains("Could not find central directory"),
        "Expected error about invalid/corrupted file, got: {}", error_msg
    );
}

#[tokio::test]
async fn test_legacy_doc_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("legacy.doc");
    
    // Create a simple text file with .doc extension to test DOC processing
    // catdoc will process this as text, which is expected behavior
    fs::write(&doc_path, b"This is test content for DOC extraction").unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Try to extract text from DOC file
    let result = ocr_service.extract_text_from_office(
        doc_path.to_str().unwrap(),
        "application/msword",
        &settings
    ).await;
    
    // DOC processing should succeed when external tools are available
    assert!(result.is_ok(), "DOC extraction should succeed when tools are available");
    let ocr_result = result.unwrap();
    
    // Verify the extraction results
    assert!(ocr_result.word_count > 0, "Should have extracted some words");
    assert!(ocr_result.text.contains("test content"), "Should contain the test text");
    assert!(ocr_result.confidence > 0.0, "Should have confidence score");
    assert!(ocr_result.preprocessing_applied.len() > 0, "Should have preprocessing steps recorded");
    
    // Verify it used an external DOC tool
    let preprocessing_info = &ocr_result.preprocessing_applied[0];
    assert!(
        preprocessing_info.contains("catdoc") || 
        preprocessing_info.contains("antiword") || 
        preprocessing_info.contains("wvText"),
        "Should indicate which DOC tool was used"
    );
}

#[tokio::test]
async fn test_legacy_doc_error_when_tools_unavailable() {
    // This test documents the expected behavior when DOC extraction tools are not available.
    // Since antiword and catdoc are available in the current test environment, this test
    // would need to be run in an environment without these tools to actually fail.
    // For now, this serves as documentation of the expected error message format.
    
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("test.doc");
    
    // Create a test DOC file
    fs::write(&doc_path, b"Test DOC content").unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Try to extract text from DOC file
    let result = ocr_service.extract_text_from_office(
        doc_path.to_str().unwrap(),
        "application/msword",
        &settings
    ).await;
    
    // Since tools are available in this environment, this should succeed
    // In an environment without DOC tools, it would fail with a helpful error message like:
    // "None of the DOC extraction tools (antiword, catdoc, wvText) are available or working."
    match result {
        Ok(ocr_result) => {
            // Tools are available - verify successful extraction
            assert!(ocr_result.word_count > 0, "Should extract text when tools are available");
            println!("DOC tools are available, extraction succeeded with {} words", ocr_result.word_count);
        }
        Err(error) => {
            // Tools are not available - verify proper error message
            let error_msg = error.to_string();
            assert!(
                error_msg.contains("DOC extraction tools") &&
                (error_msg.contains("antiword") || error_msg.contains("catdoc") || error_msg.contains("wvText")),
                "Should provide helpful error about missing DOC tools, got: {}", error_msg
            );
            println!("DOC tools not available, got expected error: {}", error_msg);
        }
    }
}

#[tokio::test]
async fn test_file_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let docx_path = temp_dir.path().join("large.docx");
    
    // Create a DOCX that would exceed size limit (simulated by very long content)
    let large_content = "x".repeat(100_000); // Large but not actually 50MB in ZIP
    let docx_data = create_test_docx(&large_content);
    fs::write(&docx_path, docx_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Extract text from large DOCX
    let result = ocr_service.extract_text_from_office(
        docx_path.to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &settings
    ).await;
    
    // Should succeed for content within limits
    assert!(result.is_ok(), "DOCX within size limits should succeed");
}

/// Helper function to create a minimal DOC file for testing
/// Note: This creates a fake DOC file since real DOC format is complex binary
fn create_fake_doc_file() -> Vec<u8> {
    // Create a DOC-like header that might fool basic detection
    // but will fail in actual conversion/extraction
    let mut doc_data = Vec::new();
    
    // DOC files start with compound document signature
    doc_data.extend_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    
    // Add some padding to make it look like a real file
    doc_data.extend_from_slice(b"This is fake DOC content for testing purposes");
    doc_data.resize(1024, 0); // Pad to reasonable size
    
    doc_data
}

#[tokio::test]
async fn test_legacy_doc_enhanced_error_message() {
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("test.doc");
    
    // Create a fake DOC file
    let doc_data = create_fake_doc_file();
    fs::write(&doc_path, doc_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Try to extract text from legacy DOC
    let result = ocr_service.extract_text_from_office(
        doc_path.to_str().unwrap(),
        "application/msword",
        &settings
    ).await;
    
    // Should fail with enhanced error message
    assert!(result.is_err(), "Legacy DOC should return an error without tools");
    let error_msg = result.unwrap_err().to_string();
    
    // Verify enhanced error message mentions extraction tools
    assert!(error_msg.contains("None of the DOC extraction tools") || error_msg.contains("All extraction methods failed"), "Should mention extraction tools failed");
    assert!(error_msg.contains("antiword"), "Should mention antiword tool");
    assert!(error_msg.contains("catdoc"), "Should mention catdoc tool");
}

// Note: DOC to DOCX conversion tests removed since we no longer use LibreOffice
// Legacy DOC files are now handled by lightweight tools (antiword/catdoc) only



#[tokio::test]
async fn test_doc_extraction_multiple_strategies() {
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("multitest.doc");
    
    // Create a fake DOC file
    let doc_data = create_fake_doc_file();
    fs::write(&doc_path, doc_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    let start_time = std::time::Instant::now();
    
    // Test Office extraction with the DOC file (this should fail as DOC files are not XML-based)
    let result = ocr_service.extract_text_from_office(
        doc_path.to_str().unwrap(),
        "application/msword",
        &settings
    ).await;
    
    // Should fail since external DOC tools are not available in test environment
    assert!(result.is_err(), "Should fail for DOC files as external tools are not available");
    let error_msg = result.unwrap_err().to_string();
    
    // Verify it mentions external tool issues for DOC files
    assert!(error_msg.contains("DOC extraction tools") || error_msg.contains("antiword") || error_msg.contains("catdoc"), 
        "Should mention external tool issues: {}", error_msg);
}

#[tokio::test]
async fn test_doc_error_message_includes_processing_time() {
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("timed.doc");
    
    // Create a fake DOC file
    let doc_data = create_fake_doc_file();
    fs::write(&doc_path, doc_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
        max_pdf_size: 100 * 1024 * 1024,
        max_office_document_size: 100 * 1024 * 1024,
    };
    
    let settings = Settings::default();
    
    // Try to extract text from legacy DOC
    let result = ocr_service.extract_text_from_office(
        doc_path.to_str().unwrap(),
        "application/msword",
        &settings
    ).await;
    
    // Should fail and include processing time in error message
    assert!(result.is_err(), "Should fail without tools");
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Processing time:") && error_msg.contains("ms"), 
        "Should include processing time: {}", error_msg);
}

// Note: UUID uniqueness test removed since we no longer use temporary conversion directories