use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::str;

use crate::models::FileInfo;

#[derive(Debug, Default)]
struct PropFindResponse {
    href: String,
    displayname: String,
    content_length: Option<i64>,
    last_modified: Option<String>,
    content_type: Option<String>,
    etag: Option<String>,
    is_collection: bool,
}

pub fn parse_propfind_response(xml_text: &str) -> Result<Vec<FileInfo>> {
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
                                resp.etag = Some(text.trim().to_string());
                            }
                            "status" if in_propstat => {
                                // Check if status is 200 OK
                                if text.contains("200") {
                                    status_ok = true;
                                }
                            }
                            _ => {}
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
                                
                                let file_info = FileInfo {
                                    path: resp.href.clone(),
                                    name,
                                    size: resp.content_length.unwrap_or(0),
                                    mime_type: resp.content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
                                    last_modified: parse_http_date(&resp.last_modified.unwrap_or_default()),
                                    etag: resp.etag.unwrap_or_else(|| format!("\"{}\"", uuid::Uuid::new_v4())),
                                    is_directory: false,
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
        assert_eq!(file.etag, "\"abc123\"");
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
}