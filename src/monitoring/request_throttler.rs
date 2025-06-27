/*!
 * Request Throttling for High-Concurrency Scenarios
 * 
 * This module provides throttling mechanisms to prevent resource exhaustion
 * when processing large numbers of concurrent requests.
 */

use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{Duration, Instant};
use tracing::{warn, info};

/// Request throttler to limit concurrent operations
#[derive(Clone)]
pub struct RequestThrottler {
    /// Semaphore to limit concurrent operations
    semaphore: Arc<Semaphore>,
    /// Maximum wait time for acquiring a permit
    max_wait_time: Duration,
    /// Name for logging purposes
    name: String,
}

impl RequestThrottler {
    /// Create a new request throttler
    pub fn new(max_concurrent: usize, max_wait_seconds: u64, name: String) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_wait_time: Duration::from_secs(max_wait_seconds),
            name,
        }
    }

    /// Acquire a permit for processing, with timeout
    pub async fn acquire_permit(&self) -> Result<ThrottlePermit, ThrottleError> {
        let start = Instant::now();
        
        // Try to acquire permit with timeout
        let permit = tokio::time::timeout(self.max_wait_time, self.semaphore.clone().acquire_owned())
            .await
            .map_err(|_| ThrottleError::Timeout)?
            .map_err(|_| ThrottleError::Cancelled)?;

        let wait_time = start.elapsed();
        
        if wait_time > Duration::from_millis(100) {
            info!("Throttler '{}': Acquired permit after {:?} wait", self.name, wait_time);
        }

        Ok(ThrottlePermit {
            _permit: permit,
            throttler_name: self.name.clone(),
        })
    }

    /// Get current available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Check if throttling is active
    pub fn is_throttling(&self) -> bool {
        self.semaphore.available_permits() == 0
    }
}

/// A permit that must be held while processing
pub struct ThrottlePermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    throttler_name: String,
}

impl Drop for ThrottlePermit {
    fn drop(&mut self) {
        // Permit is automatically released when dropped
    }
}

/// Throttling errors
#[derive(Debug)]
pub enum ThrottleError {
    /// Timeout waiting for permit
    Timeout,
    /// Operation was cancelled
    Cancelled,
}

impl std::fmt::Display for ThrottleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThrottleError::Timeout => write!(f, "Timeout waiting for throttling permit"),
            ThrottleError::Cancelled => write!(f, "Throttling operation was cancelled"),
        }
    }
}

impl std::error::Error for ThrottleError {}

/// Batch processor for handling high-volume operations
pub struct BatchProcessor<T> {
    batch_size: usize,
    flush_interval: Duration,
    processor: Box<dyn Fn(Vec<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>,
}

impl<T: Send + Clone + 'static> BatchProcessor<T> {
    /// Create a new batch processor
    pub fn new<F, Fut>(
        batch_size: usize,
        flush_interval_seconds: u64,
        processor: F,
    ) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        Self {
            batch_size,
            flush_interval: Duration::from_secs(flush_interval_seconds),
            processor: Box::new(move |items| Box::pin(processor(items))),
        }
    }

    /// Process items in batches
    pub async fn process_batch(&self, items: Vec<T>) {
        if items.is_empty() {
            return;
        }

        // Split into batches
        for chunk in items.chunks(self.batch_size) {
            let batch = chunk.to_vec();
            info!("Processing batch of {} items", batch.len());
            (self.processor)(batch).await;
            
            // Small delay between batches to prevent overwhelming the system
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_throttler_basic() {
        let throttler = RequestThrottler::new(2, 5, "test".to_string());
        
        // Should be able to acquire 2 permits
        let _permit1 = throttler.acquire_permit().await.unwrap();
        let _permit2 = throttler.acquire_permit().await.unwrap();
        
        // Third permit should be throttled
        assert_eq!(throttler.available_permits(), 0);
        assert!(throttler.is_throttling());
    }

    #[tokio::test]
    async fn test_throttler_timeout() {
        let throttler = RequestThrottler::new(1, 1, "test".to_string());
        
        let _permit = throttler.acquire_permit().await.unwrap();
        
        // This should timeout
        let result = throttler.acquire_permit().await;
        assert!(matches!(result, Err(ThrottleError::Timeout)));
    }

    #[tokio::test]
    async fn test_permit_release() {
        let throttler = RequestThrottler::new(1, 5, "test".to_string());
        
        {
            let _permit = throttler.acquire_permit().await.unwrap();
            assert_eq!(throttler.available_permits(), 0);
        } // permit dropped here
        
        // Should be available again
        assert_eq!(throttler.available_permits(), 1);
        let _permit2 = throttler.acquire_permit().await.unwrap();
    }
}