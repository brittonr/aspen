//! Rate limiting for gossip messages to prevent abuse.

use std::collections::HashMap;
use std::time::Duration;
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
        Self::new_at(rate_per_minute, burst, Instant::now())
    }

    /// Create a new token bucket with an injected clock value.
    pub fn new_at(rate_per_minute: u32, burst: u32, now: Instant) -> Self {
        let capacity = f64::from(burst);
        let rate_per_sec = f64::from(rate_per_minute) / 60.0;

        Self {
            tokens: capacity,
            capacity,
            rate_per_sec,
            last_update: now,
        }
    }

    /// Try to consume one token using the system clock.
    pub fn try_consume(&mut self) -> bool {
        self.try_consume_at(Instant::now())
    }

    /// Try to consume one token with an injected clock value.
    ///
    /// Replenishes tokens based on elapsed time before checking.
    pub fn try_consume_at(&mut self, now: Instant) -> bool {
        debug_assert!(self.tokens >= 0.0, "gossip token bucket tokens must stay non-negative");
        debug_assert!(self.capacity >= 0.0, "gossip token bucket capacity must stay non-negative");

        let effective_now = now.max(self.last_update);
        let elapsed = effective_now.duration_since(self.last_update).as_secs_f64();
        let replenished_tokens = self.tokens + elapsed * self.rate_per_sec;
        self.tokens = replenished_tokens.min(self.capacity);
        self.last_update = effective_now;

        if self.tokens < 1.0 {
            return false;
        }

        self.tokens -= 1.0;
        debug_assert!(self.tokens >= 0.0, "gossip token bucket must not go negative after consume");
        true
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
        Self::new_at(Instant::now())
    }

    /// Create a new rate limiter with an injected clock value.
    pub fn new_at(now: Instant) -> Self {
        Self {
            per_peer: HashMap::with_capacity(GOSSIP_MAX_TRACKED_PEERS),
            global: TokenBucket::new_at(GOSSIP_GLOBAL_RATE_PER_MINUTE, GOSSIP_GLOBAL_BURST, now),
        }
    }

    /// Check if a message from the given peer should be allowed.
    ///
    /// Returns `Ok(())` if allowed, `Err(RateLimitReason)` if rate limited.
    ///
    /// Tiger Style: Check global limit first (cheaper), then per-peer.
    pub fn check(&mut self, peer_id: &PublicKey) -> std::result::Result<(), RateLimitReason> {
        self.check_at(peer_id, Instant::now())
    }

    /// Check if a message from the given peer should be allowed with an injected clock value.
    pub fn check_at(&mut self, peer_id: &PublicKey, now: Instant) -> std::result::Result<(), RateLimitReason> {
        if !self.global.try_consume_at(now) {
            return Err(RateLimitReason::Global);
        }

        if let Some(entry) = self.per_peer.get_mut(peer_id) {
            entry.last_access = entry.last_access.max(now);
            if !entry.bucket.try_consume_at(now) {
                return Err(RateLimitReason::PerPeer);
            }
            return Ok(());
        }

        if self.per_peer.len() >= GOSSIP_MAX_TRACKED_PEERS {
            self.evict_oldest();
        }

        let mut bucket = TokenBucket::new_at(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST, now);
        if !bucket.try_consume_at(now) {
            return Err(RateLimitReason::PerPeer);
        }

        self.per_peer.insert(*peer_id, PeerRateEntry {
            bucket,
            last_access: now,
        });
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
        let now = Instant::now();
        let mut bucket = TokenBucket::new_at(12, 3, now); // 12/min, burst 3

        assert!(bucket.try_consume_at(now));
        assert!(bucket.try_consume_at(now));
        assert!(bucket.try_consume_at(now));
        assert!(!bucket.try_consume_at(now));
    }

    #[test]
    fn test_token_bucket_replenishes_without_sleep() {
        let now = Instant::now();
        let mut bucket = TokenBucket::new_at(60, 1, now); // 1 per second, burst 1

        assert!(bucket.try_consume_at(now));
        assert!(!bucket.try_consume_at(now));
        assert!(bucket.try_consume_at(now + Duration::from_millis(1100)));
    }

    #[test]
    fn test_token_bucket_ignores_backward_time() {
        let now = Instant::now();
        let later = now + Duration::from_secs(1);
        let mut bucket = TokenBucket::new_at(60, 1, now);

        assert!(bucket.try_consume_at(now));
        assert!(bucket.try_consume_at(later));
        assert!(!bucket.try_consume_at(now));
    }

    #[test]
    fn test_token_bucket_backward_time_does_not_double_replenish() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(1);
        let t2 = t1 + Duration::from_millis(100);
        let mut bucket = TokenBucket::new_at(60, 1, t0);

        assert!(bucket.try_consume_at(t0));
        assert!(bucket.try_consume_at(t1));
        assert!(!bucket.try_consume_at(t0));
        assert!(!bucket.try_consume_at(t2));
    }

    #[test]
    fn test_rate_limiter_allows_first_messages() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_at(now);
        let peer1 = SecretKey::from([1u8; 32]).public();
        let peer2 = SecretKey::from([2u8; 32]).public();

        assert!(limiter.check_at(&peer1, now).is_ok());
        assert!(limiter.check_at(&peer2, now).is_ok());
    }

    #[test]
    fn test_rate_limiter_per_peer_limit() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_at(now);
        let peer = SecretKey::from([3u8; 32]).public();

        assert!(limiter.check_at(&peer, now).is_ok());
        assert!(limiter.check_at(&peer, now).is_ok());
        assert!(limiter.check_at(&peer, now).is_ok());

        let result = limiter.check_at(&peer, now);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RateLimitReason::PerPeer));
    }

    #[test]
    fn test_rate_limiter_per_peer_independent() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_at(now);
        let peer1 = SecretKey::from([4u8; 32]).public();
        let peer2 = SecretKey::from([5u8; 32]).public();

        for _ in 0..3 {
            assert!(limiter.check_at(&peer1, now).is_ok());
        }
        assert!(limiter.check_at(&peer1, now).is_err());

        assert!(limiter.check_at(&peer2, now).is_ok());
        assert!(limiter.check_at(&peer2, now).is_ok());
        assert!(limiter.check_at(&peer2, now).is_ok());
    }

    #[test]
    fn test_rate_limiter_lru_eviction() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_at(now);

        for i in 0..GOSSIP_MAX_TRACKED_PEERS {
            let mut key_bytes = [0u8; 32];
            key_bytes[0] = (i & 0xFF) as u8;
            key_bytes[1] = ((i >> 8) & 0xFF) as u8;
            let peer = SecretKey::from(key_bytes).public();

            limiter.per_peer.insert(peer, PeerRateEntry {
                bucket: TokenBucket::new_at(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST, now),
                last_access: now,
            });
        }

        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);

        limiter.evict_oldest();
        let new_peer = SecretKey::from([0xFF; 32]).public();
        limiter.per_peer.insert(new_peer, PeerRateEntry {
            bucket: TokenBucket::new_at(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST, now),
            last_access: now,
        });

        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);
    }

    #[test]
    fn test_rate_limiter_global_limit() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter {
            per_peer: HashMap::with_capacity(GOSSIP_MAX_TRACKED_PEERS),
            global: TokenBucket::new_at(0, GOSSIP_GLOBAL_BURST, now),
        };

        for i in 0..GOSSIP_GLOBAL_BURST {
            let mut key_bytes = [0u8; 32];
            key_bytes[0] = (i & 0xFF) as u8;
            key_bytes[1] = ((i >> 8) & 0xFF) as u8;
            let peer = SecretKey::from(key_bytes).public();
            assert!(limiter.check_at(&peer, now).is_ok(), "message {} should be allowed", i);
        }

        let extra_peer = SecretKey::from([0xFE; 32]).public();
        let result = limiter.check_at(&extra_peer, now);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RateLimitReason::Global));
    }

    #[test]
    fn test_evict_oldest_removes_least_recent() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_at(now);

        let peer_a = SecretKey::from([0xAA; 32]).public();
        limiter.per_peer.insert(peer_a, PeerRateEntry {
            bucket: TokenBucket::new_at(12, 3, now),
            last_access: now - Duration::from_secs(100),
        });

        let peer_b = SecretKey::from([0xBB; 32]).public();
        limiter.per_peer.insert(peer_b, PeerRateEntry {
            bucket: TokenBucket::new_at(12, 3, now),
            last_access: now,
        });

        limiter.evict_oldest();

        assert!(!limiter.per_peer.contains_key(&peer_a));
        assert!(limiter.per_peer.contains_key(&peer_b));
    }

    #[test]
    fn test_check_at_updates_lru_access_time() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_at(now);
        let peer_a = SecretKey::from([0x0A; 32]).public();
        let peer_b = SecretKey::from([0x0B; 32]).public();

        assert!(limiter.check_at(&peer_a, now).is_ok());
        assert!(limiter.check_at(&peer_b, now + Duration::from_secs(1)).is_ok());
        assert!(limiter.check_at(&peer_a, now + Duration::from_secs(2)).is_ok());

        limiter.evict_oldest();

        assert!(limiter.per_peer.contains_key(&peer_a));
        assert!(!limiter.per_peer.contains_key(&peer_b));
    }

    #[test]
    fn test_check_at_backward_time_does_not_rewind_lru() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(1);
        let t2 = t1 + Duration::from_secs(1);
        let mut limiter = GossipRateLimiter::new_at(t0);
        let peer_a = SecretKey::from([0x1A; 32]).public();
        let peer_b = SecretKey::from([0x1B; 32]).public();

        assert!(limiter.check_at(&peer_a, t0).is_ok());
        assert!(limiter.check_at(&peer_b, t1).is_ok());
        assert!(limiter.check_at(&peer_a, t2).is_ok());
        assert!(limiter.check_at(&peer_a, t0).is_ok());

        limiter.evict_oldest();

        assert!(limiter.per_peer.contains_key(&peer_a));
        assert!(!limiter.per_peer.contains_key(&peer_b));
    }
}
