//! Security utilities for input validation and sanitization

use anyhow::Result;
use std::path::{Path, PathBuf, Component};
use tracing::warn;

/// Validate and sanitize file paths to prevent path traversal attacks
pub fn validate_and_sanitize_path(input_path: &str) -> Result<String> {
    // Check for null bytes (not allowed in file paths)
    if input_path.contains('\0') {
        return Err(anyhow::anyhow!("Path contains null bytes"));
    }
    
    // Check for excessively long paths
    if input_path.len() > 4096 {
        return Err(anyhow::anyhow!("Path too long (max 4096 characters)"));
    }
    
    // Convert to Path for normalization
    let path = Path::new(input_path);
    
    // Check for path traversal attempts
    for component in path.components() {
        match component {
            Component::ParentDir => {
                warn!("Path traversal attempt detected: {}", input_path);
                return Err(anyhow::anyhow!("Path traversal not allowed"));
            }
            Component::Normal(name) => {
                let name_str = name.to_string_lossy();
                
                // Check for dangerous file names
                if is_dangerous_filename(&name_str) {
                    return Err(anyhow::anyhow!("Potentially dangerous filename: {}", name_str));
                }
                
                // Check for control characters (except newline and tab which might be in file content)
                for ch in name_str.chars() {
                    if ch.is_control() && ch != '\n' && ch != '\t' {
                        return Err(anyhow::anyhow!("Filename contains control characters"));
                    }
                }
            }
            _ => {} // Allow root, current dir, and prefix components
        }
    }
    
    // Normalize the path to remove redundant components
    let normalized = normalize_path(path);
    Ok(normalized.to_string_lossy().to_string())
}

/// Validate filename for document storage
pub fn validate_filename(filename: &str) -> Result<String> {
    // Basic length check
    if filename.is_empty() {
        return Err(anyhow::anyhow!("Filename cannot be empty"));
    }
    
    if filename.len() > 255 {
        return Err(anyhow::anyhow!("Filename too long (max 255 characters)"));
    }
    
    // Check for null bytes
    if filename.contains('\0') {
        return Err(anyhow::anyhow!("Filename contains null bytes"));
    }
    
    // Check for path separators (filenames should not contain them)
    if filename.contains('/') || filename.contains('\\') {
        return Err(anyhow::anyhow!("Filename cannot contain path separators"));
    }
    
    // Check for control characters
    for ch in filename.chars() {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            return Err(anyhow::anyhow!("Filename contains control characters"));
        }
    }
    
    // Check for dangerous patterns
    if is_dangerous_filename(filename) {
        return Err(anyhow::anyhow!("Potentially dangerous filename: {}", filename));
    }
    
    // Sanitize the filename by replacing problematic characters
    let sanitized = sanitize_filename(filename);
    Ok(sanitized)
}

/// Check if a filename is potentially dangerous
fn is_dangerous_filename(filename: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    
    // Windows reserved names
    let reserved_names = [
        "con", "prn", "aux", "nul",
        "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
        "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    
    // Check if filename (without extension) matches reserved names
    let name_without_ext = filename_lower.split('.').next().unwrap_or("");
    if reserved_names.contains(&name_without_ext) {
        return true;
    }
    
    // Check for suspicious patterns
    if filename_lower.starts_with('.') && filename_lower.len() > 1 {
        // Allow common hidden files but reject suspicious ones
        let allowed_hidden = [".env", ".gitignore", ".htaccess"];
        if !allowed_hidden.iter().any(|&allowed| filename_lower.starts_with(allowed)) {
            // Be more permissive with document files that might have dots
            if !filename_lower.contains(&['.', 'd', 'o', 'c']) && 
               !filename_lower.contains(&['.', 'p', 'd', 'f']) &&
               !filename_lower.contains(&['.', 't', 'x', 't']) {
                return true;
            }
        }
    }
    
    false
}

/// Sanitize filename by replacing problematic characters
fn sanitize_filename(filename: &str) -> String {
    let mut sanitized = String::new();
    
    for ch in filename.chars() {
        match ch {
            // Replace problematic characters with underscores
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => sanitized.push('_'),
            // Allow most other characters
            _ if !ch.is_control() || ch == '\n' || ch == '\t' => sanitized.push(ch),
            // Skip control characters
            _ => {}
        }
    }
    
    // Trim whitespace from ends
    sanitized.trim().to_string()
}

/// Normalize a path by resolving . and .. components without filesystem access
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::RootDir | Component::Prefix(_) => {
                normalized.push(component);
            }
            Component::CurDir => {
                // Skip current directory references
            }
            Component::ParentDir => {
                // This should have been caught earlier, but handle it safely
                if normalized.parent().is_some() {
                    normalized.pop();
                }
                // If we can't go up, just ignore the .. component
            }
        }
    }
    
    normalized
}

/// Validate that a path is within the allowed base directory
pub fn validate_path_within_base(path: &str, base_dir: &str) -> Result<()> {
    let path_buf = PathBuf::from(path);
    let base_buf = PathBuf::from(base_dir);
    
    // Canonicalize if possible, but don't fail if paths don't exist yet
    let canonical_path = path_buf.canonicalize().unwrap_or_else(|_| {
        // If canonicalization fails, do our best with normalization
        normalize_path(&path_buf)
    });
    
    let canonical_base = base_buf.canonicalize().unwrap_or_else(|_| {
        normalize_path(&base_buf)
    });
    
    // Add debug logging to diagnose path validation issues
    eprintln!("DEBUG: Path validation:");
    eprintln!("  Input path: '{}'", path);
    eprintln!("  Input base: '{}'", base_dir);
    eprintln!("  Canonical path: '{}'", canonical_path.display());
    eprintln!("  Canonical base: '{}'", canonical_base.display());
    eprintln!("  Starts with check: {}", canonical_path.starts_with(&canonical_base));
    
    if !canonical_path.starts_with(&canonical_base) {
        return Err(anyhow::anyhow!(
            "Path '{}' is not within allowed base directory '{}' (failed after {:?})", 
            path, base_dir, std::time::Instant::now().elapsed()
        ));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_filename() {
        // Valid filenames
        assert!(validate_filename("document.pdf").is_ok());
        assert!(validate_filename("my-file_2023.docx").is_ok());
        assert!(validate_filename("report (final).txt").is_ok());
        
        // Invalid filenames
        assert!(validate_filename("").is_err());
        assert!(validate_filename("../etc/passwd").is_err());
        assert!(validate_filename("file\0name.txt").is_err());
        assert!(validate_filename("con.txt").is_err());
        assert!(validate_filename("file/name.txt").is_err());
    }

    #[test]
    fn test_validate_path() {
        // Valid paths
        assert!(validate_and_sanitize_path("documents/file.pdf").is_ok());
        assert!(validate_and_sanitize_path("./uploads/document.txt").is_ok());
        
        // Invalid paths
        assert!(validate_and_sanitize_path("../../../etc/passwd").is_err());
        assert!(validate_and_sanitize_path("documents/../config.txt").is_err());
        assert!(validate_and_sanitize_path("file\0name.txt").is_err());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("file<>name.txt"), "file__name.txt");
        assert_eq!(sanitize_filename("  report.pdf  "), "report.pdf");
        assert_eq!(sanitize_filename("file:name|test.doc"), "file_name_test.doc");
    }
}