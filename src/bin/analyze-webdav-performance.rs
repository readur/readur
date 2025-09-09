/*!
 * WebDAV Performance Analysis Tool
 * 
 * Analyzes stress test metrics and generates comprehensive reports for CI/CD pipeline
 */

use anyhow::{anyhow, Result};
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct LoopDetectionStatistics {
    total_directories_monitored: usize,
    total_directory_accesses: usize,
    suspected_loop_count: usize,
    max_accesses_per_directory: usize,
    average_accesses_per_directory: f64,
    suspected_directories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WebDAVPerformanceMetrics {
    total_operations: usize,
    successful_operations: usize,
    failed_operations: usize,
    average_operation_duration_ms: f64,
    max_operation_duration_ms: u64,
    min_operation_duration_ms: u64,
    timeout_count: usize,
    error_patterns: std::collections::HashMap<String, usize>,
    loop_detection_stats: LoopDetectionStatistics,
}

#[derive(Debug, Serialize, Deserialize)]
struct StressTestReport {
    test_suite_version: String,
    test_timestamp: chrono::DateTime<chrono::Utc>,
    overall_result: String,
    test_summary: TestSummary,
    recommendations: Vec<String>,
    performance_metrics: Option<WebDAVPerformanceMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestSummary {
    total_tests: usize,
    passed_tests: usize,
    failed_tests: usize,
    skipped_tests: usize,
}

#[derive(Debug)]
struct PerformanceAnalysis {
    overall_health: HealthStatus,
    critical_issues: Vec<String>,
    warnings: Vec<String>,
    recommendations: Vec<String>,
    metrics_summary: MetricsSummary,
}

#[derive(Debug)]
enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug)]
struct MetricsSummary {
    success_rate: f64,
    average_response_time: f64,
    max_response_time: u64,
    timeout_rate: f64,
    loop_detection_triggered: bool,
    total_operations: usize,
}

fn main() -> Result<()> {
    let matches = Command::new("WebDAV Performance Analyzer")
        .version("1.0.0")
        .about("Analyzes WebDAV stress test metrics and generates reports")
        .arg(
            Arg::new("metrics-file")
                .long("metrics-file")
                .value_name("FILE")
                .help("Path to the stress test metrics JSON file")
                .required(true),
        )
        .arg(
            Arg::new("output-format")
                .long("output-format")
                .value_name("FORMAT")
                .help("Output format: json, markdown, github-summary")
                .default_value("markdown"),
        )
        .arg(
            Arg::new("output-file")
                .long("output-file")
                .value_name("FILE")
                .help("Output file path (stdout if not specified)"),
        )
        .get_matches();

    let metrics_file = matches.get_one::<String>("metrics-file").unwrap();
    let output_format = matches.get_one::<String>("output-format").unwrap();
    let output_file = matches.get_one::<String>("output-file");

    // Load and parse metrics file
    let report = load_stress_test_report(metrics_file)?;
    
    // Analyze performance metrics
    let analysis = analyze_performance(&report)?;
    
    // Generate output based on format
    let output_content = match output_format.as_str() {
        "json" => generate_json_report(&analysis)?,
        "markdown" => generate_markdown_report(&analysis, &report)?,
        "github-summary" => generate_github_summary(&analysis, &report)?,
        _ => return Err(anyhow!("Unsupported output format: {}", output_format)),
    };

    // Write output
    if let Some(output_path) = output_file {
        fs::write(output_path, &output_content)?;
        println!("Report written to: {}", output_path);
    } else {
        println!("{}", output_content);
    }

    // Exit with appropriate code
    match analysis.overall_health {
        HealthStatus::Critical => std::process::exit(1),
        HealthStatus::Warning => std::process::exit(0), // Still success, but with warnings
        HealthStatus::Healthy => std::process::exit(0),
        HealthStatus::Unknown => std::process::exit(2),
    }
}

fn load_stress_test_report(file_path: &str) -> Result<StressTestReport> {
    if !Path::new(file_path).exists() {
        return Err(anyhow!("Metrics file not found: {}", file_path));
    }

    let content = fs::read_to_string(file_path)?;
    let report: StressTestReport = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse metrics file: {}", e))?;

    Ok(report)
}

fn analyze_performance(report: &StressTestReport) -> Result<PerformanceAnalysis> {
    let mut critical_issues = Vec::new();
    let mut warnings = Vec::new();
    let mut recommendations = Vec::new();

    let metrics_summary = if let Some(metrics) = &report.performance_metrics {
        let success_rate = if metrics.total_operations > 0 {
            (metrics.successful_operations as f64 / metrics.total_operations as f64) * 100.0
        } else {
            0.0
        };

        let timeout_rate = if metrics.total_operations > 0 {
            (metrics.timeout_count as f64 / metrics.total_operations as f64) * 100.0
        } else {
            0.0
        };

        // Analyze critical issues
        if success_rate < 50.0 {
            critical_issues.push(format!(
                "Critical: Very low success rate ({:.1}%) - indicates severe WebDAV connectivity issues",
                success_rate
            ));
        } else if success_rate < 80.0 {
            warnings.push(format!(
                "Warning: Low success rate ({:.1}%) - investigate WebDAV server performance",
                success_rate
            ));
        }

        if metrics.loop_detection_stats.suspected_loop_count > 0 {
            critical_issues.push(format!(
                "Critical: {} suspected infinite loops detected - immediate investigation required",
                metrics.loop_detection_stats.suspected_loop_count
            ));
            
            for dir in &metrics.loop_detection_stats.suspected_directories {
                critical_issues.push(format!("  - Suspected loop in directory: {}", dir));
            }
        }

        if timeout_rate > 20.0 {
            critical_issues.push(format!(
                "Critical: High timeout rate ({:.1}%) - server may be overloaded or unresponsive",
                timeout_rate
            ));
        } else if timeout_rate > 10.0 {
            warnings.push(format!(
                "Warning: Elevated timeout rate ({:.1}%) - monitor server performance",
                timeout_rate
            ));
        }

        if metrics.average_operation_duration_ms > 5000.0 {
            warnings.push(format!(
                "Warning: Slow average response time ({:.1}ms) - consider server optimization",
                metrics.average_operation_duration_ms
            ));
        }

        // Generate recommendations
        if success_rate < 90.0 {
            recommendations.push("Consider increasing retry configuration for WebDAV operations".to_string());
        }

        if timeout_rate > 5.0 {
            recommendations.push("Review WebDAV server timeout configuration and network stability".to_string());
        }

        if metrics.loop_detection_stats.suspected_loop_count > 0 {
            recommendations.push("Implement additional safeguards against directory loop patterns".to_string());
            recommendations.push("Review symlink handling and directory structure validation".to_string());
        }

        if metrics.average_operation_duration_ms > 2000.0 {
            recommendations.push("Consider implementing caching strategies for frequently accessed directories".to_string());
        }

        MetricsSummary {
            success_rate,
            average_response_time: metrics.average_operation_duration_ms,
            max_response_time: metrics.max_operation_duration_ms,
            timeout_rate,
            loop_detection_triggered: metrics.loop_detection_stats.suspected_loop_count > 0,
            total_operations: metrics.total_operations,
        }
    } else {
        warnings.push("Warning: No performance metrics available in the report".to_string());
        MetricsSummary {
            success_rate: 0.0,
            average_response_time: 0.0,
            max_response_time: 0,
            timeout_rate: 0.0,
            loop_detection_triggered: false,
            total_operations: 0,
        }
    };

    // Determine overall health
    let overall_health = if !critical_issues.is_empty() {
        HealthStatus::Critical
    } else if !warnings.is_empty() {
        HealthStatus::Warning
    } else if metrics_summary.total_operations > 0 {
        HealthStatus::Healthy
    } else {
        HealthStatus::Unknown
    };

    Ok(PerformanceAnalysis {
        overall_health,
        critical_issues,
        warnings,
        recommendations,
        metrics_summary,
    })
}

fn generate_json_report(analysis: &PerformanceAnalysis) -> Result<String> {
    let json_report = serde_json::json!({
        "overall_health": format!("{:?}", analysis.overall_health),
        "critical_issues": analysis.critical_issues,
        "warnings": analysis.warnings,
        "recommendations": analysis.recommendations,
        "metrics_summary": {
            "success_rate": analysis.metrics_summary.success_rate,
            "average_response_time_ms": analysis.metrics_summary.average_response_time,
            "max_response_time_ms": analysis.metrics_summary.max_response_time,
            "timeout_rate": analysis.metrics_summary.timeout_rate,
            "loop_detection_triggered": analysis.metrics_summary.loop_detection_triggered,
            "total_operations": analysis.metrics_summary.total_operations,
        }
    });

    Ok(serde_json::to_string_pretty(&json_report)?)
}

fn generate_markdown_report(analysis: &PerformanceAnalysis, report: &StressTestReport) -> Result<String> {
    let mut markdown = String::new();

    markdown.push_str("# WebDAV Performance Analysis Report\n\n");
    
    // Overall status
    markdown.push_str(&format!("**Overall Health Status:** {:?}\n\n", analysis.overall_health));
    markdown.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    // Test summary
    markdown.push_str("## Test Summary\n\n");
    markdown.push_str(&format!("- **Total Tests:** {}\n", report.test_summary.total_tests));
    markdown.push_str(&format!("- **Passed:** {}\n", report.test_summary.passed_tests));
    markdown.push_str(&format!("- **Failed:** {}\n", report.test_summary.failed_tests));
    markdown.push_str(&format!("- **Skipped:** {}\n\n", report.test_summary.skipped_tests));

    // Performance metrics
    markdown.push_str("## Performance Metrics\n\n");
    markdown.push_str(&format!("- **Success Rate:** {:.1}%\n", analysis.metrics_summary.success_rate));
    markdown.push_str(&format!("- **Average Response Time:** {:.1}ms\n", analysis.metrics_summary.average_response_time));
    markdown.push_str(&format!("- **Max Response Time:** {}ms\n", analysis.metrics_summary.max_response_time));
    markdown.push_str(&format!("- **Timeout Rate:** {:.1}%\n", analysis.metrics_summary.timeout_rate));
    markdown.push_str(&format!("- **Total Operations:** {}\n", analysis.metrics_summary.total_operations));
    markdown.push_str(&format!("- **Loop Detection Triggered:** {}\n\n", analysis.metrics_summary.loop_detection_triggered));

    // Critical issues
    if !analysis.critical_issues.is_empty() {
        markdown.push_str("## 🚨 Critical Issues\n\n");
        for issue in &analysis.critical_issues {
            markdown.push_str(&format!("- {}\n", issue));
        }
        markdown.push_str("\n");
    }

    // Warnings
    if !analysis.warnings.is_empty() {
        markdown.push_str("## ⚠️ Warnings\n\n");
        for warning in &analysis.warnings {
            markdown.push_str(&format!("- {}\n", warning));
        }
        markdown.push_str("\n");
    }

    // Recommendations
    if !analysis.recommendations.is_empty() {
        markdown.push_str("## 💡 Recommendations\n\n");
        for recommendation in &analysis.recommendations {
            markdown.push_str(&format!("- {}\n", recommendation));
        }
        markdown.push_str("\n");
    }

    // Write to file for GitHub Actions
    fs::write("webdav-performance-report.md", &markdown)?;

    Ok(markdown)
}

fn generate_github_summary(analysis: &PerformanceAnalysis, report: &StressTestReport) -> Result<String> {
    let mut summary = String::new();

    // Status icon based on health
    let status_icon = match analysis.overall_health {
        HealthStatus::Healthy => "✅",
        HealthStatus::Warning => "⚠️",
        HealthStatus::Critical => "🚨",
        HealthStatus::Unknown => "❓",
    };

    summary.push_str(&format!("{} **WebDAV Stress Test Results**\n\n", status_icon));

    // Quick stats table
    summary.push_str("| Metric | Value |\n");
    summary.push_str("|--------|-------|\n");
    summary.push_str(&format!("| Success Rate | {:.1}% |\n", analysis.metrics_summary.success_rate));
    summary.push_str(&format!("| Total Operations | {} |\n", analysis.metrics_summary.total_operations));
    summary.push_str(&format!("| Avg Response Time | {:.1}ms |\n", analysis.metrics_summary.average_response_time));
    summary.push_str(&format!("| Timeout Rate | {:.1}% |\n", analysis.metrics_summary.timeout_rate));
    summary.push_str(&format!("| Loop Detection | {} |\n", if analysis.metrics_summary.loop_detection_triggered { "⚠️ TRIGGERED" } else { "✅ OK" }));
    summary.push_str("\n");

    // Critical issues (collapsed section)
    if !analysis.critical_issues.is_empty() {
        summary.push_str("<details>\n");
        summary.push_str("<summary>🚨 Critical Issues</summary>\n\n");
        for issue in &analysis.critical_issues {
            summary.push_str(&format!("- {}\n", issue));
        }
        summary.push_str("\n</details>\n\n");
    }

    // Warnings (collapsed section)
    if !analysis.warnings.is_empty() {
        summary.push_str("<details>\n");
        summary.push_str("<summary>⚠️ Warnings</summary>\n\n");
        for warning in &analysis.warnings {
            summary.push_str(&format!("- {}\n", warning));
        }
        summary.push_str("\n</details>\n\n");
    }

    Ok(summary)
}