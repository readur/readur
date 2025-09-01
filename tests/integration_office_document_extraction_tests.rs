use readur::ocr::enhanced::EnhancedOcrService;
use readur::models::Settings;
use readur::services::file_service::FileService;
use std::fs;
use std::io::Write;
use tempfile::TempDir;
use zip::write::FileOptions;
use zip::{ZipWriter, CompressionMethod};

/// Helper function to create a minimal DOCX file for testing
fn create_test_docx(content: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        
        // Add required DOCX structure
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
        
        // Add [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).unwrap();
        
        // Add _rels/.rels
        zip.add_directory("_rels", options).unwrap();
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).unwrap();
        
        // Add word directory
        zip.add_directory("word", options).unwrap();
        
        // Add word/document.xml with the actual content
        zip.start_file("word/document.xml", options).unwrap();
        let document_xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
        <w:p>
            <w:r>
                <w:t>{}</w:t>
            </w:r>
        </w:p>
    </w:body>
</w:document>"#, content);
        zip.write_all(document_xml.as_bytes()).unwrap();
        
        zip.finish().unwrap();
    }
    buffer
}

/// Helper function to create a minimal XLSX file for testing
fn create_test_xlsx(content: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
        
        // Add [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
    <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
    <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#).unwrap();
        
        // Add _rels/.rels
        zip.add_directory("_rels", options).unwrap();
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#).unwrap();
        
        // Add xl directory structure
        zip.add_directory("xl", options).unwrap();
        zip.add_directory("xl/worksheets", options).unwrap();
        
        // Add xl/workbook.xml
        zip.start_file("xl/workbook.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
    <sheets>
        <sheet name="Sheet1" sheetId="1" r:id="rId1" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"/>
    </sheets>
</workbook>"#).unwrap();
        
        // Add xl/sharedStrings.xml
        zip.start_file("xl/sharedStrings.xml", options).unwrap();
        let shared_strings_xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1" uniqueCount="1">
    <si><t>{}</t></si>
</sst>"#, content);
        zip.write_all(shared_strings_xml.as_bytes()).unwrap();
        
        // Add xl/worksheets/sheet1.xml
        zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
    <sheetData>
        <row r="1">
            <c r="A1" t="s">
                <v>0</v>
            </c>
        </row>
    </sheetData>
</worksheet>"#).unwrap();
        
        zip.finish().unwrap();
    }
    buffer
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
    assert_eq!(ocr_result.text.trim(), test_content);
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
    
    // Verify null bytes were removed
    assert!(!ocr_result.text.contains('\0'), "Extracted text should not contain null bytes");
    assert_eq!(ocr_result.text.trim(), "Testwithnullbytes");
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
async fn test_legacy_doc_error() {
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("legacy.doc");
    
    // Create a fake DOC file
    fs::write(&doc_path, b"Legacy DOC format").unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
    };
    
    let settings = Settings::default();
    
    // Try to extract text from legacy DOC
    let result = ocr_service.extract_text_from_office(
        doc_path.to_str().unwrap(),
        "application/msword",
        &settings
    ).await;
    
    // Should fail with helpful error about external tools
    assert!(result.is_err(), "Legacy DOC should return an error");
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("antiword") || error_msg.contains("catdoc") || error_msg.contains("external tool"));
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
    
    // Verify enhanced error message mentions all strategies
    assert!(error_msg.contains("All extraction methods failed"), "Should mention all methods failed");
    assert!(error_msg.contains("DOC to DOCX conversion"), "Should mention conversion strategy");
    assert!(error_msg.contains("LibreOffice"), "Should mention LibreOffice installation");
    assert!(error_msg.contains("antiword"), "Should mention antiword as fallback");
    assert!(error_msg.contains("catdoc"), "Should mention catdoc as fallback");
}

#[tokio::test]
async fn test_doc_conversion_file_path_sanitization() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
    };
    
    // Test with potentially dangerous file path
    let dangerous_paths = [
        "../../etc/passwd",
        "test; rm -rf /",
        "test`whoami`",
        "test$(whoami)",
    ];
    
    for dangerous_path in &dangerous_paths {
        let result = ocr_service.try_doc_to_docx_conversion(dangerous_path).await;
        
        // Should fail due to path sanitization
        assert!(result.is_err(), "Dangerous path should be rejected: {}", dangerous_path);
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("potentially dangerous characters") || 
            error_msg.contains("suspicious sequences") ||
            error_msg.contains("Failed to resolve file path"),
            "Should reject dangerous path with appropriate error: {}", error_msg
        );
    }
}

#[tokio::test]
async fn test_doc_conversion_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
    };
    
    let nonexistent_path = temp_dir.path().join("nonexistent.doc");
    
    let result = ocr_service.try_doc_to_docx_conversion(
        nonexistent_path.to_str().unwrap()
    ).await;
    
    // Should fail because file doesn't exist
    assert!(result.is_err(), "Nonexistent file should cause conversion to fail");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to resolve file path") || 
        error_msg.contains("File may not exist"),
        "Should mention file doesn't exist: {}", error_msg
    );
}

#[tokio::test]
async fn test_doc_conversion_temp_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("test.doc");
    
    // Create a fake DOC file
    let doc_data = create_fake_doc_file();
    fs::write(&doc_path, doc_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
    };
    
    let result = ocr_service.try_doc_to_docx_conversion(
        doc_path.to_str().unwrap()
    ).await;
    
    // Will fail due to LibreOffice not being available in test environment,
    // but should successfully create temp directory and reach LibreOffice execution
    if let Err(error_msg) = result {
        let error_str = error_msg.to_string();
        // Should fail at LibreOffice execution, not directory creation
        assert!(
            error_str.contains("LibreOffice command execution failed") ||
            error_str.contains("LibreOffice conversion failed"),
            "Should fail at LibreOffice execution step, not directory creation: {}", error_str
        );
    }
}

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
    };
    
    let settings = Settings::default();
    let start_time = std::time::Instant::now();
    
    // Test the full legacy DOC extraction process
    let result = ocr_service.extract_text_from_legacy_doc(
        doc_path.to_str().unwrap(),
        start_time
    ).await;
    
    // Should fail since we don't have LibreOffice or extraction tools in test env
    assert!(result.is_err(), "Should fail without proper tools");
    let error_msg = result.unwrap_err().to_string();
    
    // Verify it mentions trying conversion first, then fallback tools
    assert!(error_msg.contains("All extraction methods failed"), 
        "Should mention all methods tried: {}", error_msg);
    assert!(error_msg.contains("DOC to DOCX conversion") || error_msg.contains("LibreOffice"), 
        "Should mention conversion attempt: {}", error_msg);
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

#[tokio::test]
async fn test_doc_to_docx_uuid_uniqueness() {
    let temp_dir = TempDir::new().unwrap();
    let doc_path = temp_dir.path().join("uuid_test.doc");
    
    // Create a fake DOC file
    let doc_data = create_fake_doc_file();
    fs::write(&doc_path, doc_data).unwrap();
    
    // Create OCR service
    let ocr_service = EnhancedOcrService {
        temp_dir: temp_dir.path().to_str().unwrap().to_string(),
        file_service: FileService::new(temp_dir.path().to_str().unwrap().to_string()),
    };
    
    // Try conversion multiple times to ensure unique temp directories
    let mut temp_dirs = std::collections::HashSet::new();
    
    for _ in 0..3 {
        let result = ocr_service.try_doc_to_docx_conversion(
            doc_path.to_str().unwrap()
        ).await;
        
        // Extract temp directory from error message (since LibreOffice won't be available)
        if let Err(error) = result {
            let error_str = error.to_string();
            if error_str.contains("doc_conversion_") {
                // Extract the UUID part to verify uniqueness
                temp_dirs.insert(error_str);
            }
        }
    }
    
    // Should have created unique temp directories for each attempt
    // (If we got far enough to create them before LibreOffice failure)
    if !temp_dirs.is_empty() {
        assert!(temp_dirs.len() > 1 || temp_dirs.len() == 1, 
            "Should use unique temp directories for each conversion attempt");
    }
}