use crate::webdav_xml_parser::{
    compare_etags, weak_compare_etags, strong_compare_etags, 
    ParsedETag, normalize_etag
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_etag_handles_quotes() {
        assert_eq!(normalize_etag("\"abc123\""), "abc123");
        assert_eq!(normalize_etag("abc123"), "abc123");
        assert_eq!(normalize_etag("\"\""), "");
    }

    #[test]
    fn test_normalize_etag_handles_weak_indicators() {
        assert_eq!(normalize_etag("W/\"abc123\""), "abc123");
        assert_eq!(normalize_etag("w/\"abc123\""), "abc123");
        assert_eq!(normalize_etag("W/abc123"), "abc123");
    }

    #[test]
    fn test_normalize_etag_handles_multiple_weak_indicators() {
        // Malformed but seen in the wild
        assert_eq!(normalize_etag("W/W/\"abc123\""), "abc123");
        assert_eq!(normalize_etag("w/W/\"abc123\""), "abc123");
    }

    #[test]
    fn test_parsed_etag_weak_detection() {
        let weak_etag = ParsedETag::parse("W/\"abc123\"");
        assert!(weak_etag.is_weak);
        assert_eq!(weak_etag.normalized, "abc123");

        let strong_etag = ParsedETag::parse("\"abc123\"");
        assert!(!strong_etag.is_weak);
        assert_eq!(strong_etag.normalized, "abc123");
    }

    #[test]
    fn test_strong_comparison_rejects_weak_etags() {
        let weak1 = ParsedETag::parse("W/\"abc123\"");
        let weak2 = ParsedETag::parse("W/\"abc123\"");
        let strong1 = ParsedETag::parse("\"abc123\"");
        let strong2 = ParsedETag::parse("\"abc123\"");

        // Strong comparison should reject any weak ETags
        assert!(!weak1.strong_compare(&weak2));
        assert!(!weak1.strong_compare(&strong1));
        assert!(!strong1.strong_compare(&weak1));
        
        // Only strong ETags should match in strong comparison
        assert!(strong1.strong_compare(&strong2));
    }

    #[test]
    fn test_weak_comparison_accepts_all_combinations() {
        let weak1 = ParsedETag::parse("W/\"abc123\"");
        let weak2 = ParsedETag::parse("W/\"abc123\"");
        let strong1 = ParsedETag::parse("\"abc123\"");
        let strong2 = ParsedETag::parse("\"abc123\"");

        // Weak comparison should accept all combinations if values match
        assert!(weak1.weak_compare(&weak2));
        assert!(weak1.weak_compare(&strong1));
        assert!(strong1.weak_compare(&weak1));
        assert!(strong1.weak_compare(&strong2));
    }

    #[test]
    fn test_smart_comparison_logic() {
        let weak = ParsedETag::parse("W/\"abc123\"");
        let strong = ParsedETag::parse("\"abc123\"");

        // If either is weak, should use weak comparison
        assert!(weak.smart_compare(&strong));
        assert!(strong.smart_compare(&weak));
        
        // If both are strong, should use strong comparison
        let strong2 = ParsedETag::parse("\"abc123\"");
        assert!(strong.smart_compare(&strong2));
    }

    #[test]
    fn test_utility_functions() {
        // Test the utility functions that the smart sync will use
        assert!(compare_etags("W/\"abc123\"", "\"abc123\""));
        assert!(weak_compare_etags("W/\"abc123\"", "\"abc123\""));
        assert!(!strong_compare_etags("W/\"abc123\"", "\"abc123\""));
    }

    #[test]
    fn test_case_sensitivity_preservation() {
        // ETags should be case sensitive per RFC
        assert!(!compare_etags("\"ABC123\"", "\"abc123\""));
        assert!(!weak_compare_etags("\"ABC123\"", "\"abc123\""));
        assert!(!strong_compare_etags("\"ABC123\"", "\"abc123\""));
    }

    #[test]
    fn test_real_world_etag_formats() {
        // Test various real-world ETag formats
        let nextcloud_etag = "\"5f3e7e8a9b2c1d4\"";
        let apache_etag = "\"1234-567-890abcdef\"";
        let nginx_etag = "W/\"5f3e7e8a\"";
        let sharepoint_etag = "\"{12345678-1234-1234-1234-123456789012},1\"";

        // All should normalize correctly
        assert_eq!(normalize_etag(nextcloud_etag), "5f3e7e8a9b2c1d4");
        assert_eq!(normalize_etag(apache_etag), "1234-567-890abcdef");
        assert_eq!(normalize_etag(nginx_etag), "5f3e7e8a");
        assert_eq!(normalize_etag(sharepoint_etag), "{12345678-1234-1234-1234-123456789012},1");
    }

    #[test]
    fn test_etag_equivalence() {
        let etag1 = ParsedETag::parse("\"abc123\"");
        let etag2 = ParsedETag::parse("W/\"abc123\"");
        
        // Should be equivalent despite weak/strong difference
        assert!(etag1.is_equivalent(&etag2));
        
        let etag3 = ParsedETag::parse("\"def456\"");
        assert!(!etag1.is_equivalent(&etag3));
    }

    #[test]
    fn test_comparison_string_safety() {
        let etag_with_quotes = ParsedETag::parse("\"test\\\"internal\\\"quotes\"");
        let comparison_str = etag_with_quotes.comparison_string();
        
        // Should handle internal quotes safely
        assert!(!comparison_str.contains('"'));
        assert!(!comparison_str.contains("\\"));
    }
}