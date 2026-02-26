//! Rate limiting for gossip messages to prevent abuse.

use std::collections::HashMap;
use std::time::Instant;

use iroh::PublicKey;

use super::constants::GOSSIP_GLOBAL_BURST;
use super::constants::GOSSIP_GLOBAL_RATE_PER_MINUTE;
use super::constants::GOSSIP_MAX_TRACKED_PEERS;
use super::constants::GOSSIP_PER_PEER_BURST;
use super::constants::GOSSIP_PER_PEER_RATE_PER_MINUTE;

/// Token bucket state for rate limiting.
///
/// Tracks tokens and last update time for a single rate limit bucket.
/// Tokens are replenished over time up to the burst capacity.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Current number of available tokens.
    tokens: f64,
    /// Maximum tokens (burst capacity).
    capacity: f64,
    /// Tokens added per second.
    rate_per_sec: f64,
    /// Last time tokens were updated.
    last_update: Instant,
}

impl TokenBucket {
    /// Create a new token bucket with the specified rate and burst capacity.
    pub fn new(rate_per_minute: u32, burst: u32) -> Self {
        let capacity = f64::from(burst);
        Self {
            tokens: capacity, // Start full
            capacity,
            rate_per_sec: f64::from(rate_per_minute) / 60.0,
            last_update: Instant::now(),
        }
    }

    /// Try to consume one token. Returns true if allowed, false if rate limited.
    ///
    /// Replenishes tokens based on elapsed time before checking.
    pub fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        // Replenish tokens based on elapsed time
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_update = now;

        // Try to consume one token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Per-peer rate limit entry with LRU tracking.
#[derive(Debug)]
pub struct PeerRateEntry {
    pub bucket: TokenBucket,
    /// Last access time for LRU eviction.
    pub last_access: Instant,
}

/// Rate limiter for gossip messages.
///
/// Implements two-tier rate limiting:
/// 1. Per-peer: Limits messages from any single peer (prevents individual abuse)
/// 2. Global: Limits total message rate (prevents cluster-wide DoS)
///
/// Tiger Style: Fixed bounds on tracked peers (LRU eviction), explicit rate limits.
///
/// # Security Properties
///
/// - Prevents single-peer message flooding
/// - Prevents Sybil attacks with many peers each sending moderate traffic
/// - Rate limiting happens BEFORE signature verification to save CPU
/// - Bounded memory: max 256 tracked peers (~16KB overhead)
#[derive(Debug)]
pub struct GossipRateLimiter {
    /// Per-peer rate limit buckets, keyed by PublicKey.
    /// Tiger Style: Bounded to GOSSIP_MAX_TRACKED_PEERS with LRU eviction.
    per_peer: HashMap<PublicKey, PeerRateEntry>,
    /// Global rate limit bucket for all messages.
    global: TokenBucket,
}

impl GossipRateLimiter {
    /// Create a new rate limiter with configured limits.
    pub fn new() -> Self {
        Self {
            per_peer: HashMap::with_capacity(GOSSIP_MAX_TRACKED_PEERS),
            global: TokenBucket::new(GOSSIP_GLOBAL_RATE_PER_MINUTE, GOSSIP_GLOBAL_BURST),
        }
    }

    /// Check if a message from the given peer should be allowed.
    ///
    /// Returns `Ok(())` if allowed, `Err(RateLimitReason)` if rate limited.
    ///
    /// Tiger Style: Check global limit first (cheaper), then per-peer.
    pub fn check(&mut self, peer_id: &PublicKey) -> std::result::Result<(), RateLimitReason> {
        // Check global limit first (single bucket, cheaper)
        if !self.global.try_consume() {
            return Err(RateLimitReason::Global);
        }

        // Check per-peer limit
        let now = Instant::now();

        // Get or create per-peer entry
        if let Some(entry) = self.per_peer.get_mut(peer_id) {
            entry.last_access = now;
            if !entry.bucket.try_consume() {
                return Err(RateLimitReason::PerPeer);
            }
        } else {
            // New peer - enforce LRU eviction if at capacity
            if self.per_peer.len() >= GOSSIP_MAX_TRACKED_PEERS {
                self.evict_oldest();
            }

            // Create new entry (starts with full bucket, so first message always allowed)
            let mut bucket = TokenBucket::new(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST);
            bucket.try_consume(); // Consume token for this message
            self.per_peer.insert(*peer_id, PeerRateEntry {
                bucket,
                last_access: now,
            });
        }

        Ok(())
    }

    /// Evict the least recently accessed peer entry.
    ///
    /// Tiger Style: O(n) scan is acceptable for bounded n=256.
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self.per_peer.iter().min_by_key(|(_, entry)| entry.last_access).map(|(key, _)| *key) {
            self.per_peer.remove(&oldest_key);
        }
    }
}

impl Default for GossipRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reason for rate limiting a gossip message.
#[derive(Debug, Clone, Copy)]
pub enum RateLimitReason {
    /// Per-peer rate limit exceeded.
    PerPeer,
    /// Global rate limit exceeded.
    Global,
}

#[cfg(test)]
mod tests {
    use iroh::SecretKey;

    use super::*;

    #[test]
    fn test_token_bucket_allows_burst() {
        let mut bucket = TokenBucket::new(12, 3); // 12/min, burst 3

        // Should allow burst capacity
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());

        // Fourth should fail (burst exhausted)
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_token_bucket_replenishes() {
        let mut bucket = TokenBucket::new(60, 1); // 1 per second, burst 1

        // Exhaust the bucket
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());

        // Wait for replenishment (1+ second)
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Should have replenished
        assert!(bucket.try_consume());
    }

    #[test]
    fn test_rate_limiter_allows_first_messages() {
        let mut limiter = GossipRateLimiter::new();
        let peer1 = SecretKey::from([1u8; 32]).public();
        let peer2 = SecretKey::from([2u8; 32]).public();

        // First message from each peer should be allowed
        assert!(limiter.check(&peer1).is_ok());
        assert!(limiter.check(&peer2).is_ok());
    }

    #[test]
    fn test_rate_limiter_per_peer_limit() {
        let mut limiter = GossipRateLimiter::new();
        let peer = SecretKey::from([3u8; 32]).public();

        // Exhaust per-peer burst (3 messages)
        assert!(limiter.check(&peer).is_ok());
        assert!(limiter.check(&peer).is_ok());
        assert!(limiter.check(&peer).is_ok());

        // Fourth should be rate limited
        let result = limiter.check(&peer);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RateLimitReason::PerPeer));
    }

    #[test]
    fn test_rate_limiter_per_peer_independent() {
        let mut limiter = GossipRateLimiter::new();
        let peer1 = SecretKey::from([4u8; 32]).public();
        let peer2 = SecretKey::from([5u8; 32]).public();

        // Exhaust peer1's burst
        for _ in 0..3 {
            assert!(limiter.check(&peer1).is_ok());
        }
        assert!(limiter.check(&peer1).is_err());

        // peer2 should still have full burst
        assert!(limiter.check(&peer2).is_ok());
        assert!(limiter.check(&peer2).is_ok());
        assert!(limiter.check(&peer2).is_ok());
    }

    #[test]
    fn test_rate_limiter_lru_eviction() {
        // Test eviction logic directly without rate limiting interference
        let mut limiter = GossipRateLimiter::new();

        // Manually insert entries up to capacity
        let now = Instant::now();
        for i in 0..GOSSIP_MAX_TRACKED_PEERS {
            let mut key_bytes = [0u8; 32];
            key_bytes[0] = (i & 0xFF) as u8;
            key_bytes[1] = ((i >> 8) & 0xFF) as u8;
            let peer = SecretKey::from(key_bytes).public();

            // Directly insert entries to avoid rate limiting
            limiter.per_peer.insert(peer, PeerRateEntry {
                bucket: TokenBucket::new(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST),
                last_access: now,
            });
        }

        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);

        // Trigger eviction by adding one more
        limiter.evict_oldest();
        let new_peer = SecretKey::from([0xFF; 32]).public();
        limiter.per_peer.insert(new_peer, PeerRateEntry {
            bucket: TokenBucket::new(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST),
            last_access: now,
        });

        // Should still be at capacity (not over)
        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);
    }

    #[test]
    fn test_rate_limiter_global_limit() {
        // Test with a custom bucket that has zero replenishment rate
        // to avoid time-based replenishment affecting the test
        let mut limiter = GossipRateLimiter {
            per_peer: HashMap::with_capacity(GOSSIP_MAX_TRACKED_PEERS),
            // Use 0 rate per minute so no replenishment during test
            global: TokenBucket::new(0, GOSSIP_GLOBAL_BURST),
        };

        // Exhaust global burst (100 messages from different peers)
        for i in 0..GOSSIP_GLOBAL_BURST {
            let mut key_bytes = [0u8; 32];
            key_bytes[0] = (i & 0xFF) as u8;
            key_bytes[1] = ((i >> 8) & 0xFF) as u8;
            let peer = SecretKey::from(key_bytes).public();
            assert!(limiter.check(&peer).is_ok(), "message {} should be allowed", i);
        }

        // Next message should hit global limit
        let extra_peer = SecretKey::from([0xFE; 32]).public();
        let result = limiter.check(&extra_peer);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RateLimitReason::Global));
    }

    #[test]
    fn test_evict_oldest_removes_least_recent() {
        let mut limiter = GossipRateLimiter::new();
        let now = Instant::now();

        // Insert peer A with older timestamp
        let peer_a = SecretKey::from([0xAA; 32]).public();
        limiter.per_peer.insert(peer_a, PeerRateEntry {
            bucket: TokenBucket::new(12, 3),
            last_access: now - std::time::Duration::from_secs(100),
        });

        // Insert peer B with newer timestamp
        let peer_b = SecretKey::from([0xBB; 32]).public();
        limiter.per_peer.insert(peer_b, PeerRateEntry {
            bucket: TokenBucket::new(12, 3),
            last_access: now,
        });

        // Evict oldest
        limiter.evict_oldest();

        // peer_a (older) should be evicted, peer_b should remain
        assert!(!limiter.per_peer.contains_key(&peer_a));
        assert!(limiter.per_peer.contains_key(&peer_b));
    }
}
