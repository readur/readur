use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::config::WebDAVConfig;
use super::connection::WebDAVConnection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub overall_health_score: i32, // 0-100
    pub issues: Vec<ValidationIssue>,
    pub recommendations: Vec<ValidationRecommendation>,
    pub summary: ValidationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub issue_type: ValidationIssueType,
    pub severity: ValidationSeverity,
    pub directory_path: String,
    pub description: String,
    pub details: Option<serde_json::Value>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationIssueType {
    /// Directory exists on server but not in our tracking
    Untracked,
    /// Directory in our tracking but missing on server  
    Missing,
    /// ETag mismatch between server and our cache
    ETagMismatch,
    /// Directory hasn't been scanned in a very long time
    Stale,
    /// Server errors when accessing directory
    Inaccessible,
    /// ETag support seems unreliable for this directory
    ETagUnreliable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,    // No action needed, just FYI
    Warning, // Should investigate but not urgent
    Error,   // Needs immediate attention
    Critical, // System integrity at risk
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRecommendation {
    pub action: ValidationAction,
    pub reason: String,
    pub affected_directories: Vec<String>,
    pub priority: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationAction {
    /// Run a deep scan of specific directories
    DeepScanRequired,
    /// Clear and rebuild directory tracking
    RebuildTracking,
    /// ETag support is unreliable, switch to periodic scans
    DisableETagOptimization,
    /// Clean up orphaned database entries
    CleanupDatabase,
    /// Server configuration issue needs attention
    CheckServerConfiguration,
    /// No action needed, system is healthy
    NoActionRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total_directories_checked: usize,
    pub healthy_directories: usize,
    pub directories_with_issues: usize,
    pub critical_issues: usize,
    pub warning_issues: usize,
    pub info_issues: usize,
    pub validation_duration_ms: u64,
}

pub struct WebDAVValidator {
    connection: WebDAVConnection,
    config: WebDAVConfig,
}

impl WebDAVValidator {
    pub fn new(connection: WebDAVConnection, config: WebDAVConfig) -> Self {
        Self { connection, config }
    }

    /// Performs comprehensive validation of WebDAV setup and directory tracking
    pub async fn validate_system(&self) -> Result<ValidationReport> {
        let start_time = std::time::Instant::now();
        info!("🔍 Starting WebDAV system validation");

        let mut issues = Vec::new();
        let mut total_checked = 0;

        // Test basic connectivity
        match self.connection.test_connection().await {
            Ok(result) if !result.success => {
                issues.push(ValidationIssue {
                    issue_type: ValidationIssueType::Inaccessible,
                    severity: ValidationSeverity::Critical,
                    directory_path: "/".to_string(),
                    description: format!("WebDAV server connection failed: {}", result.message),
                    details: None,
                    detected_at: chrono::Utc::now(),
                });
            }
            Err(e) => {
                issues.push(ValidationIssue {
                    issue_type: ValidationIssueType::Inaccessible,
                    severity: ValidationSeverity::Critical,
                    directory_path: "/".to_string(),
                    description: format!("WebDAV server connectivity test failed: {}", e),
                    details: None,
                    detected_at: chrono::Utc::now(),
                });
            }
            _ => {
                debug!("✅ Basic connectivity test passed");
            }
        }

        // Validate each watch folder
        for folder in &self.config.watch_folders {
            total_checked += 1;
            if let Err(e) = self.validate_watch_folder(folder, &mut issues).await {
                warn!("Failed to validate watch folder '{}': {}", folder, e);
            }
        }

        // Test ETag reliability
        self.validate_etag_support(&mut issues).await?;

        // Generate recommendations based on issues
        let recommendations = self.generate_recommendations(&issues);

        let validation_duration = start_time.elapsed().as_millis() as u64;
        let health_score = self.calculate_health_score(&issues);

        let summary = ValidationSummary {
            total_directories_checked: total_checked,
            healthy_directories: total_checked - issues.len(),
            directories_with_issues: issues.len(),
            critical_issues: issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Critical)).count(),
            warning_issues: issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Warning)).count(),
            info_issues: issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Info)).count(),
            validation_duration_ms: validation_duration,
        };

        info!("✅ WebDAV validation completed in {}ms. Health score: {}/100", 
            validation_duration, health_score);

        Ok(ValidationReport {
            overall_health_score: health_score,
            issues,
            recommendations,
            summary,
        })
    }

    /// Validates a specific watch folder
    async fn validate_watch_folder(&self, folder: &str, issues: &mut Vec<ValidationIssue>) -> Result<()> {
        debug!("Validating watch folder: {}", folder);

        // Test PROPFIND access
        match self.connection.test_propfind(folder).await {
            Ok(_) => {
                debug!("✅ Watch folder '{}' is accessible", folder);
            }
            Err(e) => {
                issues.push(ValidationIssue {
                    issue_type: ValidationIssueType::Inaccessible,
                    severity: ValidationSeverity::Error,
                    directory_path: folder.to_string(),
                    description: format!("Cannot access watch folder: {}", e),
                    details: Some(serde_json::json!({
                        "error": e.to_string(),
                        "folder": folder
                    })),
                    detected_at: chrono::Utc::now(),
                });
            }
        }

        Ok(())
    }

    /// Tests ETag support reliability
    async fn validate_etag_support(&self, issues: &mut Vec<ValidationIssue>) -> Result<()> {
        debug!("Testing ETag support reliability");

        // Test ETag consistency across multiple requests
        for folder in &self.config.watch_folders {
            if let Err(e) = self.test_etag_consistency(folder, issues).await {
                warn!("ETag consistency test failed for '{}': {}", folder, e);
            }
        }

        Ok(())
    }

    /// Tests ETag consistency for a specific folder
    async fn test_etag_consistency(&self, folder: &str, issues: &mut Vec<ValidationIssue>) -> Result<()> {
        // Make two consecutive PROPFIND requests and compare ETags
        let etag1 = self.get_folder_etag(folder).await?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let etag2 = self.get_folder_etag(folder).await?;

        if etag1 != etag2 && etag1.is_some() && etag2.is_some() {
            issues.push(ValidationIssue {
                issue_type: ValidationIssueType::ETagUnreliable,
                severity: ValidationSeverity::Warning,
                directory_path: folder.to_string(),
                description: "ETag values are inconsistent across requests".to_string(),
                details: Some(serde_json::json!({
                    "etag1": etag1,
                    "etag2": etag2,
                    "folder": folder
                })),
                detected_at: chrono::Utc::now(),
            });
        }

        Ok(())
    }

    /// Gets the ETag for a folder
    async fn get_folder_etag(&self, folder: &str) -> Result<Option<String>> {
        let url = self.connection.get_url_for_path(folder);
        
        let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
            <D:propfind xmlns:D="DAV:">
                <D:prop>
                    <D:getetag/>
                </D:prop>
            </D:propfind>"#;

        let response = self.connection
            .authenticated_request(
                reqwest::Method::from_bytes(b"PROPFIND")?,
                &url,
                Some(propfind_body.to_string()),
                Some(vec![
                    ("Depth", "0"),
                    ("Content-Type", "application/xml"),
                ]),
            )
            .await?;

        let body = response.text().await?;
        
        // Parse ETag from XML response (simplified)
        if let Some(start) = body.find("<D:getetag>") {
            if let Some(end) = body[start..].find("</D:getetag>") {
                let etag = &body[start + 11..start + end];
                return Ok(Some(etag.trim_matches('"').to_string()));
            }
        }

        Ok(None)
    }

    /// Generates recommendations based on detected issues
    fn generate_recommendations(&self, issues: &Vec<ValidationIssue>) -> Vec<ValidationRecommendation> {
        let mut recommendations = Vec::new();
        let mut directories_by_issue: HashMap<ValidationIssueType, Vec<String>> = HashMap::new();

        // Group directories by issue type
        for issue in issues {
            directories_by_issue
                .entry(issue.issue_type.clone())
                .or_insert_with(Vec::new)
                .push(issue.directory_path.clone());
        }

        // Generate recommendations for each issue type
        for (issue_type, directories) in directories_by_issue {
            let recommendation = match issue_type {
                ValidationIssueType::Inaccessible => ValidationRecommendation {
                    action: ValidationAction::CheckServerConfiguration,
                    reason: "Some directories are inaccessible. Check server configuration and permissions.".to_string(),
                    affected_directories: directories,
                    priority: ValidationSeverity::Critical,
                },
                ValidationIssueType::ETagUnreliable => ValidationRecommendation {
                    action: ValidationAction::DisableETagOptimization,
                    reason: "ETag support appears unreliable. Consider disabling ETag optimization.".to_string(),
                    affected_directories: directories,
                    priority: ValidationSeverity::Warning,
                },
                ValidationIssueType::Missing => ValidationRecommendation {
                    action: ValidationAction::CleanupDatabase,
                    reason: "Some tracked directories no longer exist on the server.".to_string(),
                    affected_directories: directories,
                    priority: ValidationSeverity::Warning,
                },
                ValidationIssueType::Stale => ValidationRecommendation {
                    action: ValidationAction::DeepScanRequired,
                    reason: "Some directories haven't been scanned recently.".to_string(),
                    affected_directories: directories,
                    priority: ValidationSeverity::Info,
                },
                _ => ValidationRecommendation {
                    action: ValidationAction::DeepScanRequired,
                    reason: "General validation issues detected.".to_string(),
                    affected_directories: directories,
                    priority: ValidationSeverity::Warning,
                },
            };
            recommendations.push(recommendation);
        }

        if recommendations.is_empty() {
            recommendations.push(ValidationRecommendation {
                action: ValidationAction::NoActionRequired,
                reason: "System validation passed successfully.".to_string(),
                affected_directories: Vec::new(),
                priority: ValidationSeverity::Info,
            });
        }

        recommendations
    }

    /// Calculates overall health score based on issues
    fn calculate_health_score(&self, issues: &Vec<ValidationIssue>) -> i32 {
        if issues.is_empty() {
            return 100;
        }

        let mut penalty = 0;
        for issue in issues {
            let issue_penalty = match issue.severity {
                ValidationSeverity::Critical => 30,
                ValidationSeverity::Error => 20,
                ValidationSeverity::Warning => 10,
                ValidationSeverity::Info => 5,
            };
            penalty += issue_penalty;
        }

        std::cmp::max(0, 100 - penalty)
    }
}