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