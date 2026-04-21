//! Generic circuit breaker state machine.
//!
//! Three-state breaker: closed (normal) → open (rejecting) → half-open (probing).
//! Pure state machine with no I/O and no async. Time is passed explicitly as
//! milliseconds from the shell layer.
//!
//! Based on the pattern from rio-build's `CacheCheckBreaker`.

use core::time::Duration;

const MIN_FAILURE_THRESHOLD: u32 = 1;
const DURATION_MILLIS_OVERFLOW_SENTINEL: u64 = u64::MAX;

/// Circuit breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — requests flow through.
    Closed,
    /// Rejecting requests after consecutive failures exceeded threshold.
    Open,
    /// Timeout elapsed — allowing one probe request through.
    HalfOpen,
}

/// A generic circuit breaker that tracks consecutive failures and transitions
/// between closed, open, and half-open states.
///
/// # Usage
///
/// ```rust
/// use core::time::Duration;
/// use aspen_core::circuit_breaker::CircuitBreaker;
///
/// const BREAKER_THRESHOLD: u32 = 3;
/// const BREAKER_OPEN_DURATION: Duration = Duration::from_secs(30);
/// const INITIAL_TIME_MS: u64 = 1_000;
/// const HALF_OPEN_TIME_MS: u64 = 31_000;
///
/// let mut cb = CircuitBreaker::new(BREAKER_THRESHOLD, BREAKER_OPEN_DURATION);
///
/// // Record failures — stays closed until threshold
/// assert!(!cb.should_reject(INITIAL_TIME_MS));
/// cb.record_failure(INITIAL_TIME_MS);
/// cb.record_failure(INITIAL_TIME_MS);
/// assert!(!cb.should_reject(INITIAL_TIME_MS));
///
/// // Third failure trips the breaker
/// cb.record_failure(INITIAL_TIME_MS);
/// assert!(cb.should_reject(INITIAL_TIME_MS));
///
/// // After the timeout window, the breaker moves to half-open
/// assert!(!cb.should_reject(HALF_OPEN_TIME_MS));
/// ```
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Number of consecutive failures recorded.
    consecutive_failures: u32,
    /// When the breaker was tripped open, in shell-provided milliseconds.
    opened_at_ms: Option<u64>,
    /// Number of consecutive failures required to trip the breaker.
    threshold: u32,
    /// How long the breaker stays open before transitioning to half-open.
    open_duration: Duration,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    ///
    /// - `threshold`: consecutive failures before tripping (must be ≥ 1)
    /// - `open_duration`: time the breaker stays open before allowing a probe
    pub fn new(threshold: u32, open_duration: Duration) -> Self {
        Self {
            consecutive_failures: 0,
            opened_at_ms: None,
            threshold: normalized_threshold(threshold),
            open_duration,
        }
    }

    /// Record a failure. Returns `true` if the breaker is now open (reject).
    pub fn record_failure(&mut self, now_ms: u64) -> bool {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= self.threshold {
            self.opened_at_ms = Some(now_ms);
        }
        self.opened_at_ms.is_some()
    }

    /// Record a success. Resets the failure counter and closes the breaker.
    /// Returns `true` if this transitioned the breaker from open/half-open to closed.
    pub fn record_success(&mut self) -> bool {
        let was_open = self.opened_at_ms.is_some();
        self.consecutive_failures = 0;
        self.opened_at_ms = None;
        was_open
    }

    /// Check whether requests should be rejected.
    ///
    /// Returns `true` when the breaker is open and the timeout has not elapsed.
    /// Returns `false` when closed or when the open duration has elapsed (half-open).
    pub fn should_reject(&self, now_ms: u64) -> bool {
        self.state(now_ms) == CircuitState::Open
    }

    /// Get the current state of the breaker.
    pub fn state(&self, now_ms: u64) -> CircuitState {
        match self.opened_at_ms {
            None => CircuitState::Closed,
            Some(opened_at_ms) => state_for_open_breaker(opened_at_ms, now_ms, self.open_duration),
        }
    }

    /// Number of consecutive failures recorded so far.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// The configured threshold.
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// The configured open duration.
    pub fn open_duration(&self) -> Duration {
        self.open_duration
    }

    /// Reset the breaker to closed state. Same as `record_success()`.
    pub fn reset(&mut self) {
        let _ = self.record_success();
    }
}

fn normalized_threshold(threshold: u32) -> u32 {
    threshold.max(MIN_FAILURE_THRESHOLD)
}

fn elapsed_ms(start_ms: u64, now_ms: u64) -> u64 {
    now_ms.saturating_sub(start_ms)
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(DURATION_MILLIS_OVERFLOW_SENTINEL)
}

fn state_for_open_breaker(opened_at_ms: u64, now_ms: u64, open_duration: Duration) -> CircuitState {
    if elapsed_ms(opened_at_ms, now_ms) >= duration_millis(open_duration) {
        CircuitState::HalfOpen
    } else {
        CircuitState::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIVE_FAILURE_THRESHOLD: u32 = 5;
    const THREE_FAILURE_THRESHOLD: u32 = 3;
    const ZERO_THRESHOLD: u32 = 0;
    const THIRTY_SECONDS: Duration = Duration::from_secs(30);
    const INITIAL_TIME_MS: u64 = 1_000;
    const HALF_OPEN_TIME_MS: u64 = 31_000;
    const ALMOST_HALF_OPEN_TIME_MS: u64 = 29_000;
    const OVERFLOW_SENTINEL: u64 = 1_000;

    #[test]
    fn stays_closed_under_threshold() {
        let mut cb = CircuitBreaker::new(FIVE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        for _attempt in 0..4 {
            assert!(!cb.record_failure(INITIAL_TIME_MS));
            assert!(!cb.should_reject(INITIAL_TIME_MS));
            assert_eq!(cb.state(INITIAL_TIME_MS), CircuitState::Closed);
        }
    }

    #[test]
    fn trips_open_at_threshold() {
        let mut cb = CircuitBreaker::new(FIVE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        for _attempt in 0..4 {
            cb.record_failure(INITIAL_TIME_MS);
        }
        assert!(cb.record_failure(INITIAL_TIME_MS));
        assert!(cb.should_reject(INITIAL_TIME_MS));
        assert_eq!(cb.state(INITIAL_TIME_MS), CircuitState::Open);
    }

    #[test]
    fn success_closes_breaker() {
        let mut cb = CircuitBreaker::new(THREE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        for _attempt in 0..THREE_FAILURE_THRESHOLD {
            cb.record_failure(INITIAL_TIME_MS);
        }
        assert!(cb.should_reject(INITIAL_TIME_MS));

        cb.record_success();
        assert!(!cb.should_reject(INITIAL_TIME_MS));
        assert_eq!(cb.state(INITIAL_TIME_MS), CircuitState::Closed);
        assert_eq!(cb.consecutive_failures(), 0);
    }

    #[test]
    fn auto_close_after_timeout() {
        let mut cb = CircuitBreaker::new(THREE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        for _attempt in 0..THREE_FAILURE_THRESHOLD {
            cb.record_failure(INITIAL_TIME_MS);
        }
        assert!(cb.should_reject(INITIAL_TIME_MS));

        assert!(!cb.should_reject(HALF_OPEN_TIME_MS));
        assert_eq!(cb.state(HALF_OPEN_TIME_MS), CircuitState::HalfOpen);
    }

    #[test]
    fn half_open_failure_reopens() {
        let mut cb = CircuitBreaker::new(THREE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        for _attempt in 0..THREE_FAILURE_THRESHOLD {
            cb.record_failure(INITIAL_TIME_MS);
        }

        assert_eq!(cb.state(HALF_OPEN_TIME_MS), CircuitState::HalfOpen);
        cb.record_failure(HALF_OPEN_TIME_MS);
        assert!(cb.should_reject(HALF_OPEN_TIME_MS));
    }

    #[test]
    fn saturating_failure_counter() {
        let mut cb = CircuitBreaker::new(u32::MAX, THIRTY_SECONDS);

        cb.consecutive_failures = u32::MAX - 1;
        cb.record_failure(OVERFLOW_SENTINEL);
        assert_eq!(cb.consecutive_failures(), u32::MAX);

        cb.record_failure(OVERFLOW_SENTINEL);
        assert_eq!(cb.consecutive_failures(), u32::MAX);
    }

    #[test]
    fn threshold_minimum_is_one() {
        let cb = CircuitBreaker::new(ZERO_THRESHOLD, THIRTY_SECONDS);
        assert_eq!(cb.threshold(), MIN_FAILURE_THRESHOLD);
    }

    #[test]
    fn reset_clears_state() {
        let mut cb = CircuitBreaker::new(THREE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        for _attempt in 0..THREE_FAILURE_THRESHOLD {
            cb.record_failure(INITIAL_TIME_MS);
        }
        assert!(cb.should_reject(INITIAL_TIME_MS));

        cb.reset();
        assert!(!cb.should_reject(INITIAL_TIME_MS));
        assert_eq!(cb.consecutive_failures(), 0);
        assert_eq!(cb.state(INITIAL_TIME_MS), CircuitState::Closed);
    }

    #[test]
    fn interleaved_success_resets_counter() {
        let mut cb = CircuitBreaker::new(THREE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        cb.record_failure(INITIAL_TIME_MS);
        cb.record_failure(INITIAL_TIME_MS);
        cb.record_success();
        cb.record_failure(INITIAL_TIME_MS);
        cb.record_failure(INITIAL_TIME_MS);
        assert!(!cb.should_reject(INITIAL_TIME_MS));
        assert_eq!(cb.state(INITIAL_TIME_MS), CircuitState::Closed);
    }

    #[test]
    fn still_open_before_timeout() {
        let mut cb = CircuitBreaker::new(THREE_FAILURE_THRESHOLD, THIRTY_SECONDS);

        for _attempt in 0..THREE_FAILURE_THRESHOLD {
            cb.record_failure(INITIAL_TIME_MS);
        }

        assert!(cb.should_reject(ALMOST_HALF_OPEN_TIME_MS));
        assert_eq!(cb.state(ALMOST_HALF_OPEN_TIME_MS), CircuitState::Open);
    }

    #[test]
    fn state_uses_saturating_elapsed_when_time_moves_backward() {
        let mut cb = CircuitBreaker::new(THREE_FAILURE_THRESHOLD, THIRTY_SECONDS);
        let earlier_time_ms = 0;

        for _attempt in 0..THREE_FAILURE_THRESHOLD {
            cb.record_failure(INITIAL_TIME_MS);
        }

        assert!(cb.should_reject(earlier_time_ms));
        assert_eq!(cb.state(earlier_time_ms), CircuitState::Open);
    }
}
