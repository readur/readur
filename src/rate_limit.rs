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
        self.check_inner(key, true).await.map(|_| ())
    }

    /// Count an attempt up front, before the work it limits. Concurrent
    /// attempts therefore can never exceed the limit. Returns the recorded
    /// timestamp, which [`RateLimiter::release`] takes to un-count an attempt
    /// that turned out not to need limiting (e.g. a successful login).
    pub async fn reserve(&self, key: &K) -> Result<Instant, u64> {
        self.check_inner(key, true).await
    }

    /// Remove one attempt previously counted by [`RateLimiter::reserve`].
    pub async fn release(&self, key: &K, reserved_at: Instant) {
        let mut entries = self.entries.lock().await;
        if let Some(timestamps) = entries.get_mut(key) {
            if let Some(pos) = timestamps.iter().position(|t| *t == reserved_at) {
                timestamps.remove(pos);
            }
            if timestamps.is_empty() {
                entries.remove(key);
            }
        }
    }

    async fn check_inner(&self, key: &K, count: bool) -> Result<Instant, u64> {
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
        Ok(now)
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
    /// Failed logins per (lowercased username, client IP) pair (10 per 15 min)
    pub login_failures_by_account_ip: RateLimiter<(String, IpAddr)>,
    /// Failed logins per (lowercased) username from any address
    /// (100 per 15 min). Kept well above the per-pair limit so failures
    /// from other clients cannot easily lock an account.
    pub login_failures_by_username: RateLimiter<String>,
    /// Failed logins per client IP (50 per 15 min)
    pub login_failures_by_ip: RateLimiter<IpAddr>,
    /// Self-registrations per client IP (10/hour)
    pub registration_by_ip: RateLimiter<IpAddr>,
    /// Other auth endpoints (OIDC login start and handoff exchange), per
    /// client IP (30/min)
    pub auth_misc_by_ip: RateLimiter<IpAddr>,
}

impl RateLimiters {
    pub fn new() -> Self {
        Self {
            login_failures_by_account_ip: RateLimiter::new(10, Duration::from_secs(900)),
            login_failures_by_username: RateLimiter::new(100, Duration::from_secs(900)),
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
        self.login_failures_by_account_ip.cleanup().await;
        self.login_failures_by_username.cleanup().await;
        self.login_failures_by_ip.cleanup().await;
        self.registration_by_ip.cleanup().await;
        self.auth_misc_by_ip.cleanup().await;
    }
}

/// Password-guess limits counted before a password is verified; see
/// [`RateLimiters::reserve_password_attempt`].
pub struct PasswordAttempt {
    account: (String, IpAddr),
    reserved: Vec<(Limiter, Instant)>,
}

#[derive(Clone, Copy)]
enum Limiter {
    AccountIp,
    Username,
    Ip,
}

impl RateLimiters {
    /// Count one password attempt against the per-(account, IP), per-account
    /// and (with `per_ip`) per-IP failure limits before the password is
    /// checked, so concurrent guesses cannot slip past a limit. Call
    /// [`PasswordAttempt::succeeded`] when the password was right; a failed
    /// attempt simply stays counted. On `Err(retry_after)` nothing is counted.
    pub async fn reserve_password_attempt(
        &self,
        username: &str,
        ip: IpAddr,
        per_ip: bool,
    ) -> Result<PasswordAttempt, u64> {
        let mut attempt = PasswordAttempt { account: (username.to_lowercase(), ip), reserved: Vec::new() };
        let limiters: &[Limiter] =
            if per_ip { &[Limiter::AccountIp, Limiter::Username, Limiter::Ip] } else { &[Limiter::AccountIp, Limiter::Username] };
        for limiter in limiters {
            let result = match limiter {
                Limiter::AccountIp => self.login_failures_by_account_ip.reserve(&attempt.account).await,
                Limiter::Username => self.login_failures_by_username.reserve(&attempt.account.0).await,
                Limiter::Ip => self.login_failures_by_ip.reserve(&attempt.account.1).await,
            };
            match result {
                Ok(at) => attempt.reserved.push((*limiter, at)),
                Err(retry_after) => {
                    attempt.succeeded(self).await;
                    return Err(retry_after);
                }
            }
        }
        Ok(attempt)
    }
}

impl PasswordAttempt {
    /// Un-count this attempt: it was not a failed guess.
    pub async fn succeeded(self, limiters: &RateLimiters) {
        for (limiter, at) in self.reserved {
            match limiter {
                Limiter::AccountIp => limiters.login_failures_by_account_ip.release(&self.account, at).await,
                Limiter::Username => limiters.login_failures_by_username.release(&self.account.0, at).await,
                Limiter::Ip => limiters.login_failures_by_ip.release(&self.account.1, at).await,
            }
        }
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn released_reservations_do_not_count() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        for _ in 0..5 {
            let at = limiter.reserve(&"k").await.unwrap();
            limiter.release(&"k", at).await;
        }
        limiter.reserve(&"k").await.unwrap();
        limiter.reserve(&"k").await.unwrap();
        assert!(limiter.reserve(&"k").await.is_err());
    }

    #[tokio::test]
    async fn concurrent_password_attempts_cannot_exceed_the_limit() {
        let limiters = Arc::new(RateLimiters::new());
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        // Hold every reservation open, as slow password checks would.
        let tasks: Vec<_> = (0..40)
            .map(|_| {
                let limiters = limiters.clone();
                tokio::spawn(async move { limiters.reserve_password_attempt("Alice", ip, true).await.is_ok() })
            })
            .collect();
        let mut admitted = 0;
        for task in tasks {
            admitted += task.await.unwrap() as usize;
        }
        // The per-(account, IP) limit is 10 failures per window.
        assert_eq!(admitted, 10);
    }

    #[tokio::test]
    async fn successful_attempts_are_released_from_every_limit() {
        let limiters = RateLimiters::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));
        for _ in 0..50 {
            let attempt = limiters.reserve_password_attempt("bob", ip, true).await.unwrap();
            attempt.succeeded(&limiters).await;
        }
        assert!(limiters.login_failures_by_ip.entries.lock().await.is_empty());
        assert!(limiters.login_failures_by_username.entries.lock().await.is_empty());
    }

    #[tokio::test]
    async fn a_refused_attempt_counts_nothing() {
        let limiters = RateLimiters::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 3));
        for _ in 0..10 {
            limiters.reserve_password_attempt("carol", ip, true).await.unwrap();
        }
        assert!(limiters.reserve_password_attempt("carol", ip, true).await.is_err());
        // Only the ten admitted attempts count against the per-IP limit.
        let entries = limiters.login_failures_by_ip.entries.lock().await;
        assert_eq!(entries.get(&ip).map(Vec::len), Some(10));
    }
}
