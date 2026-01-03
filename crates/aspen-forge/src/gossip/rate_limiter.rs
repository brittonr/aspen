//! Rate limiting for Forge gossip messages.
//!
//! Implements two-tier rate limiting to prevent DoS attacks:
//! - Per-peer rate limiting to prevent individual peers from flooding
//! - Global rate limiting to prevent distributed flooding attacks
//!
//! Rate limiting is applied BEFORE signature verification to save CPU.

use std::collections::HashMap;
use std::time::Instant;

use iroh::PublicKey;

use crate::constants::{
    FORGE_GOSSIP_GLOBAL_BURST, FORGE_GOSSIP_GLOBAL_RATE_PER_MINUTE, FORGE_GOSSIP_MAX_TRACKED_PEERS,
    FORGE_GOSSIP_PER_PEER_BURST, FORGE_GOSSIP_PER_PEER_RATE_PER_MINUTE,
};

/// Reason for rate limiting a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitReason {
    /// Per-peer rate limit exceeded.
    PerPeer,
    /// Global rate limit exceeded.
    Global,
}

impl std::fmt::Display for RateLimitReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitReason::PerPeer => write!(f, "per-peer rate limit exceeded"),
            RateLimitReason::Global => write!(f, "global rate limit exceeded"),
        }
    }
}

/// Token bucket for rate limiting.
///
/// Implements the token bucket algorithm with configurable rate and burst capacity.
#[derive(Debug)]
struct TokenBucket {
    /// Current number of tokens.
    tokens: f64,
    /// Maximum capacity (burst size).
    capacity: f64,
    /// Token replenishment rate (tokens per second).
    rate_per_sec: f64,
    /// Last time tokens were replenished.
    last_update: Instant,
}

impl TokenBucket {
    /// Create a new token bucket.
    ///
    /// # Arguments
    ///
    /// - `rate_per_minute`: Number of tokens replenished per minute
    /// - `burst`: Maximum burst capacity (tokens start full)
    fn new(rate_per_minute: u32, burst: u32) -> Self {
        Self {
            tokens: burst as f64,
            capacity: burst as f64,
            rate_per_sec: rate_per_minute as f64 / 60.0,
            last_update: Instant::now(),
        }
    }

    /// Try to consume one token.
    ///
    /// Returns true if a token was available and consumed, false if rate limited.
    fn try_consume(&mut self) -> bool {
        // Replenish tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
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

/// Entry for tracking per-peer rate limiting.
#[derive(Debug)]
struct PeerRateEntry {
    bucket: TokenBucket,
    last_access: Instant,
}

impl PeerRateEntry {
    fn new() -> Self {
        Self {
            bucket: TokenBucket::new(FORGE_GOSSIP_PER_PEER_RATE_PER_MINUTE, FORGE_GOSSIP_PER_PEER_BURST),
            last_access: Instant::now(),
        }
    }
}

/// Two-tier rate limiter for Forge gossip messages.
///
/// Provides both per-peer and global rate limiting to prevent:
/// - Individual peers from flooding the network (per-peer)
/// - Distributed attacks from many peers (global)
///
/// Uses LRU eviction for per-peer tracking to bound memory usage.
pub struct ForgeGossipRateLimiter {
    /// Per-peer rate limiters.
    per_peer: HashMap<PublicKey, PeerRateEntry>,
    /// Global rate limiter.
    global: TokenBucket,
}

impl ForgeGossipRateLimiter {
    /// Create a new rate limiter with default Forge-specific limits.
    pub fn new() -> Self {
        Self {
            per_peer: HashMap::new(),
            global: TokenBucket::new(FORGE_GOSSIP_GLOBAL_RATE_PER_MINUTE, FORGE_GOSSIP_GLOBAL_BURST),
        }
    }

    /// Check if a message from the given peer is allowed.
    ///
    /// Rate limiting is applied in order:
    /// 1. Global limit (cheapest check)
    /// 2. Per-peer limit (hashmap lookup)
    ///
    /// Returns Ok(()) if allowed, Err(RateLimitReason) if rate limited.
    pub fn check(&mut self, peer_id: &PublicKey) -> Result<(), RateLimitReason> {
        // Check global limit first (cheaper)
        if !self.global.try_consume() {
            return Err(RateLimitReason::Global);
        }

        // Evict oldest entry if at capacity
        if self.per_peer.len() >= FORGE_GOSSIP_MAX_TRACKED_PEERS && !self.per_peer.contains_key(peer_id) {
            self.evict_oldest();
        }

        // Get or create per-peer entry
        let entry = self.per_peer.entry(*peer_id).or_insert_with(PeerRateEntry::new);
        entry.last_access = Instant::now();

        // Check per-peer limit
        if !entry.bucket.try_consume() {
            return Err(RateLimitReason::PerPeer);
        }

        Ok(())
    }

    /// Evict the least recently accessed peer entry.
    fn evict_oldest(&mut self) {
        if self.per_peer.is_empty() {
            return;
        }

        // Find the oldest entry
        let oldest_key = self.per_peer.iter().min_by_key(|(_, entry)| entry.last_access).map(|(key, _)| *key);

        if let Some(key) = oldest_key {
            self.per_peer.remove(&key);
        }
    }

    /// Get the number of tracked peers.
    #[cfg(test)]
    pub fn tracked_peer_count(&self) -> usize {
        self.per_peer.len()
    }

    /// Add a peer entry for testing (bypasses rate limits).
    #[cfg(test)]
    pub fn add_peer_for_test(&mut self, peer_id: &PublicKey) {
        // Evict oldest entry if at capacity
        if self.per_peer.len() >= FORGE_GOSSIP_MAX_TRACKED_PEERS && !self.per_peer.contains_key(peer_id) {
            self.evict_oldest();
        }

        let entry = self.per_peer.entry(*peer_id).or_insert_with(PeerRateEntry::new);
        entry.last_access = Instant::now();
    }
}

impl Default for ForgeGossipRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    #[test]
    fn test_token_bucket_allows_burst() {
        let mut bucket = TokenBucket::new(60, 5); // 1 per second, burst of 5

        // Should allow burst
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }

        // Next one should fail (burst exhausted)
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_token_bucket_replenishes() {
        let mut bucket = TokenBucket::new(6000, 1); // 100 per second, burst of 1

        // Exhaust the bucket
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());

        // Wait a bit for replenishment (in real code; we simulate by adjusting last_update)
        bucket.last_update = Instant::now() - std::time::Duration::from_millis(100);

        // Should have replenished some tokens
        assert!(bucket.try_consume());
    }

    #[test]
    fn test_rate_limiter_allows_initial_messages() {
        let mut limiter = ForgeGossipRateLimiter::new();
        let peer = test_key();

        // Should allow initial burst
        for _ in 0..FORGE_GOSSIP_PER_PEER_BURST {
            assert!(limiter.check(&peer).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_after_burst() {
        let mut limiter = ForgeGossipRateLimiter::new();
        let peer = test_key();

        // Exhaust burst
        for _ in 0..FORGE_GOSSIP_PER_PEER_BURST {
            let _ = limiter.check(&peer);
        }

        // Next should be rate limited (per-peer)
        assert_eq!(limiter.check(&peer), Err(RateLimitReason::PerPeer));
    }

    #[test]
    fn test_rate_limiter_tracks_different_peers() {
        let mut limiter = ForgeGossipRateLimiter::new();
        let peer1 = test_key();
        let peer2 = test_key();

        // Exhaust peer1's burst
        for _ in 0..FORGE_GOSSIP_PER_PEER_BURST {
            let _ = limiter.check(&peer1);
        }

        // peer1 should be limited
        assert_eq!(limiter.check(&peer1), Err(RateLimitReason::PerPeer));

        // peer2 should still have quota
        assert!(limiter.check(&peer2).is_ok());
    }

    #[test]
    fn test_rate_limiter_evicts_oldest() {
        let mut limiter = ForgeGossipRateLimiter::new();

        // Add entries up to the limit (using test helper to bypass global limit)
        for _ in 0..FORGE_GOSSIP_MAX_TRACKED_PEERS {
            let peer = test_key();
            limiter.add_peer_for_test(&peer);
        }

        assert_eq!(limiter.tracked_peer_count(), FORGE_GOSSIP_MAX_TRACKED_PEERS);

        // Add one more - should evict oldest
        let new_peer = test_key();
        limiter.add_peer_for_test(&new_peer);

        // Should still be at max (evicted one, added one)
        assert_eq!(limiter.tracked_peer_count(), FORGE_GOSSIP_MAX_TRACKED_PEERS);
    }

    #[test]
    fn test_rate_limit_reason_display() {
        assert_eq!(RateLimitReason::PerPeer.to_string(), "per-peer rate limit exceeded");
        assert_eq!(RateLimitReason::Global.to_string(), "global rate limit exceeded");
    }
}
