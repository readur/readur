/*!
 * Simple Throttling Test - Use runtime database connection
 * 
 * This test uses the same database configuration as the running server
 * to validate the throttling mechanism works correctly.
 */

use anyhow::Result;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::time::{Duration, Instant, sleep};
use tracing::{info, warn, error};
use uuid::Uuid;

use readur::{
    db::Database,
    ocr::queue::OcrQueueService,
    ocr::enhanced::EnhancedOcrService,
    services::file_service::FileService,
    storage::{StorageConfig, factory::create_storage_backend},
};

// Use the same database URL as the running server
fn get_test_db_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string())
}

async fn create_test_file_service(temp_path: &str) -> FileService {
    let storage_config = StorageConfig::Local { upload_path: temp_path.to_string() };
    let storage_backend = create_storage_backend(storage_config).await.unwrap();
    FileService::with_storage(temp_path.to_string(), storage_backend)
}

struct SimpleThrottleTest {
    pool: PgPool,
    queue_service: Arc<OcrQueueService>,
}

impl SimpleThrottleTest {
    async fn new() -> Result<Self> {
        let db_url = get_test_db_url();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&db_url)
            .await?;
        
        let db = Database::new(&db_url).await?;
        
        // Create queue service with throttling (max 15 concurrent jobs)
        let queue_service = Arc::new(OcrQueueService::new(
            db.clone(), 
            pool.clone(), 
            15  // This should prevent DB pool exhaustion
        ));
        
        Ok(Self {
            pool,
            queue_service,
        })
    }
    
    async fn create_test_user(&self) -> Result<Uuid> {
        let user_id = Uuid::new_v4();
        // Use UUID for guaranteed uniqueness across concurrent test execution
        let test_id = Uuid::new_v4().simple().to_string();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("throttle_test_{}_{}_{}", test_id, nanos, Uuid::new_v4().simple());
        let email = format!("throttle_{}_{}@{}.example.com", test_id, nanos, Uuid::new_v4().simple());
        
        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, role)
            VALUES ($1, $2, $3, $4, 'user')
            "#
        )
        .bind(user_id)
        .bind(&username)
        .bind(&email)
        .bind("test_hash")
        .execute(&self.pool)
        .await?;
        
        info!("✅ Created test user: {}", user_id);
        Ok(user_id)
    }
    
    async fn create_test_documents(&self, user_id: Uuid, count: usize) -> Result<Vec<Uuid>> {
        let mut doc_ids = Vec::new();
        
        info!("📝 Creating {} test documents for throttling test", count);
        
        for i in 1..=count {
            let content = format!("THROTTLE-TEST-CONTENT-{:03}-{}", i, Uuid::new_v4());
            let filename = format!("throttle_test_{:03}.txt", i);
            let doc_id = Uuid::new_v4();
            
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
            .bind(format!("/tmp/throttle_test_{}.txt", doc_id))
            .bind(content.len() as i64)
            .bind("text/plain")
            .bind(&content)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
            
            // Enqueue for OCR processing
            let priority = 10 - (i % 5) as i32;
            self.queue_service.enqueue_document(doc_id, priority, content.len() as i64).await?;
            
            doc_ids.push(doc_id);
            
            if i % 10 == 0 {
                info!("  ✅ Created {} documents so far", i);
            }
        }
        
        info!("✅ All {} documents created and enqueued", count);
        Ok(doc_ids)
    }
    
    async fn simulate_concurrent_processing(&self, workers: usize, max_time_seconds: u64) -> Result<()> {
        info!("🏭 Starting {} concurrent workers for {} seconds", workers, max_time_seconds);
        
        let mut handles = Vec::new();
        let end_time = Instant::now() + Duration::from_secs(max_time_seconds);
        
        for worker_id in 1..=workers {
            let queue_service = self.queue_service.clone();
            let worker_end_time = end_time;
            
            let handle = tokio::spawn(async move {
                let worker_name = format!("worker-{}", worker_id);
                let file_service = create_test_file_service("/tmp").await;
                let ocr_service = EnhancedOcrService::new("/tmp".to_string(), file_service);
                let mut jobs_processed = 0;
                
                info!("Worker {} starting", worker_name);
                
                while Instant::now() < worker_end_time {
                    match queue_service.dequeue().await {
                        Ok(Some(item)) => {
                            info!("Worker {} processing job {} for document {}", 
                                  worker_name, item.id, item.document_id);
                            
                            // Process with built-in throttling
                            if let Err(e) = queue_service.process_item(item, &ocr_service).await {
                                error!("Worker {} processing error: {}", worker_name, e);
                            } else {
                                jobs_processed += 1;
                            }
                        }
                        Ok(None) => {
                            // No jobs available, wait a bit
                            sleep(Duration::from_millis(100)).await;
                        }
                        Err(e) => {
                            error!("Worker {} dequeue error: {}", worker_name, e);
                            sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
                
                info!("Worker {} completed, processed {} jobs", worker_name, jobs_processed);
                jobs_processed
            });
            
            handles.push(handle);
        }
        
        // Wait for all workers to complete
        let mut total_processed = 0;
        for handle in handles {
            let jobs_processed = handle.await?;
            total_processed += jobs_processed;
        }
        
        info!("🏁 All workers completed. Total jobs processed: {}", total_processed);
        Ok(())
    }
    
    async fn check_results(&self, expected_docs: &[Uuid]) -> Result<TestResults> {
        info!("🔍 Checking results for {} documents", expected_docs.len());
        
        let mut results = TestResults {
            total: expected_docs.len(),
            completed: 0,
            failed: 0,
            pending: 0,
            processing: 0,
            empty_content: 0,
        };
        
        for doc_id in expected_docs {
            let row = sqlx::query(
                "SELECT ocr_status, ocr_text FROM documents WHERE id = $1"
            )
            .bind(doc_id)
            .fetch_one(&self.pool)
            .await?;
            
            let status: Option<String> = row.get("ocr_status");
            let ocr_text: Option<String> = row.get("ocr_text");
            
            match status.as_deref() {
                Some("completed") => {
                    results.completed += 1;
                    if ocr_text.as_deref().unwrap_or("").is_empty() {
                        results.empty_content += 1;
                    }
                }
                Some("failed") => results.failed += 1,
                Some("processing") => results.processing += 1,
                Some("pending") => results.pending += 1,
                _ => {}
            }
        }
        
        Ok(results)
    }
    
    async fn cleanup(&self, user_id: Uuid) -> Result<()> {
        // Clean up test data
        sqlx::query("DELETE FROM documents WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        
        info!("✅ Cleanup completed");
        Ok(())
    }
}

#[derive(Debug)]
struct TestResults {
    total: usize,
    completed: usize,
    failed: usize,
    pending: usize,
    processing: usize,
    empty_content: usize,
}

impl TestResults {
    fn completion_rate(&self) -> f64 {
        if self.total == 0 { return 0.0; }
        ((self.completed + self.failed) as f64 / self.total as f64) * 100.0
    }
}

#[tokio::test]
async fn test_throttling_with_25_documents() {
    println!("🚀 THROTTLING TEST - 25 DOCUMENTS");
    println!("=================================");
    
    let test = SimpleThrottleTest::new().await
        .expect("Failed to initialize test");
    
    // Create test user
    let user_id = test.create_test_user().await
        .expect("Failed to create test user");
    
    // Create 25 test documents (this previously caused empty content)
    let doc_count = 25;
    let doc_ids = test.create_test_documents(user_id, doc_count).await
        .expect("Failed to create test documents");
    
    // Start concurrent processing for 60 seconds
    test.simulate_concurrent_processing(5, 60).await
        .expect("Failed to process documents");
    
    // Wait a bit more for any remaining jobs
    sleep(Duration::from_secs(10)).await;
    
    // Check results
    let results = test.check_results(&doc_ids).await
        .expect("Failed to check results");
    
    // Cleanup
    test.cleanup(user_id).await.expect("Failed to cleanup");
    
    // Print results
    println!("\n🏆 TEST RESULTS:");
    println!("================");
    println!("📊 Total Documents: {}", results.total);
    println!("✅ Completed: {}", results.completed);
    println!("❌ Failed: {}", results.failed);
    println!("⏳ Pending: {}", results.pending);
    println!("🔄 Processing: {}", results.processing);
    println!("🚫 Empty Content: {}", results.empty_content);
    println!("📈 Completion Rate: {:.1}%", results.completion_rate());
    
    // Key assertion: No empty content (this was the main issue before throttling)
    assert_eq!(results.empty_content, 0, 
               "Found {} documents with empty content! Throttling failed to prevent DB pool exhaustion", 
               results.empty_content);
    
    // Should have reasonable completion rate
    assert!(results.completion_rate() >= 70.0, 
           "Completion rate too low: {:.1}% (expected >= 70%)", results.completion_rate());
    
    println!("🎉 Throttling test PASSED! No empty content found.");
}

#[tokio::test]
async fn test_throttling_with_50_documents() {
    println!("🚀 THROTTLING TEST - 50 DOCUMENTS");
    println!("=================================");
    
    let test = SimpleThrottleTest::new().await
        .expect("Failed to initialize test");
    
    // Create test user
    let user_id = test.create_test_user().await
        .expect("Failed to create test user");
    
    // Create 50 test documents (this should definitely test the throttling)
    let doc_count = 50;
    let doc_ids = test.create_test_documents(user_id, doc_count).await
        .expect("Failed to create test documents");
    
    // Start concurrent processing for 120 seconds (longer for more documents)
    test.simulate_concurrent_processing(8, 120).await
        .expect("Failed to process documents");
    
    // Wait a bit more for any remaining jobs
    sleep(Duration::from_secs(15)).await;
    
    // Check results
    let results = test.check_results(&doc_ids).await
        .expect("Failed to check results");
    
    // Cleanup
    test.cleanup(user_id).await.expect("Failed to cleanup");
    
    // Print results
    println!("\n🏆 TEST RESULTS:");
    println!("================");
    println!("📊 Total Documents: {}", results.total);
    println!("✅ Completed: {}", results.completed);
    println!("❌ Failed: {}", results.failed);
    println!("⏳ Pending: {}", results.pending);
    println!("🔄 Processing: {}", results.processing);
    println!("🚫 Empty Content: {}", results.empty_content);
    println!("📈 Completion Rate: {:.1}%", results.completion_rate());
    
    // Key assertion: No empty content (this was the main issue before throttling)
    assert_eq!(results.empty_content, 0, 
               "Found {} documents with empty content! Throttling failed to prevent DB pool exhaustion", 
               results.empty_content);
    
    // Should have reasonable completion rate even with high load
    assert!(results.completion_rate() >= 60.0, 
           "Completion rate too low: {:.1}% (expected >= 60%)", results.completion_rate());
    
    println!("🎉 High-load throttling test PASSED! No empty content found with 50 documents.");
}