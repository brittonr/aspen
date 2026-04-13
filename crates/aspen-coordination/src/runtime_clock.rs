//! Runtime clock boundary helpers for coordination shell code.
//!
//! Coordination primitives own timeout and measurement boundaries here so the
//! state-transition code can stay deterministic everywhere else.

#![allow(
    ambient_clock,
    reason = "coordination shell code owns runtime timeout and measurement boundaries"
)]

use std::time::Duration;
use std::time::Instant;

#[inline]
pub(crate) fn optional_deadline(timeout: Option<Duration>) -> Option<Instant> {
    timeout.map(|duration| Instant::now() + duration)
}

#[inline]
pub(crate) fn deadline_after(timeout: Duration) -> Instant {
    Instant::now() + timeout
}

#[inline]
pub(crate) fn deadline_reached(deadline: Instant) -> bool {
    Instant::now() >= deadline
}

#[inline]
pub(crate) fn timeout_elapsed(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(deadline_reached)
}

#[inline]
pub(crate) fn wait_would_exceed_deadline(deadline: Instant, wait: Duration) -> bool {
    Instant::now() + wait > deadline
}

#[inline]
pub(crate) fn measurement_start() -> Instant {
    Instant::now()
}
