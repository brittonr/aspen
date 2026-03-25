//! Generic circuit breaker state machine.
//!
//! Three-state breaker: closed (normal) → open (rejecting) → half-open (probing).
//! Pure state machine with no I/O, no async. Time is passed explicitly via `Instant`.
//!
//! Based on the pattern from rio-build's `CacheCheckBreaker`.

use std::time::Duration;
use std::time::Instant;

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
/// use aspen_core::circuit_breaker::CircuitBreaker;
/// use std::time::{Duration, Instant};
///
/// let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
/// let now = Instant::now();
///
/// // Record failures — stays closed until threshold
/// assert!(!cb.should_reject(now));
/// cb.record_failure(now);
/// cb.record_failure(now);
/// assert!(!cb.should_reject(now));
///
/// // Third failure trips the breaker
/// cb.record_failure(now);
/// assert!(cb.should_reject(now));
///
/// // Success resets
/// cb.record_success();
/// assert!(!cb.should_reject(now));
/// ```
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Number of consecutive failures recorded.
    consecutive_failures: u32,
    /// When the breaker was tripped open. `None` when closed.
    opened_at: Option<Instant>,
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
            opened_at: None,
            threshold: threshold.max(1),
            open_duration,
        }
    }

    /// Record a failure. Returns `true` if the breaker is now open (reject).
    pub fn record_failure(&mut self, now: Instant) -> bool {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= self.threshold {
            // Trip open (or re-trip from half-open with fresh timestamp)
            self.opened_at = Some(now);
        }
        self.opened_at.is_some()
    }

    /// Record a success. Resets the failure counter and closes the breaker.
    /// Returns `true` if this transitioned the breaker from open/half-open to closed.
    pub fn record_success(&mut self) -> bool {
        let was_open = self.opened_at.is_some();
        self.consecutive_failures = 0;
        self.opened_at = None;
        was_open
    }

    /// Check whether requests should be rejected.
    ///
    /// Returns `true` when the breaker is open and the timeout has not elapsed.
    /// Returns `false` when closed or when the open_duration has elapsed (half-open).
    pub fn should_reject(&self, now: Instant) -> bool {
        match self.opened_at {
            None => false,
            Some(opened_at) => now.duration_since(opened_at) < self.open_duration,
        }
    }

    /// Get the current state of the breaker.
    pub fn state(&self, now: Instant) -> CircuitState {
        match self.opened_at {
            None => CircuitState::Closed,
            Some(opened_at) => {
                if now.duration_since(opened_at) >= self.open_duration {
                    CircuitState::HalfOpen
                } else {
                    CircuitState::Open
                }
            }
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
        self.record_success();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // r[verify snix.store.circuit-breaker]
    fn stays_closed_under_threshold() {
        let mut cb = CircuitBreaker::new(5, Duration::from_secs(30));
        let now = Instant::now();

        for _ in 0..4 {
            assert!(!cb.record_failure(now));
            assert!(!cb.should_reject(now));
            assert_eq!(cb.state(now), CircuitState::Closed);
        }
    }

    #[test]
    fn trips_open_at_threshold() {
        let mut cb = CircuitBreaker::new(5, Duration::from_secs(30));
        let now = Instant::now();

        for _ in 0..4 {
            cb.record_failure(now);
        }
        // Fifth failure trips it
        assert!(cb.record_failure(now));
        assert!(cb.should_reject(now));
        assert_eq!(cb.state(now), CircuitState::Open);
    }

    #[test]
    fn success_closes_breaker() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let now = Instant::now();

        for _ in 0..3 {
            cb.record_failure(now);
        }
        assert!(cb.should_reject(now));

        cb.record_success();
        assert!(!cb.should_reject(now));
        assert_eq!(cb.state(now), CircuitState::Closed);
        assert_eq!(cb.consecutive_failures(), 0);
    }

    #[test]
    fn auto_close_after_timeout() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let now = Instant::now();

        for _ in 0..3 {
            cb.record_failure(now);
        }
        assert!(cb.should_reject(now));

        // After open_duration elapses, transitions to half-open
        let later = now + Duration::from_secs(31);
        assert!(!cb.should_reject(later));
        assert_eq!(cb.state(later), CircuitState::HalfOpen);
    }

    #[test]
    fn half_open_failure_reopens() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let now = Instant::now();

        for _ in 0..3 {
            cb.record_failure(now);
        }

        // Wait for half-open
        let later = now + Duration::from_secs(31);
        assert_eq!(cb.state(later), CircuitState::HalfOpen);

        // Probe failure keeps it open (failure count still above threshold)
        cb.record_failure(later);
        assert!(cb.should_reject(later));
    }

    #[test]
    fn saturating_failure_counter() {
        let mut cb = CircuitBreaker::new(u32::MAX, Duration::from_secs(30));
        let now = Instant::now();

        // Manually set near overflow
        cb.consecutive_failures = u32::MAX - 1;
        cb.record_failure(now);
        assert_eq!(cb.consecutive_failures(), u32::MAX);

        // One more doesn't overflow
        cb.record_failure(now);
        assert_eq!(cb.consecutive_failures(), u32::MAX);
    }

    #[test]
    fn threshold_minimum_is_one() {
        let cb = CircuitBreaker::new(0, Duration::from_secs(30));
        assert_eq!(cb.threshold(), 1);
    }

    #[test]
    fn reset_clears_state() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let now = Instant::now();

        for _ in 0..3 {
            cb.record_failure(now);
        }
        assert!(cb.should_reject(now));

        cb.reset();
        assert!(!cb.should_reject(now));
        assert_eq!(cb.consecutive_failures(), 0);
        assert_eq!(cb.state(now), CircuitState::Closed);
    }

    #[test]
    fn interleaved_success_resets_counter() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let now = Instant::now();

        cb.record_failure(now);
        cb.record_failure(now);
        cb.record_success(); // resets counter
        cb.record_failure(now);
        cb.record_failure(now);
        // Only 2 consecutive failures, not 3
        assert!(!cb.should_reject(now));
        assert_eq!(cb.state(now), CircuitState::Closed);
    }

    #[test]
    fn still_open_before_timeout() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let now = Instant::now();

        for _ in 0..3 {
            cb.record_failure(now);
        }

        // 29 seconds later — still open
        let almost = now + Duration::from_secs(29);
        assert!(cb.should_reject(almost));
        assert_eq!(cb.state(almost), CircuitState::Open);
    }
}
