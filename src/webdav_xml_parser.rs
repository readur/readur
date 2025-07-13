use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::str;
use serde_json;

use crate::models::FileIngestionInfo;

#[derive(Debug, Default)]
struct PropFindResponse {
    href: String,
    displayname: String,
    content_length: Option<i64>,
    last_modified: Option<String>,
    content_type: Option<String>,
    etag: Option<String>,
    is_collection: bool,
    creation_date: Option<String>,
    owner: Option<String>,
    group: Option<String>,
    permissions: Option<String>,
    owner_display_name: Option<String>,
    metadata: Option<serde_json::Value>,
}

pub fn parse_propfind_response(xml_text: &str) -> Result<Vec<FileIngestionInfo>> {
    let mut reader = Reader::from_str(xml_text);
    reader.config_mut().trim_text(true);
    
    let mut files = Vec::new();
    let mut current_response: Option<PropFindResponse> = None;
    let mut current_element = String::new();
    let mut in_response = false;
    let mut in_propstat = false;
    let mut in_prop = false;
    let mut in_resourcetype = false;
    let mut status_ok = false;
    
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = get_local_name(&e)?;
                
                match name.as_str() {
                    "response" => {
                        in_response = true;
                        current_response = Some(PropFindResponse::default());
                    }
                    "propstat" => {
                        in_propstat = true;
                    }
                    "prop" => {
                        in_prop = true;
                    }
                    "resourcetype" => {
                        in_resourcetype = true;
                    }
                    "collection" if in_resourcetype => {
                        if let Some(ref mut resp) = current_response {
                            resp.is_collection = true;
                        }
                    }
                    _ => {
                        current_element = name;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape()?.to_string();
                
                if in_response && !text.trim().is_empty() {
                    if let Some(ref mut resp) = current_response {
                        match current_element.as_str() {
                            "href" => {
                                resp.href = text.trim().to_string();
                            }
                            "displayname" => {
                                resp.displayname = text.trim().to_string();
                            }
                            "getcontentlength" => {
                                resp.content_length = text.trim().parse().ok();
                            }
                            "getlastmodified" => {
                                resp.last_modified = Some(text.trim().to_string());
                            }
                            "getcontenttype" => {
                                resp.content_type = Some(text.trim().to_string());
                            }
                            "getetag" => {
                                resp.etag = Some(normalize_etag(&text));
                            }
                            "creationdate" => {
                                resp.creation_date = Some(text.trim().to_string());
                            }
                            "owner" => {
                                resp.owner = Some(text.trim().to_string());
                            }
                            "group" => {
                                resp.group = Some(text.trim().to_string());
                            }
                            "status" if in_propstat => {
                                // Check if status is 200 OK
                                if text.contains("200") {
                                    status_ok = true;
                                }
                            }
                            _ => {
                                // Store any other properties as generic metadata
                                // This handles vendor-specific properties from any WebDAV server
                                if !text.trim().is_empty() && in_prop {
                                    if resp.metadata.is_none() {
                                        resp.metadata = Some(serde_json::Value::Object(serde_json::Map::new()));
                                    }
                                    
                                    if let Some(serde_json::Value::Object(ref mut map)) = resp.metadata {
                                        // Special handling for known properties
                                        match current_element.as_str() {
                                            "permissions" | "oc:permissions" => {
                                                resp.permissions = Some(text.trim().to_string());
                                                map.insert("permissions_raw".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "fileid" | "oc:fileid" => {
                                                map.insert("file_id".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "owner-id" | "oc:owner-id" => {
                                                map.insert("owner_id".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "owner-display-name" | "oc:owner-display-name" => {
                                                resp.owner_display_name = Some(text.trim().to_string());
                                                map.insert("owner_display_name".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "has-preview" | "nc:has-preview" => {
                                                if let Ok(val) = text.trim().parse::<bool>() {
                                                    map.insert("has_preview".to_string(), serde_json::Value::Bool(val));
                                                }
                                            }
                                            _ => {
                                                // Store any other property as-is
                                                map.insert(current_element.clone(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = get_local_name_from_end(&e)?;
                
                match name.as_str() {
                    "response" => {
                        if let Some(resp) = current_response.take() {
                            // Only add files (not directories) with valid properties
                            if !resp.is_collection && status_ok && !resp.href.is_empty() {
                                // Extract filename from href
                                let name = if resp.displayname.is_empty() {
                                    resp.href
                                        .split('/')
                                        .last()
                                        .unwrap_or("")
                                        .to_string()
                                } else {
                                    resp.displayname.clone()
                                };
                                
                                // Decode URL-encoded characters
                                let name = urlencoding::decode(&name)
                                    .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&name))
                                    .to_string();
                                
                                // Parse creation date
                                let created_at = resp.creation_date
                                    .as_ref()
                                    .and_then(|d| parse_http_date(d));
                                
                                // Parse permissions (Nextcloud/ownCloud format)
                                let permissions_int = resp.permissions
                                    .as_ref()
                                    .and_then(|p| {
                                        // Nextcloud permissions are a string like "RGDNVW"
                                        // Convert to Unix-style octal permissions
                                        if p.chars().all(|c| c.is_uppercase()) {
                                            // This is Nextcloud format
                                            let mut perms = 0u32;
                                            if p.contains('R') { perms |= 0o444; } // Read
                                            if p.contains('W') { perms |= 0o222; } // Write
                                            if p.contains('D') { perms |= 0o111; } // Delete (execute-like)
                                            Some(perms)
                                        } else {
                                            // Try to parse as numeric
                                            p.parse().ok()
                                        }
                                    });
                                
                                // Use the metadata collected during parsing
                                let metadata = resp.metadata;
                                
                                let file_info = FileIngestionInfo {
                                    path: resp.href.clone(),
                                    name,
                                    size: resp.content_length.unwrap_or(0),
                                    mime_type: resp.content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
                                    last_modified: parse_http_date(&resp.last_modified.unwrap_or_default()),
                                    etag: resp.etag.unwrap_or_else(|| format!("\"{}\"", uuid::Uuid::new_v4())),
                                    is_directory: false,
                                    created_at,
                                    permissions: permissions_int,
                                    owner: resp.owner.or(resp.owner_display_name),
                                    group: resp.group,
                                    metadata,
                                };
                                
                                files.push(file_info);
                            }
                        }
                        in_response = false;
                        status_ok = false;
                    }
                    "propstat" => {
                        in_propstat = false;
                    }
                    "prop" => {
                        in_prop = false;
                    }
                    "resourcetype" => {
                        in_resourcetype = false;
                    }
                    _ => {}
                }
                
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow!("XML parsing error: {}", e)),
            _ => {}
        }
        
        buf.clear();
    }
    
    Ok(files)
}

/// Parse PROPFIND response including both files and directories
/// This is used for shallow directory scans where we need to track directory structure
pub fn parse_propfind_response_with_directories(xml_text: &str) -> Result<Vec<FileIngestionInfo>> {
    let mut reader = Reader::from_str(xml_text);
    reader.config_mut().trim_text(true);
    
    let mut files = Vec::new();
    let mut current_response: Option<PropFindResponse> = None;
    let mut current_element = String::new();
    let mut in_response = false;
    let mut in_propstat = false;
    let mut in_prop = false;
    let mut in_resourcetype = false;
    let mut status_ok = false;
    
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = get_local_name(&e)?;
                
                match name.as_str() {
                    "response" => {
                        in_response = true;
                        current_response = Some(PropFindResponse::default());
                    }
                    "propstat" => {
                        in_propstat = true;
                    }
                    "prop" => {
                        in_prop = true;
                    }
                    "resourcetype" => {
                        in_resourcetype = true;
                    }
                    "collection" if in_resourcetype => {
                        if let Some(ref mut resp) = current_response {
                            resp.is_collection = true;
                        }
                    }
                    _ => {
                        current_element = name;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape()?.to_string();
                
                if in_response && !text.trim().is_empty() {
                    if let Some(ref mut resp) = current_response {
                        match current_element.as_str() {
                            "href" => {
                                resp.href = text.trim().to_string();
                            }
                            "displayname" => {
                                resp.displayname = text.trim().to_string();
                            }
                            "getcontentlength" => {
                                resp.content_length = text.trim().parse().ok();
                            }
                            "getlastmodified" => {
                                resp.last_modified = Some(text.trim().to_string());
                            }
                            "getcontenttype" => {
                                resp.content_type = Some(text.trim().to_string());
                            }
                            "getetag" => {
                                resp.etag = Some(normalize_etag(&text));
                            }
                            "creationdate" => {
                                resp.creation_date = Some(text.trim().to_string());
                            }
                            "owner" => {
                                resp.owner = Some(text.trim().to_string());
                            }
                            "group" => {
                                resp.group = Some(text.trim().to_string());
                            }
                            "status" if in_propstat => {
                                // Check if status is 200 OK
                                if text.contains("200") {
                                    status_ok = true;
                                }
                            }
                            _ => {
                                // Store any other properties as generic metadata
                                if !text.trim().is_empty() && in_prop {
                                    if resp.metadata.is_none() {
                                        resp.metadata = Some(serde_json::Value::Object(serde_json::Map::new()));
                                    }
                                    
                                    if let Some(serde_json::Value::Object(ref mut map)) = resp.metadata {
                                        match current_element.as_str() {
                                            "permissions" | "oc:permissions" => {
                                                resp.permissions = Some(text.trim().to_string());
                                                map.insert("permissions_raw".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "fileid" | "oc:fileid" => {
                                                map.insert("file_id".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "owner-id" | "oc:owner-id" => {
                                                map.insert("owner_id".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "owner-display-name" | "oc:owner-display-name" => {
                                                resp.owner_display_name = Some(text.trim().to_string());
                                                map.insert("owner_display_name".to_string(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                            "has-preview" | "nc:has-preview" => {
                                                if let Ok(val) = text.trim().parse::<bool>() {
                                                    map.insert("has_preview".to_string(), serde_json::Value::Bool(val));
                                                }
                                            }
                                            _ => {
                                                map.insert(current_element.clone(), serde_json::Value::String(text.trim().to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = get_local_name_from_end(&e)?;
                
                match name.as_str() {
                    "response" => {
                        if let Some(resp) = current_response.take() {
                            // Include both files AND directories with valid properties
                            if status_ok && !resp.href.is_empty() {
                                // Extract name from href
                                let name = if resp.displayname.is_empty() {
                                    resp.href
                                        .split('/')
                                        .filter(|s| !s.is_empty())
                                        .last()
                                        .unwrap_or("")
                                        .to_string()
                                } else {
                                    resp.displayname.clone()
                                };
                                
                                // Decode URL-encoded characters
                                let name = urlencoding::decode(&name)
                                    .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&name))
                                    .to_string();
                                
                                // Parse creation date
                                let created_at = resp.creation_date
                                    .as_ref()
                                    .and_then(|d| parse_http_date(d));
                                
                                // Parse permissions
                                let permissions_int = resp.permissions
                                    .as_ref()
                                    .and_then(|p| {
                                        if p.chars().all(|c| c.is_uppercase()) {
                                            let mut perms = 0u32;
                                            if p.contains('R') { perms |= 0o444; }
                                            if p.contains('W') { perms |= 0o222; }
                                            if p.contains('D') { perms |= 0o111; }
                                            Some(perms)
                                        } else {
                                            p.parse().ok()
                                        }
                                    });
                                
                                let file_info = FileIngestionInfo {
                                    path: resp.href.clone(),
                                    name,
                                    size: resp.content_length.unwrap_or(0),
                                    mime_type: if resp.is_collection {
                                        "".to_string()
                                    } else {
                                        resp.content_type.unwrap_or_else(|| "application/octet-stream".to_string())
                                    },
                                    last_modified: parse_http_date(&resp.last_modified.unwrap_or_default()),
                                    etag: resp.etag.unwrap_or_else(|| format!("\"{}\"", uuid::Uuid::new_v4())),
                                    is_directory: resp.is_collection,
                                    created_at,
                                    permissions: permissions_int,
                                    owner: resp.owner.or(resp.owner_display_name),
                                    group: resp.group,
                                    metadata: resp.metadata,
                                };
                                
                                files.push(file_info);
                            }
                        }
                        in_response = false;
                        status_ok = false;
                    }
                    "propstat" => {
                        in_propstat = false;
                    }
                    "prop" => {
                        in_prop = false;
                    }
                    "resourcetype" => {
                        in_resourcetype = false;
                    }
                    _ => {}
                }
                
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow!("XML parsing error: {}", e)),
            _ => {}
        }
        
        buf.clear();
    }
    
    Ok(files)
}

fn get_local_name(e: &BytesStart) -> Result<String> {
    let qname = e.name();
    let local = qname.local_name();
    let name = str::from_utf8(local.as_ref())
        .map_err(|e| anyhow!("Invalid UTF-8 in element name: {}", e))?;
    Ok(name.to_string())
}

fn get_local_name_from_end(e: &quick_xml::events::BytesEnd) -> Result<String> {
    let qname = e.name();
    let local = qname.local_name();
    let name = str::from_utf8(local.as_ref())
        .map_err(|e| anyhow!("Invalid UTF-8 in element name: {}", e))?;
    Ok(name.to_string())
}

fn parse_http_date(date_str: &str) -> Option<DateTime<Utc>> {
    if date_str.is_empty() {
        return None;
    }
    
    // Try to parse RFC 2822 format (used by WebDAV)
    DateTime::parse_from_rfc2822(date_str)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|| {
            // Try RFC 3339 as fallback
            DateTime::parse_from_rfc3339(date_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        })
        .or_else(|| {
            // Try a custom format as last resort
            chrono::NaiveDateTime::parse_from_str(date_str, "%a, %d %b %Y %H:%M:%S GMT")
                .ok()
                .map(|ndt| DateTime::from_naive_utc_and_offset(ndt, Utc))
        })
}

/// Normalize ETag by removing quotes and weak ETag prefix
/// This ensures consistent ETag comparison across different WebDAV servers
/// 
/// Examples:
/// - `"abc123"` → `abc123`
/// - `W/"abc123"` → `abc123`
/// - `abc123` → `abc123`
/// Comprehensive ETag parser that handles all the weird edge cases found in real WebDAV servers
pub fn normalize_etag(etag: &str) -> String {
    let mut result = etag.trim().to_string();
    
    // Handle multiple weak indicators (malformed but seen in the wild)
    while result.starts_with("W/") || result.starts_with("w/") {
        if result.starts_with("W/") {
            result = result[2..].trim().to_string();
        } else if result.starts_with("w/") {
            result = result[2..].trim().to_string();
        }
    }
    
    // Handle quoted ETags - be careful with escaped quotes
    if result.starts_with('"') && result.ends_with('"') && result.len() > 1 {
        result = result[1..result.len()-1].to_string();
    }
    
    // Handle some edge cases where quotes might be escaped inside
    // This handles cases like: "etag-with-\"internal\"-quotes"
    if result.contains("\\\"") {
        // For display purposes, we keep the escaped quotes as-is
        // The server will handle the proper interpretation
    }
    
    // Handle empty ETags or whitespace-only ETags
    if result.trim().is_empty() {
        return "".to_string(); // Return empty string for empty ETags
    }
    
    result
}

/// Advanced ETag parser with detailed information about the ETag format
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedETag {
    pub original: String,
    pub normalized: String,
    pub is_weak: bool,
    pub format_type: ETagFormat,
    pub has_internal_quotes: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ETagFormat {
    Simple,           // "abc123"
    Weak,            // W/"abc123"
    Hash,            // MD5/SHA1/SHA256 hashes
    UUID,            // UUID format
    Timestamp,       // Contains timestamp
    Versioned,       // Version information
    Encoded,         // Base64 or URL encoded
    Complex,         // Microsoft/SharePoint complex formats
    PathBased,       // Contains path information
    JSONLike,        // Contains JSON-like data
    XMLLike,         // Contains XML-like data
    Unknown,         // Unrecognized format
}

impl ParsedETag {
    pub fn parse(etag: &str) -> Self {
        let original = etag.to_string();
        let normalized = normalize_etag(etag);
        
        // Detect if it's a weak ETag
        let is_weak = etag.trim().starts_with("W/") || etag.trim().starts_with("w/");
        
        // Detect internal quotes
        let has_internal_quotes = normalized.contains('"') || normalized.contains("\\'");
        
        // Classify the ETag format
        let format_type = classify_etag_format(&normalized);
        
        ParsedETag {
            original,
            normalized,
            is_weak,
            format_type,
            has_internal_quotes,
        }
    }
    
    /// Check if two ETags are equivalent (ignoring weak/strong differences)
    pub fn is_equivalent(&self, other: &ParsedETag) -> bool {
        self.normalized == other.normalized
    }
    
    /// Get a safe string for comparison that handles edge cases
    pub fn comparison_string(&self) -> String {
        // For comparison, we normalize further by removing internal quotes and whitespace
        self.normalized
            .replace("\\\"", "")
            .replace('"', "")
            .trim()
            .to_string()
    }
}

fn classify_etag_format(etag: &str) -> ETagFormat {
    let lower = etag.to_lowercase();
    
    // Check for UUIDs (with or without dashes/braces)
    if is_uuid_like(etag) {
        return ETagFormat::UUID;
    }
    
    // Check for hash formats (MD5, SHA1, SHA256)
    if is_hash_like(etag) {
        return ETagFormat::Hash;
    }
    
    // Check for timestamp formats
    if contains_timestamp(etag) {
        return ETagFormat::Timestamp;
    }
    
    // Check for version information
    if contains_version_info(etag) {
        return ETagFormat::Versioned;
    }
    
    // Check for encoding indicators
    if is_encoded_format(etag) {
        return ETagFormat::Encoded;
    }
    
    // Check for Microsoft/SharePoint formats
    if is_microsoft_format(etag) {
        return ETagFormat::Complex;
    }
    
    // Check for path-like ETags
    if contains_path_info(etag) {
        return ETagFormat::PathBased;
    }
    
    // Check for JSON-like content
    if etag.contains('{') && etag.contains('}') {
        return ETagFormat::JSONLike;
    }
    
    // Check for XML-like content
    if etag.contains('<') && etag.contains('>') {
        return ETagFormat::XMLLike;
    }
    
    // Simple format for everything else
    if etag.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        ETagFormat::Simple
    } else {
        ETagFormat::Unknown
    }
}

fn is_uuid_like(s: &str) -> bool {
    // UUID patterns: 8-4-4-4-12 hex digits
    let uuid_regex = regex::Regex::new(r"^[0-9a-fA-F]{8}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{12}$").unwrap();
    uuid_regex.is_match(s) || s.contains("GUID") || (s.starts_with('{') && s.ends_with('}') && s.len() > 30)
}

fn is_hash_like(s: &str) -> bool {
    // MD5 (32 hex), SHA1 (40 hex), SHA256 (64 hex)
    let hex_only = s.chars().all(|c| c.is_ascii_hexdigit());
    hex_only && (s.len() == 32 || s.len() == 40 || s.len() == 64)
}

fn contains_timestamp(s: &str) -> bool {
    s.contains("timestamp") || s.contains("mtime") || s.contains("ts:") || 
    s.contains("epoch") || s.contains("T") && s.contains("Z") ||
    s.contains("1648") || s.contains("202") // Common timestamp prefixes
}

fn contains_version_info(s: &str) -> bool {
    s.contains("version") || s.contains("rev:") || s.contains("v1.") || 
    s.contains("revision") || s.contains("commit") || s.contains("branch")
}

fn is_encoded_format(s: &str) -> bool {
    s.contains("base64:") || s.contains("gzip:") || s.contains("url-encoded:") ||
    (s.ends_with("==") || s.ends_with("=")) && s.len() > 10 // Base64-like
}

fn is_microsoft_format(s: &str) -> bool {
    s.contains("SP") && (s.contains("Replication") || s.contains("FileVersion")) ||
    s.contains("ChangeKey") || s.contains("#ReplDigest") ||
    s.contains("CQA") // Common in Exchange ETags
}

fn contains_path_info(s: &str) -> bool {
    s.contains("/") && (s.contains(".") || s.contains("file://") || s.contains("./"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_propfind() {
        let xml = r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/webdav/test.pdf</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>test.pdf</d:displayname>
                        <d:getcontentlength>1024</d:getcontentlength>
                        <d:getlastmodified>Mon, 01 Jan 2024 12:00:00 GMT</d:getlastmodified>
                        <d:getcontenttype>application/pdf</d:getcontenttype>
                        <d:getetag>"abc123"</d:getetag>
                        <d:resourcetype/>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#;

        let files = parse_propfind_response(xml).unwrap();
        assert_eq!(files.len(), 1);
        
        let file = &files[0];
        assert_eq!(file.name, "test.pdf");
        assert_eq!(file.size, 1024);
        assert_eq!(file.mime_type, "application/pdf");
        assert_eq!(file.etag, "abc123");
        assert!(!file.is_directory);
    }

    #[test]
    fn test_parse_propfind_with_directory() {
        let xml = r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/webdav/Documents/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>Documents</d:displayname>
                        <d:resourcetype>
                            <d:collection/>
                        </d:resourcetype>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
            <d:response>
                <d:href>/webdav/Documents/file.txt</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>file.txt</d:displayname>
                        <d:getcontentlength>256</d:getcontentlength>
                        <d:getcontenttype>text/plain</d:getcontenttype>
                        <d:resourcetype/>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#;

        let files = parse_propfind_response(xml).unwrap();
        assert_eq!(files.len(), 1); // Only the file, not the directory
        
        let file = &files[0];
        assert_eq!(file.name, "file.txt");
        assert_eq!(file.size, 256);
    }

    #[test]
    fn test_parse_nextcloud_response() {
        let xml = r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:" xmlns:s="http://sabredav.org/ns" xmlns:oc="http://owncloud.org/ns">
            <d:response>
                <d:href>/remote.php/dav/files/admin/Documents/report.pdf</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>report.pdf</d:displayname>
                        <d:getcontentlength>2048000</d:getcontentlength>
                        <d:getlastmodified>Mon, 15 Jan 2024 14:30:00 GMT</d:getlastmodified>
                        <d:getcontenttype>application/pdf</d:getcontenttype>
                        <d:getetag>"pdf123"</d:getetag>
                        <d:resourcetype/>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#;

        let files = parse_propfind_response(xml).unwrap();
        assert_eq!(files.len(), 1);
        
        let file = &files[0];
        assert_eq!(file.name, "report.pdf");
        assert_eq!(file.path, "/remote.php/dav/files/admin/Documents/report.pdf");
        assert_eq!(file.size, 2048000);
        assert_eq!(file.etag, "pdf123"); // ETag should be normalized (quotes removed)
        assert!(file.last_modified.is_some());
    }

    #[test]
    fn test_parse_url_encoded_filenames() {
        let xml = r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/webdav/File%20with%20spaces.pdf</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>File with spaces.pdf</d:displayname>
                        <d:getcontentlength>1024</d:getcontentlength>
                        <d:getcontenttype>application/pdf</d:getcontenttype>
                        <d:resourcetype/>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#;

        let files = parse_propfind_response(xml).unwrap();
        assert_eq!(files.len(), 1);
        
        let file = &files[0];
        assert_eq!(file.name, "File with spaces.pdf");
    }

    #[test]
    fn test_empty_response() {
        let xml = r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
        </d:multistatus>"#;

        let files = parse_propfind_response(xml).unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_normalize_etag() {
        // Test various ETag formats that WebDAV servers might return
        assert_eq!(normalize_etag("abc123"), "abc123");
        assert_eq!(normalize_etag("\"abc123\""), "abc123");
        assert_eq!(normalize_etag("W/\"abc123\""), "abc123");
        assert_eq!(normalize_etag("  \"abc123\"  "), "abc123");
        assert_eq!(normalize_etag("W/\"abc-123-def\""), "abc-123-def");
        assert_eq!(normalize_etag(""), "");
        assert_eq!(normalize_etag("\"\""), "");
        assert_eq!(normalize_etag("W/\"\""), "");
    }
}