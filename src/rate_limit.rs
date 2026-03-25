/*!
 * Rate Limiting for API Endpoints
 *
 * Provides both IP-based rate limiting (for public endpoints) and
 * user-based rate limiting (for authenticated endpoints).
 */

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

/// A generic rate limiter that tracks request counts per key within a sliding window.
#[derive(Clone)]
pub struct RateLimiter<K: std::hash::Hash + Eq + Clone> {
    entries: Arc<Mutex<HashMap<K, Vec<Instant>>>>,
    max_requests: u32,
    window: Duration,
}

impl<K: std::hash::Hash + Eq + Clone> RateLimiter<K> {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// Check if a request is allowed for the given key.
    /// Returns Ok(()) if allowed, Err(remaining_seconds) if rate limited.
    pub async fn check(&self, key: &K) -> Result<(), u64> {
        let mut entries = self.entries.lock().await;
        let now = Instant::now();
        let cutoff = now - self.window;

        let timestamps = entries.entry(key.clone()).or_insert_with(Vec::new);

        // Remove expired entries
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.max_requests as usize {
            // Calculate when the oldest entry in the window expires
            let oldest = timestamps.first().copied().unwrap_or(now);
            let retry_after = self.window.saturating_sub(now.duration_since(oldest));
            return Err(retry_after.as_secs().max(1));
        }

        timestamps.push(now);
        Ok(())
    }

    /// Periodically clean up expired entries to prevent memory growth.
    pub async fn cleanup(&self) {
        let mut entries = self.entries.lock().await;
        let cutoff = Instant::now() - self.window;
        entries.retain(|_, timestamps| {
            timestamps.retain(|t| *t > cutoff);
            !timestamps.is_empty()
        });
    }
}

/// Collection of rate limiters for different endpoint categories.
#[derive(Clone)]
pub struct RateLimiters {
    /// IP-based limiter for public shared link password attempts (10/min per IP)
    pub shared_link_password: RateLimiter<IpAddr>,
    /// IP-based limiter for general public shared link access (60/min per IP)
    pub shared_link_public: RateLimiter<IpAddr>,
    /// User-based limiter for comment creation (10/min per user)
    pub comment_creation: RateLimiter<Uuid>,
    /// User-based limiter for shared link creation (20/hour per user)
    pub shared_link_creation: RateLimiter<Uuid>,
}

impl RateLimiters {
    pub fn new() -> Self {
        Self {
            shared_link_password: RateLimiter::new(10, Duration::from_secs(60)),
            shared_link_public: RateLimiter::new(60, Duration::from_secs(60)),
            comment_creation: RateLimiter::new(10, Duration::from_secs(60)),
            shared_link_creation: RateLimiter::new(20, Duration::from_secs(3600)),
        }
    }

    /// Run cleanup on all limiters. Call this periodically (e.g., every 5 minutes).
    pub async fn cleanup_all(&self) {
        self.shared_link_password.cleanup().await;
        self.shared_link_public.cleanup().await;
        self.comment_creation.cleanup().await;
        self.shared_link_creation.cleanup().await;
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}
