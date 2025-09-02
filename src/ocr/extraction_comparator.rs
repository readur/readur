use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Configuration for text extraction mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub mode: ExtractionMode,
    pub timeout_seconds: u64,
    pub enable_detailed_logging: bool,
}

/// Extraction modes available for Office documents
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ExtractionMode {
    /// Try library-based extraction first, fallback to XML if it fails (default behavior)
    LibraryFirst,
    /// Try XML-based extraction first, fallback to library if it fails
    XmlFirst,
    /// Always run both extractions and compare results (for analysis)
    CompareAlways,
    /// Use only library-based extraction
    LibraryOnly,
    /// Use only XML-based extraction
    XmlOnly,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            mode: ExtractionMode::LibraryFirst,
            timeout_seconds: 120,
            enable_detailed_logging: false,
        }
    }
}

/// Result from a single extraction method
#[derive(Debug, Clone)]
pub struct SingleExtractionResult {
    pub text: String,
    pub confidence: f32,
    pub processing_time: Duration,
    pub word_count: usize,
    pub method_name: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Detailed comparison metrics between two text extraction methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// Overall similarity score between texts (0.0 to 1.0)
    pub similarity_score: f32,
    /// Levenshtein distance between texts
    pub levenshtein_distance: usize,
    /// Text length difference (absolute)
    pub length_difference: usize,
    /// Word count difference (absolute)
    pub word_count_difference: usize,
    /// Performance comparison
    pub performance_metrics: PerformanceComparison,
    /// Text content analysis
    pub content_analysis: ContentAnalysis,
    /// Method-specific results
    pub library_result: Option<MethodResult>,
    pub xml_result: Option<MethodResult>,
    /// Recommended method based on analysis
    pub recommended_method: String,
    /// Analysis timestamp
    pub timestamp: std::time::SystemTime,
}

/// Performance comparison between methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    /// Processing time difference in milliseconds
    pub time_difference_ms: i64,
    /// Faster method name
    pub faster_method: String,
    /// Speed improvement factor (how many times faster)
    pub speed_improvement_factor: f32,
    /// Memory usage comparison (if available)
    pub memory_usage_difference: Option<i64>,
}

/// Content analysis of extracted texts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysis {
    /// Characters unique to library extraction
    pub library_unique_chars: usize,
    /// Characters unique to XML extraction
    pub xml_unique_chars: usize,
    /// Common characters count
    pub common_chars: usize,
    /// Unique words in library extraction
    pub library_unique_words: usize,
    /// Unique words in XML extraction
    pub xml_unique_words: usize,
    /// Common words count
    pub common_words: usize,
    /// Potential formatting differences detected
    pub formatting_differences: Vec<String>,
}

/// Result summary for a specific extraction method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodResult {
    pub method_name: String,
    pub success: bool,
    pub processing_time_ms: u64,
    pub text_length: usize,
    pub word_count: usize,
    pub confidence: f32,
    pub error_message: Option<String>,
}

/// Main comparison engine for text extraction methods
pub struct ExtractionComparator {
    config: ExtractionConfig,
}

impl ExtractionComparator {
    /// Create a new extraction comparator
    pub fn new(config: ExtractionConfig) -> Self {
        Self { config }
    }
    
    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(ExtractionConfig::default())
    }
    
    /// Compare two extraction results and generate comprehensive analysis
    pub fn compare_extractions(
        &self,
        library_result: Option<SingleExtractionResult>,
        xml_result: Option<SingleExtractionResult>,
    ) -> Result<ComparisonReport> {
        let start_time = Instant::now();
        
        debug!("Starting extraction comparison analysis");
        
        // Validate inputs
        if library_result.is_none() && xml_result.is_none() {
            return Err(anyhow!("At least one extraction result must be provided for comparison"));
        }
        
        let mut report = ComparisonReport {
            similarity_score: 0.0,
            levenshtein_distance: 0,
            length_difference: 0,
            word_count_difference: 0,
            performance_metrics: PerformanceComparison {
                time_difference_ms: 0,
                faster_method: "N/A".to_string(),
                speed_improvement_factor: 1.0,
                memory_usage_difference: None,
            },
            content_analysis: ContentAnalysis {
                library_unique_chars: 0,
                xml_unique_chars: 0,
                common_chars: 0,
                library_unique_words: 0,
                xml_unique_words: 0,
                common_words: 0,
                formatting_differences: Vec::new(),
            },
            library_result: None,
            xml_result: None,
            recommended_method: "Unknown".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        
        // Convert results to method results
        if let Some(ref lib_result) = library_result {
            report.library_result = Some(MethodResult {
                method_name: lib_result.method_name.clone(),
                success: lib_result.success,
                processing_time_ms: lib_result.processing_time.as_millis() as u64,
                text_length: lib_result.text.len(),
                word_count: lib_result.word_count,
                confidence: lib_result.confidence,
                error_message: lib_result.error_message.clone(),
            });
        }
        
        if let Some(ref xml_result) = xml_result {
            report.xml_result = Some(MethodResult {
                method_name: xml_result.method_name.clone(),
                success: xml_result.success,
                processing_time_ms: xml_result.processing_time.as_millis() as u64,
                text_length: xml_result.text.len(),
                word_count: xml_result.word_count,
                confidence: xml_result.confidence,
                error_message: xml_result.error_message.clone(),
            });
        }
        
        // Perform comparison only if both extractions succeeded
        if let (Some(lib_result), Some(xml_result)) = (&library_result, &xml_result) {
            if lib_result.success && xml_result.success {
                // Calculate text similarity
                report.similarity_score = self.calculate_similarity(&lib_result.text, &xml_result.text)?;
                report.levenshtein_distance = self.levenshtein_distance(&lib_result.text, &xml_result.text);
                
                // Calculate differences
                report.length_difference = (lib_result.text.len() as i64 - xml_result.text.len() as i64).abs() as usize;
                report.word_count_difference = (lib_result.word_count as i64 - xml_result.word_count as i64).abs() as usize;
                
                // Performance comparison
                let lib_time_ms = lib_result.processing_time.as_millis() as i64;
                let xml_time_ms = xml_result.processing_time.as_millis() as i64;
                
                report.performance_metrics.time_difference_ms = lib_time_ms - xml_time_ms;
                
                if lib_time_ms < xml_time_ms {
                    report.performance_metrics.faster_method = lib_result.method_name.clone();
                    report.performance_metrics.speed_improvement_factor = xml_time_ms as f32 / lib_time_ms.max(1) as f32;
                } else {
                    report.performance_metrics.faster_method = xml_result.method_name.clone();
                    report.performance_metrics.speed_improvement_factor = lib_time_ms as f32 / xml_time_ms.max(1) as f32;
                }
                
                // Content analysis
                report.content_analysis = self.analyze_content(&lib_result.text, &xml_result.text)?;
                
                // Determine recommended method
                report.recommended_method = self.determine_recommended_method(&report, lib_result, xml_result);
                
                if self.config.enable_detailed_logging {
                    info!(
                        "Extraction comparison completed: similarity={:.2}, levenshtein={}, faster_method={}, speed_improvement={:.2}x",
                        report.similarity_score,
                        report.levenshtein_distance,
                        report.performance_metrics.faster_method,
                        report.performance_metrics.speed_improvement_factor
                    );
                }
            } else {
                // One or both extractions failed
                if lib_result.success {
                    report.recommended_method = lib_result.method_name.clone();
                } else if xml_result.success {
                    report.recommended_method = xml_result.method_name.clone();
                } else {
                    report.recommended_method = "Neither method succeeded".to_string();
                }
            }
        } else if let Some(lib_result) = &library_result {
            report.recommended_method = if lib_result.success {
                lib_result.method_name.clone()
            } else {
                "No successful extraction".to_string()
            };
        } else if let Some(xml_result) = &xml_result {
            report.recommended_method = if xml_result.success {
                xml_result.method_name.clone()
            } else {
                "No successful extraction".to_string()
            };
        }
        
        let analysis_time = start_time.elapsed();
        debug!("Extraction comparison analysis completed in {:?}", analysis_time);
        
        Ok(report)
    }
    
    /// Calculate similarity between two texts using normalized Levenshtein distance
    pub fn calculate_similarity(&self, text1: &str, text2: &str) -> Result<f32> {
        if text1.is_empty() && text2.is_empty() {
            return Ok(1.0);
        }
        
        if text1.is_empty() || text2.is_empty() {
            return Ok(0.0);
        }
        
        // For very large texts (>10K chars), use a more efficient similarity metric
        // The Levenshtein sampling approach gives very inaccurate results
        if text1.len() > 10_000 || text2.len() > 10_000 {
            info!("Using efficient similarity calculation for large texts ({} and {} chars)", 
                  text1.len(), text2.len());
            
            // Use multiple metrics for better accuracy
            
            // 1. Character count similarity
            let char_similarity = 1.0 - ((text1.len() as f32 - text2.len() as f32).abs() 
                                         / text1.len().max(text2.len()) as f32);
            
            // 2. Word count similarity  
            let words1 = text1.split_whitespace().count();
            let words2 = text2.split_whitespace().count();
            let word_similarity = 1.0 - ((words1 as f32 - words2 as f32).abs() 
                                         / words1.max(words2) as f32);
            
            // 3. Sample-based content similarity (compare first and last 5K chars)
            let sample_size = 5000;
            let sample1_start = &text1[..text1.len().min(sample_size)];
            let sample2_start = &text2[..text2.len().min(sample_size)];
            let start_distance = self.levenshtein_distance(sample1_start, sample2_start);
            let start_similarity = 1.0 - (start_distance as f32 / sample1_start.len().max(sample2_start.len()) as f32);
            
            let sample1_end = if text1.len() > sample_size {
                &text1[text1.len() - sample_size..]
            } else {
                text1
            };
            let sample2_end = if text2.len() > sample_size {
                &text2[text2.len() - sample_size..]
            } else {
                text2
            };
            let end_distance = self.levenshtein_distance(sample1_end, sample2_end);
            let end_similarity = 1.0 - (end_distance as f32 / sample1_end.len().max(sample2_end.len()) as f32);
            
            // Weighted average favoring content similarity
            let similarity = (char_similarity * 0.15 + 
                            word_similarity * 0.15 + 
                            start_similarity * 0.35 + 
                            end_similarity * 0.35).min(1.0).max(0.0);
            
            info!("Large text similarity components: char={:.2}, word={:.2}, start={:.2}, end={:.2} -> overall={:.2}",
                  char_similarity, word_similarity, start_similarity, end_similarity, similarity);
            
            return Ok(similarity);
        }
        
        // For smaller texts, use full Levenshtein distance
        let distance = self.levenshtein_distance(text1, text2);
        let max_len = text1.len().max(text2.len());
        
        if max_len == 0 {
            Ok(1.0)
        } else {
            Ok(1.0 - (distance as f32 / max_len as f32))
        }
    }
    
    /// Calculate Levenshtein distance between two strings with memory safety limits
    pub fn levenshtein_distance(&self, text1: &str, text2: &str) -> usize {
        // Memory safety limits to prevent OOM attacks
        const MAX_TEXT_LENGTH: usize = 10_000; // Max 10K characters per text
        const MAX_MATRIX_SIZE: usize = 100_000_000; // Max 100M matrix elements
        
        let len1 = text1.chars().count();
        let len2 = text2.chars().count();
        
        // Early returns for empty strings
        if len1 == 0 {
            return len2.min(MAX_TEXT_LENGTH);
        }
        if len2 == 0 {
            return len1.min(MAX_TEXT_LENGTH);
        }
        
        // Check for potential memory exhaustion
        if len1 > MAX_TEXT_LENGTH || len2 > MAX_TEXT_LENGTH {
            warn!(
                "Text lengths exceed safe limit for Levenshtein calculation: {} and {} chars (max: {}). \
                Using sampling approach to estimate distance.",
                len1, len2, MAX_TEXT_LENGTH
            );
            
            // Use sampling for very large texts to estimate distance
            return self.estimate_levenshtein_distance_for_large_texts(text1, text2, MAX_TEXT_LENGTH);
        }
        
        // Check if matrix would be too large (prevent OOM)
        let matrix_size = (len1 + 1) * (len2 + 1);
        if matrix_size > MAX_MATRIX_SIZE {
            warn!(
                "Matrix size too large for safe Levenshtein calculation: {} elements (max: {}). \
                Using sampling approach to estimate distance.",
                matrix_size, MAX_MATRIX_SIZE
            );
            
            return self.estimate_levenshtein_distance_for_large_texts(text1, text2, MAX_TEXT_LENGTH);
        }
        
        // Safe to proceed with full calculation
        let chars1: Vec<char> = text1.chars().collect();
        let chars2: Vec<char> = text2.chars().collect();
        
        // Use space-optimized approach for large but manageable texts
        if len1 > 1000 || len2 > 1000 {
            return self.levenshtein_distance_space_optimized(&chars1, &chars2);
        }
        
        // Standard algorithm for smaller texts
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
        
        // Initialize first row and column
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }
        
        // Fill the matrix
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                
                matrix[i][j] = (matrix[i - 1][j] + 1)              // deletion
                    .min(matrix[i][j - 1] + 1)                     // insertion
                    .min(matrix[i - 1][j - 1] + cost);             // substitution
            }
        }
        
        matrix[len1][len2]
    }
    
    /// Space-optimized Levenshtein distance calculation using only two rows
    fn levenshtein_distance_space_optimized(&self, chars1: &[char], chars2: &[char]) -> usize {
        let len1 = chars1.len();
        let len2 = chars2.len();
        
        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }
        
        // Use only two rows instead of full matrix to save memory
        let mut prev_row = vec![0; len2 + 1];
        let mut curr_row = vec![0; len2 + 1];
        
        // Initialize first row
        for j in 0..=len2 {
            prev_row[j] = j;
        }
        
        for i in 1..=len1 {
            curr_row[0] = i;
            
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                
                curr_row[j] = (prev_row[j] + 1)              // deletion
                    .min(curr_row[j - 1] + 1)                // insertion
                    .min(prev_row[j - 1] + cost);            // substitution
            }
            
            // Swap rows
            std::mem::swap(&mut prev_row, &mut curr_row);
        }
        
        prev_row[len2]
    }
    
    /// Estimate Levenshtein distance for very large texts using sampling
    fn estimate_levenshtein_distance_for_large_texts(&self, text1: &str, text2: &str, sample_size: usize) -> usize {
        // Sample from beginning, middle, and end of both texts
        let sample1 = self.create_representative_sample(text1, sample_size);
        let sample2 = self.create_representative_sample(text2, sample_size);
        
        // Calculate distance on samples
        let sample_distance = self.levenshtein_distance_space_optimized(
            &sample1.chars().collect::<Vec<_>>(),
            &sample2.chars().collect::<Vec<_>>()
        );
        
        // Extrapolate to full text size (rough approximation)
        let text1_len = text1.chars().count();
        let text2_len = text2.chars().count();
        let max_len = text1_len.max(text2_len);
        let sample_len = sample1.chars().count().max(sample2.chars().count());
        
        if sample_len == 0 {
            return max_len;
        }
        
        // Scale up the sample distance proportionally
        let scaling_factor = max_len as f64 / sample_len as f64;
        let estimated_distance = (sample_distance as f64 * scaling_factor) as usize;
        
        // Cap at maximum possible distance
        estimated_distance.min(max_len)
    }
    
    /// Create a representative sample from a large text
    fn create_representative_sample(&self, text: &str, max_sample_size: usize) -> String {
        let char_count = text.chars().count();
        
        if char_count <= max_sample_size {
            return text.to_string();
        }
        
        // Take samples from beginning, middle, and end
        let chunk_size = max_sample_size / 3;
        let chars: Vec<char> = text.chars().collect();
        
        let mut sample = String::new();
        
        // Beginning
        let begin_end = chunk_size.min(chars.len());
        sample.extend(chars[0..begin_end].iter());
        
        // Middle
        if chars.len() > chunk_size * 2 {
            let mid_start = (chars.len() - chunk_size) / 2;
            let mid_end = (mid_start + chunk_size).min(chars.len());
            sample.extend(chars[mid_start..mid_end].iter());
        }
        
        // End
        if chars.len() > chunk_size {
            let end_start = chars.len().saturating_sub(chunk_size);
            sample.extend(chars[end_start..].iter());
        }
        
        sample
    }
    
    /// Analyze content differences between two texts
    fn analyze_content(&self, library_text: &str, xml_text: &str) -> Result<ContentAnalysis> {
        // Character-level analysis
        let lib_chars: std::collections::HashSet<char> = library_text.chars().collect();
        let xml_chars: std::collections::HashSet<char> = xml_text.chars().collect();
        
        let common_chars = lib_chars.intersection(&xml_chars).count();
        let library_unique_chars = lib_chars.difference(&xml_chars).count();
        let xml_unique_chars = xml_chars.difference(&lib_chars).count();
        
        // Word-level analysis
        let lib_words: std::collections::HashSet<&str> = library_text.split_whitespace().collect();
        let xml_words: std::collections::HashSet<&str> = xml_text.split_whitespace().collect();
        
        let common_words = lib_words.intersection(&xml_words).count();
        let library_unique_words = lib_words.difference(&xml_words).count();
        let xml_unique_words = xml_words.difference(&lib_words).count();
        
        // Detect potential formatting differences
        let mut formatting_differences = Vec::new();
        
        // Check for whitespace differences
        let lib_whitespace_count = library_text.chars().filter(|c| c.is_whitespace()).count();
        let xml_whitespace_count = xml_text.chars().filter(|c| c.is_whitespace()).count();
        
        if (lib_whitespace_count as i64 - xml_whitespace_count as i64).abs() > 10 {
            formatting_differences.push("Significant whitespace differences detected".to_string());
        }
        
        // Check for punctuation differences
        let lib_punct_count = library_text.chars().filter(|c| c.is_ascii_punctuation()).count();
        let xml_punct_count = xml_text.chars().filter(|c| c.is_ascii_punctuation()).count();
        
        if (lib_punct_count as i64 - xml_punct_count as i64).abs() > 5 {
            formatting_differences.push("Punctuation differences detected".to_string());
        }
        
        // Check for potential encoding issues
        if library_text.contains('�') || xml_text.contains('�') {
            formatting_differences.push("Potential character encoding issues detected".to_string());
        }
        
        Ok(ContentAnalysis {
            library_unique_chars,
            xml_unique_chars,
            common_chars,
            library_unique_words,
            xml_unique_words,
            common_words,
            formatting_differences,
        })
    }
    
    /// Determine the recommended extraction method based on comparison results
    fn determine_recommended_method(
        &self,
        report: &ComparisonReport,
        library_result: &SingleExtractionResult,
        xml_result: &SingleExtractionResult,
    ) -> String {
        // If one method failed, recommend the successful one
        if !library_result.success && xml_result.success {
            return xml_result.method_name.clone();
        }
        if library_result.success && !xml_result.success {
            return library_result.method_name.clone();
        }
        if !library_result.success && !xml_result.success {
            return "Neither method succeeded".to_string();
        }
        
        // Both methods succeeded, analyze quality
        let mut library_score = 0.0;
        let mut xml_score = 0.0;
        
        // Factor 1: Text length (longer is generally better for document extraction)
        if library_result.text.len() > xml_result.text.len() {
            library_score += 1.0;
        } else if xml_result.text.len() > library_result.text.len() {
            xml_score += 1.0;
        }
        
        // Factor 2: Word count (more words usually means better extraction)
        if library_result.word_count > xml_result.word_count {
            library_score += 1.0;
        } else if xml_result.word_count > library_result.word_count {
            xml_score += 1.0;
        }
        
        // Factor 3: Processing speed (faster is better, but weight it less)
        if library_result.processing_time < xml_result.processing_time {
            library_score += 0.5;
        } else if xml_result.processing_time < library_result.processing_time {
            xml_score += 0.5;
        }
        
        // Factor 4: Confidence score
        if library_result.confidence > xml_result.confidence {
            library_score += 0.5;
        } else if xml_result.confidence > library_result.confidence {
            xml_score += 0.5;
        }
        
        // Factor 5: Content richness (unique content might indicate better extraction)
        if report.content_analysis.library_unique_chars > report.content_analysis.xml_unique_chars {
            library_score += 0.3;
        } else if report.content_analysis.xml_unique_chars > report.content_analysis.library_unique_chars {
            xml_score += 0.3;
        }
        
        // Determine winner
        if library_score > xml_score {
            library_result.method_name.clone()
        } else if xml_score > library_score {
            xml_result.method_name.clone()
        } else {
            // Tie - default to library method as it's typically more mature
            format!("Tie (defaulting to {})", library_result.method_name)
        }
    }
    
    /// Get a summary of differences between two texts
    pub fn get_text_differences(&self, text1: &str, text2: &str, max_diff_lines: usize) -> Vec<String> {
        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();
        
        let mut differences = Vec::new();
        let max_lines = lines1.len().max(lines2.len());
        
        for i in 0..max_lines.min(max_diff_lines) {
            let line1 = lines1.get(i).unwrap_or(&"");
            let line2 = lines2.get(i).unwrap_or(&"");
            
            if line1 != line2 {
                if line1.is_empty() {
                    differences.push(format!("Line {}: Added in method 2: '{}'", i + 1, line2));
                } else if line2.is_empty() {
                    differences.push(format!("Line {}: Removed in method 2: '{}'", i + 1, line1));
                } else {
                    differences.push(format!("Line {}: '{}' -> '{}'", i + 1, line1, line2));
                }
            }
        }
        
        if max_lines > max_diff_lines {
            differences.push(format!("... ({} more lines not shown)", max_lines - max_diff_lines));
        }
        
        differences
    }
}

impl From<SingleExtractionResult> for super::enhanced::OcrResult {
    /// Convert SingleExtractionResult to OcrResult for compatibility
    fn from(result: SingleExtractionResult) -> Self {
        super::enhanced::OcrResult {
            text: result.text,
            confidence: result.confidence,
            processing_time_ms: result.processing_time.as_millis() as u64,
            word_count: result.word_count,
            preprocessing_applied: vec![result.method_name],
            processed_image_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    fn create_test_result(text: &str, method: &str, time_ms: u64, success: bool) -> SingleExtractionResult {
        SingleExtractionResult {
            text: text.to_string(),
            confidence: if success { 95.0 } else { 0.0 },
            processing_time: Duration::from_millis(time_ms),
            word_count: text.split_whitespace().count(),
            method_name: method.to_string(),
            success,
            error_message: if success { None } else { Some("Test error".to_string()) },
        }
    }
    
    #[test]
    fn test_levenshtein_distance() {
        let comparator = ExtractionComparator::default();
        
        // Identical strings
        assert_eq!(comparator.levenshtein_distance("hello", "hello"), 0);
        
        // One character difference
        assert_eq!(comparator.levenshtein_distance("hello", "hallo"), 1);
        
        // Empty strings
        assert_eq!(comparator.levenshtein_distance("", ""), 0);
        assert_eq!(comparator.levenshtein_distance("hello", ""), 5);
        assert_eq!(comparator.levenshtein_distance("", "world"), 5);
        
        // Completely different
        assert_eq!(comparator.levenshtein_distance("abc", "xyz"), 3);
    }
    
    #[test]
    fn test_calculate_similarity() {
        let comparator = ExtractionComparator::default();
        
        // Identical strings should have similarity 1.0
        let sim = comparator.calculate_similarity("hello world", "hello world").unwrap();
        assert!((sim - 1.0).abs() < 0.01);
        
        // Completely different strings should have low similarity
        let sim = comparator.calculate_similarity("abc", "xyz").unwrap();
        assert!(sim < 0.5);
        
        // Empty strings
        let sim = comparator.calculate_similarity("", "").unwrap();
        assert!((sim - 1.0).abs() < 0.01);
        
        let sim = comparator.calculate_similarity("hello", "").unwrap();
        assert!((sim - 0.0).abs() < 0.01);
    }
    
    #[test]
    fn test_compare_extractions_both_successful() {
        let comparator = ExtractionComparator::default();
        
        let lib_result = create_test_result("Hello world test document", "Library", 100, true);
        let xml_result = create_test_result("Hello world test document", "XML", 150, true);
        
        let report = comparator.compare_extractions(Some(lib_result), Some(xml_result)).unwrap();
        
        assert!((report.similarity_score - 1.0).abs() < 0.01); // Identical text
        assert_eq!(report.levenshtein_distance, 0);
        assert_eq!(report.performance_metrics.faster_method, "Library");
        assert!(report.performance_metrics.speed_improvement_factor > 1.0);
    }
    
    #[test]
    fn test_compare_extractions_one_failed() {
        let comparator = ExtractionComparator::default();
        
        let lib_result = create_test_result("Hello world", "Library", 100, true);
        let xml_result = create_test_result("", "XML", 0, false);
        
        let report = comparator.compare_extractions(Some(lib_result), Some(xml_result)).unwrap();
        
        assert_eq!(report.recommended_method, "Library");
        assert!(report.library_result.is_some());
        assert!(report.xml_result.is_some());
        assert!(report.library_result.as_ref().unwrap().success);
        assert!(!report.xml_result.as_ref().unwrap().success);
    }
    
    #[test]
    fn test_get_text_differences() {
        let comparator = ExtractionComparator::default();
        
        let text1 = "Line 1\nLine 2\nLine 3";
        let text2 = "Line 1\nModified Line 2\nLine 3\nNew Line 4";
        
        let differences = comparator.get_text_differences(text1, text2, 10);
        
        assert!(differences.len() >= 1);
        assert!(differences.iter().any(|d| d.contains("Modified Line 2")));
    }
    
    #[test]
    fn test_content_analysis() {
        let comparator = ExtractionComparator::default();
        
        let lib_text = "Hello world! This is a test.";
        let xml_text = "Hello world? This was a test!";
        
        let analysis = comparator.analyze_content(lib_text, xml_text).unwrap();
        
        assert!(analysis.common_chars > 0);
        assert!(analysis.common_words > 0);
        assert!(analysis.library_unique_chars > 0 || analysis.xml_unique_chars > 0);
    }
}