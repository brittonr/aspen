//! Fuzz target for clock drift detection.
//!
//! This target fuzzes NTP-style offset calculations used for detecting
//! clock drift between cluster nodes. Tests ensure no panics or integer
//! overflows occur with arbitrary timestamp inputs.
//!
//! Attack vectors tested:
//! - Integer overflow in offset calculations
//! - Timestamps at u64 boundaries
//! - Inverted timestamps (recv < send)
//! - Very large time differences
//! - Zero timestamps
//! - EWMA smoothing edge cases

use bolero::check;
use bolero_generator::{Driver, TypeGenerator};

/// Maximum expected RTT before we consider timestamps suspicious
const MAX_RTT_MS: i64 = 60_000; // 1 minute

/// EWMA smoothing factor
const EWMA_ALPHA: f64 = 0.1;

#[derive(Debug, Clone)]
struct FuzzTimestamps {
    /// Client send timestamp (local time when request sent)
    client_send_ms: u64,
    /// Server receive timestamp (server time when request received)
    server_recv_ms: u64,
    /// Server send timestamp (server time when response sent)
    server_send_ms: u64,
    /// Client receive timestamp (local time when response received)
    client_recv_ms: u64,
    /// Previous smoothed offset for EWMA
    prev_smoothed_offset_ms: i64,
}

impl TypeGenerator for FuzzTimestamps {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        Some(FuzzTimestamps {
            client_send_ms: driver.produce::<u64>()?,
            server_recv_ms: driver.produce::<u64>()?,
            server_send_ms: driver.produce::<u64>()?,
            client_recv_ms: driver.produce::<u64>()?,
            prev_smoothed_offset_ms: driver.produce::<i64>()?,
        })
    }
}

/// NTP-style offset calculation
/// offset = ((t2 - t1) + (t3 - t4)) / 2
/// where:
///   t1 = client_send_ms (local)
///   t2 = server_recv_ms (remote)
///   t3 = server_send_ms (remote)
///   t4 = client_recv_ms (local)
fn compute_offset_ms(timestamps: &FuzzTimestamps) -> Option<i64> {
    // Use checked arithmetic to detect overflow
    let t2_minus_t1 = (timestamps.server_recv_ms as i128) - (timestamps.client_send_ms as i128);
    let t3_minus_t4 = (timestamps.server_send_ms as i128) - (timestamps.client_recv_ms as i128);

    let sum = t2_minus_t1.checked_add(t3_minus_t4)?;
    let offset = sum / 2;

    // Check if result fits in i64
    if offset > i64::MAX as i128 || offset < i64::MIN as i128 {
        return None;
    }

    Some(offset as i64)
}

/// RTT calculation
/// rtt = (t4 - t1) - (t3 - t2)
fn compute_rtt_ms(timestamps: &FuzzTimestamps) -> Option<i64> {
    // Use checked arithmetic
    let total_time = (timestamps.client_recv_ms as i128) - (timestamps.client_send_ms as i128);
    let server_time = (timestamps.server_send_ms as i128) - (timestamps.server_recv_ms as i128);

    let rtt = total_time.checked_sub(server_time)?;

    // Check if result fits in i64
    if rtt > i64::MAX as i128 || rtt < i64::MIN as i128 {
        return None;
    }

    Some(rtt as i64)
}

/// EWMA smoothing for offset
fn smooth_offset(current: i64, prev_smoothed: i64) -> i64 {
    let smoothed = EWMA_ALPHA * (current as f64) + (1.0 - EWMA_ALPHA) * (prev_smoothed as f64);
    smoothed as i64
}

#[test]
fn fuzz_clock_drift() {
    check!().with_type::<FuzzTimestamps>().for_each(|input| {
        // Test offset calculation with arbitrary timestamps
        let offset = compute_offset_ms(input);

        // Test RTT calculation
        let rtt = compute_rtt_ms(input);

        // If both succeed, test the relationship
        if let (Some(offset_val), Some(rtt_val)) = (offset, rtt) {
            // RTT should typically be non-negative, but we don't enforce
            // since clock issues could cause negative RTT briefly
            let _ = rtt_val;

            // Test EWMA smoothing
            let smoothed = smooth_offset(offset_val, input.prev_smoothed_offset_ms);

            // Smoothed value should be between current and previous
            // (or equal if alpha is 0 or 1, or if they're equal)
            let _ = smoothed;

            // Verify smoothing is bounded
            if input.prev_smoothed_offset_ms != 0 && offset_val != 0 {
                // Smoothed value should be in a reasonable range
                let min_val = offset_val.min(input.prev_smoothed_offset_ms);
                let max_val = offset_val.max(input.prev_smoothed_offset_ms);
                // Due to EWMA, result should be between min and max
                // (allowing for floating point rounding)
                assert!(
                    smoothed >= min_val - 1 && smoothed <= max_val + 1,
                    "EWMA result {} should be between {} and {}",
                    smoothed,
                    min_val,
                    max_val
                );
            }
        }

        // Test edge cases directly
        let edge_cases = [
            // All zeros
            FuzzTimestamps {
                client_send_ms: 0,
                server_recv_ms: 0,
                server_send_ms: 0,
                client_recv_ms: 0,
                prev_smoothed_offset_ms: 0,
            },
            // All max
            FuzzTimestamps {
                client_send_ms: u64::MAX,
                server_recv_ms: u64::MAX,
                server_send_ms: u64::MAX,
                client_recv_ms: u64::MAX,
                prev_smoothed_offset_ms: i64::MAX,
            },
            // Mixed extremes
            FuzzTimestamps {
                client_send_ms: 0,
                server_recv_ms: u64::MAX,
                server_send_ms: 0,
                client_recv_ms: u64::MAX,
                prev_smoothed_offset_ms: i64::MIN,
            },
        ];

        for edge in &edge_cases {
            // These should not panic
            let _ = compute_offset_ms(edge);
            let _ = compute_rtt_ms(edge);
        }

        // Test timestamp validation logic
        // Negative RTT might indicate clock issues
        if let Some(rtt_val) = rtt {
            let is_suspicious = rtt_val < 0 || rtt_val > MAX_RTT_MS;
            let _ = is_suspicious;
        }
    });
}
