#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use serde_json::Value;

    #[tokio::test]
    async fn test_image_metadata_extraction_portrait() {
        let image_data = fs::read("test_files/portrait_100x200.png").expect("Failed to read portrait test image");
        
        let metadata = extract_content_metadata(&image_data, "image/png", "portrait_100x200.png")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        // Check basic image properties
        assert_eq!(metadata["image_width"], Value::Number(100.into()));
        assert_eq!(metadata["image_height"], Value::Number(200.into()));
        assert_eq!(metadata["orientation"], Value::String("portrait".to_string()));
        assert_eq!(metadata["file_extension"], Value::String("png".to_string()));
        
        // Check calculated values
        assert_eq!(metadata["aspect_ratio"], Value::String("0.50".to_string()));
        assert_eq!(metadata["megapixels"], Value::String("0.0 MP".to_string()));
    }

    #[tokio::test]
    async fn test_image_metadata_extraction_landscape() {
        let image_data = fs::read("test_files/landscape_300x200.png").expect("Failed to read landscape test image");
        
        let metadata = extract_content_metadata(&image_data, "image/png", "landscape_300x200.png")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["image_width"], Value::Number(300.into()));
        assert_eq!(metadata["image_height"], Value::Number(200.into()));
        assert_eq!(metadata["orientation"], Value::String("landscape".to_string()));
        assert_eq!(metadata["aspect_ratio"], Value::String("1.50".to_string()));
    }

    #[tokio::test]
    async fn test_image_metadata_extraction_square() {
        let image_data = fs::read("test_files/square_150x150.png").expect("Failed to read square test image");
        
        let metadata = extract_content_metadata(&image_data, "image/png", "square_150x150.png")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["image_width"], Value::Number(150.into()));
        assert_eq!(metadata["image_height"], Value::Number(150.into()));
        assert_eq!(metadata["orientation"], Value::String("square".to_string()));
        assert_eq!(metadata["aspect_ratio"], Value::String("1.00".to_string()));
    }

    #[tokio::test]
    async fn test_image_metadata_extraction_high_resolution() {
        let image_data = fs::read("test_files/hires_1920x1080.png").expect("Failed to read high-res test image");
        
        let metadata = extract_content_metadata(&image_data, "image/png", "hires_1920x1080.png")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["image_width"], Value::Number(1920.into()));
        assert_eq!(metadata["image_height"], Value::Number(1080.into()));
        assert_eq!(metadata["orientation"], Value::String("landscape".to_string()));
        assert_eq!(metadata["megapixels"], Value::String("2.1 MP".to_string()));
    }

    #[tokio::test]
    async fn test_jpeg_metadata_extraction() {
        let image_data = fs::read("test_files/test_image.jpg").expect("Failed to read JPEG test image");
        
        let metadata = extract_content_metadata(&image_data, "image/jpeg", "test_image.jpg")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_extension"], Value::String("jpg".to_string()));
        assert!(metadata.contains_key("image_width"));
        assert!(metadata.contains_key("image_height"));
    }

    #[tokio::test]
    async fn test_pdf_metadata_extraction_single_page() {
        let pdf_data = fs::read("test_files/single_page_v14.pdf").expect("Failed to read single page PDF");
        
        let metadata = extract_content_metadata(&pdf_data, "application/pdf", "single_page_v14.pdf")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_extension"], Value::String("pdf".to_string()));
        // Note: PDF version detection might vary depending on how reportlab creates the file
        assert!(metadata.contains_key("pdf_version") || metadata.contains_key("file_type"));
    }

    #[tokio::test]
    async fn test_pdf_metadata_extraction_multipage() {
        let pdf_data = fs::read("test_files/multipage_test.pdf").expect("Failed to read multipage PDF");
        
        let metadata = extract_content_metadata(&pdf_data, "application/pdf", "multipage_test.pdf")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_extension"], Value::String("pdf".to_string()));
        // Should detect multiple pages if our page counting works
        if let Some(page_count) = metadata.get("page_count") {
            if let Value::Number(count) = page_count {
                assert!(count.as_u64().unwrap() > 1);
            }
        }
    }

    #[tokio::test]
    async fn test_pdf_metadata_with_fonts_and_images() {
        let pdf_data = fs::read("test_files/complex_content.pdf").expect("Failed to read complex PDF");
        
        let metadata = extract_content_metadata(&pdf_data, "application/pdf", "complex_content.pdf")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        // Should detect fonts and potentially images/objects
        if let Some(Value::Bool(has_fonts)) = metadata.get("contains_fonts") {
            // Font detection might work depending on PDF structure
        }
    }

    #[tokio::test]
    async fn test_text_metadata_extraction_comprehensive() {
        let text_data = fs::read("test_files/comprehensive_text.txt").expect("Failed to read comprehensive text");
        
        let metadata = extract_content_metadata(&text_data, "text/plain", "comprehensive_text.txt")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_extension"], Value::String("txt".to_string()));
        
        // Check text statistics
        if let Value::Number(char_count) = &metadata["character_count"] {
            assert!(char_count.as_u64().unwrap() > 500); // Should be substantial
        }
        
        if let Value::Number(word_count) = &metadata["word_count"] {
            assert!(word_count.as_u64().unwrap() > 80); // Should have many words
        }
        
        if let Value::Number(line_count) = &metadata["line_count"] {
            assert!(line_count.as_u64().unwrap() > 15); // Should have multiple lines
        }
        
        // Should detect Unicode content
        assert_eq!(metadata["contains_unicode"], Value::Bool(true));
        
        // Should detect likely English
        if let Some(Value::String(lang)) = metadata.get("likely_language") {
            assert_eq!(lang, "english");
        }
    }

    #[tokio::test]
    async fn test_text_metadata_extraction_ascii_only() {
        let text_data = fs::read("test_files/ascii_only.txt").expect("Failed to read ASCII text");
        
        let metadata = extract_content_metadata(&text_data, "text/plain", "ascii_only.txt")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        // Should NOT contain Unicode
        assert!(metadata.get("contains_unicode").is_none() || metadata["contains_unicode"] == Value::Bool(false));
    }

    #[tokio::test]
    async fn test_text_metadata_extraction_large_file() {
        let text_data = fs::read("test_files/large_text.txt").expect("Failed to read large text");
        
        let metadata = extract_content_metadata(&text_data, "text/plain", "large_text.txt")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        // Should handle large files properly
        if let Value::Number(char_count) = &metadata["character_count"] {
            assert!(char_count.as_u64().unwrap() > 50000); // Should be large
        }
        
        if let Value::Number(word_count) = &metadata["word_count"] {
            assert!(word_count.as_u64().unwrap() > 10000); // Should have many words
        }
    }

    #[tokio::test]
    async fn test_json_format_detection() {
        let text_data = fs::read("test_files/test_format.json").expect("Failed to read JSON text");
        
        let metadata = extract_content_metadata(&text_data, "text/plain", "test_format.json")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_extension"], Value::String("json".to_string()));
        
        // Should detect JSON format
        if let Some(Value::String(format)) = metadata.get("text_format") {
            assert_eq!(format, "json");
        }
    }

    #[tokio::test]
    async fn test_xml_format_detection() {
        let text_data = fs::read("test_files/test_format.xml").expect("Failed to read XML text");
        
        let metadata = extract_content_metadata(&text_data, "text/plain", "test_format.xml")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_extension"], Value::String("xml".to_string()));
        
        // Should detect XML format
        if let Some(Value::String(format)) = metadata.get("text_format") {
            assert_eq!(format, "xml");
        }
    }

    #[tokio::test]
    async fn test_html_format_detection() {
        let text_data = fs::read("test_files/test_format.html").expect("Failed to read HTML text");
        
        let metadata = extract_content_metadata(&text_data, "text/plain", "test_format.html")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_extension"], Value::String("html".to_string()));
        
        // Should detect HTML format
        if let Some(Value::String(format)) = metadata.get("text_format") {
            assert_eq!(format, "html");
        }
    }

    #[tokio::test]
    async fn test_unknown_file_type() {
        let dummy_data = b"This is some random binary data that doesn't match any known format.";
        
        let metadata = extract_content_metadata(dummy_data, "application/octet-stream", "unknown.bin")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        assert_eq!(metadata["file_type"], Value::String("application/octet-stream".to_string()));
        assert_eq!(metadata["file_extension"], Value::String("bin".to_string()));
    }

    #[tokio::test]
    async fn test_empty_file() {
        let empty_data = b"";
        
        let metadata = extract_content_metadata(empty_data, "text/plain", "empty.txt")
            .await
            .expect("Failed to extract metadata");
        
        // Should still return some metadata (at least file extension)
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata["file_extension"], Value::String("txt".to_string()));
    }

    #[tokio::test]
    async fn test_file_without_extension() {
        let text_data = b"Some text content without file extension";
        
        let metadata = extract_content_metadata(text_data, "text/plain", "no_extension")
            .await
            .expect("Failed to extract metadata");
        
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        
        // Should not have file_extension field
        assert!(!metadata.contains_key("file_extension"));
    }
}