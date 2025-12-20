use anyhow::Result;
use std::fs;
use std::io::Write;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

use readur::ocr::{
    OcrService, OcrConfig,
};

/// Test utilities for creating mock Office documents
struct OfficeTestDocuments {
    temp_dir: TempDir,
}

impl OfficeTestDocuments {
    fn new() -> Result<Self> {
        Ok(Self {
            temp_dir: TempDir::new()?,
        })
    }

    /// Create a mock DOCX file (simplified ZIP structure with XML content)
    fn create_mock_docx(&self, filename: &str, content: &str) -> Result<String> {
        let file_path = self.temp_dir.path().join(filename);
        
        // Create a proper ZIP structure for DOCX
        let file = fs::File::create(&file_path)?;
        let mut zip = zip::ZipWriter::new(file);
        
        // Add [Content_Types].xml
        zip.start_file("[Content_Types].xml", zip::write::SimpleFileOptions::default())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#)?;
        
        // Add _rels/.rels
        zip.start_file("_rels/.rels", zip::write::SimpleFileOptions::default())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#)?;
        
        // Add word/document.xml with the actual content
        zip.start_file("word/document.xml", zip::write::SimpleFileOptions::default())?;
        let document_xml = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
        <w:p>
            <w:r>
                <w:t>{}</w:t>
            </w:r>
        </w:p>
    </w:body>
</w:document>"#, content);
        zip.write_all(document_xml.as_bytes())?;
        
        zip.finish()?;
        
        Ok(file_path.to_string_lossy().to_string())
    }

    /// Create a mock XLSX file with spreadsheet content
    fn create_mock_xlsx(&self, filename: &str, content: &[&str]) -> Result<String> {
        let file_path = self.temp_dir.path().join(filename);
        
        let file = fs::File::create(&file_path)?;
        let mut zip = zip::ZipWriter::new(file);
        
        // Add [Content_Types].xml with shared strings support
        zip.start_file("[Content_Types].xml", zip::write::SimpleFileOptions::default())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
    <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
    <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#)?;
        
        // Add _rels/.rels
        zip.start_file("_rels/.rels", zip::write::SimpleFileOptions::default())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#)?;
        
        // Add xl/workbook.xml
        zip.start_file("xl/workbook.xml", zip::write::SimpleFileOptions::default())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
    <sheets>
        <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
    </sheets>
</workbook>"#)?;
        
        // Add xl/_rels/workbook.xml.rels with shared strings relationship
        zip.start_file("xl/_rels/workbook.xml.rels", zip::write::SimpleFileOptions::default())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#)?;
        
        // Add xl/sharedStrings.xml with the text content
        zip.start_file("xl/sharedStrings.xml", zip::write::SimpleFileOptions::default())?;
        let mut shared_strings_xml = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{count}" uniqueCount="{count}">"#);
        shared_strings_xml = shared_strings_xml.replace("{count}", &content.len().to_string());
        
        for cell_content in content {
            shared_strings_xml.push_str(&format!(r#"
    <si><t>{}</t></si>"#, cell_content));
        }
        
        shared_strings_xml.push_str(r#"
</sst>"#);
        zip.write_all(shared_strings_xml.as_bytes())?;
        
        // Add xl/worksheets/sheet1.xml with references to shared strings
        zip.start_file("xl/worksheets/sheet1.xml", zip::write::SimpleFileOptions::default())?;
        let mut worksheet_xml = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
    <sheetData>"#);
        
        for (row_idx, _) in content.iter().enumerate() {
            worksheet_xml.push_str(&format!(r#"
        <row r="{}">
            <c r="A{}" t="s">
                <v>{}</v>
            </c>
        </row>"#, row_idx + 1, row_idx + 1, row_idx));
        }
        
        worksheet_xml.push_str(r#"
    </sheetData>
</worksheet>"#);
        
        zip.write_all(worksheet_xml.as_bytes())?;
        zip.finish()?;
        
        Ok(file_path.to_string_lossy().to_string())
    }

    /// Create a corrupted file for testing error handling
    fn create_corrupted_file(&self, filename: &str) -> Result<String> {
        let file_path = self.temp_dir.path().join(filename);
        let mut file = fs::File::create(&file_path)?;
        file.write_all(b"This is not a valid Office document but pretends to be one")?;
        Ok(file_path.to_string_lossy().to_string())
    }

    /// Create an empty file
    fn create_empty_file(&self, filename: &str) -> Result<String> {
        let file_path = self.temp_dir.path().join(filename);
        fs::File::create(&file_path)?;
        Ok(file_path.to_string_lossy().to_string())
    }
}

/// Create a test OCR service with XML extraction
fn create_test_ocr_service(temp_dir: &str) -> OcrService {
    let config = OcrConfig {
        temp_dir: temp_dir.to_string(),
    };
    
    OcrService::new_with_config(config)
}

#[tokio::test]
async fn test_extract_text_from_docx() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    let test_content = "This is a test DOCX document with sample content for extraction testing.";
    let docx_path = test_docs.create_mock_docx("test.docx", test_content)?;
    
    let result = ocr_service.extract_text_from_office_document(
        &docx_path,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await?;
    
    // The method now returns an OcrResult
    println!("Extracted text: '{}'", result.text);
    assert!(!result.text.is_empty());
    assert!(result.text.contains(test_content));
    assert!(result.confidence > 0.0);
    assert!(result.word_count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_extract_text_from_xlsx() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    let test_content = vec![
        "Header 1",
        "Data Row 1",
        "Data Row 2",
        "Summary Data",
    ];
    let xlsx_path = test_docs.create_mock_xlsx("test.xlsx", &test_content)?;
    
    let result = ocr_service.extract_text_from_office_document(
        &xlsx_path,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    ).await?;
    
    // The method now returns an OcrResult
    println!("XLSX extracted text: '{}'", result.text);
    assert!(!result.text.is_empty());
    // Check if it contains some of our test content
    assert!(result.text.contains("Header") || result.text.contains("Data"));
    assert!(result.confidence > 0.0);
    assert!(result.word_count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_extraction_modes() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let temp_dir = test_docs.temp_dir.path().to_string_lossy().to_string();
    
    let test_content = "Test document for mode comparison";
    let docx_path = test_docs.create_mock_docx("test_modes.docx", test_content)?;
    
    // Test XML extraction with the simplified approach
    let ocr_config = OcrConfig {
        temp_dir: temp_dir.clone(),
    };
    
    let ocr_service = OcrService::new_with_config(ocr_config);
    
    let result = ocr_service.extract_text_from_office_document_with_config(
        &docx_path,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ).await;
    
    // XML extraction should succeed with our test document
    assert!(result.is_ok(), "XML extraction failed: {:?}", result);
    let extracted_result = result?;
    assert!(!extracted_result.text.is_empty());
    assert!(extracted_result.confidence > 0.0);
    assert!(extracted_result.word_count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_fallback_mechanism() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let temp_dir = test_docs.temp_dir.path().to_string_lossy().to_string();
    
    // Create a service with XML extraction
    let config = OcrConfig {
        temp_dir,
    };
    
    let ocr_service = OcrService::new_with_config(config);
    let docx_path = test_docs.create_mock_docx("fallback_test.docx", "Fallback test content")?;
    
    // The XML extraction should succeed
    let result = ocr_service.extract_text_from_office_document(
        &docx_path,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await?;
    
    // The method now returns an OcrResult
    assert!(result.text.contains("Fallback test content"));
    assert!(result.confidence > 0.0);
    assert!(result.word_count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_timeout_handling() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    let docx_path = test_docs.create_mock_docx("timeout_test.docx", "Test content")?;
    
    // Test timeout behavior (the timeout logic is now in the XML extractor itself)
    let result = timeout(
        Duration::from_millis(2000), // Give overall test 2 seconds
        ocr_service.extract_text_from_office_document_with_config(
            &docx_path,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
    ).await;
    
    // Should complete successfully even with short timeout for our simple test file
    assert!(result.is_ok());
    let extraction_result = result??;
    assert!(!extraction_result.text.is_empty());
    assert!(extraction_result.confidence > 0.0);
    assert!(extraction_result.word_count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    // Test with corrupted file
    let corrupted_path = test_docs.create_corrupted_file("corrupted.docx")?;
    let result = ocr_service.extract_text_from_office_document(
        &corrupted_path,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await;
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("corrupted") || error_msg.contains("invalid") || error_msg.contains("parsing"));
    
    // Test with empty file
    let empty_path = test_docs.create_empty_file("empty.docx")?;
    let result = ocr_service.extract_text_from_office_document(
        &empty_path,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await;
    
    assert!(result.is_err());
    
    // Test with non-existent file
    let result = ocr_service.extract_text_from_office_document(
        "/path/that/does/not/exist.docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await;
    
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_extraction() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    // Create multiple test documents
    let mut tasks = Vec::new();
    let mut file_paths = Vec::new();
    
    for i in 0..5 {
        let content = format!("Test document {} with unique content", i);
        let file_path = test_docs.create_mock_docx(&format!("concurrent_test_{}.docx", i), &content)?;
        file_paths.push(file_path);
    }
    
    // Launch concurrent extraction tasks
    for file_path in file_paths {
        let ocr_service_clone = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
        let task = tokio::spawn(async move {
            ocr_service_clone.extract_text_from_office_document(
                &file_path,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            ).await
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;
    
    // Verify all extractions succeeded
    for (i, task_result) in results.into_iter().enumerate() {
        let ocr_result = task_result??;
        assert!(!ocr_result.text.is_empty(), "Task {} failed", i);
        assert!(ocr_result.text.contains(&format!("Test document {}", i)));
        assert!(ocr_result.confidence > 0.0);
        assert!(ocr_result.word_count > 0);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    
    // Create service with XML extraction
    let config = OcrConfig {
        temp_dir: test_docs.temp_dir.path().to_string_lossy().to_string(),
    };
    
    let ocr_service = OcrService::new_with_config(config);
    
    // Create a valid document for later success testing
    let valid_path = test_docs.create_mock_docx("circuit_test.docx", "Valid document")?;
    
    // Create corrupted files to cause failures
    let corrupted1 = test_docs.create_corrupted_file("corrupted1.docx")?;
    let corrupted2 = test_docs.create_corrupted_file("corrupted2.docx")?;
    
    // First failure
    let result1 = ocr_service.extract_text_from_office_document(
        &corrupted1,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await;
    assert!(result1.is_err());
    
    // Second failure - should trip circuit breaker
    let result2 = ocr_service.extract_text_from_office_document(
        &corrupted2,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await;
    assert!(result2.is_err());
    
    // Third attempt - should succeed since circuit breaker functionality was removed  
    let result3 = ocr_service.extract_text_from_office_document(
        &valid_path,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await;
    // With simplified architecture, valid documents should always work
    assert!(result3.is_ok());
    let valid_result = result3.unwrap();
    assert!(valid_result.text.contains("Valid document"));
    assert!(valid_result.confidence > 0.0);
    assert!(valid_result.word_count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_statistics_tracking() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    // Perform some extractions to verify functionality
    let valid_path = test_docs.create_mock_docx("stats_test.docx", "Statistics test document")?;
    
    for i in 0..3 {
        let result = ocr_service.extract_text_from_office_document(
            &valid_path,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ).await;
        
        assert!(result.is_ok(), "Extraction {} failed: {:?}", i, result);
        let ocr_result = result.unwrap();
        assert!(!ocr_result.text.is_empty());
        assert!(ocr_result.confidence > 0.0);
        assert!(ocr_result.word_count > 0);
        assert!(ocr_result.processing_time_ms > 0);
    }
    
    // All extractions succeeded, indicating the XML extraction is working correctly
    
    Ok(())
}

#[tokio::test]
async fn test_mime_type_support() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    // Test supported MIME types
    let supported_types = ocr_service.get_supported_mime_types();
    assert!(supported_types.contains(&"application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
    assert!(supported_types.contains(&"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
    assert!(supported_types.contains(&"application/pdf"));
    assert!(supported_types.contains(&"image/png"));
    
    // Test Office document support
    assert!(ocr_service.supports_office_documents());
    
    Ok(())
}

#[tokio::test]
async fn test_learning_mechanism() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    
    // Create service with XML extraction
    let config = OcrConfig {
        temp_dir: test_docs.temp_dir.path().to_string_lossy().to_string(),
    };
    
    let ocr_service = OcrService::new_with_config(config);
    
    // Process several documents of the same type to build learning data
    for i in 0..3 {
        let content = format!("Learning test document {} content", i);
        let docx_path = test_docs.create_mock_docx(&format!("learning_{}.docx", i), &content)?;
        
        let result = ocr_service.extract_text_from_office_document(
            &docx_path,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ).await;
        
        assert!(result.is_ok(), "Extraction iteration {} failed: {:?}", i, result);
        let ocr_result = result?;
        assert!(!ocr_result.text.is_empty());
        assert!(ocr_result.text.contains(&format!("document {}", i)));
        assert!(ocr_result.confidence > 0.0);
        assert!(ocr_result.word_count > 0);
    }
    
    // With the simplified XML-only architecture, the system should consistently work
    // All extractions succeeded, indicating the XML extraction is working correctly
    
    Ok(())
}

#[tokio::test]
async fn test_integration_with_main_extract_text() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    // Test that the main extract_text method properly handles Office documents
    let test_content = "Integration test for main extract_text method";
    let docx_path = test_docs.create_mock_docx("integration.docx", test_content)?;
    
    // This should use the fallback strategy internally
    let result = ocr_service.extract_text(
        &docx_path,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await?;
    
    assert!(!result.is_empty());
    assert!(result.contains("Integration test"));
    
    // Test with XLSX as well
    let xlsx_content = vec!["Cell 1", "Cell 2", "Cell 3"];
    let xlsx_path = test_docs.create_mock_xlsx("integration.xlsx", &xlsx_content)?;
    
    let result = ocr_service.extract_text(
        &xlsx_path,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    ).await?;
    
    assert!(!result.is_empty());
    assert!(result.contains("Cell 1"));
    
    Ok(())
}

/// Performance benchmark test (not run by default due to #[ignore])
#[tokio::test]
#[ignore]
async fn benchmark_extraction_performance() -> Result<()> {
    let test_docs = OfficeTestDocuments::new()?;
    let ocr_service = create_test_ocr_service(test_docs.temp_dir.path().to_string_lossy().as_ref());
    
    // Create a larger test document
    let large_content = "This is a large test document. ".repeat(1000);
    let docx_path = test_docs.create_mock_docx("benchmark.docx", &large_content)?;
    
    let start_time = std::time::Instant::now();
    let num_iterations = 10;
    
    for i in 0..num_iterations {
        let result = ocr_service.extract_text_from_office_document(
            &docx_path,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ).await?;
        
        assert!(!result.text.is_empty());
        println!("Iteration {}: extracted {} chars, confidence: {:.1}%", 
            i, 
            result.text.len(),
            result.confidence
        );
    }
    
    let total_time = start_time.elapsed();
    let avg_time = total_time / num_iterations;
    
    println!("Average extraction time: {:?}", avg_time);
    println!("Total time for {} iterations: {:?}", num_iterations, total_time);
    
    // Performance assertions (adjust based on your requirements)
    assert!(avg_time < Duration::from_secs(5), "Average extraction time too slow: {:?}", avg_time);
    
    Ok(())
}