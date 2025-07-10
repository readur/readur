use anyhow::Result;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Extract metadata from file content based on file type
pub async fn extract_content_metadata(file_data: &[u8], mime_type: &str, filename: &str) -> Result<Option<Value>> {
    let mut metadata = Map::new();
    
    match mime_type {
        // Image files - extract basic image info
        mime if mime.starts_with("image/") => {
            if let Ok(img_metadata) = extract_image_metadata(file_data).await {
                metadata.extend(img_metadata);
            }
        }
        
        // PDF files - extract basic PDF info
        "application/pdf" => {
            if let Ok(pdf_metadata) = extract_pdf_metadata(file_data).await {
                metadata.extend(pdf_metadata);
            }
        }
        
        // Text files - extract basic text info
        "text/plain" => {
            if let Ok(text_metadata) = extract_text_metadata(file_data).await {
                metadata.extend(text_metadata);
            }
        }
        
        _ => {
            // For other file types, add basic file information
            metadata.insert("file_type".to_string(), Value::String(mime_type.to_string()));
        }
    }
    
    // Add filename-based metadata
    if let Some(extension) = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str()) 
    {
        metadata.insert("file_extension".to_string(), Value::String(extension.to_lowercase()));
    }
    
    if metadata.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(metadata)))
    }
}

/// Extract metadata from image files
async fn extract_image_metadata(file_data: &[u8]) -> Result<Map<String, Value>> {
    let mut metadata = Map::new();
    
    // Try to load image and get basic properties
    if let Ok(img) = image::load_from_memory(file_data) {
        metadata.insert("image_width".to_string(), Value::Number(img.width().into()));
        metadata.insert("image_height".to_string(), Value::Number(img.height().into()));
        metadata.insert("image_format".to_string(), Value::String(format!("{:?}", img.color())));
        
        // Calculate aspect ratio
        let aspect_ratio = img.width() as f64 / img.height() as f64;
        metadata.insert("aspect_ratio".to_string(), Value::String(format!("{:.2}", aspect_ratio)));
        
        // Determine orientation
        let orientation = if img.width() > img.height() {
            "landscape"
        } else if img.height() > img.width() {
            "portrait"
        } else {
            "square"
        };
        metadata.insert("orientation".to_string(), Value::String(orientation.to_string()));
        
        // Calculate megapixels
        let megapixels = (img.width() as f64 * img.height() as f64) / 1_000_000.0;
        metadata.insert("megapixels".to_string(), Value::String(format!("{:.1} MP", megapixels)));
    }
    
    Ok(metadata)
}

/// Extract metadata from PDF files
async fn extract_pdf_metadata(file_data: &[u8]) -> Result<Map<String, Value>> {
    let mut metadata = Map::new();
    
    // Basic PDF detection and info
    if file_data.len() >= 5 && &file_data[0..4] == b"%PDF" {
        // Extract PDF version from header
        if let Some(version_end) = file_data[0..20].iter().position(|&b| b == b'\n' || b == b'\r') {
            if let Ok(header) = std::str::from_utf8(&file_data[0..version_end]) {
                if let Some(version) = header.strip_prefix("%PDF-") {
                    metadata.insert("pdf_version".to_string(), Value::String(version.to_string()));
                }
            }
        }
        
        // Try to count pages by counting "Type /Page" entries
        let content = String::from_utf8_lossy(file_data);
        let page_count = content.matches("/Type /Page").count();
        if page_count > 0 {
            metadata.insert("page_count".to_string(), Value::Number(page_count.into()));
        }
        
        // Look for basic PDF info
        if content.contains("/Linearized") {
            metadata.insert("linearized".to_string(), Value::Bool(true));
        }
        
        // Check for encryption
        if content.contains("/Encrypt") {
            metadata.insert("encrypted".to_string(), Value::Bool(true));
        }
        
        // Try to find creation/modification dates in metadata
        if let Some(creation_start) = content.find("/CreationDate") {
            if let Some(date_start) = content[creation_start..].find('(') {
                if let Some(date_end) = content[creation_start + date_start..].find(')') {
                    let date_str = &content[creation_start + date_start + 1..creation_start + date_start + date_end];
                    metadata.insert("pdf_creation_date".to_string(), Value::String(date_str.to_string()));
                }
            }
        }
        
        // Basic content analysis
        if content.contains("/Font") {
            metadata.insert("contains_fonts".to_string(), Value::Bool(true));
        }
        
        if content.contains("/Image") || content.contains("/XObject") {
            metadata.insert("contains_images".to_string(), Value::Bool(true));
        }
    }
    
    Ok(metadata)
}

/// Extract metadata from text files
async fn extract_text_metadata(file_data: &[u8]) -> Result<Map<String, Value>> {
    let mut metadata = Map::new();
    
    if let Ok(text) = std::str::from_utf8(file_data) {
        // Basic text statistics
        let char_count = text.chars().count();
        let word_count = text.split_whitespace().count();
        let line_count = text.lines().count();
        
        metadata.insert("character_count".to_string(), Value::Number(char_count.into()));
        metadata.insert("word_count".to_string(), Value::Number(word_count.into()));
        metadata.insert("line_count".to_string(), Value::Number(line_count.into()));
        
        // Detect text encoding characteristics
        if text.chars().any(|c| !c.is_ascii()) {
            metadata.insert("contains_unicode".to_string(), Value::Bool(true));
        }
        
        // Check for common file formats within text
        if text.trim_start().starts_with("<?xml") {
            metadata.insert("text_format".to_string(), Value::String("xml".to_string()));
        } else if text.trim_start().starts_with('{') || text.trim_start().starts_with('[') {
            metadata.insert("text_format".to_string(), Value::String("json".to_string()));
        } else if text.contains("<!DOCTYPE html") || text.contains("<html") {
            metadata.insert("text_format".to_string(), Value::String("html".to_string()));
        }
        
        // Basic language detection (very simple)
        let english_words = ["the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by"];
        let english_count = english_words.iter()
            .map(|&word| text.to_lowercase().matches(word).count())
            .sum::<usize>();
        
        if english_count > word_count / 20 {  // If more than 5% are common English words
            metadata.insert("likely_language".to_string(), Value::String("english".to_string()));
        }
    }
    
    Ok(metadata)
}