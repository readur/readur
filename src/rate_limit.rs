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

    /// Check if a request is allowed for the given key, and count it.
    /// Returns Ok(()) if allowed, Err(remaining_seconds) if rate limited.
    pub async fn check(&self, key: &K) -> Result<(), u64> {
        self.check_inner(key, true).await
    }

    /// Check whether the key is currently limited without counting an event.
    /// Pair with [`RateLimiter::record`] to limit only failed attempts.
    pub async fn peek(&self, key: &K) -> Result<(), u64> {
        self.check_inner(key, false).await
    }

    /// Count an event for the key (e.g. a failed login).
    pub async fn record(&self, key: &K) {
        let mut entries = self.entries.lock().await;
        entries.entry(key.clone()).or_insert_with(Vec::new).push(Instant::now());
    }

    async fn check_inner(&self, key: &K, count: bool) -> Result<(), u64> {
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

        if count {
            timestamps.push(now);
        }
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
    /// User-based limiter for API key creation (10/hour per user)
    pub api_key_creation: RateLimiter<Uuid>,
    /// Failed logins per (lowercased) username (10 per 15 min)
    pub login_failures_by_username: RateLimiter<String>,
    /// Failed logins per client IP (50 per 15 min)
    pub login_failures_by_ip: RateLimiter<IpAddr>,
    /// Self-registrations per client IP (10/hour)
    pub registration_by_ip: RateLimiter<IpAddr>,
    /// Session-bound auth endpoints (OIDC handoff exchange, password change),
    /// per client IP (30/min)
    pub auth_misc_by_ip: RateLimiter<IpAddr>,
}

impl RateLimiters {
    pub fn new() -> Self {
        Self {
            login_failures_by_username: RateLimiter::new(10, Duration::from_secs(900)),
            login_failures_by_ip: RateLimiter::new(50, Duration::from_secs(900)),
            registration_by_ip: RateLimiter::new(10, Duration::from_secs(3600)),
            auth_misc_by_ip: RateLimiter::new(30, Duration::from_secs(60)),
            shared_link_password: RateLimiter::new(10, Duration::from_secs(60)),
            shared_link_public: RateLimiter::new(60, Duration::from_secs(60)),
            comment_creation: RateLimiter::new(10, Duration::from_secs(60)),
            shared_link_creation: RateLimiter::new(20, Duration::from_secs(3600)),
            api_key_creation: RateLimiter::new(10, Duration::from_secs(3600)),
        }
    }

    /// Run cleanup on all limiters. Call this periodically (e.g., every 5 minutes).
    pub async fn cleanup_all(&self) {
        self.shared_link_password.cleanup().await;
        self.shared_link_public.cleanup().await;
        self.comment_creation.cleanup().await;
        self.shared_link_creation.cleanup().await;
        self.api_key_creation.cleanup().await;
        self.login_failures_by_username.cleanup().await;
        self.login_failures_by_ip.cleanup().await;
        self.registration_by_ip.cleanup().await;
        self.auth_misc_by_ip.cleanup().await;
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}
