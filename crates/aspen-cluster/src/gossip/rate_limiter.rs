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
use crate::verified::GossipAdmissionTransition as VerifiedGossipAdmissionTransition;
use crate::verified::PeerRateState as VerifiedPeerRateState;
use crate::verified::TokenBucketConfig as VerifiedTokenBucketConfig;
use crate::verified::TokenBucketState as VerifiedTokenBucketState;
use crate::verified::TwoTierRateLimitReason;
use crate::verified::evaluate_gossip_admission;
use crate::verified::should_evict_oldest;
use crate::verified::try_consume_token_at;

/// Token bucket state for rate limiting.
#[derive(Debug, Clone)]
struct TokenBucket {
    config: VerifiedTokenBucketConfig,
    state: VerifiedTokenBucketState,
}

impl TokenBucket {
    /// Create a new token bucket with the specified rate and burst capacity.
    fn new(rate_per_minute: u32, burst: u32) -> Self {
        Self::new_at(rate_per_minute, burst, 0)
    }

    /// Create a new token bucket with an injected timestamp in milliseconds.
    fn new_at(rate_per_minute: u32, burst: u32, now_ms: u64) -> Self {
        let config = VerifiedTokenBucketConfig::new(rate_per_minute, burst);
        let state = VerifiedTokenBucketState::full(config, now_ms);
        Self { config, state }
    }

    /// Try to consume one token using the zero-based test clock.
    fn try_consume(&mut self) -> bool {
        self.try_consume_at(0)
    }

    /// Try to consume one token with an injected timestamp in milliseconds.
    fn try_consume_at(&mut self, now_ms: u64) -> bool {
        debug_assert!(self.state.tokens >= 0.0, "gossip token bucket tokens must stay non-negative");
        debug_assert!(self.config.capacity > 0, "gossip token bucket capacity must stay positive");

        let transition = try_consume_token_at(self.state, self.config, now_ms);
        self.state = transition.next_state;
        transition.result.is_consumed()
    }

    #[inline]
    fn config(&self) -> VerifiedTokenBucketConfig {
        self.config
    }

    #[inline]
    fn state(&self) -> VerifiedTokenBucketState {
        self.state
    }

    #[inline]
    fn set_state(&mut self, state: VerifiedTokenBucketState) {
        self.state = state;
    }

    #[cfg(test)]
    fn tokens(&self) -> f64 {
        self.state.tokens
    }

    #[cfg(test)]
    fn last_update_ms(&self) -> u64 {
        self.state.last_update_ms
    }
}

/// Per-peer rate limit entry with LRU tracking.
#[derive(Debug)]
struct PeerRateEntry {
    bucket: TokenBucket,
    /// Last access time for LRU eviction.
    last_access_ms: u64,
}

impl PeerRateEntry {
    fn from_verified_state(config: VerifiedTokenBucketConfig, state: VerifiedPeerRateState) -> Self {
        Self {
            bucket: TokenBucket {
                config,
                state: state.bucket,
            },
            last_access_ms: state.last_access_ms,
        }
    }

    fn verified_state(&self) -> VerifiedPeerRateState {
        VerifiedPeerRateState {
            bucket: self.bucket.state(),
            last_access_ms: self.last_access_ms,
        }
    }

    fn apply_verified_state(&mut self, state: VerifiedPeerRateState) {
        self.bucket.set_state(state.bucket);
        self.last_access_ms = state.last_access_ms;
    }
}

/// Rate limiter for gossip messages.
///
/// Implements two-tier rate limiting:
/// 1. Per-peer: Limits messages from any single peer (prevents individual abuse)
/// 2. Global: Limits total message rate (prevents cluster-wide DoS)
///
/// Tiger Style: Fixed bounds on tracked peers (LRU eviction), explicit rate limits.
#[derive(Debug)]
pub struct GossipRateLimiter {
    /// Shared origin used to convert runtime `Instant` values into deterministic ms.
    time_origin: Instant,
    /// Per-peer rate limit buckets, keyed by PublicKey.
    per_peer: HashMap<PublicKey, PeerRateEntry>,
    /// Maximum tracked peers before LRU eviction.
    max_tracked_peers: u32,
    /// Shared per-peer bucket configuration.
    per_peer_config: VerifiedTokenBucketConfig,
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
        Self::new_with_limits_at(
            now,
            GOSSIP_GLOBAL_RATE_PER_MINUTE,
            GOSSIP_GLOBAL_BURST,
            GOSSIP_PER_PEER_RATE_PER_MINUTE,
            GOSSIP_PER_PEER_BURST,
            GOSSIP_MAX_TRACKED_PEERS as u32,
        )
    }

    /// Check if a message from the given peer should be allowed.
    pub fn check(&mut self, peer_id: &PublicKey) -> std::result::Result<(), RateLimitReason> {
        self.check_at(peer_id, Instant::now())
    }

    /// Check if a message from the given peer should be allowed with an injected clock value.
    pub fn check_at(&mut self, peer_id: &PublicKey, now: Instant) -> std::result::Result<(), RateLimitReason> {
        let now_ms = self.instant_to_ms(now);
        let peer_state = self.per_peer.get(peer_id).map(PeerRateEntry::verified_state);
        let transition = evaluate_gossip_admission(
            self.global.state(),
            self.global.config(),
            peer_state,
            self.per_peer_config,
            now_ms,
        );

        self.global.set_state(transition.next_global);
        if let Some(next_peer) = transition.next_peer {
            self.upsert_peer_state(*peer_id, next_peer);
        }

        match transition.outcome {
            Ok(()) => Ok(()),
            Err(reason) => Err(RateLimitReason::from(reason)),
        }
    }

    /// Evict the least recently accessed peer entry.
    ///
    /// Tiger Style: O(n) scan is acceptable for bounded n=256.
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) =
            self.per_peer.iter().min_by_key(|(_, entry)| entry.last_access_ms).map(|(key, _)| *key)
        {
            self.per_peer.remove(&oldest_key);
        }
    }

    fn new_with_limits_at(
        now: Instant,
        global_rate_per_minute: u32,
        global_burst: u32,
        per_peer_rate_per_minute: u32,
        per_peer_burst: u32,
        max_tracked_peers: u32,
    ) -> Self {
        let now_ms = 0;
        Self {
            time_origin: now,
            per_peer: HashMap::with_capacity(max_tracked_peers as usize),
            max_tracked_peers,
            per_peer_config: VerifiedTokenBucketConfig::new(per_peer_rate_per_minute, per_peer_burst),
            global: TokenBucket::new_at(global_rate_per_minute, global_burst, now_ms),
        }
    }

    fn instant_to_ms(&self, now: Instant) -> u64 {
        let Some(elapsed) = now.checked_duration_since(self.time_origin) else {
            return 0;
        };
        duration_to_millis_u64(elapsed)
    }

    fn upsert_peer_state(&mut self, peer_id: PublicKey, next_peer: VerifiedPeerRateState) {
        if let Some(entry) = self.per_peer.get_mut(&peer_id) {
            entry.apply_verified_state(next_peer);
            return;
        }

        if should_evict_oldest(self.per_peer.len() as u32, self.max_tracked_peers) {
            self.evict_oldest();
        }

        self.per_peer.insert(peer_id, PeerRateEntry::from_verified_state(self.per_peer_config, next_peer));
    }
}

impl Default for GossipRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reason for rate limiting a gossip message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitReason {
    /// Per-peer rate limit exceeded.
    PerPeer,
    /// Global rate limit exceeded.
    Global,
}

impl From<TwoTierRateLimitReason> for RateLimitReason {
    fn from(value: TwoTierRateLimitReason) -> Self {
        match value {
            TwoTierRateLimitReason::Global => Self::Global,
            TwoTierRateLimitReason::PerPeer => Self::PerPeer,
        }
    }
}

#[inline]
fn duration_to_millis_u64(duration: Duration) -> u64 {
    let millis = duration.as_millis();
    if millis > u128::from(u64::MAX) {
        u64::MAX
    } else {
        millis as u64
    }
}

#[cfg(test)]
mod tests {
    use iroh::SecretKey;

    use super::*;
    use crate::verified::GossipAdmissionTransition;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 0.001, "actual={actual}, expected={expected}");
    }

    #[test]
    fn test_token_bucket_allows_burst() {
        let mut bucket = TokenBucket::new_at(12, 3, 0);

        assert!(bucket.try_consume_at(0));
        assert!(bucket.try_consume_at(0));
        assert!(bucket.try_consume_at(0));
        assert!(!bucket.try_consume_at(0));
    }

    #[test]
    fn test_token_bucket_replenishes_without_sleep() {
        let mut bucket = TokenBucket::new_at(60, 1, 0);

        assert!(bucket.try_consume_at(0));
        assert!(!bucket.try_consume_at(0));
        assert!(bucket.try_consume_at(1100));
    }

    #[test]
    fn test_token_bucket_ignores_backward_time() {
        let mut bucket = TokenBucket::new_at(60, 1, 0);

        assert!(bucket.try_consume_at(0));
        assert!(bucket.try_consume_at(1000));
        assert!(!bucket.try_consume_at(0));
    }

    #[test]
    fn test_token_bucket_backward_time_does_not_double_replenish() {
        let mut bucket = TokenBucket::new_at(60, 1, 0);

        assert!(bucket.try_consume_at(0));
        assert!(bucket.try_consume_at(1000));
        assert!(!bucket.try_consume_at(0));
        assert!(!bucket.try_consume_at(1100));
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
        assert_eq!(result, Err(RateLimitReason::PerPeer));
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
        assert_eq!(limiter.check_at(&peer1, now), Err(RateLimitReason::PerPeer));

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

            limiter.per_peer.insert(
                peer,
                PeerRateEntry::from_verified_state(limiter.per_peer_config, VerifiedPeerRateState {
                    bucket: VerifiedTokenBucketState::full(limiter.per_peer_config, i as u64),
                    last_access_ms: i as u64,
                }),
            );
        }

        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);

        limiter.evict_oldest();
        let new_peer = SecretKey::from([0xFF; 32]).public();
        limiter.per_peer.insert(
            new_peer,
            PeerRateEntry::from_verified_state(limiter.per_peer_config, VerifiedPeerRateState {
                bucket: VerifiedTokenBucketState::full(limiter.per_peer_config, 0),
                last_access_ms: 999,
            }),
        );

        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);
    }

    #[test]
    fn test_rate_limiter_global_limit() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_with_limits_at(
            now,
            0,
            GOSSIP_GLOBAL_BURST,
            0,
            GOSSIP_PER_PEER_BURST,
            GOSSIP_MAX_TRACKED_PEERS as u32,
        );

        for i in 0..GOSSIP_GLOBAL_BURST {
            let mut key_bytes = [0u8; 32];
            key_bytes[0] = (i & 0xFF) as u8;
            key_bytes[1] = ((i >> 8) & 0xFF) as u8;
            let peer = SecretKey::from(key_bytes).public();
            assert!(limiter.check_at(&peer, now).is_ok(), "message {i} should be allowed");
        }

        let extra_peer = SecretKey::from([0xFE; 32]).public();
        let result = limiter.check_at(&extra_peer, now);
        assert_eq!(result, Err(RateLimitReason::Global));
    }

    #[test]
    fn test_per_peer_rejection_does_not_spend_global_budget() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_with_limits_at(now, 0, 2, 0, 1, 8);
        let noisy_peer = SecretKey::from([0xA1; 32]).public();
        let healthy_peer_a = SecretKey::from([0xB2; 32]).public();
        let healthy_peer_b = SecretKey::from([0xC3; 32]).public();

        assert!(limiter.check_at(&noisy_peer, now).is_ok());
        assert_eq!(limiter.check_at(&noisy_peer, now), Err(RateLimitReason::PerPeer));
        assert_eq!(limiter.check_at(&noisy_peer, now), Err(RateLimitReason::PerPeer));

        assert_close(limiter.global.tokens(), 1.0);
        assert!(limiter.check_at(&healthy_peer_a, now).is_ok());
        assert_eq!(limiter.check_at(&healthy_peer_b, now), Err(RateLimitReason::Global));
    }

    #[test]
    fn test_evict_oldest_removes_least_recent() {
        let now = Instant::now();
        let mut limiter = GossipRateLimiter::new_at(now);

        let peer_a = SecretKey::from([0xAA; 32]).public();
        limiter.per_peer.insert(
            peer_a,
            PeerRateEntry::from_verified_state(limiter.per_peer_config, VerifiedPeerRateState {
                bucket: VerifiedTokenBucketState::full(limiter.per_peer_config, 0),
                last_access_ms: 0,
            }),
        );

        let peer_b = SecretKey::from([0xBB; 32]).public();
        limiter.per_peer.insert(
            peer_b,
            PeerRateEntry::from_verified_state(limiter.per_peer_config, VerifiedPeerRateState {
                bucket: VerifiedTokenBucketState::full(limiter.per_peer_config, 100),
                last_access_ms: 100,
            }),
        );

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

    #[test]
    fn test_runtime_wrapper_matches_verified_core_for_backward_timestamps() {
        let origin = Instant::now();
        let mut limiter = GossipRateLimiter::new_with_limits_at(origin, 60, 2, 60, 2, 8);
        let peer = SecretKey::from([0x44; 32]).public();
        let sequence = [0_u64, 1000, 0, 1500, 500, 2500];
        let mut expected_global = VerifiedTokenBucketState::full(limiter.global.config(), 0);
        let mut expected_peer: Option<VerifiedPeerRateState> = None;

        for now_ms in sequence {
            let transition = evaluate_gossip_admission(
                expected_global,
                limiter.global.config(),
                expected_peer,
                limiter.per_peer_config,
                now_ms,
            );
            apply_expected_transition(&mut expected_global, &mut expected_peer, transition);

            let actual = limiter.check_at(&peer, origin + Duration::from_millis(now_ms));
            assert_eq!(actual.map(|_| ()), convert_outcome(transition.outcome));
            assert_bucket_state_eq(limiter.global.state(), expected_global);
            assert_eq!(limiter.per_peer.get(&peer).map(PeerRateEntry::verified_state), expected_peer);
        }
    }

    #[test]
    fn test_runtime_wrapper_matches_verified_core_lru_monotonicity() {
        let origin = Instant::now();
        let mut limiter = GossipRateLimiter::new_with_limits_at(origin, 0, 16, 0, 1, 2);
        let peer_a = SecretKey::from([0x51; 32]).public();
        let peer_b = SecretKey::from([0x52; 32]).public();
        let peer_c = SecretKey::from([0x53; 32]).public();
        let mut expected_global = VerifiedTokenBucketState::full(limiter.global.config(), 0);
        let mut expected_peers = HashMap::<PublicKey, VerifiedPeerRateState>::new();

        let events = [
            (peer_a, 0_u64),
            (peer_b, 1000),
            (peer_a, 2000),
            (peer_a, 0),
            (peer_c, 2500),
        ];

        for (peer, now_ms) in events {
            let transition = evaluate_gossip_admission(
                expected_global,
                limiter.global.config(),
                expected_peers.get(&peer).copied(),
                limiter.per_peer_config,
                now_ms,
            );
            apply_expected_map_transition(
                &mut expected_global,
                &mut expected_peers,
                peer,
                transition,
                limiter.max_tracked_peers,
            );

            let actual = limiter.check_at(&peer, origin + Duration::from_millis(now_ms));
            assert_eq!(actual.map(|_| ()), convert_outcome(transition.outcome));
            assert_bucket_state_eq(limiter.global.state(), expected_global);
            assert_eq!(collect_peer_states(&limiter), expected_peers);
        }

        assert!(limiter.per_peer.contains_key(&peer_a));
        assert!(limiter.per_peer.contains_key(&peer_c));
        assert!(!limiter.per_peer.contains_key(&peer_b));
    }

    fn convert_outcome(outcome: Result<(), TwoTierRateLimitReason>) -> Result<(), RateLimitReason> {
        outcome.map_err(RateLimitReason::from)
    }

    fn apply_expected_transition(
        expected_global: &mut VerifiedTokenBucketState,
        expected_peer: &mut Option<VerifiedPeerRateState>,
        transition: GossipAdmissionTransition,
    ) {
        *expected_global = transition.next_global;
        if let Some(next_peer) = transition.next_peer {
            *expected_peer = Some(next_peer);
        }
    }

    fn apply_expected_map_transition(
        expected_global: &mut VerifiedTokenBucketState,
        expected_peers: &mut HashMap<PublicKey, VerifiedPeerRateState>,
        peer: PublicKey,
        transition: GossipAdmissionTransition,
        max_tracked_peers: u32,
    ) {
        *expected_global = transition.next_global;
        if let Some(next_peer) = transition.next_peer {
            if !expected_peers.contains_key(&peer)
                && should_evict_oldest(expected_peers.len() as u32, max_tracked_peers)
            {
                evict_oldest_peer(expected_peers);
            }
            expected_peers.insert(peer, next_peer);
        }
    }

    fn evict_oldest_peer(expected_peers: &mut HashMap<PublicKey, VerifiedPeerRateState>) {
        if let Some(oldest_key) =
            expected_peers.iter().min_by_key(|(_, state)| state.last_access_ms).map(|(peer, _)| *peer)
        {
            expected_peers.remove(&oldest_key);
        }
    }

    fn collect_peer_states(limiter: &GossipRateLimiter) -> HashMap<PublicKey, VerifiedPeerRateState> {
        limiter.per_peer.iter().map(|(peer, entry)| (*peer, entry.verified_state())).collect()
    }

    fn assert_bucket_state_eq(actual: VerifiedTokenBucketState, expected: VerifiedTokenBucketState) {
        assert_close(actual.tokens, expected.tokens);
        assert_eq!(actual.last_update_ms, expected.last_update_ms);
    }
}
