/// MIME type detection module for improved file type identification
/// 
/// This module provides functions for detecting file MIME types using multiple methods:
/// 1. Content-based detection using magic bytes (most reliable)
/// 2. Server-provided MIME type (when available and trusted)
/// 3. Extension-based fallback (least reliable, but covers edge cases)
/// 
/// The goal is to provide accurate MIME type detection that's particularly important
/// for OCR processing where incorrectly classified image files can cause issues.

use std::path::Path;
use tracing::{debug, warn};

/// Strategy for MIME type detection
#[derive(Debug, Clone, PartialEq)]
pub enum DetectionStrategy {
    /// Use content-based detection (magic bytes) - most reliable
    ContentBased,
    /// Trust server-provided MIME type if available, fallback to content
    TrustServer,
    /// Use extension-based detection - least reliable but fastest
    ExtensionOnly,
    /// Comprehensive strategy: server -> content -> extension -> fallback
    Comprehensive,
}

/// Result of MIME type detection with metadata about the detection method used
#[derive(Debug, Clone)]
pub struct MimeDetectionResult {
    pub mime_type: String,
    pub confidence: MimeConfidence,
    pub detection_method: DetectionMethod,
    pub original_server_type: Option<String>,
    pub detected_extension: Option<String>,
}

/// Confidence level of the MIME type detection
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MimeConfidence {
    /// Low confidence - extension-based or fallback detection
    Low,
    /// Medium confidence - mime_guess library detection
    Medium,
    /// High confidence - magic byte detection or trusted server
    High,
    /// Very high confidence - content analysis confirms server type
    VeryHigh,
}

/// Method used for MIME type detection
#[derive(Debug, Clone, PartialEq)]
pub enum DetectionMethod {
    /// Detected using magic bytes/file signature
    MagicBytes,
    /// Provided by the server and trusted
    ServerProvided,
    /// Detected using file extension
    Extension,
    /// Fallback to default type
    Fallback,
    /// Hybrid approach using multiple methods
    Hybrid,
}

impl MimeDetectionResult {
    /// Create a result for server-provided MIME type
    pub fn from_server(mime_type: String) -> Self {
        Self {
            mime_type,
            confidence: MimeConfidence::High,
            detection_method: DetectionMethod::ServerProvided,
            original_server_type: None,
            detected_extension: None,
        }
    }

    /// Create a result for content-based detection
    pub fn from_content(mime_type: String, server_type: Option<String>) -> Self {
        Self {
            mime_type,
            confidence: MimeConfidence::High,
            detection_method: DetectionMethod::MagicBytes,
            original_server_type: server_type,
            detected_extension: None,
        }
    }

    /// Create a result for extension-based detection
    pub fn from_extension(mime_type: String, extension: String) -> Self {
        Self {
            mime_type,
            confidence: MimeConfidence::Medium,
            detection_method: DetectionMethod::Extension,
            original_server_type: None,
            detected_extension: Some(extension),
        }
    }

    /// Create a fallback result
    pub fn fallback() -> Self {
        Self {
            mime_type: "application/octet-stream".to_string(),
            confidence: MimeConfidence::Low,
            detection_method: DetectionMethod::Fallback,
            original_server_type: None,
            detected_extension: None,
        }
    }

    /// Check if the detected MIME type indicates an image file
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    /// Check if the detected MIME type indicates a document file
    pub fn is_document(&self) -> bool {
        matches!(self.mime_type.as_str(),
            "application/pdf" |
            "application/msword" |
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" |
            "application/vnd.ms-excel" |
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" |
            "application/vnd.ms-powerpoint" |
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" |
            "text/plain" |
            "text/rtf" |
            "application/rtf"
        )
    }

    /// Check if this MIME type is suitable for OCR processing
    pub fn is_ocr_suitable(&self) -> bool {
        self.is_image() || self.mime_type == "application/pdf"
    }
}

/// Detect MIME type for WebDAV discovery phase (when we only have file metadata)
/// 
/// This function is called during the initial WebDAV XML parsing when we don't
/// have access to the actual file content yet.
/// 
/// # Arguments
/// * `filename` - The filename/path of the file
/// * `server_mime_type` - MIME type provided by the WebDAV server, if any
/// * `strategy` - Detection strategy to use
/// 
/// # Returns
/// A `MimeDetectionResult` with the best available MIME type determination
pub fn detect_mime_for_discovery(
    filename: &str,
    server_mime_type: Option<&str>,
    strategy: DetectionStrategy,
) -> MimeDetectionResult {
    debug!("Detecting MIME type for discovery: filename={}, server_type={:?}, strategy={:?}", 
           filename, server_mime_type, strategy);

    match strategy {
        DetectionStrategy::ContentBased => {
            // During discovery, we can't analyze content, so fall back to extension
            detect_from_extension(filename, server_mime_type)
        }
        DetectionStrategy::TrustServer => {
            if let Some(server_type) = server_mime_type {
                if is_trusted_server_mime_type(server_type) {
                    return MimeDetectionResult::from_server(server_type.to_string());
                }
            }
            // Fallback to extension-based detection
            detect_from_extension(filename, server_mime_type)
        }
        DetectionStrategy::ExtensionOnly => {
            detect_from_extension(filename, server_mime_type)
        }
        DetectionStrategy::Comprehensive => {
            // Use server type if trusted, otherwise extension-based
            if let Some(server_type) = server_mime_type {
                if is_trusted_server_mime_type(server_type) {
                    return MimeDetectionResult::from_server(server_type.to_string());
                }
            }
            detect_from_extension(filename, server_mime_type)
        }
    }
}

/// Detect MIME type when file content is available (during file download/processing)
/// 
/// This provides the most accurate detection using magic bytes from the actual file content.
/// 
/// # Arguments
/// * `content` - The first few bytes of the file content (at least 512 bytes recommended)
/// * `filename` - The filename for fallback detection
/// * `server_mime_type` - MIME type provided by the server, if any
/// 
/// # Returns
/// A `MimeDetectionResult` with high-confidence MIME type detection
pub fn detect_mime_from_content(
    content: &[u8],
    filename: &str,
    server_mime_type: Option<&str>,
) -> MimeDetectionResult {
    debug!("Detecting MIME type from content: filename={}, server_type={:?}, content_len={}", 
           filename, server_mime_type, content.len());

    // First, try magic byte detection
    if let Some(detected_type) = infer::get(content) {
        let mime_type = detected_type.mime_type().to_string();
        debug!("Magic bytes detected MIME type: {}", mime_type);

        // If server provided a type, check for consistency
        if let Some(server_type) = server_mime_type {
            if are_mime_types_compatible(&mime_type, server_type) {
                // Both agree - very high confidence
                let mut result = MimeDetectionResult::from_content(mime_type, Some(server_type.to_string()));
                result.confidence = MimeConfidence::VeryHigh;
                result.detection_method = DetectionMethod::Hybrid;
                return result;
            } else {
                // Content detection overrides server type - trust the bytes
                warn!("MIME type mismatch: server={}, content={} for file {}", 
                      server_type, mime_type, filename);
                return MimeDetectionResult::from_content(mime_type, Some(server_type.to_string()));
            }
        } else {
            // Only content detection available
            return MimeDetectionResult::from_content(mime_type, None);
        }
    }

    // Magic bytes detection failed, fall back to server type if trusted
    if let Some(server_type) = server_mime_type {
        if is_trusted_server_mime_type(server_type) {
            debug!("Using trusted server MIME type: {}", server_type);
            return MimeDetectionResult::from_server(server_type.to_string());
        }
    }

    // Fall back to extension-based detection
    debug!("Content detection failed, falling back to extension detection");
    detect_from_extension(filename, server_mime_type)
}

/// Update an existing MIME type with content-based detection if available
/// 
/// This function is useful for re-detecting MIME types when file content becomes
/// available after initial discovery.
/// 
/// # Arguments
/// * `current_mime_type` - The currently assigned MIME type
/// * `content` - File content for analysis
/// * `filename` - Filename for context
/// 
/// # Returns
/// A new `MimeDetectionResult` if detection improves confidence, or None if no change needed
pub fn update_mime_type_with_content(
    current_mime_type: &str,
    content: &[u8],
    filename: &str,
) -> Option<MimeDetectionResult> {
    let new_result = detect_mime_from_content(content, filename, Some(current_mime_type));
    
    // Only update if we have higher confidence or detected a different type
    if new_result.confidence > MimeConfidence::Medium || 
       new_result.mime_type != current_mime_type {
        Some(new_result)
    } else {
        None
    }
}

/// Detect MIME type from file extension using mime_guess library
fn detect_from_extension(filename: &str, server_mime_type: Option<&str>) -> MimeDetectionResult {
    let path = Path::new(filename);
    
    if let Some(mime_type) = mime_guess::from_path(path).first() {
        let mime_str = mime_type.to_string();
        debug!("Extension-based detection: {} -> {}", filename, mime_str);
        
        let mut result = MimeDetectionResult::from_extension(
            mime_str,
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_string()
        );
        result.original_server_type = server_mime_type.map(|s| s.to_string());
        result
    } else {
        debug!("Extension-based detection failed for: {}", filename);
        let mut result = MimeDetectionResult::fallback();
        result.original_server_type = server_mime_type.map(|s| s.to_string());
        result
    }
}

/// Check if a server-provided MIME type should be trusted
/// 
/// Some servers return generic types like "application/octet-stream" which
/// aren't useful, while others provide accurate information.
fn is_trusted_server_mime_type(mime_type: &str) -> bool {
    !matches!(mime_type,
        "application/octet-stream" |
        "application/binary" |
        "binary/octet-stream" |
        "" |
        "unknown"
    )
}

/// Check if two MIME types are compatible/equivalent
/// 
/// Some servers might return slightly different but equivalent MIME types
/// (e.g., "image/jpg" vs "image/jpeg")
fn are_mime_types_compatible(type1: &str, type2: &str) -> bool {
    if type1 == type2 {
        return true;
    }

    // Handle common variations
    match (type1, type2) {
        ("image/jpeg", "image/jpg") | ("image/jpg", "image/jpeg") => true,
        ("image/tiff", "image/tif") | ("image/tif", "image/tiff") => true,
        ("text/plain", "text/txt") | ("text/txt", "text/plain") => true,
        _ => {
            // Check if they have the same primary type (e.g., both are "image/*")
            let parts1: Vec<&str> = type1.split('/').collect();
            let parts2: Vec<&str> = type2.split('/').collect();
            
            parts1.len() == 2 && parts2.len() == 2 && parts1[0] == parts2[0]
        }
    }
}

/// Legacy function for backward compatibility
/// 
/// This maintains the same interface as the original `get_mime_type_from_extension`
/// function but uses the new detection system.
pub fn get_mime_type_from_extension(extension: &str) -> String {
    let fake_filename = format!("file.{}", extension);
    let result = detect_from_extension(&fake_filename, None);
    result.mime_type
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_detection_from_extension() {
        let result = detect_mime_for_discovery(
            "test.pdf", 
            None, 
            DetectionStrategy::ExtensionOnly
        );
        assert_eq!(result.mime_type, "application/pdf");
        assert_eq!(result.detection_method, DetectionMethod::Extension);
    }

    #[test]
    fn test_server_type_trust() {
        // Trusted server type
        let result = detect_mime_for_discovery(
            "test.pdf",
            Some("application/pdf"),
            DetectionStrategy::TrustServer
        );
        assert_eq!(result.mime_type, "application/pdf");
        assert_eq!(result.detection_method, DetectionMethod::ServerProvided);

        // Untrusted server type should fall back
        let result = detect_mime_for_discovery(
            "test.pdf",
            Some("application/octet-stream"),
            DetectionStrategy::TrustServer
        );
        assert_eq!(result.mime_type, "application/pdf");
        assert_eq!(result.detection_method, DetectionMethod::Extension);
    }

    #[test]
    fn test_mime_type_compatibility() {
        assert!(are_mime_types_compatible("image/jpeg", "image/jpg"));
        assert!(are_mime_types_compatible("image/jpg", "image/jpeg"));
        assert!(are_mime_types_compatible("text/plain", "text/plain"));
        assert!(!are_mime_types_compatible("image/jpeg", "text/plain"));
    }

    #[test]
    fn test_content_based_detection() {
        // PDF magic bytes
        let pdf_header = b"%PDF-1.4";
        let result = detect_mime_from_content(pdf_header, "test.pdf", None);
        assert_eq!(result.mime_type, "application/pdf");
        assert_eq!(result.detection_method, DetectionMethod::MagicBytes);
        assert_eq!(result.confidence, MimeConfidence::High);

        // JPEG magic bytes
        let jpeg_header = [0xFF, 0xD8, 0xFF];
        let result = detect_mime_from_content(&jpeg_header, "test.jpg", None);
        assert_eq!(result.mime_type, "image/jpeg");
    }

    #[test]
    fn test_hybrid_detection() {
        // Content and server agree
        let pdf_header = b"%PDF-1.4";
        let result = detect_mime_from_content(pdf_header, "test.pdf", Some("application/pdf"));
        assert_eq!(result.mime_type, "application/pdf");
        assert_eq!(result.detection_method, DetectionMethod::Hybrid);
        assert_eq!(result.confidence, MimeConfidence::VeryHigh);
    }

    #[test]
    fn test_legacy_compatibility() {
        assert_eq!(get_mime_type_from_extension("pdf"), "application/pdf");
        assert_eq!(get_mime_type_from_extension("jpg"), "image/jpeg");
        assert_eq!(get_mime_type_from_extension("png"), "image/png");
    }

    #[test]
    fn test_ocr_suitability() {
        let pdf_result = MimeDetectionResult::from_content("application/pdf".to_string(), None);
        assert!(pdf_result.is_ocr_suitable());

        let image_result = MimeDetectionResult::from_content("image/jpeg".to_string(), None);
        assert!(image_result.is_ocr_suitable());

        let text_result = MimeDetectionResult::from_content("text/plain".to_string(), None);
        assert!(!text_result.is_ocr_suitable());
    }
}