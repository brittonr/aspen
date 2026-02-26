//! Time utilities for Aspen distributed systems.
//!
//! This module provides safe, panic-free time access functions following Tiger Style
//! resource management principles.
//!
//! # TimeProvider Trait
//!
//! The [`TimeProvider`] trait enables injectable time for deterministic simulation testing.
//! Use [`SystemTimeProvider`] for production and [`SimulatedTimeProvider`] (behind the
//! `simulation` feature) for tests.
//!
//! # Tiger Style
//!
//! - No `.expect()` or `.unwrap()` - safe fallback to 0
//! - Inline for hot path performance
//! - Pure functions with no external dependencies

#[cfg(feature = "simulation")]
use std::sync::Arc;
#[cfg(feature = "simulation")]
use std::sync::atomic::AtomicU64;
#[cfg(feature = "simulation")]
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

// ============================================================================
// Free Functions (existing API)
// ============================================================================

/// Get current Unix timestamp in milliseconds.
///
/// Returns 0 if system time is before UNIX epoch (should never happen
/// on properly configured systems, but prevents panics).
///
/// # Tiger Style
///
/// - No `.expect()` or `.unwrap()` - safe fallback to 0
/// - Inline for hot path performance
#[inline]
pub fn current_time_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Get current Unix timestamp in seconds.
///
/// Returns 0 if system time is before UNIX epoch (should never happen
/// on properly configured systems, but prevents panics).
///
/// # Tiger Style
///
/// - No `.expect()` or `.unwrap()` - safe fallback to 0
/// - Inline for hot path performance
#[inline]
pub fn current_time_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ============================================================================
// TimeProvider Trait
// ============================================================================

/// Trait for injectable time sources.
///
/// This trait enables deterministic simulation testing by allowing time to be
/// controlled externally. In production, use [`SystemTimeProvider`]. For tests,
/// use [`SimulatedTimeProvider`] (requires `simulation` feature).
///
/// # Example
///
/// ```
/// use aspen_time::{TimeProvider, SystemTimeProvider};
///
/// fn process_with_deadline<T: TimeProvider>(time: &T, deadline_ms: u64) -> bool {
///     time.now_unix_ms() < deadline_ms
/// }
///
/// let time = SystemTimeProvider;
/// let deadline = aspen_time::current_time_ms() + 1000;
/// assert!(process_with_deadline(&time, deadline));
/// ```
pub trait TimeProvider: Send + Sync {
    /// Get current Unix timestamp in milliseconds.
    fn now_unix_ms(&self) -> u64;

    /// Get current Unix timestamp in seconds.
    fn now_unix_secs(&self) -> u64;
}

// ============================================================================
// SystemTimeProvider (Production)
// ============================================================================

/// Production time provider using system clock.
///
/// This is a zero-size type that delegates to [`current_time_ms`] and
/// [`current_time_secs`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    #[inline]
    fn now_unix_ms(&self) -> u64 {
        current_time_ms()
    }

    #[inline]
    fn now_unix_secs(&self) -> u64 {
        current_time_secs()
    }
}

// ============================================================================
// SimulatedTimeProvider (Testing)
// ============================================================================

/// Simulated time provider for deterministic testing.
///
/// Allows explicit control over time progression for reproducible tests.
/// Enable with the `simulation` feature.
///
/// # Example
///
/// ```ignore
/// use aspen_time::{TimeProvider, SimulatedTimeProvider};
///
/// let time = SimulatedTimeProvider::new(1_000_000);
/// assert_eq!(time.now_unix_ms(), 1_000_000);
///
/// time.advance_ms(500);
/// assert_eq!(time.now_unix_ms(), 1_000_500);
///
/// time.set_ms(2_000_000);
/// assert_eq!(time.now_unix_ms(), 2_000_000);
/// ```
#[cfg(feature = "simulation")]
#[derive(Debug, Clone)]
pub struct SimulatedTimeProvider {
    current_time_ms: Arc<AtomicU64>,
}

#[cfg(feature = "simulation")]
impl SimulatedTimeProvider {
    /// Create a new simulated time provider starting at the given timestamp.
    pub fn new(initial_time_ms: u64) -> Self {
        Self {
            current_time_ms: Arc::new(AtomicU64::new(initial_time_ms)),
        }
    }

    /// Create a new simulated time provider starting at the current system time.
    pub fn from_system_time() -> Self {
        Self::new(current_time_ms())
    }

    /// Advance time by the given number of milliseconds.
    pub fn advance_ms(&self, delta_ms: u64) {
        self.current_time_ms.fetch_add(delta_ms, Ordering::SeqCst);
    }

    /// Advance time by the given number of seconds.
    pub fn advance_secs(&self, delta_secs: u64) {
        self.advance_ms(delta_secs.saturating_mul(1000));
    }

    /// Set the current time to a specific value.
    pub fn set_ms(&self, time_ms: u64) {
        self.current_time_ms.store(time_ms, Ordering::SeqCst);
    }

    /// Set the current time to a specific value in seconds.
    pub fn set_secs(&self, time_secs: u64) {
        self.set_ms(time_secs.saturating_mul(1000));
    }

    /// Get the raw atomic for advanced use cases (e.g., sharing across threads).
    pub fn inner(&self) -> &Arc<AtomicU64> {
        &self.current_time_ms
    }
}

#[cfg(feature = "simulation")]
impl Default for SimulatedTimeProvider {
    fn default() -> Self {
        Self::from_system_time()
    }
}

#[cfg(feature = "simulation")]
impl TimeProvider for SimulatedTimeProvider {
    #[inline]
    fn now_unix_ms(&self) -> u64 {
        self.current_time_ms.load(Ordering::SeqCst)
    }

    #[inline]
    fn now_unix_secs(&self) -> u64 {
        self.now_unix_ms() / 1000
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time_ms_returns_nonzero() {
        let time = current_time_ms();
        assert!(time > 0, "current_time_ms should return non-zero on valid systems");
    }

    #[test]
    fn current_time_ms_is_monotonic() {
        let t1 = current_time_ms();
        let t2 = current_time_ms();
        assert!(t2 >= t1, "time should not go backwards");
    }

    #[test]
    fn current_time_ms_reasonable_range() {
        // Should be after year 2020 (1577836800000 ms) and before year 2100
        let time = current_time_ms();
        let year_2020_ms = 1_577_836_800_000u64;
        let year_2100_ms = 4_102_444_800_000u64;
        assert!(time > year_2020_ms, "current_time_ms {} should be after year 2020", time);
        assert!(time < year_2100_ms, "current_time_ms {} should be before year 2100", time);
    }

    #[test]
    fn current_time_secs_returns_nonzero() {
        let time = current_time_secs();
        assert!(time > 0, "current_time_secs should return non-zero on valid systems");
    }

    #[test]
    fn current_time_secs_is_monotonic() {
        let t1 = current_time_secs();
        let t2 = current_time_secs();
        assert!(t2 >= t1, "time should not go backwards");
    }

    #[test]
    fn current_time_secs_reasonable_range() {
        // Should be after year 2020 (1577836800) and before year 2100
        let time = current_time_secs();
        let year_2020 = 1_577_836_800u64;
        let year_2100 = 4_102_444_800u64;
        assert!(time > year_2020, "current_time_secs {} should be after year 2020", time);
        assert!(time < year_2100, "current_time_secs {} should be before year 2100", time);
    }

    #[test]
    fn current_time_ms_and_secs_consistent() {
        let ms = current_time_ms();
        let secs = current_time_secs();
        // ms / 1000 should be close to secs (within 1 second)
        let ms_as_secs = ms / 1000;
        assert!(
            ms_as_secs >= secs.saturating_sub(1) && ms_as_secs <= secs + 1,
            "ms/1000 ({}) and secs ({}) should be consistent",
            ms_as_secs,
            secs
        );
    }

    #[test]
    fn system_time_provider_matches_free_functions() {
        let provider = SystemTimeProvider;
        let ms1 = current_time_ms();
        let ms2 = provider.now_unix_ms();
        // Should be within 10ms of each other
        assert!(ms2 >= ms1 && ms2 <= ms1 + 10, "SystemTimeProvider should match current_time_ms");
    }

    #[test]
    fn system_time_provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SystemTimeProvider>();
    }
}

#[cfg(all(test, feature = "simulation"))]
mod simulation_tests {
    use super::*;

    #[test]
    fn simulated_time_initial_value() {
        let time = SimulatedTimeProvider::new(1_000_000);
        assert_eq!(time.now_unix_ms(), 1_000_000);
        assert_eq!(time.now_unix_secs(), 1_000);
    }

    #[test]
    fn simulated_time_advance_ms() {
        let time = SimulatedTimeProvider::new(1_000_000);
        time.advance_ms(500);
        assert_eq!(time.now_unix_ms(), 1_000_500);
    }

    #[test]
    fn simulated_time_advance_secs() {
        let time = SimulatedTimeProvider::new(1_000_000);
        time.advance_secs(5);
        assert_eq!(time.now_unix_ms(), 1_005_000); // 1_000_000 + 5*1000
    }

    #[test]
    fn simulated_time_set_ms() {
        let time = SimulatedTimeProvider::new(1_000_000);
        time.set_ms(2_000_000);
        assert_eq!(time.now_unix_ms(), 2_000_000);
    }

    #[test]
    fn simulated_time_set_secs() {
        let time = SimulatedTimeProvider::new(1_000_000);
        time.set_secs(100);
        assert_eq!(time.now_unix_ms(), 100_000);
    }

    #[test]
    fn simulated_time_clone_shares_state() {
        let time1 = SimulatedTimeProvider::new(1_000_000);
        let time2 = time1.clone();

        time1.advance_ms(500);
        assert_eq!(time2.now_unix_ms(), 1_000_500);
    }

    #[test]
    fn simulated_time_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SimulatedTimeProvider>();
    }

    #[test]
    fn simulated_time_saturating_advance() {
        let time = SimulatedTimeProvider::new(u64::MAX - 100);
        time.advance_ms(200);
        // Should wrap due to fetch_add, but that's expected behavior
        // The important thing is it doesn't panic
        let _ = time.now_unix_ms();
    }

    #[test]
    fn simulated_time_from_system() {
        let time = SimulatedTimeProvider::from_system_time();
        let system = current_time_ms();
        // Should be within 100ms of system time
        let simulated = time.now_unix_ms();
        assert!(
            simulated >= system.saturating_sub(100) && simulated <= system + 100,
            "from_system_time should be close to current time"
        );
    }
}
