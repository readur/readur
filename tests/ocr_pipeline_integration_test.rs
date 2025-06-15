/*!
 * OCR Pipeline Integration Test - Run the full pipeline internally
 * 
 * This test runs the OCR pipeline components directly instead of through HTTP,
 * giving us complete visibility into the corruption process.
 */

use anyhow::Result;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use uuid::Uuid;

use readur::{
    config::Config,
    db::Database,
    models::Document,
    file_service::FileService,
    enhanced_ocr::EnhancedOcrService,
    ocr_queue::{OcrQueueService, OcrQueueItem},
    db_guardrails_simple::DocumentTransactionManager,
};

const TEST_DB_URL: &str = "postgresql://readur_user:readur_password@localhost:5432/readur";

struct OCRPipelineTestHarness {
    db: Database,
    pool: PgPool,
    file_service: FileService,
    ocr_service: EnhancedOcrService,
    queue_service: OcrQueueService,
    transaction_manager: DocumentTransactionManager,
}

impl OCRPipelineTestHarness {
    async fn new() -> Result<Self> {
        // Initialize database connection
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(TEST_DB_URL)
            .await?;
        
        let db = Database::new(TEST_DB_URL).await?;
        
        // Initialize services
        let file_service = FileService::new("./test_uploads".to_string());
        let ocr_service = EnhancedOcrService::new("/tmp".to_string());
        let queue_service = OcrQueueService::new(db.clone(), pool.clone(), 4);
        let transaction_manager = DocumentTransactionManager::new(pool.clone());
        
        // Ensure test upload directory exists
        std::fs::create_dir_all("./test_uploads").unwrap_or_default();
        
        Ok(Self {
            db,
            pool,
            file_service,
            ocr_service,
            queue_service,
            transaction_manager,
        })
    }
    
    async fn create_test_user(&self) -> Result<Uuid> {
        let user_id = Uuid::new_v4();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, role)
            VALUES ($1, $2, $3, $4, 'user')
            "#
        )
        .bind(user_id)
        .bind(format!("test_user_{}", timestamp))
        .bind(format!("test_{}@example.com", timestamp))
        .bind("dummy_hash") // We're not testing authentication
        .execute(&self.pool)
        .await?;
        
        info!("✅ Created test user: {}", user_id);
        Ok(user_id)
    }
    
    async fn create_test_document(&self, user_id: Uuid, content: &str, filename: &str) -> Result<(Uuid, String)> {
        let doc_id = Uuid::new_v4();
        let file_path = format!("./test_uploads/{}.txt", doc_id);
        
        // Write content to file
        tokio::fs::write(&file_path, content).await?;
        
        // Create document record
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, filename, original_filename, file_path, file_size, 
                mime_type, content, user_id, ocr_status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', NOW(), NOW())
            "#
        )
        .bind(doc_id)
        .bind(filename)
        .bind(filename)
        .bind(&file_path)
        .bind(content.len() as i64)
        .bind("text/plain")
        .bind(content) // Store original content for comparison
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        
        info!("✅ Created document: {} -> {} ({} bytes)", doc_id, filename, content.len());
        Ok((doc_id, file_path))
    }
    
    async fn enqueue_document_for_ocr(&self, doc_id: Uuid, priority: i32, file_size: i64) -> Result<Uuid> {
        let queue_ids = self.queue_service.enqueue_document(doc_id, priority, file_size).await?;
        info!("✅ Enqueued document {} for OCR processing", doc_id);
        Ok(queue_ids)
    }
    
    async fn get_document_details(&self, doc_id: Uuid) -> Result<DocumentDetails> {
        let row = sqlx::query(
            r#"
            SELECT id, filename, file_path, ocr_status, ocr_text, ocr_confidence, 
                   ocr_word_count, ocr_processing_time_ms, ocr_error, content
            FROM documents 
            WHERE id = $1
            "#
        )
        .bind(doc_id)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(DocumentDetails {
            id: row.get("id"),
            filename: row.get("filename"),
            file_path: row.get("file_path"),
            ocr_status: row.get("ocr_status"),
            ocr_text: row.get("ocr_text"),
            ocr_confidence: row.get("ocr_confidence"),
            ocr_word_count: row.get("ocr_word_count"),
            ocr_processing_time_ms: row.get("ocr_processing_time_ms"),
            ocr_error: row.get("ocr_error"),
            original_content: row.get("content"),
        })
    }
    
    async fn get_queue_item(&self, doc_id: Uuid) -> Result<Option<QueueItemDetails>> {
        let row = sqlx::query(
            r#"
            SELECT id, document_id, status, priority, attempts, max_attempts,
                   worker_id, created_at, started_at, completed_at, error_message
            FROM ocr_queue 
            WHERE document_id = $1
            "#
        )
        .bind(doc_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(r) => Ok(Some(QueueItemDetails {
                id: r.get("id"),
                document_id: r.get("document_id"),
                status: r.get("status"),
                priority: r.get("priority"),
                attempts: r.get("attempts"),
                max_attempts: r.get("max_attempts"),
                worker_id: r.get("worker_id"),
                error_message: r.get("error_message"),
            })),
            None => Ok(None),
        }
    }
    
    async fn process_single_ocr_job(&self, worker_id: &str) -> Result<Option<ProcessingResult>> {
        info!("🔄 Worker {} attempting to dequeue job", worker_id);
        
        // Step 1: Dequeue a job
        let item = match self.queue_service.dequeue().await? {
            Some(item) => {
                info!("✅ Worker {} claimed job {} for document {}", 
                      worker_id, item.id, item.document_id);
                item
            }
            None => {
                info!("📭 No jobs available for worker {}", worker_id);
                return Ok(None);
            }
        };
        
        let doc_id = item.document_id;
        let job_id = item.id;
        
        // Step 2: Get document details
        let doc_details = self.get_document_details(doc_id).await?;
        info!("📄 Processing document: {} ({})", doc_details.filename, doc_details.file_path);
        
        // Step 3: Read file content to verify it matches expected
        let file_content = match tokio::fs::read_to_string(&doc_details.file_path).await {
            Ok(content) => {
                info!("📖 Read file content: {} chars", content.len());
                content
            }
            Err(e) => {
                error!("❌ Failed to read file {}: {}", doc_details.file_path, e);
                return Ok(Some(ProcessingResult {
                    doc_id,
                    job_id,
                    success: false,
                    error: Some(format!("File read error: {}", e)),
                    ocr_text: None,
                    original_content: doc_details.original_content,
                    file_content: None,
                }));
            }
        };
        
        // Step 4: Verify file content matches database content
        if let Some(ref original) = doc_details.original_content {
            if file_content != *original {
                warn!("⚠️  File content mismatch for document {}!", doc_id);
                warn!("   Expected: {}", original);
                warn!("   File contains: {}", file_content);
            } else {
                info!("✅ File content matches database content");
            }
        }
        
        // Step 5: Run OCR processing
        info!("🔍 Starting OCR processing for document {}", doc_id);
        let settings = readur::models::Settings::default();
        
        let ocr_result = match self.ocr_service.extract_text(&doc_details.file_path, "text/plain", &settings).await {
            Ok(result) => {
                info!("✅ OCR extraction successful: {:.1}% confidence, {} words", 
                      result.confidence, result.word_count);
                info!("📝 OCR Text: {}", result.text);
                result
            }
            Err(e) => {
                error!("❌ OCR extraction failed: {}", e);
                return Ok(Some(ProcessingResult {
                    doc_id,
                    job_id,
                    success: false,
                    error: Some(format!("OCR error: {}", e)),
                    ocr_text: None,
                    original_content: doc_details.original_content,
                    file_content: Some(file_content),
                }));
            }
        };
        
        // Step 6: Update document with OCR results using transaction manager
        info!("💾 Saving OCR results to database");
        let update_result = self.transaction_manager.update_ocr_with_validation(
            doc_id,
            &doc_details.filename,
            &ocr_result.text,
            ocr_result.confidence as f64,
            ocr_result.word_count as i32,
            ocr_result.processing_time_ms as i64,
        ).await;
        
        match update_result {
            Ok(true) => {
                info!("✅ OCR results saved successfully for document {}", doc_id);
                Ok(Some(ProcessingResult {
                    doc_id,
                    job_id,
                    success: true,
                    error: None,
                    ocr_text: Some(ocr_result.text),
                    original_content: doc_details.original_content,
                    file_content: Some(file_content),
                }))
            }
            Ok(false) => {
                warn!("⚠️  OCR update validation failed for document {}", doc_id);
                Ok(Some(ProcessingResult {
                    doc_id,
                    job_id,
                    success: false,
                    error: Some("OCR update validation failed".to_string()),
                    ocr_text: Some(ocr_result.text),
                    original_content: doc_details.original_content,
                    file_content: Some(file_content),
                }))
            }
            Err(e) => {
                error!("❌ Failed to save OCR results: {}", e);
                Ok(Some(ProcessingResult {
                    doc_id,
                    job_id,
                    success: false,
                    error: Some(format!("Database error: {}", e)),
                    ocr_text: Some(ocr_result.text),
                    original_content: doc_details.original_content,
                    file_content: Some(file_content),
                }))
            }
        }
    }
    
    async fn simulate_concurrent_workers(&self, num_workers: usize, max_iterations: usize) -> Result<Vec<ProcessingResult>> {
        info!("🏭 Starting {} concurrent OCR workers", num_workers);
        
        let mut handles = Vec::new();
        
        for worker_num in 1..=num_workers {
            let worker_id = format!("test-worker-{}", worker_num);
            // Clone the components we need rather than the whole harness
            let queue_service = self.queue_service.clone();
            let transaction_manager = self.transaction_manager.clone();
            let ocr_service = EnhancedOcrService::new("/tmp".to_string());
            let pool = self.pool.clone();
            
            let handle = tokio::spawn(async move {
                let mut results = Vec::new();
                
                for iteration in 1..=max_iterations {
                    info!("Worker {} iteration {}", worker_id, iteration);
                    
                    // Simulate the OCR processing within this spawned task
                    let item = match queue_service.dequeue().await {
                        Ok(Some(item)) => {
                            info!("✅ Worker {} claimed job {} for document {}", 
                                  worker_id, item.id, item.document_id);
                            item
                        }
                        Ok(None) => {
                            info!("📭 No jobs available for worker {}", worker_id);
                            sleep(Duration::from_millis(10)).await;
                            continue;
                        }
                        Err(e) => {
                            error!("Worker {} error: {}", worker_id, e);
                            break;
                        }
                    };
                    
                    let doc_id = item.document_id;
                    let job_id = item.id;
                    
                    // Get document details 
                    let doc_details = match sqlx::query(
                        r#"
                        SELECT id, filename, original_filename, file_path, file_size, 
                               mime_type, content, user_id, ocr_status, created_at, updated_at
                        FROM documents 
                        WHERE id = $1
                        "#
                    )
                    .bind(doc_id)
                    .fetch_one(&pool)
                    .await {
                        Ok(row) => row,
                        Err(e) => {
                            error!("❌ Failed to get document details: {}", e);
                            continue;
                        }
                    };
                    
                    let filename: String = doc_details.get("filename");
                    let file_path: String = doc_details.get("file_path");
                    let original_content: Option<String> = doc_details.get("content");
                    
                    // Read file content 
                    let file_content = match tokio::fs::read_to_string(&file_path).await {
                        Ok(content) => {
                            info!("📖 Read file content: {} chars", content.len());
                            content
                        }
                        Err(e) => {
                            error!("❌ Failed to read file {}: {}", file_path, e);
                            results.push(ProcessingResult {
                                doc_id,
                                job_id,
                                success: false,
                                error: Some(format!("File read error: {}", e)),
                                ocr_text: None,
                                original_content,
                                file_content: None,
                            });
                            continue;
                        }
                    };
                    
                    // Verify file content matches database
                    if let Some(ref original) = original_content {
                        if file_content != *original {
                            warn!("⚠️  File content mismatch for document {}!", doc_id);
                            warn!("   Expected: {}", original);
                            warn!("   File contains: {}", file_content);
                        } else {
                            info!("✅ File content matches database content");
                        }
                    }
                    
                    // Run OCR processing
                    info!("🔍 Starting OCR processing for document {}", doc_id);
                    let settings = readur::models::Settings::default();
                    
                    let ocr_result = match ocr_service.extract_text(&file_path, "text/plain", &settings).await {
                        Ok(result) => {
                            info!("✅ OCR extraction successful: {:.1}% confidence, {} words", 
                                  result.confidence, result.word_count);
                            info!("📝 OCR Text: {}", result.text);
                            result
                        }
                        Err(e) => {
                            error!("❌ OCR extraction failed: {}", e);
                            results.push(ProcessingResult {
                                doc_id,
                                job_id,
                                success: false,
                                error: Some(format!("OCR error: {}", e)),
                                ocr_text: None,
                                original_content,
                                file_content: Some(file_content),
                            });
                            continue;
                        }
                    };
                    
                    // Update document with OCR results using transaction manager
                    info!("💾 Saving OCR results to database");
                    let update_result = transaction_manager.update_ocr_with_validation(
                        doc_id,
                        &filename,
                        &ocr_result.text,
                        ocr_result.confidence as f64,
                        ocr_result.word_count as i32,
                        ocr_result.processing_time_ms as i64,
                    ).await;
                    
                    match update_result {
                        Ok(true) => {
                            info!("✅ OCR results saved successfully for document {}", doc_id);
                            results.push(ProcessingResult {
                                doc_id,
                                job_id,
                                success: true,
                                error: None,
                                ocr_text: Some(ocr_result.text),
                                original_content,
                                file_content: Some(file_content),
                            });
                        }
                        Ok(false) => {
                            warn!("⚠️  OCR update validation failed for document {}", doc_id);
                            results.push(ProcessingResult {
                                doc_id,
                                job_id,
                                success: false,
                                error: Some("OCR update validation failed".to_string()),
                                ocr_text: Some(ocr_result.text),
                                original_content,
                                file_content: Some(file_content),
                            });
                        }
                        Err(e) => {
                            error!("❌ Failed to save OCR results: {}", e);
                            results.push(ProcessingResult {
                                doc_id,
                                job_id,
                                success: false,
                                error: Some(format!("Database error: {}", e)),
                                ocr_text: Some(ocr_result.text),
                                original_content,
                                file_content: Some(file_content),
                            });
                        }
                    }
                    
                    // Small delay between iterations
                    sleep(Duration::from_millis(1)).await;
                }
                
                results
            });
            
            handles.push(handle);
        }
        
        // Wait for all workers to complete
        let mut all_results = Vec::new();
        for handle in handles {
            let worker_results = handle.await?;
            all_results.extend(worker_results);
        }
        
        info!("🏁 All workers completed. Total jobs processed: {}", all_results.len());
        Ok(all_results)
    }
    
    async fn cleanup(&self) -> Result<()> {
        // Clean up test files
        let _ = tokio::fs::remove_dir_all("./test_uploads").await;
        Ok(())
    }
}

#[derive(Debug)]
struct DocumentDetails {
    id: Uuid,
    filename: String,
    file_path: String,
    ocr_status: Option<String>,
    ocr_text: Option<String>,
    ocr_confidence: Option<f64>,
    ocr_word_count: Option<i32>,
    ocr_processing_time_ms: Option<i64>,
    ocr_error: Option<String>,
    original_content: Option<String>,
}

#[derive(Debug)]
struct QueueItemDetails {
    id: Uuid,
    document_id: Uuid,
    status: String,
    priority: i32,
    attempts: i32,
    max_attempts: i32,
    worker_id: Option<String>,
    error_message: Option<String>,
}

#[derive(Debug)]
struct ProcessingResult {
    doc_id: Uuid,
    job_id: Uuid,
    success: bool,
    error: Option<String>,
    ocr_text: Option<String>,
    original_content: Option<String>,
    file_content: Option<String>,
}

#[tokio::test]
async fn test_high_concurrency_ocr_pipeline_internal() {
    println!("🚀 HIGH CONCURRENCY OCR PIPELINE INTERNAL TEST");
    println!("===============================================");
    
    let harness = OCRPipelineTestHarness::new().await
        .expect("Failed to initialize test harness");
    
    // Create test user
    let user_id = harness.create_test_user().await
        .expect("Failed to create test user");
    
    // Create 5 test documents with unique content
    let test_documents = vec![
        ("DOC-ALPHA-SIGNATURE-001", "test_alpha.txt"),
        ("DOC-BRAVO-SIGNATURE-002", "test_bravo.txt"), 
        ("DOC-CHARLIE-SIGNATURE-003", "test_charlie.txt"),
        ("DOC-DELTA-SIGNATURE-004", "test_delta.txt"),
        ("DOC-ECHO-SIGNATURE-005", "test_echo.txt"),
    ];
    
    println!("\n📝 Creating test documents:");
    let mut doc_ids = Vec::new();
    
    for (i, (content, filename)) in test_documents.iter().enumerate() {
        let (doc_id, _) = harness.create_test_document(user_id, content, filename).await
            .expect("Failed to create document");
        
        // Enqueue for OCR processing
        harness.enqueue_document_for_ocr(doc_id, 100 - i as i32, content.len() as i64).await
            .expect("Failed to enqueue document");
        
        doc_ids.push((doc_id, content.to_string()));
        println!("  ✅ {}: {} -> {}", i+1, filename, content);
    }
    
    // Simulate high concurrency with 5 workers processing simultaneously
    println!("\n🏭 Starting concurrent OCR processing:");
    let processing_results = harness.simulate_concurrent_workers(5, 10).await
        .expect("Failed to run concurrent workers");
    
    // Analyze results
    println!("\n📊 PROCESSING RESULTS ANALYSIS:");
    println!("===============================");
    
    let mut successful_count = 0;
    let mut failed_count = 0;
    let mut corruption_detected = false;
    
    for result in &processing_results {
        println!("\nDocument {}: {}", result.doc_id, if result.success { "✅ SUCCESS" } else { "❌ FAILED" });
        
        if result.success {
            successful_count += 1;
            
            // Find the expected content for this document
            if let Some((_, expected_content)) = doc_ids.iter().find(|(id, _)| *id == result.doc_id) {
                let actual_ocr = result.ocr_text.as_deref().unwrap_or("");
                
                if actual_ocr == expected_content {
                    println!("  ✅ Content matches expected");
                } else {
                    println!("  ❌ CORRUPTION DETECTED!");
                    println!("    Expected: {}", expected_content);
                    println!("    OCR Result: {}", actual_ocr);
                    corruption_detected = true;
                    
                    // Check if file content was correct
                    if let Some(ref file_content) = result.file_content {
                        if file_content == expected_content {
                            println!("    📁 File content was correct - corruption in OCR pipeline");
                        } else {
                            println!("    📁 File content was also wrong - corruption in file system");
                        }
                    }
                }
            }
        } else {
            failed_count += 1;
            println!("  Error: {}", result.error.as_deref().unwrap_or("Unknown"));
        }
    }
    
    // Final verification - check database state
    println!("\n🔍 FINAL DATABASE STATE VERIFICATION:");
    println!("=====================================");
    
    for (doc_id, expected_content) in &doc_ids {
        let details = harness.get_document_details(*doc_id).await
            .expect("Failed to get document details");
        
        println!("\nDocument {}:", doc_id);
        println!("  Status: {}", details.ocr_status.as_deref().unwrap_or("unknown"));
        println!("  Expected: {}", expected_content);
        println!("  OCR Text: {}", details.ocr_text.as_deref().unwrap_or("(none)"));
        
        if details.ocr_status == Some("completed".to_string()) {
            let actual_text = details.ocr_text.as_deref().unwrap_or("");
            if actual_text != expected_content {
                println!("  ❌ DATABASE CORRUPTION CONFIRMED");
                corruption_detected = true;
            } else {
                println!("  ✅ Database content correct");
            }
        }
    }
    
    // Cleanup
    harness.cleanup().await.expect("Failed to cleanup");
    
    // Final results
    println!("\n🏆 FINAL RESULTS:");
    println!("=================");
    println!("✅ Successful: {}", successful_count);
    println!("❌ Failed: {}", failed_count);
    println!("🔬 Total processed: {}", processing_results.len());
    
    if corruption_detected {
        panic!("🚨 OCR CORRUPTION DETECTED in internal pipeline test!");
    } else {
        println!("🎉 No corruption detected in high-concurrency test!");
    }
}