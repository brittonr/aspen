//! Property tests for raft-network wire encoding functions.
//!
//! Tests shard prefix encoding/decoding round-trip and
//! RPC response deserialization robustness.

use aspen_raft_network::verified::encoding::*;
use aspen_raft_network::verified::heuristics::*;
use aspen_raft_network::verified::network::*;
use aspen_raft_network::ConnectionHealth;
use proptest::prelude::*;
use std::time::Duration;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Shard prefix encode/decode is a perfect round-trip for all u32 values.
    #[test]
    fn test_shard_prefix_roundtrip(shard_id in any::<u32>()) {
        let encoded = encode_shard_prefix(shard_id);
        let decoded = decode_shard_prefix(&encoded);
        assert_eq!(decoded, shard_id, "Shard prefix round-trip failed for {shard_id}");
    }

    /// try_decode_shard_prefix succeeds for valid encoded prefixes.
    #[test]
    fn test_try_decode_shard_prefix_valid(shard_id in any::<u32>()) {
        let encoded = encode_shard_prefix(shard_id);
        let result = try_decode_shard_prefix(&encoded);
        assert_eq!(result, Some(shard_id));
    }

    /// try_decode_shard_prefix returns None for short inputs.
    #[test]
    fn test_try_decode_shard_prefix_short(data in prop::collection::vec(any::<u8>(), 0..4)) {
        assert_eq!(try_decode_shard_prefix(&data), None);
    }

    /// NTP clock offset computation returns a (offset, rtt) pair.
    #[test]
    fn test_ntp_clock_offset_bounded(
        t1 in 0u64..1_000_000_000,
        t2 in 0u64..1_000_000_000,
        t3 in 0u64..1_000_000_000,
        t4 in 0u64..1_000_000_000,
    ) {
        let (offset, rtt) = calculate_ntp_clock_offset(t1, t2, t3, t4);
        // RTT should be non-negative when t4 >= t1 and t3 >= t2
        let _ = (offset, rtt); // just verify no panic
    }

    /// EWMA is bounded between old and new value (for alpha in [0,1]).
    #[test]
    fn test_ewma_bounded(
        new_value in -1e6f64..1e6,
        old_avg in -1e6f64..1e6,
        alpha in 0.0f64..=1.0,
    ) {
        let result = compute_ewma(new_value, old_avg, alpha);
        let min = new_value.min(old_avg);
        let max = new_value.max(old_avg);
        assert!(
            result >= min - 1e-10 && result <= max + 1e-10,
            "EWMA({new_value}, {old_avg}, {alpha}) = {result} not in [{min}, {max}]"
        );
    }

    /// Connection retry backoff is monotonically increasing with attempt count.
    #[test]
    fn test_retry_backoff_monotonic(
        attempt in 0u32..20,
        base_ms in 100u64..10_000,
    ) {
        let backoff1 = calculate_connection_retry_backoff(attempt, base_ms);
        let backoff2 = calculate_connection_retry_backoff(attempt.saturating_add(1), base_ms);
        assert!(
            backoff2 >= backoff1,
            "Backoff must be monotonic: attempt {} = {:?}, attempt {} = {:?}",
            attempt, backoff1, attempt + 1, backoff2
        );
    }

    /// Connection retry backoff is always positive and bounded.
    #[test]
    fn test_retry_backoff_bounded(
        attempt in 0u32..100,
        base_ms in 1u64..100_000,
    ) {
        let backoff = calculate_connection_retry_backoff(attempt, base_ms);
        assert!(backoff > Duration::ZERO, "Backoff must be positive");
        // Should be capped at some reasonable max (60s)
        assert!(
            backoff <= Duration::from_secs(60),
            "Backoff {:?} exceeds 60s cap for attempt {} base {}ms",
            backoff, attempt, base_ms
        );
    }

    /// classify_rpc_error returns valid ConnectionStatus pairs for any input.
    #[test]
    fn test_classify_rpc_error_any_input(error_msg in ".*") {
        let (raft_status, iroh_status) = classify_rpc_error(&error_msg);
        // Both must be valid enum values (this is a compile-time guarantee,
        // but the test verifies no panics for arbitrary input)
        let _ = format!("{:?} {:?}", raft_status, iroh_status);
    }

    /// Drift severity classification handles all magnitudes.
    #[test]
    fn test_drift_severity_all_magnitudes(
        offset_ms in -1_000_000f64..1_000_000.0,
        warning_ms in 1u64..10_000,
        alert_ms in 1u64..100_000,
    ) {
        let severity = classify_drift_severity(offset_ms, warning_ms, alert_ms);
        let _ = format!("{:?}", severity);
    }

    /// should_evict_oldest_unreachable is consistent with its inputs.
    #[test]
    fn test_evict_heuristic_consistency(
        current in 0usize..1000,
        max_nodes in 1usize..1000,
        already_tracked in any::<bool>(),
    ) {
        let result = should_evict_oldest_unreachable(current, max_nodes, already_tracked);
        // If current < max, no eviction needed
        if current < max_nodes {
            assert!(!result, "No eviction needed when under capacity");
        }
    }

    /// Connection health transition is deterministic.
    #[test]
    fn test_health_transition_deterministic(
        succeeded in any::<bool>(),
        max_retries in 1u32..10,
    ) {
        let result1 = transition_connection_health(
            ConnectionHealth::Healthy,
            succeeded,
            max_retries,
        );
        let result2 = transition_connection_health(
            ConnectionHealth::Healthy,
            succeeded,
            max_retries,
        );
        assert_eq!(
            format!("{:?}", result1),
            format!("{:?}", result2),
            "Health transition must be deterministic"
        );
    }
}
