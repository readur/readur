/// Common utilities and shared functions for WebDAV services

/// Build a standardized User-Agent string for all WebDAV requests
/// This ensures consistent identification across all WebDAV operations
pub fn build_user_agent() -> String {
    format!("Readur/{} (WebDAV-Sync; +https://github.com/readur)", 
            env!("CARGO_PKG_VERSION"))
}