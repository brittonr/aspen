//! Pure helper functions for gossip peer discovery.

use std::time::Duration;

/// Pure function for calculating backoff duration.
///
/// Tiger Style: Pure function for testability and predictable behavior.
pub fn calculate_backoff_duration(restart_count: usize, backoff_durations: &[Duration]) -> Duration {
    let idx = restart_count.min(backoff_durations.len().saturating_sub(1));
    backoff_durations[idx]
}
