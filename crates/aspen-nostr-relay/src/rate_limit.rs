//! Token-bucket rate limiting for EVENT submissions.
//!
//! Two independent dimensions: per-source-IP and per-author-pubkey.
//! Stale buckets are evicted periodically to bound memory.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::constants;

/// A single token bucket.
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    rate: f64,
    burst: f64,
}

impl TokenBucket {
    fn new(rate: f64, burst: f64) -> Self {
        Self {
            tokens: burst,
            last_refill: Instant::now(),
            rate,
            burst,
        }
    }

    /// Try to consume one token. Returns `true` if allowed, `false` if rate limited.
    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate).min(self.burst);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Entry in the rate limit map, wrapping the bucket with a last-access time.
struct BucketEntry {
    bucket: TokenBucket,
    last_access: Instant,
}

/// Rate limiter with per-IP and per-pubkey token buckets.
pub struct RateLimiter {
    ip_buckets: DashMap<IpAddr, BucketEntry>,
    pubkey_buckets: DashMap<String, BucketEntry>,
    ip_rate: f64,
    ip_burst: f64,
    pubkey_rate: f64,
    pubkey_burst: f64,
    enabled: bool,
}

impl RateLimiter {
    /// Create a new rate limiter from config values.
    ///
    /// If both IP and pubkey rates are zero, rate limiting is disabled.
    pub fn new(
        events_per_second_per_ip: u32,
        events_burst_per_ip: u32,
        events_per_second_per_pubkey: u32,
        events_burst_per_pubkey: u32,
    ) -> Self {
        let enabled = events_per_second_per_ip > 0 || events_per_second_per_pubkey > 0;
        Self {
            ip_buckets: DashMap::new(),
            pubkey_buckets: DashMap::new(),
            ip_rate: events_per_second_per_ip as f64,
            ip_burst: events_burst_per_ip as f64,
            pubkey_rate: events_per_second_per_pubkey as f64,
            pubkey_burst: events_burst_per_pubkey as f64,
            enabled,
        }
    }

    /// Whether rate limiting is active.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check IP rate limit. Returns `true` if allowed.
    pub fn check_ip(&self, addr: IpAddr) -> bool {
        if self.ip_rate == 0.0 {
            return true;
        }
        let now = Instant::now();
        let mut entry = self.ip_buckets.entry(addr).or_insert_with(|| BucketEntry {
            bucket: TokenBucket::new(self.ip_rate, self.ip_burst),
            last_access: now,
        });
        entry.last_access = now;
        entry.bucket.try_consume()
    }

    /// Check pubkey rate limit. Returns `true` if allowed.
    pub fn check_pubkey(&self, pubkey_hex: &str) -> bool {
        if self.pubkey_rate == 0.0 {
            return true;
        }
        let now = Instant::now();
        let mut entry = self.pubkey_buckets.entry(pubkey_hex.to_string()).or_insert_with(|| BucketEntry {
            bucket: TokenBucket::new(self.pubkey_rate, self.pubkey_burst),
            last_access: now,
        });
        entry.last_access = now;
        entry.bucket.try_consume()
    }

    /// Spawn a background task that periodically evicts stale buckets.
    pub fn start_cleanup_task(self: &Arc<Self>, cancel: CancellationToken) {
        let limiter = Arc::clone(self);
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(constants::RATE_LIMIT_CLEANUP_INTERVAL_SECS);
            let ttl = std::time::Duration::from_secs(constants::RATE_LIMIT_BUCKET_TTL_SECS);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        let now = Instant::now();
                        let ip_before = limiter.ip_buckets.len();
                        let pk_before = limiter.pubkey_buckets.len();
                        limiter.ip_buckets.retain(|_, e| now.duration_since(e.last_access) < ttl);
                        limiter.pubkey_buckets.retain(|_, e| now.duration_since(e.last_access) < ttl);
                        let ip_evicted = ip_before - limiter.ip_buckets.len();
                        let pk_evicted = pk_before - limiter.pubkey_buckets.len();
                        if ip_evicted > 0 || pk_evicted > 0 {
                            debug!(ip_evicted, pk_evicted, "rate limit bucket cleanup");
                        }
                    }
                    _ = cancel.cancelled() => break,
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_allows_burst() {
        let mut bucket = TokenBucket::new(10.0, 20.0);
        for _ in 0..20 {
            assert!(bucket.try_consume());
        }
        // 21st should fail — burst exhausted
        assert!(!bucket.try_consume());
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(10.0, 5.0);
        // Exhaust burst
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume());

        // Simulate time passing (cheat by moving last_refill back)
        bucket.last_refill = Instant::now() - std::time::Duration::from_secs(1);
        // Should have ~10 tokens refilled, capped at burst (5)
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume());
    }

    #[test]
    fn rate_limiter_independent_ip_buckets() {
        let limiter = RateLimiter::new(10, 2, 0, 0);
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        // Exhaust ip1's burst
        assert!(limiter.check_ip(ip1));
        assert!(limiter.check_ip(ip1));
        assert!(!limiter.check_ip(ip1));

        // ip2 should still work
        assert!(limiter.check_ip(ip2));
    }

    #[test]
    fn rate_limiter_independent_pubkey_buckets() {
        let limiter = RateLimiter::new(0, 0, 5, 2);
        let pk1 = "a".repeat(64);
        let pk2 = "b".repeat(64);

        assert!(limiter.check_pubkey(&pk1));
        assert!(limiter.check_pubkey(&pk1));
        assert!(!limiter.check_pubkey(&pk1));

        assert!(limiter.check_pubkey(&pk2));
    }

    #[test]
    fn rate_limiter_disabled_when_zero() {
        let limiter = RateLimiter::new(0, 0, 0, 0);
        assert!(!limiter.is_enabled());
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        // Should always allow
        for _ in 0..100 {
            assert!(limiter.check_ip(ip));
            assert!(limiter.check_pubkey("anything"));
        }
    }

    #[tokio::test]
    async fn stale_bucket_cleanup() {
        let limiter = Arc::new(RateLimiter::new(10, 5, 5, 5));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        limiter.check_ip(ip);
        assert_eq!(limiter.ip_buckets.len(), 1);

        // Manually age the bucket beyond TTL
        limiter.ip_buckets.get_mut(&ip).unwrap().last_access =
            Instant::now() - std::time::Duration::from_secs(constants::RATE_LIMIT_BUCKET_TTL_SECS + 1);

        // Run cleanup inline
        let ttl = std::time::Duration::from_secs(constants::RATE_LIMIT_BUCKET_TTL_SECS);
        let now = Instant::now();
        limiter.ip_buckets.retain(|_, e| now.duration_since(e.last_access) < ttl);
        assert_eq!(limiter.ip_buckets.len(), 0);
    }
}
