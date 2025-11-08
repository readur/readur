//! Security utilities for input validation and sanitization

use anyhow::Result;
use std::path::{Path, PathBuf, Component};
use tracing::{warn, debug};
use rand::Rng;

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
    
    // Convert both paths to absolute paths for consistent comparison
    let current_dir = std::env::current_dir().unwrap_or_default();
    
    let absolute_base = if base_buf.is_absolute() {
        base_buf
    } else {
        current_dir.join(&base_buf)
    };
    
    let absolute_path = if path_buf.is_absolute() {
        path_buf
    } else {
        current_dir.join(&path_buf)
    };
    
    // Try to canonicalize both paths, with consistent fallback behavior
    let canonical_base = absolute_base.canonicalize().unwrap_or_else(|_| {
        // If canonicalization fails, use the absolute path through normalize_path
        normalize_path(&absolute_base)
    });
    
    let canonical_path = if absolute_path.exists() {
        // If the file exists, canonicalize it
        absolute_path.canonicalize().unwrap_or_else(|_| normalize_path(&absolute_path))
    } else {
        // If file doesn't exist, try to canonicalize its parent directory and append the filename
        if let Some(parent) = absolute_path.parent() {
            if let Some(filename) = absolute_path.file_name() {
                let canonical_parent = parent.canonicalize().unwrap_or_else(|_| normalize_path(parent));
                canonical_parent.join(filename)
            } else {
                normalize_path(&absolute_path)
            }
        } else {
            normalize_path(&absolute_path)
        }
    };
    
    // Add debug logging to diagnose path validation issues
    debug!("Path validation: input_path='{}', base_dir='{}'", path, base_dir);
    debug!("Path validation: absolute_path='{}', absolute_base='{}'", absolute_path.display(), absolute_base.display());
    debug!("Path validation: canonical_path='{}', canonical_base='{}'", canonical_path.display(), canonical_base.display());
    debug!("Path validation: starts_with_check={}", canonical_path.starts_with(&canonical_base));
    
    if !canonical_path.starts_with(&canonical_base) {
        return Err(anyhow::anyhow!(
            "Path '{}' is not within allowed base directory '{}'", 
            path, base_dir
        ));
    }
    
    Ok(())
}

/// Generate a cryptographically secure random password
///
/// Generates a password with the specified length containing:
/// - Uppercase letters (A-Z)
/// - Lowercase letters (a-z)
/// - Numbers (0-9)
/// - Special characters (!@#$%^&*-_=+)
///
/// # Arguments
/// * `length` - The desired password length (minimum 12, recommended 24+)
///
/// # Returns
/// A randomly generated password string
pub fn generate_secure_password(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789\
                            !@#$%^&*-_=+";

    let mut rng = rand::thread_rng();

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
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

    #[test]
    fn test_validate_path_within_base() {
        use std::fs;
        
        // Setup test directories
        let test_base = "test_uploads_validation";
        let test_docs = format!("{}/documents", test_base);
        
        // Clean up any existing test directories
        fs::remove_dir_all(test_base).unwrap_or(());
        
        // Test 1: Neither base nor parent exists
        let result = validate_path_within_base(
            "./test_uploads_validation/documents/test.txt",
            "./test_uploads_validation"
        );
        assert!(result.is_ok(), "Should allow paths within base even when directories don't exist");
        
        // Test 2: Base exists but parent doesn't (the problematic case)
        fs::create_dir_all(test_base).unwrap();
        let result = validate_path_within_base(
            "./test_uploads_validation/documents/test.txt",
            "./test_uploads_validation"
        );
        assert!(result.is_ok(), "Should allow subdirectory paths when base exists but parent doesn't");
        
        // Test 3: Both base and parent exist
        fs::create_dir_all(&test_docs).unwrap();
        let result = validate_path_within_base(
            "./test_uploads_validation/documents/test.txt",
            "./test_uploads_validation"
        );
        assert!(result.is_ok(), "Should allow paths when both base and parent exist");
        
        // Test 4: Path outside base directory should fail
        let result = validate_path_within_base(
            "../outside.txt",
            "./test_uploads_validation"
        );
        assert!(result.is_err(), "Should reject paths outside base directory");
        
        // Test 5: Absolute paths
        let current_dir = std::env::current_dir().unwrap();
        let abs_base = current_dir.join(test_base);
        let abs_path = abs_base.join("documents/test.txt");
        
        let result = validate_path_within_base(
            &abs_path.to_string_lossy(),
            &abs_base.to_string_lossy()
        );
        assert!(result.is_ok(), "Should handle absolute paths correctly");
        
        // Test 6: Mixed absolute and relative paths
        let result = validate_path_within_base(
            &abs_path.to_string_lossy(),
            "./test_uploads_validation"
        );
        assert!(result.is_ok(), "Should handle mixed absolute/relative paths");
        
        // Clean up
        fs::remove_dir_all(test_base).unwrap_or(());
    }

    #[test]
    fn test_validate_path_within_base_traversal_attempts() {
        use std::fs;

        let test_base = "test_security_validation";
        fs::create_dir_all(test_base).unwrap_or(());

        // Test various path traversal attempts
        let traversal_attempts = vec![
            "../../../etc/passwd",
            "./test_security_validation/../../../etc/passwd",
            "test_security_validation/../outside.txt",
            "./test_security_validation/documents/../../outside.txt",
        ];

        for attempt in traversal_attempts {
            let result = validate_path_within_base(attempt, "./test_security_validation");
            assert!(result.is_err(), "Should reject path traversal attempt: {}", attempt);
        }

        // Clean up
        fs::remove_dir_all(test_base).unwrap_or(());
    }

    #[test]
    fn test_generate_secure_password_length() {
        // Test default length
        let password = generate_secure_password(24);
        assert_eq!(password.len(), 24, "Password should be exactly 24 characters");

        // Test different lengths
        let password_12 = generate_secure_password(12);
        assert_eq!(password_12.len(), 12, "Password should be exactly 12 characters");

        let password_32 = generate_secure_password(32);
        assert_eq!(password_32.len(), 32, "Password should be exactly 32 characters");
    }

    #[test]
    fn test_generate_secure_password_character_composition() {
        let password = generate_secure_password(100); // Use longer password for better test coverage

        // Check for uppercase letters
        let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
        assert!(has_uppercase, "Password should contain at least one uppercase letter");

        // Check for lowercase letters
        let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
        assert!(has_lowercase, "Password should contain at least one lowercase letter");

        // Check for digits
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        assert!(has_digit, "Password should contain at least one digit");

        // Check for special characters
        let special_chars = "!@#$%^&*-_=+";
        let has_special = password.chars().any(|c| special_chars.contains(c));
        assert!(has_special, "Password should contain at least one special character");

        // Ensure all characters are from the allowed charset
        let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*-_=+";
        for ch in password.chars() {
            assert!(charset.contains(ch), "Password contains invalid character: {}", ch);
        }
    }
}