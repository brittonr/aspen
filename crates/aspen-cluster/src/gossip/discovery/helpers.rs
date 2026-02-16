//! Pure helper functions for gossip peer discovery.

use std::time::Duration;

/// Pure function for calculating backoff duration.
///
/// Tiger Style: Pure function for testability and predictable behavior.
pub fn calculate_backoff_duration(restart_count: usize, backoff_durations: &[Duration]) -> Duration {
    // Tiger Style: backoff_durations must not be empty
    debug_assert!(!backoff_durations.is_empty(), "backoff_durations must not be empty");

    let idx = restart_count.min(backoff_durations.len().saturating_sub(1));

    // Tiger Style: index must be valid after min operation
    debug_assert!(idx < backoff_durations.len(), "idx {} must be < len {}", idx, backoff_durations.len());

    backoff_durations[idx]
}
