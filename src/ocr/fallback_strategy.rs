use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::xml_extractor::{OfficeExtractionResult, XmlOfficeExtractor};

#[cfg(test)]
use anyhow::anyhow;

/// Configuration for XML-based Office document extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Enable XML extraction
    pub enabled: bool,
    /// Maximum number of retry attempts for transient failures
    pub max_retries: u32,
    /// Initial retry delay in milliseconds
    pub initial_retry_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
    /// Timeout for XML extraction in seconds
    pub xml_timeout_seconds: u64,
}


impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 30000,
            xml_timeout_seconds: 180,
        }
    }
}



/// Statistics for monitoring XML extraction performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackStats {
    pub total_extractions: u64,
    pub xml_successes: u64,
    pub retry_attempts: u64,
    pub average_processing_time_ms: f64,
    pub success_rate_percentage: f64,
}

impl Default for FallbackStats {
    fn default() -> Self {
        Self {
            total_extractions: 0,
            xml_successes: 0,
            retry_attempts: 0,
            average_processing_time_ms: 0.0,
            success_rate_percentage: 100.0,
        }
    }
}

/// XML-based Office document extraction service
pub struct FallbackStrategy {
    config: FallbackConfig,
    xml_extractor: XmlOfficeExtractor,
    stats: std::sync::Arc<std::sync::RwLock<FallbackStats>>,
}

impl FallbackStrategy {
    /// Create a new XML extraction service
    pub fn new(config: FallbackConfig, temp_dir: String) -> Self {
        Self {
            config,
            xml_extractor: XmlOfficeExtractor::new(temp_dir),
            stats: std::sync::Arc::new(std::sync::RwLock::new(FallbackStats::default())),
        }
    }

    /// Extract Office document using XML extraction
    pub async fn extract_with_fallback(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<OfficeExtractionResult> {
        let start_time = std::time::Instant::now();
        let document_type = self.get_document_type(mime_type);
        
        info!("Starting XML extraction for {} (type: {})", file_path, document_type);
        
        // Update total extraction count
        if let Ok(mut stats) = self.stats.write() {
            stats.total_extractions += 1;
        }

        // Use XML extraction as the only method
        let result = self.execute_xml_extraction(file_path, mime_type).await;

        let processing_time = start_time.elapsed();
        
        // Update statistics  
        self.update_stats(&result, processing_time).await;

        result
    }

    /// Execute XML extraction directly 
    async fn execute_xml_extraction(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<OfficeExtractionResult> {
        let result = self.xml_extractor.extract_text_from_office(file_path, mime_type).await?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.write() {
            stats.xml_successes += 1;
        }
        
        Ok(result)
    }


    /// Get document type from MIME type
    fn get_document_type(&self, mime_type: &str) -> String {
        match mime_type {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx".to_string(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx".to_string(),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx".to_string(),
            "application/msword" => "doc".to_string(),
            "application/vnd.ms-excel" => "xls".to_string(),
            "application/vnd.ms-powerpoint" => "ppt".to_string(),
            "application/pdf" => "pdf".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Update statistics after extraction
    async fn update_stats(&self, result: &Result<OfficeExtractionResult>, processing_time: std::time::Duration) {
        if let Ok(mut stats) = self.stats.write() {
            let processing_time_ms = processing_time.as_millis() as f64;
            
            // Update average processing time using exponential moving average
            let alpha = 0.1; // Smoothing factor
            stats.average_processing_time_ms = 
                alpha * processing_time_ms + (1.0 - alpha) * stats.average_processing_time_ms;
            
            // Update success rate with proper division by zero protection
            let total_attempts = stats.total_extractions;
            let successful_attempts = stats.xml_successes;
            
            if total_attempts > 0 {
                stats.success_rate_percentage = (successful_attempts as f64 / total_attempts as f64) * 100.0;
            } else if result.is_ok() {
                stats.success_rate_percentage = 100.0;
            }
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> FallbackStats {
        self.stats.read()
            .map(|stats| stats.clone())
            .unwrap_or_else(|_| {
                warn!("Failed to acquire read lock on stats, returning default");
                FallbackStats::default()
            })
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        if let Ok(mut stats) = self.stats.write() {
            *stats = FallbackStats::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_strategy() -> (FallbackStrategy, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FallbackConfig::default();
        let strategy = FallbackStrategy::new(config, temp_dir.path().to_string_lossy().to_string());
        (strategy, temp_dir)
    }

    #[tokio::test] 
    async fn test_stats_tracking() {
        let (strategy, _temp_dir) = create_test_strategy();
        
        let initial_stats = strategy.get_stats().await;
        assert_eq!(initial_stats.total_extractions, 0);
        
        // Simulate some operations by updating stats directly
        if let Ok(mut stats) = strategy.stats.write() {
            stats.total_extractions = 10;
            stats.xml_successes = 9;
            // Calculate success rate manually as update_stats would do
            stats.success_rate_percentage = (9.0 / 10.0) * 100.0;
        }
        
        let updated_stats = strategy.get_stats().await;
        assert_eq!(updated_stats.total_extractions, 10);
        assert_eq!(updated_stats.xml_successes, 9);
        assert_eq!(updated_stats.success_rate_percentage, 90.0); // 9 successes out of 10
    }

    #[test]
    fn test_get_document_type() {
        let (strategy, _temp_dir) = create_test_strategy();
        
        assert_eq!(strategy.get_document_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document"), "docx");
        assert_eq!(strategy.get_document_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"), "xlsx");
        assert_eq!(strategy.get_document_type("application/vnd.openxmlformats-officedocument.presentationml.presentation"), "pptx");
        assert_eq!(strategy.get_document_type("application/pdf"), "pdf");
        assert_eq!(strategy.get_document_type("unknown/type"), "unknown");
    }
}