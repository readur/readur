/*!
 * Throttled High Concurrency OCR Test
 * 
 * This test verifies that our new throttling mechanism properly handles
 * high concurrency scenarios (50+ documents) without database connection
 * pool exhaustion or corrupting OCR results.
 */

use anyhow::Result;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tracing::{info, warn, error};
use uuid::Uuid;

use readur::{
    config::Config,
    db::Database,
    models::{Document, Settings},
    file_service::FileService,
    enhanced_ocr::EnhancedOcrService,
    ocr_queue::OcrQueueService,
    db_guardrails_simple::DocumentTransactionManager,
    request_throttler::RequestThrottler,
};

const TEST_DB_URL: &str = "postgresql://readur:readur@localhost:5432/readur";

struct ThrottledTestHarness {
    db: Database,
    pool: PgPool,
    file_service: FileService,
    queue_service: Arc<OcrQueueService>,
    transaction_manager: DocumentTransactionManager,
}

impl ThrottledTestHarness {
    async fn new() -> Result<Self> {
        // Initialize database with proper connection limits
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(30)  // Higher limit for stress testing
            .acquire_timeout(std::time::Duration::from_secs(15))
            .connect(TEST_DB_URL)
            .await?;
        
        let db = Database::new(TEST_DB_URL).await?;
        
        // Initialize services
        let file_service = FileService::new("./test_uploads".to_string());
        
        // Create throttled queue service - this is the key improvement
        let queue_service = Arc::new(OcrQueueService::new(
            db.clone(), 
            pool.clone(), 
            15  // Limit to 15 concurrent OCR jobs to prevent DB pool exhaustion
        ));
        
        let transaction_manager = DocumentTransactionManager::new(pool.clone());
        
        // Ensure test upload directory exists
        std::fs::create_dir_all("./test_uploads").unwrap_or_default();
        
        Ok(Self {
            db,
            pool,
            file_service,
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
        .bind(format!("throttle_test_user_{}", timestamp))
        .bind(format!("throttle_test_{}@example.com", timestamp))
        .bind("dummy_hash")
        .execute(&self.pool)
        .await?;
        
        info!("✅ Created test user: {}", user_id);
        Ok(user_id)
    }
    
    async fn create_test_documents(&self, user_id: Uuid, count: usize) -> Result<Vec<(Uuid, String)>> {
        let mut documents = Vec::new();
        
        info!("📝 Creating {} test documents", count);
        
        for i in 1..=count {
            let content = format!("THROTTLE-TEST-DOC-{:03}-UNIQUE-CONTENT-{}", i, Uuid::new_v4());
            let filename = format!("throttle_test_{:03}.txt", i);
            let doc_id = Uuid::new_v4();
            let file_path = format!("./test_uploads/{}.txt", doc_id);
            
            // Write content to file
            tokio::fs::write(&file_path, &content).await?;
            
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
            .bind(&filename)
            .bind(&filename)
            .bind(&file_path)
            .bind(content.len() as i64)
            .bind("text/plain")
            .bind(&content)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
            
            // Enqueue for OCR processing with random priority
            let priority = 10 - (i % 5) as i32; // Priorities from 5-10
            self.queue_service.enqueue_document(doc_id, priority, content.len() as i64).await?;
            
            documents.push((doc_id, content));
            
            if i % 10 == 0 {
                info!("  ✅ Created {} documents so far", i);
            }
        }
        
        info!("✅ All {} documents created and enqueued", count);
        Ok(documents)
    }
    
    async fn start_throttled_workers(&self, num_workers: usize) -> Result<()> {
        info!("🏭 Starting {} throttled OCR workers", num_workers);
        
        let mut handles = Vec::new();
        
        for worker_num in 1..=num_workers {
            let queue_service = self.queue_service.clone();
            
            let handle = tokio::spawn(async move {
                let worker_id = format!("throttled-worker-{}", worker_num);
                info!("Worker {} starting", worker_id);
                
                // Each worker runs for a limited time to avoid infinite loops
                let start_time = Instant::now();
                let max_runtime = Duration::from_secs(300); // 5 minutes max
                
                // Run a simplified worker loop instead of calling start_worker
                // start_worker() consumes the Arc<Self>, so we can't call it multiple times
                loop {
                    if start_time.elapsed() > max_runtime {
                        break;
                    }
                    
                    // Process a single job if available
                    match queue_service.dequeue().await {
                        Ok(Some(item)) => {
                            info!("Worker {} processing job {}", worker_id, item.id);
                            // Process item using the built-in throttling
                            let ocr_service = readur::enhanced_ocr::EnhancedOcrService::new("/tmp".to_string());
                            if let Err(e) = queue_service.process_item(item, &ocr_service).await {
                                error!("Worker {} processing error: {}", worker_id, e);
                            }
                        }
                        Ok(None) => {
                            // No jobs available, wait a bit
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        Err(e) => {
                            error!("Worker {} dequeue error: {}", worker_id, e);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
                
                info!("Worker {} completed", worker_id);
            });
            
            handles.push(handle);
        }
        
        // Don't wait for all workers to complete - they run in background
        Ok(())
    }
    
    async fn wait_for_completion(&self, expected_docs: usize, timeout_minutes: u64) -> Result<()> {
        let start_time = Instant::now();
        let timeout = Duration::from_secs(timeout_minutes * 60);
        
        info!("⏳ Waiting for {} documents to complete (timeout: {} minutes)", expected_docs, timeout_minutes);
        
        loop {
            if start_time.elapsed() > timeout {
                warn!("⏰ Timeout reached waiting for OCR completion");
                break;
            }
            
            // Check completion status
            let completed_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM documents WHERE ocr_status = 'completed'"
            )
            .fetch_one(&self.pool)
            .await?;
            
            let failed_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
            )
            .fetch_one(&self.pool)
            .await?;
            
            let processing_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM documents WHERE ocr_status = 'processing'"
            )
            .fetch_one(&self.pool)
            .await?;
            
            let pending_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM documents WHERE ocr_status = 'pending'"
            )
            .fetch_one(&self.pool)
            .await?;
            
            info!("📊 Status: {} completed, {} failed, {} processing, {} pending", 
                  completed_count, failed_count, processing_count, pending_count);
            
            if completed_count + failed_count >= expected_docs as i64 {
                info!("✅ All documents have been processed!");
                break;
            }
            
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        
        Ok(())
    }
    
    async fn verify_results(&self, expected_documents: &[(Uuid, String)]) -> Result<ThrottleTestResults> {
        info!("🔍 Verifying OCR results for {} documents", expected_documents.len());
        
        let mut results = ThrottleTestResults {
            total_documents: expected_documents.len(),
            completed: 0,
            failed: 0,
            corrupted: 0,
            empty_content: 0,
            correct_content: 0,
        };
        
        for (doc_id, expected_content) in expected_documents {
            let row = sqlx::query(
                r#"
                SELECT ocr_status, ocr_text, ocr_error, filename
                FROM documents 
                WHERE id = $1
                "#
            )
            .bind(doc_id)
            .fetch_one(&self.pool)
            .await?;
            
            let status: Option<String> = row.get("ocr_status");
            let ocr_text: Option<String> = row.get("ocr_text");
            let ocr_error: Option<String> = row.get("ocr_error");
            let filename: String = row.get("filename");
            
            match status.as_deref() {
                Some("completed") => {
                    results.completed += 1;
                    
                    match ocr_text.as_deref() {
                        Some(text) if text.is_empty() => {
                            warn!("❌ Document {} ({}) has empty OCR content", doc_id, filename);
                            results.empty_content += 1;
                        }
                        Some(text) if text == expected_content => {
                            results.correct_content += 1;
                        }
                        Some(text) => {
                            warn!("❌ Document {} ({}) has corrupted content:", doc_id, filename);
                            warn!("   Expected: {}", expected_content);
                            warn!("   Got: {}", text);
                            results.corrupted += 1;
                        }
                        None => {
                            warn!("❌ Document {} ({}) has NULL OCR content", doc_id, filename);
                            results.empty_content += 1;
                        }
                    }
                }
                Some("failed") => {
                    results.failed += 1;
                    info!("⚠️  Document {} ({}) failed: {}", doc_id, filename, 
                          ocr_error.as_deref().unwrap_or("Unknown error"));
                }
                other => {
                    warn!("❓ Document {} ({}) has unexpected status: {:?}", doc_id, filename, other);
                }
            }
        }
        
        Ok(results)
    }
    
    async fn cleanup(&self) -> Result<()> {
        // Clean up test files
        let _ = tokio::fs::remove_dir_all("./test_uploads").await;
        Ok(())
    }
}

#[derive(Debug)]
struct ThrottleTestResults {
    total_documents: usize,
    completed: usize,
    failed: usize,
    corrupted: usize,
    empty_content: usize,
    correct_content: usize,
}

impl ThrottleTestResults {
    fn success_rate(&self) -> f64 {
        if self.total_documents == 0 { return 0.0; }
        (self.correct_content as f64 / self.total_documents as f64) * 100.0
    }
    
    fn completion_rate(&self) -> f64 {
        if self.total_documents == 0 { return 0.0; }
        ((self.completed + self.failed) as f64 / self.total_documents as f64) * 100.0
    }
}

#[tokio::test]
async fn test_throttled_high_concurrency_50_documents() {
    println!("🚀 THROTTLED HIGH CONCURRENCY TEST - 50 DOCUMENTS");
    println!("================================================");
    
    let harness = ThrottledTestHarness::new().await
        .expect("Failed to initialize throttled test harness");
    
    // Create test user
    let user_id = harness.create_test_user().await
        .expect("Failed to create test user");
    
    // Create 50 test documents
    let document_count = 50;
    let test_documents = harness.create_test_documents(user_id, document_count).await
        .expect("Failed to create test documents");
    
    // Start multiple throttled workers
    harness.start_throttled_workers(5).await
        .expect("Failed to start throttled workers");
    
    // Wait for completion with generous timeout
    harness.wait_for_completion(document_count, 10).await
        .expect("Failed to wait for completion");
    
    // Verify results
    let results = harness.verify_results(&test_documents).await
        .expect("Failed to verify results");
    
    // Cleanup
    harness.cleanup().await.expect("Failed to cleanup");
    
    // Print detailed results
    println!("\n🏆 THROTTLED TEST RESULTS:");
    println!("========================");
    println!("📊 Total Documents: {}", results.total_documents);
    println!("✅ Completed: {}", results.completed);
    println!("❌ Failed: {}", results.failed);
    println!("🔧 Correct Content: {}", results.correct_content);
    println!("🚫 Empty Content: {}", results.empty_content);
    println!("💥 Corrupted Content: {}", results.corrupted);
    println!("📈 Success Rate: {:.1}%", results.success_rate());
    println!("📊 Completion Rate: {:.1}%", results.completion_rate());
    
    // Assertions
    assert!(results.completion_rate() >= 90.0, 
           "Completion rate too low: {:.1}% (expected >= 90%)", results.completion_rate());
    
    assert!(results.empty_content == 0, 
           "Found {} documents with empty content (should be 0 with throttling)", results.empty_content);
    
    assert!(results.corrupted == 0, 
           "Found {} documents with corrupted content (should be 0 with throttling)", results.corrupted);
    
    assert!(results.success_rate() >= 80.0, 
           "Success rate too low: {:.1}% (expected >= 80%)", results.success_rate());
    
    println!("🎉 Throttled high concurrency test PASSED!");
}