use anyhow::{anyhow, Result};
use tracing::{info, warn};
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::time::{timeout, Duration};
use super::enhanced::OcrResult;

/// Result structure for Office document text extraction
#[derive(Debug, Clone)]
pub struct OfficeExtractionResult {
    pub text: String,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub word_count: usize,
    pub extraction_method: String,
}

impl From<OfficeExtractionResult> for OcrResult {
    /// Convert OfficeExtractionResult to OcrResult for compatibility with the main OCR service
    fn from(office_result: OfficeExtractionResult) -> Self {
        OcrResult {
            text: office_result.text,
            confidence: office_result.confidence,
            processing_time_ms: office_result.processing_time_ms,
            word_count: office_result.word_count,
            preprocessing_applied: vec![office_result.extraction_method],
            processed_image_path: None, // XML extraction doesn't produce processed images
        }
    }
}

/// Extraction context for tracking progress and supporting cancellation
pub struct ExtractionContext {
    /// Flag to indicate if the operation should be cancelled
    pub cancelled: Arc<AtomicBool>,
    /// Total decompressed size across all ZIP entries (for ZIP bomb protection)
    pub total_decompressed_size: Arc<AtomicU64>,
    /// Maximum allowed total decompressed size
    pub max_total_decompressed_size: u64,
}

impl ExtractionContext {
    pub fn new(max_total_decompressed_size: u64) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            total_decompressed_size: Arc::new(AtomicU64::new(0)),
            max_total_decompressed_size,
        }
    }
    
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
    
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    
    pub fn add_decompressed_bytes(&self, bytes: u64) -> Result<()> {
        let new_total = self.total_decompressed_size.fetch_add(bytes, Ordering::SeqCst) + bytes;
        if new_total > self.max_total_decompressed_size {
            return Err(anyhow!(
                "Total decompressed size ({:.1} MB) exceeds maximum allowed ({:.1} MB). \
                This may be a ZIP bomb attack attempting to exhaust system resources.",
                new_total as f64 / (1024.0 * 1024.0),
                self.max_total_decompressed_size as f64 / (1024.0 * 1024.0)
            ));
        }
        Ok(())
    }
}

/// XML-based Office document extractor with security features
pub struct XmlOfficeExtractor {
    /// Temporary directory for file processing
    pub temp_dir: String,
}

impl XmlOfficeExtractor {
    // Security limits to prevent ZIP bombs and memory exhaustion attacks
    const MAX_DECOMPRESSED_SIZE: u64 = 100 * 1024 * 1024; // 100MB total decompressed size across all entries
    const MAX_XML_SIZE: u64 = 10 * 1024 * 1024; // 10MB per XML file
    const MAX_ZIP_ENTRIES: usize = 1000; // Maximum number of entries to process
    const MAX_ENTRY_NAME_LENGTH: usize = 255; // Maximum length of entry names
    const MAX_OFFICE_SIZE: u64 = 50 * 1024 * 1024; // 50MB max Office document size
    
    // Operation timeout constants
    const DEFAULT_TIMEOUT_SECONDS: u64 = 120; // 2 minutes default timeout
    const MAX_TIMEOUT_SECONDS: u64 = 600; // 10 minutes maximum timeout
    
    // XML processing constants
    const XML_READ_BUFFER_SIZE: usize = 8192; // 8KB chunks for reading
    const MAX_WORKSHEETS_TO_CHECK: usize = 50; // Maximum worksheets to check in Excel files
    const WORD_LENGTH_ESTIMATE: usize = 5; // Average characters per word for estimation
    const MAX_WORD_COUNT_DISPLAY: usize = 10_000_000; // Maximum word count to prevent display issues
    
    // XML entity limits to prevent expansion attacks
    const MAX_ENTITY_EXPANSIONS: usize = 1000; // Maximum number of entity expansions
    const MAX_ENTITY_DEPTH: usize = 10; // Maximum depth of nested entity references

    /// Create a new XML Office extractor
    pub fn new(temp_dir: String) -> Self {
        Self { temp_dir }
    }
    
    /// Create a secure XML reader with protection against entity expansion attacks
    fn create_secure_xml_reader(xml_content: &str) -> quick_xml::Reader<&[u8]> {
        use quick_xml::Reader;
        
        let mut reader = Reader::from_str(xml_content);
        let config = reader.config_mut();
        
        // Security configurations to prevent XML attacks
        config.trim_text(true);
        config.check_end_names = false; // Performance: disable end name checking
        config.expand_empty_elements = false; // Security: don't expand empty elements
        
        // Note: quick-xml doesn't support external entity expansion by default,
        // but we're being explicit about security configurations
        
        reader
    }
    
    /// Parse workbook.xml to get actual worksheet references instead of guessing
    fn get_worksheet_names_from_workbook(archive: &mut zip::ZipArchive<std::fs::File>, context: &ExtractionContext) -> Result<Vec<String>> {
        use quick_xml::events::Event;
        
        // Try to read workbook.xml
        let mut workbook_file = match archive.by_name("xl/workbook.xml") {
            Ok(file) => file,
            Err(_) => {
                // Fall back to the old method if workbook.xml doesn't exist
                warn!("workbook.xml not found, falling back to sequential worksheet detection");
                return Ok((1..=Self::MAX_WORKSHEETS_TO_CHECK)
                    .map(|i| format!("sheet{}.xml", i))
                    .collect());
            }
        };
        
        let xml_content = Self::read_zip_entry_safely(&mut workbook_file, Self::MAX_XML_SIZE, context)?;
        drop(workbook_file);
        
        let mut reader = Self::create_secure_xml_reader(&xml_content);
        
        let mut worksheets = Vec::new();
        let mut buf = Vec::new();
        
        // Parse workbook.xml to find sheet references
        loop {
            if context.is_cancelled() {
                return Err(anyhow!("Operation cancelled while parsing workbook.xml"));
            }
            
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if e.name().as_ref() == b"sheet" {
                        // Look for the r:id attribute to get the worksheet relationship
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                if attr.key.as_ref() == b"r:id" {
                                    let sheet_id = String::from_utf8_lossy(&attr.value);
                                    // Convert relationship ID to worksheet filename
                                    // Typical pattern: rId1 -> sheet1.xml, rId2 -> sheet2.xml, etc.
                                    if let Some(sheet_num) = sheet_id.strip_prefix("rId") {
                                        worksheets.push(format!("sheet{}.xml", sheet_num));
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!("Error parsing workbook.xml, falling back to sequential detection: {}", e);
                    // Fall back to old method on parse error
                    return Ok((1..=Self::MAX_WORKSHEETS_TO_CHECK)
                        .map(|i| format!("sheet{}.xml", i))
                        .collect());
                }
                _ => {}
            }
            buf.clear();
        }
        
        if worksheets.is_empty() {
            // Fall back if no worksheets found
            warn!("No worksheets found in workbook.xml, falling back to sequential detection");
            Ok((1..=Self::MAX_WORKSHEETS_TO_CHECK)
                .map(|i| format!("sheet{}.xml", i))
                .collect())
        } else {
            info!("Found {} worksheets in workbook.xml", worksheets.len());
            Ok(worksheets)
        }
    }

    /// Remove null bytes from text to prevent PostgreSQL errors
    /// This is the ONLY sanitization we do - preserving all other original content
    fn remove_null_bytes(text: &str) -> String {
        let original_len = text.len();
        let cleaned: String = text.chars().filter(|&c| c != '\0').collect();
        
        // Log if we found and removed null bytes (shouldn't happen with valid documents)
        let cleaned_len = cleaned.len();
        if cleaned_len < original_len {
            let null_bytes_removed = text.chars().filter(|&c| c == '\0').count();
            warn!(
                "Removed {} null bytes from extracted text (original: {} chars, cleaned: {} chars). \
                This indicates corrupted or malformed document data.",
                null_bytes_removed, original_len, cleaned_len
            );
        }
        
        cleaned
    }

    /// Validates ZIP entry names to prevent directory traversal attacks
    fn validate_zip_entry_name(entry_name: &str) -> Result<()> {
        // Check entry name length
        if entry_name.len() > Self::MAX_ENTRY_NAME_LENGTH {
            return Err(anyhow!(
                "ZIP entry name too long ({}). Maximum allowed length is {} characters for security reasons.",
                entry_name.len(),
                Self::MAX_ENTRY_NAME_LENGTH
            ));
        }

        // Check for directory traversal attempts
        if entry_name.contains("..") {
            return Err(anyhow!(
                "ZIP entry contains directory traversal sequence '..': '{}'. This is blocked for security reasons.",
                entry_name
            ));
        }

        // Check for absolute paths
        if entry_name.starts_with('/') || entry_name.starts_with('\\') {
            return Err(anyhow!(
                "ZIP entry contains absolute path: '{}'. This is blocked for security reasons.",
                entry_name
            ));
        }

        // Check for Windows drive letters
        if entry_name.len() >= 2 && entry_name.chars().nth(1) == Some(':') {
            return Err(anyhow!(
                "ZIP entry contains Windows drive letter: '{}'. This is blocked for security reasons.",
                entry_name
            ));
        }

        // Check for suspicious characters
        let suspicious_chars = ['<', '>', '|', '*', '?'];
        if entry_name.chars().any(|c| suspicious_chars.contains(&c)) {
            return Err(anyhow!(
                "ZIP entry contains suspicious characters: '{}'. This is blocked for security reasons.",
                entry_name
            ));
        }

        Ok(())
    }

    /// Safely reads content from a ZIP entry with size limits and cancellation support
    fn read_zip_entry_safely<R: std::io::Read>(
        reader: &mut R, 
        max_size: u64, 
        context: &ExtractionContext
    ) -> Result<String> {
        use std::io::Read;
        
        let mut buffer = Vec::new();
        let mut total_read = 0u64;
        let mut temp_buf = [0u8; Self::XML_READ_BUFFER_SIZE];
        
        loop {
            // Check for cancellation
            if context.is_cancelled() {
                return Err(anyhow!("Operation cancelled by user"));
            }
            
            match reader.read(&mut temp_buf)? {
                0 => break, // EOF
                bytes_read => {
                    total_read += bytes_read as u64;
                    
                    // Check if we've exceeded the per-file size limit
                    if total_read > max_size {
                        return Err(anyhow!(
                            "ZIP entry content exceeds maximum allowed size of {:.1} MB. \
                            This may be a ZIP bomb attack. Current size: {:.1} MB.",
                            max_size as f64 / (1024.0 * 1024.0),
                            total_read as f64 / (1024.0 * 1024.0)
                        ));
                    }
                    
                    // Update total decompressed size across all entries
                    context.add_decompressed_bytes(bytes_read as u64)?;
                    
                    buffer.extend_from_slice(&temp_buf[..bytes_read]);
                }
            }
        }
        
        // Convert to string, handling encoding issues gracefully
        String::from_utf8(buffer).or_else(|e| {
            // Try to recover as much valid UTF-8 as possible
            let bytes = e.into_bytes();
            let lossy = String::from_utf8_lossy(&bytes);
            Ok(lossy.into_owned())
        })
    }

    /// Extract text from Office documents using XML parsing with timeout and cancellation support
    pub async fn extract_text_from_office(&self, file_path: &str, mime_type: &str) -> Result<OfficeExtractionResult> {
        self.extract_text_from_office_with_timeout(file_path, mime_type, Self::DEFAULT_TIMEOUT_SECONDS).await
    }
    
    /// Extract text from Office documents with custom timeout
    pub async fn extract_text_from_office_with_timeout(
        &self, 
        file_path: &str, 
        mime_type: &str,
        timeout_seconds: u64
    ) -> Result<OfficeExtractionResult> {
        let timeout_duration = Duration::from_secs(timeout_seconds.min(Self::MAX_TIMEOUT_SECONDS));
        
        let extraction_future = self.extract_text_from_office_internal(file_path, mime_type);
        
        match timeout(timeout_duration, extraction_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!(
                "Office document text extraction timed out after {} seconds for file '{}'. \
                The document may be very large or complex. Consider:\n\
                1. Converting to PDF format first\n\
                2. Splitting large documents into smaller parts\n\
                3. Increasing the timeout if this is expected behavior",
                timeout_seconds,
                file_path
            ))
        }
    }
    
    /// Internal extraction method with cancellation support
    async fn extract_text_from_office_internal(&self, file_path: &str, mime_type: &str) -> Result<OfficeExtractionResult> {
        let start_time = Instant::now();
        info!("Extracting text from Office document: {} (type: {})", file_path, mime_type);
        
        // Check file size before processing
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len();
        
        if file_size > Self::MAX_OFFICE_SIZE {
            return Err(anyhow!(
                "Office document too large: {:.1} MB (max: {:.1} MB). Consider converting to PDF or splitting the document.",
                file_size as f64 / (1024.0 * 1024.0),
                Self::MAX_OFFICE_SIZE as f64 / (1024.0 * 1024.0)
            ));
        }
        
        // Create extraction context for ZIP bomb protection and cancellation support
        let context = ExtractionContext::new(Self::MAX_DECOMPRESSED_SIZE);
        
        match mime_type {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                self.extract_text_from_docx(file_path, start_time, &context).await
            }
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                self.extract_text_from_xlsx(file_path, start_time, &context).await
            }
            "application/msword" => {
                self.extract_text_from_legacy_doc(file_path, start_time).await
            }
            "application/vnd.ms-excel" => {
                self.extract_text_from_legacy_excel(file_path, start_time).await
            }
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
                // For PPTX, provide guidance for now as it's complex
                Err(anyhow!(
                    "PowerPoint files (PPTX) are not yet supported for text extraction. \
                    To extract content from '{}', please:\n\
                    1. Export/Print the presentation as PDF (recommended)\n\
                    2. Use 'File' > 'Export' > 'Create Handouts' in PowerPoint\n\
                    3. Copy text content from slides into a text document\n\
                    \nPDF export will preserve both text and visual elements.",
                    file_path
                ))
            }
            _ => {
                Err(anyhow!(
                    "Office document type '{}' is not supported for text extraction (file: {}). \
                    Please convert the document to PDF format or plain text for processing.",
                    mime_type, file_path
                ))
            }
        }
    }

    /// Extract text from DOCX files using ZIP + XML parsing
    async fn extract_text_from_docx(&self, file_path: &str, start_time: Instant, context: &ExtractionContext) -> Result<OfficeExtractionResult> {
        info!("Starting DOCX text extraction: {}", file_path);
        
        // Move CPU-intensive operations to blocking thread pool
        let file_path_clone = file_path.to_string();
        let context_clone = ExtractionContext::new(context.max_total_decompressed_size);
        let extraction_result = tokio::task::spawn_blocking(move || -> Result<String> {
            use zip::ZipArchive;
            use quick_xml::events::Event;
            
            // Open the DOCX file as a ZIP archive
            let file = std::fs::File::open(&file_path_clone)?;
            let mut archive = ZipArchive::new(file)?;
            
            // Security check: Validate ZIP archive structure
            let entry_count = archive.len();
            if entry_count > Self::MAX_ZIP_ENTRIES {
                return Err(anyhow!(
                    "ZIP archive contains too many entries ({}). Maximum allowed is {} for security reasons. \
                    This may be a ZIP bomb attack.",
                    entry_count,
                    Self::MAX_ZIP_ENTRIES
                ));
            }

            // Validate all entry names before processing to prevent directory traversal
            for i in 0..entry_count {
                let entry = archive.by_index(i)?;
                let entry_name = entry.name();
                Self::validate_zip_entry_name(entry_name)?;
            }
            
            // Try to extract the main document content from word/document.xml
            let mut document_xml = match archive.by_name("word/document.xml") {
                Ok(file) => file,
                Err(_) => {
                    return Err(anyhow!(
                        "Invalid DOCX file: missing word/document.xml. The file '{}' may be corrupted or not a valid DOCX document.",
                        file_path_clone
                    ));
                }
            };
            
            // Security: Use size-limited reading to prevent ZIP bomb attacks
            let xml_content = Self::read_zip_entry_safely(&mut document_xml, Self::MAX_XML_SIZE, &context_clone)?;
            drop(document_xml); // Close the archive entry
            
            // Parse the XML and extract text content
            let mut reader = Self::create_secure_xml_reader(&xml_content);
            
            let mut text_content = Vec::new();
            let mut in_text_element = false;
            let mut buf = Vec::new();
            
            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        // Look for text elements (w:t tags contain the actual text)
                        if e.name().as_ref() == b"w:t" {
                            in_text_element = true;
                        }
                    }
                    Ok(Event::Text(e)) => {
                        if in_text_element {
                            // Extract and decode the text content
                            let text = e.unescape().map_err(|e| anyhow!("Text unescape error: {}", e))?;
                            text_content.push(text.into_owned());
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        if e.name().as_ref() == b"w:t" {
                            in_text_element = false;
                        }
                        // Add space after paragraph breaks
                        if e.name().as_ref() == b"w:p" {
                            text_content.push(" ".to_string());
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => {
                        return Err(anyhow!(
                            "XML parsing error in DOCX file '{}': {}. The file may be corrupted.",
                            file_path_clone, e
                        ));
                    }
                    _ => {}
                }
                buf.clear();
            }
            
            // Join all text content
            let raw_text = text_content.join("");
            
            if raw_text.trim().is_empty() {
                return Err(anyhow!(
                    "No text content found in DOCX file '{}'. The document may be empty or contain only images/objects.",
                    file_path_clone
                ));
            }
            
            Ok(raw_text)
            
        }).await??;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        // Only remove null bytes - preserve all original formatting
        let cleaned_text = Self::remove_null_bytes(&extraction_result);
        let word_count = self.count_words_safely(&cleaned_text);
        
        info!(
            "DOCX extraction completed: {} words extracted from '{}' in {}ms",
            word_count, file_path, processing_time
        );
        
        Ok(OfficeExtractionResult {
            text: cleaned_text,
            confidence: 100.0, // Direct text extraction has perfect confidence
            processing_time_ms: processing_time,
            word_count,
            extraction_method: "DOCX XML extraction".to_string(),
        })
    }

    /// Extract text from XLSX files using ZIP + XML parsing
    async fn extract_text_from_xlsx(&self, file_path: &str, start_time: Instant, context: &ExtractionContext) -> Result<OfficeExtractionResult> {
        info!("Starting XLSX text extraction: {}", file_path);
        
        // Move CPU-intensive operations to blocking thread pool
        let file_path_clone = file_path.to_string();
        let context_clone = ExtractionContext::new(context.max_total_decompressed_size);
        let extraction_result = tokio::task::spawn_blocking(move || -> Result<String> {
            use zip::ZipArchive;
            use quick_xml::events::Event;
            
            // Open the XLSX file as a ZIP archive
            let file = std::fs::File::open(&file_path_clone)?;
            let mut archive = ZipArchive::new(file)?;
            
            // Security check: Validate ZIP archive structure
            let entry_count = archive.len();
            if entry_count > Self::MAX_ZIP_ENTRIES {
                return Err(anyhow!(
                    "ZIP archive contains too many entries ({}). Maximum allowed is {} for security reasons. \
                    This may be a ZIP bomb attack.",
                    entry_count,
                    Self::MAX_ZIP_ENTRIES
                ));
            }

            // Validate all entry names before processing to prevent directory traversal
            for i in 0..entry_count {
                let entry = archive.by_index(i)?;
                let entry_name = entry.name();
                Self::validate_zip_entry_name(entry_name)?;
            }
            
            // First, extract shared strings (xl/sharedStrings.xml)
            let mut shared_strings = Vec::new();
            if let Ok(mut shared_strings_file) = archive.by_name("xl/sharedStrings.xml") {
                // Security: Use size-limited reading to prevent ZIP bomb attacks
                let xml_content = Self::read_zip_entry_safely(&mut shared_strings_file, Self::MAX_XML_SIZE, &context_clone)?;
                drop(shared_strings_file);
                
                // Parse shared strings
                let mut reader = Self::create_secure_xml_reader(&xml_content);
                let mut buf = Vec::new();
                let mut in_string = false;
                let mut current_string = String::new();
                
                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Start(ref e)) => {
                            if e.name().as_ref() == b"t" {
                                in_string = true;
                                current_string.clear();
                            }
                        }
                        Ok(Event::Text(e)) => {
                            if in_string {
                                let text = e.unescape().map_err(|e| anyhow!("Text unescape error: {}", e))?;
                                current_string.push_str(&text);
                            }
                        }
                        Ok(Event::End(ref e)) => {
                            if e.name().as_ref() == b"t" {
                                in_string = false;
                                shared_strings.push(current_string.clone());
                                current_string.clear();
                            }
                        }
                        Ok(Event::Eof) => break,
                        Err(e) => {
                            return Err(anyhow!(
                                "XML parsing error in Excel shared strings: {}. The file may be corrupted.",
                                e
                            ));
                        }
                        _ => {}
                    }
                    buf.clear();
                }
            }
            
            // Now extract worksheet data
            let mut all_text = Vec::new();
            let mut worksheet_count = 0;
            
            // Get actual worksheet names from workbook.xml instead of guessing
            let worksheet_names = Self::get_worksheet_names_from_workbook(&mut archive, &context_clone)?;
            
            // Process each worksheet
            for worksheet_filename in worksheet_names {
                let worksheet_path = format!("xl/worksheets/{}", worksheet_filename);
                
                if let Ok(mut worksheet_file) = archive.by_name(&worksheet_path) {
                    worksheet_count += 1;
                    // Security: Use size-limited reading to prevent ZIP bomb attacks
                    let xml_content = Self::read_zip_entry_safely(&mut worksheet_file, Self::MAX_XML_SIZE, &context_clone)?;
                    drop(worksheet_file);
                    
                    // Parse worksheet data
                    let mut reader = Self::create_secure_xml_reader(&xml_content);
                    let mut buf = Vec::new();
                    let mut in_cell_value = false;
                    let mut current_cell_type = String::new();
                    
                    loop {
                        match reader.read_event_into(&mut buf) {
                            Ok(Event::Start(ref e)) => {
                                if e.name().as_ref() == b"c" {
                                    // Cell element - check if it has a type attribute
                                    current_cell_type.clear();
                                    for attr in e.attributes() {
                                        if let Ok(attr) = attr {
                                            if attr.key.as_ref() == b"t" {
                                                current_cell_type = String::from_utf8_lossy(&attr.value).to_string();
                                            }
                                        }
                                    }
                                } else if e.name().as_ref() == b"v" {
                                    // Cell value
                                    in_cell_value = true;
                                }
                            }
                            Ok(Event::Text(e)) => {
                                if in_cell_value {
                                    let text = e.unescape().map_err(|e| anyhow!("Text unescape error: {}", e))?;
                                    
                                    // If this is a shared string reference (t="s"), look up the string
                                    if current_cell_type == "s" {
                                        if let Ok(index) = text.parse::<usize>() {
                                            if let Some(shared_string) = shared_strings.get(index) {
                                                all_text.push(shared_string.clone());
                                            }
                                        }
                                    } else {
                                        // Direct value
                                        all_text.push(text.into_owned());
                                    }
                                }
                            }
                            Ok(Event::End(ref e)) => {
                                if e.name().as_ref() == b"v" {
                                    in_cell_value = false;
                                }
                            }
                            Ok(Event::Eof) => break,
                            Err(e) => {
                                return Err(anyhow!(
                                    "XML parsing error in Excel worksheet {}: {}. The file may be corrupted.",
                                    worksheet_path, e
                                ));
                            }
                            _ => {}
                        }
                        buf.clear();
                    }
                }
            }
            
            if worksheet_count == 0 {
                return Err(anyhow!(
                    "Invalid XLSX file: no worksheets found in '{}'. The file may be corrupted or not a valid Excel document.",
                    file_path_clone
                ));
            }
            
            // Join all text content with spaces
            let raw_text = all_text.join(" ");
            
            if raw_text.trim().is_empty() {
                return Err(anyhow!(
                    "No text content found in Excel file '{}'. The spreadsheet may be empty or contain only formulas/formatting.",
                    file_path_clone
                ));
            }
            
            Ok(raw_text)
            
        }).await??;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        // Only remove null bytes - preserve all original formatting
        let cleaned_text = Self::remove_null_bytes(&extraction_result);
        let word_count = self.count_words_safely(&cleaned_text);
        
        info!(
            "XLSX extraction completed: {} words extracted from '{}' in {}ms",
            word_count, file_path, processing_time
        );
        
        Ok(OfficeExtractionResult {
            text: cleaned_text,
            confidence: 100.0, // Direct text extraction has perfect confidence
            processing_time_ms: processing_time,
            word_count,
            extraction_method: "XLSX XML extraction".to_string(),
        })
    }

    /// Extract text from legacy DOC files - provide guidance for now
    async fn extract_text_from_legacy_doc(&self, file_path: &str, start_time: Instant) -> Result<OfficeExtractionResult> {
        info!("Processing legacy DOC file: {}", file_path);
        
        let _processing_time = start_time.elapsed().as_millis() as u64;
        
        // Legacy DOC files are complex binary format, suggest conversion
        Err(anyhow!(
            "Legacy Word files (.doc) are not directly supported for text extraction due to their complex binary format. \
            To process the content from '{}', please:\n\
            1. Open the file in Microsoft Word, LibreOffice Writer, or Google Docs\n\
            2. Save/Export as DOCX format (recommended) or PDF\n\
            3. Alternatively, install external tools like antiword or catdoc\n\
            \nDOCX format provides better compatibility and more reliable text extraction.",
            file_path
        ))
    }

    /// Extract text from legacy Excel files - provide guidance for now
    async fn extract_text_from_legacy_excel(&self, file_path: &str, start_time: Instant) -> Result<OfficeExtractionResult> {
        info!("Processing legacy Excel (XLS) file: {}", file_path);
        
        let _processing_time = start_time.elapsed().as_millis() as u64;
        
        // Legacy XLS files are complex binary format, suggest conversion
        Err(anyhow!(
            "Legacy Excel files (.xls) are not directly supported for text extraction due to their complex binary format. \
            To process the content from '{}', please:\n\
            1. Open the file in Microsoft Excel, LibreOffice Calc, or Google Sheets\n\
            2. Save/Export as XLSX format (recommended) or CSV\n\
            3. Alternatively, export as PDF to preserve formatting\n\
            \nXLSX format provides better compatibility and more reliable text extraction.",
            file_path
        ))
    }

    /// Safely count words to prevent overflow on very large texts
    pub fn count_words_safely(&self, text: &str) -> usize {
        // For very large texts, sample to estimate word count to prevent overflow
        if text.len() > 1_000_000 { // > 1MB of text
            // Sample first 100KB and extrapolate
            let sample_size = 100_000;
            let sample_text = &text[..sample_size.min(text.len())];
            let sample_words = self.count_words_in_text(sample_text);
            let estimated_total = (sample_words as f64 * (text.len() as f64 / sample_size as f64)) as usize;
            
            // Cap at reasonable maximum to prevent display issues
            estimated_total.min(10_000_000) // Max 10M words
        } else {
            self.count_words_in_text(text)
        }
    }

    fn count_words_in_text(&self, text: &str) -> usize {
        let whitespace_words = text.split_whitespace().count();
        
        // If we have exactly 1 "word" but it's very long (likely continuous text), try enhanced detection
        // OR if we have no whitespace words but text exists
        let is_continuous_text = whitespace_words == 1 && text.len() > 15; // 15+ chars suggests it might be continuous
        let is_no_words = whitespace_words == 0 && !text.trim().is_empty();
        
        if is_continuous_text || is_no_words {
            // Count total alphanumeric characters first
            let alphanumeric_chars = text.chars().filter(|c| c.is_alphanumeric()).count();
            
            // If no alphanumeric content, it's pure punctuation/symbols
            if alphanumeric_chars == 0 {
                return 0;
            }
            
            // For continuous text, look for word boundaries using multiple strategies
            let mut word_count = 0;
            
            // Strategy 1: Count transitions from lowercase to uppercase (camelCase detection)
            let chars: Vec<char> = text.chars().collect();
            let mut camel_transitions = 0;
            
            for i in 1..chars.len() {
                let prev_char = chars[i-1];
                let curr_char = chars[i];
                
                // Count transitions from lowercase letter to uppercase letter
                if prev_char.is_lowercase() && curr_char.is_uppercase() {
                    camel_transitions += 1;
                }
                // Count transitions from letter to digit or digit to letter
                else if (prev_char.is_alphabetic() && curr_char.is_numeric()) ||
                        (prev_char.is_numeric() && curr_char.is_alphabetic()) {
                    camel_transitions += 1;
                }
            }
            
            // If we found camelCase transitions, estimate words
            if camel_transitions > 0 {
                word_count = camel_transitions + 1; // +1 for the first word
            }
            
            // Strategy 2: If no camelCase detected, estimate based on character count
            if word_count == 0 {
                // Estimate based on typical word length (4-6 characters per word)
                word_count = (alphanumeric_chars / 5).max(1);
            }
            
            word_count
        } else {
            whitespace_words
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_extractor() -> (XmlOfficeExtractor, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let extractor = XmlOfficeExtractor::new(temp_dir.path().to_string_lossy().to_string());
        (extractor, temp_dir)
    }

    #[test]
    fn test_validate_zip_entry_name() {
        // Valid names should pass
        assert!(XmlOfficeExtractor::validate_zip_entry_name("word/document.xml").is_ok());
        assert!(XmlOfficeExtractor::validate_zip_entry_name("xl/worksheets/sheet1.xml").is_ok());
        
        // Invalid names should fail
        assert!(XmlOfficeExtractor::validate_zip_entry_name("../../../etc/passwd").is_err());
        assert!(XmlOfficeExtractor::validate_zip_entry_name("/etc/passwd").is_err());
        assert!(XmlOfficeExtractor::validate_zip_entry_name("C:\\windows\\system32\\cmd.exe").is_err());
        assert!(XmlOfficeExtractor::validate_zip_entry_name("file<script>alert(1)</script>.xml").is_err());
        
        // Too long name should fail
        let long_name = "a".repeat(300);
        assert!(XmlOfficeExtractor::validate_zip_entry_name(&long_name).is_err());
    }

    #[test]
    fn test_remove_null_bytes() {
        let text_with_nulls = "Hello\0World\0Test";
        let cleaned = XmlOfficeExtractor::remove_null_bytes(text_with_nulls);
        assert_eq!(cleaned, "HelloWorldTest");
        
        let text_without_nulls = "Hello World Test";
        let cleaned = XmlOfficeExtractor::remove_null_bytes(text_without_nulls);
        assert_eq!(cleaned, "Hello World Test");
    }

    #[test]
    fn test_count_words_safely() {
        let (extractor, _temp_dir) = create_test_extractor();
        
        // Normal text
        assert_eq!(extractor.count_words_safely("Hello world test"), 3);
        
        // Empty text
        assert_eq!(extractor.count_words_safely(""), 0);
        assert_eq!(extractor.count_words_safely("   "), 0);
        
        // Continuous text without spaces
        assert!(extractor.count_words_safely("HelloWorldTestingCamelCase") > 0);
        
        // Very large text should not panic
        let large_text = "word ".repeat(500_000); // 2MB+ of text
        let word_count = extractor.count_words_safely(&large_text);
        assert!(word_count > 0);
        assert!(word_count <= 10_000_000); // Should be capped
    }

    #[test]
    fn test_read_zip_entry_safely() {
        use std::io::Cursor;
        
        let context = ExtractionContext::new(10 * 1024 * 1024); // 10MB limit
        
        // Test normal sized content
        let small_content = b"Hello World";
        let mut cursor = Cursor::new(small_content);
        let result = XmlOfficeExtractor::read_zip_entry_safely(&mut cursor, 1024, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World");
        
        // Test oversized content
        let large_content = vec![b'A'; 2048];
        let mut cursor = Cursor::new(large_content);
        let result = XmlOfficeExtractor::read_zip_entry_safely(&mut cursor, 1024, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum allowed size"));
    }
}