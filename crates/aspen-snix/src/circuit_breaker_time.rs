//! Shell-time adapter for circuit breaker calls.
//!
//! The reusable circuit breaker core accepts time as explicit milliseconds, so
//! this module keeps wall-clock access at the SNIX service shell boundary.

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const MILLIS_OVERFLOW_SENTINEL: u64 = u64::MAX;

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "SNIX service shells pass wall-clock milliseconds into the pure circuit-breaker core"
)]
pub(crate) fn now_ms() -> u64 {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    millis_u128_to_u64(elapsed.as_millis())
}

fn millis_u128_to_u64(millis: u128) -> u64 {
    u64::try_from(millis).unwrap_or(MILLIS_OVERFLOW_SENTINEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL_MILLIS_U64: u64 = 123;
    const SMALL_MILLIS: u128 = SMALL_MILLIS_U64 as u128;
    const ONE_PAST_U64_MAX: u128 = (u64::MAX as u128) + 1;

    #[test]
    fn millis_conversion_preserves_in_range_values() {
        assert_eq!(millis_u128_to_u64(SMALL_MILLIS), SMALL_MILLIS_U64);
    }

    #[test]
    fn millis_conversion_saturates_overflow() {
        assert_eq!(millis_u128_to_u64(ONE_PAST_U64_MAX), MILLIS_OVERFLOW_SENTINEL);
    }
}
