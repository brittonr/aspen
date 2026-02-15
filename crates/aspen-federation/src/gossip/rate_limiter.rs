//! Rate limiting for federation gossip messages.

use std::collections::HashMap;
use std::time::Instant;

use iroh::PublicKey;

use super::CLUSTER_RATE_BURST;
use super::CLUSTER_RATE_PER_MINUTE;
use super::GLOBAL_RATE_BURST;
use super::GLOBAL_RATE_PER_MINUTE;
use super::MAX_TRACKED_CLUSTERS;

/// Token bucket for rate limiting.
#[derive(Debug, Clone)]
pub(super) struct TokenBucket {
    tokens: f64,
    capacity: f64,
    rate_per_sec: f64,
    last_update: Instant,
}

impl TokenBucket {
    pub(super) fn new(rate_per_minute: u32, burst: u32) -> Self {
        let capacity = f64::from(burst);
        Self {
            tokens: capacity,
            capacity,
            rate_per_sec: f64::from(rate_per_minute) / 60.0,
            last_update: Instant::now(),
        }
    }

    pub(super) fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_update = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Per-cluster rate limit entry.
#[derive(Debug)]
struct ClusterRateEntry {
    bucket: TokenBucket,
    last_access: Instant,
}

/// Rate limiter for federation gossip.
#[derive(Debug)]
pub(super) struct FederationRateLimiter {
    per_cluster: HashMap<PublicKey, ClusterRateEntry>,
    global: TokenBucket,
}

impl FederationRateLimiter {
    pub(super) fn new() -> Self {
        Self {
            per_cluster: HashMap::with_capacity(MAX_TRACKED_CLUSTERS),
            global: TokenBucket::new(GLOBAL_RATE_PER_MINUTE, GLOBAL_RATE_BURST),
        }
    }

    pub(super) fn check(&mut self, cluster_key: &PublicKey) -> bool {
        // Check global limit first
        if !self.global.try_consume() {
            return false;
        }

        let now = Instant::now();

        // Check per-cluster limit
        if let Some(entry) = self.per_cluster.get_mut(cluster_key) {
            entry.last_access = now;
            if !entry.bucket.try_consume() {
                return false;
            }
        } else {
            // New cluster - enforce LRU eviction
            if self.per_cluster.len() >= MAX_TRACKED_CLUSTERS {
                self.evict_oldest();
            }

            let mut bucket = TokenBucket::new(CLUSTER_RATE_PER_MINUTE, CLUSTER_RATE_BURST);
            bucket.try_consume();
            self.per_cluster.insert(*cluster_key, ClusterRateEntry {
                bucket,
                last_access: now,
            });
        }

        true
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) =
            self.per_cluster.iter().min_by_key(|(_, entry)| entry.last_access).map(|(key, _)| *key)
        {
            self.per_cluster.remove(&oldest_key);
        }
    }
}
