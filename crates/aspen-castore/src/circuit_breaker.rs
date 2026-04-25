//! Local irpc client circuit breaker.
//!
//! The reusable castore crate must not depend on Aspen's core shell just to
//! protect outbound irpc clients. This module keeps the state machine private
//! to castore and takes time as an explicit shell-provided `Instant`.

use std::time::Duration;
use std::time::Instant;

const MIN_FAILURE_THRESHOLD: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub(crate) struct CircuitBreaker {
    consecutive_failures: u32,
    opened_at: Option<Instant>,
    threshold: u32,
    open_duration: Duration,
}

impl CircuitBreaker {
    pub(crate) fn new(threshold: u32, open_duration: Duration) -> Self {
        Self {
            consecutive_failures: 0,
            opened_at: None,
            threshold: threshold.max(MIN_FAILURE_THRESHOLD),
            open_duration,
        }
    }

    pub(crate) fn record_failure(&mut self, now: Instant) -> bool {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= self.threshold {
            self.opened_at = Some(now);
        }
        self.opened_at.is_some()
    }

    pub(crate) fn record_success(&mut self) -> bool {
        let was_open = self.opened_at.is_some();
        self.consecutive_failures = 0;
        self.opened_at = None;
        was_open
    }

    pub(crate) fn should_reject(&self, now: Instant) -> bool {
        self.state(now) == CircuitState::Open
    }

    pub(crate) fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    fn state(&self, now: Instant) -> CircuitState {
        match self.opened_at {
            None => CircuitState::Closed,
            Some(opened_at) if has_open_duration_elapsed(opened_at, now, self.open_duration) => CircuitState::HalfOpen,
            Some(_) => CircuitState::Open,
        }
    }
}

fn has_open_duration_elapsed(opened_at: Instant, now: Instant, open_duration: Duration) -> bool {
    match now.checked_duration_since(opened_at) {
        Some(elapsed) => elapsed >= open_duration,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAILURE_THRESHOLD: u32 = 3;
    const ZERO_THRESHOLD: u32 = 0;
    const OPEN_DURATION_MS: u64 = 100;
    const BEFORE_OPEN_DURATION_MS: u64 = OPEN_DURATION_MS - 1;
    const AFTER_OPEN_DURATION_MS: u64 = OPEN_DURATION_MS;

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new(FAILURE_THRESHOLD, Duration::from_millis(OPEN_DURATION_MS))
    }

    #[test]
    fn opens_after_threshold_failures() {
        let now = Instant::now();
        let mut breaker = breaker();

        assert!(!breaker.record_failure(now));
        assert!(!breaker.should_reject(now));
        assert!(!breaker.record_failure(now));
        assert!(!breaker.should_reject(now));
        assert!(breaker.record_failure(now));
        assert!(breaker.should_reject(now));
        assert_eq!(breaker.consecutive_failures(), FAILURE_THRESHOLD);
    }

    #[test]
    fn half_open_after_duration_elapses() {
        let now = Instant::now();
        let mut breaker = breaker();

        for _ in 0..FAILURE_THRESHOLD {
            breaker.record_failure(now);
        }

        assert!(breaker.should_reject(now + Duration::from_millis(BEFORE_OPEN_DURATION_MS)));
        assert!(!breaker.should_reject(now + Duration::from_millis(AFTER_OPEN_DURATION_MS)));
    }

    #[test]
    fn success_resets_open_breaker() {
        let now = Instant::now();
        let mut breaker = breaker();

        for _ in 0..FAILURE_THRESHOLD {
            breaker.record_failure(now);
        }

        assert!(breaker.record_success());
        assert!(!breaker.should_reject(now));
        assert_eq!(breaker.consecutive_failures(), 0);
    }

    #[test]
    fn zero_threshold_is_normalized() {
        let now = Instant::now();
        let mut breaker = CircuitBreaker::new(ZERO_THRESHOLD, Duration::from_millis(OPEN_DURATION_MS));

        assert!(breaker.record_failure(now));
        assert!(breaker.should_reject(now));
    }

    #[test]
    fn clock_regression_keeps_breaker_open() {
        let now = Instant::now();
        let earlier = now.checked_sub(Duration::from_millis(AFTER_OPEN_DURATION_MS)).unwrap_or(now);
        let mut breaker = breaker();

        for _ in 0..FAILURE_THRESHOLD {
            breaker.record_failure(now);
        }

        assert!(breaker.should_reject(earlier));
    }
}
