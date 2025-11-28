//! Time anchor for creating monotonic SystemTime values.
//!
//! The anchor pattern solves the clock skew problem by:
//! 1. Recording a SystemTime and Instant pair at startup
//! 2. Recording all events using monotonic Instant
//! 3. Converting Instant to SystemTime via the anchor for serialization
//!
//! This ensures timestamps are monotonic even when the system clock adjusts.
//!
//! Based on the pattern used by tokio-console to handle clock skew.

use std::time::{SystemTime, Instant, UNIX_EPOCH};

/// A time anchor for creating monotonic, clock-skew-resistant SystemTime values.
///
/// # Why This Matters
///
/// `SystemTime` is not monotonic and can go backwards due to NTP adjustments,
/// causing duration calculations to fail. The `TimeAnchor` pattern maintains
/// monotonicity by using `Instant` (which is monotonic) as the basis for time
/// measurements, while anchoring to `SystemTime` for serialization.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use std::thread;
/// # fn example() {
/// let anchor = TimeAnchor::new();
///
/// let t1 = anchor.now();
/// thread::sleep(Duration::from_millis(10));
/// let t2 = anchor.now();
///
/// // Guaranteed: t2 >= t1, even if system clock adjusts
/// assert!(t2 >= t1);
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TimeAnchor {
    system_time: SystemTime,
    instant: Instant,
}

impl TimeAnchor {
    /// Create a new anchor at the current time.
    ///
    /// This captures both `SystemTime::now()` and `Instant::now()`,
    /// establishing the reference point for monotonic timestamps.
    pub fn new() -> Self {
        Self {
            system_time: SystemTime::now(),
            instant: Instant::now(),
        }
    }

    /// Convert an `Instant` to a monotonic `SystemTime`.
    ///
    /// This is the core of the anchor pattern. It calculates the duration
    /// elapsed since the anchor instant and adds it to the anchor's system time.
    ///
    /// # Guarantees
    ///
    /// - Results are monotonically increasing
    /// - Never affected by system clock adjustments
    /// - Serializable as standard `SystemTime`
    /// - Safe to use for distributed timestamps
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::Instant;
    /// # fn example() {
    /// let anchor = TimeAnchor::new();
    /// let later = Instant::now();
    /// let system_time = anchor.to_system_time(later);
    /// // system_time is monotonic and serializable
    /// # }
    /// ```
    pub fn to_system_time(&self, instant: Instant) -> SystemTime {
        let elapsed = instant.duration_since(self.instant);
        self.system_time + elapsed
    }

    /// Get the current time as a monotonic SystemTime.
    ///
    /// This is equivalent to `anchor.to_system_time(Instant::now())`.
    ///
    /// # Guarantees
    ///
    /// - Always returns a valid `SystemTime` (never panics)
    /// - Results are monotonically increasing
    /// - Safe to call repeatedly
    ///
    /// # Example
    ///
    /// ```
    /// # fn example() {
    /// let anchor = TimeAnchor::new();
    /// let timestamp = anchor.now();
    /// println!("Current time: {:?}", timestamp);
    /// # }
    /// ```
    pub fn now(&self) -> SystemTime {
        self.to_system_time(Instant::now())
    }

    /// Get current time as Unix timestamp (seconds since epoch).
    ///
    /// # Guarantees
    ///
    /// - Never fails (unlike `duration_since` on SystemTime)
    /// - Results are monotonically increasing
    /// - Safe to use in production without error handling
    ///
    /// # Example
    ///
    /// ```
    /// # fn example() {
    /// let anchor = TimeAnchor::new();
    /// let secs = anchor.unix_timestamp_secs();
    /// println!("Seconds since epoch: {}", secs);
    /// # }
    /// ```
    pub fn unix_timestamp_secs(&self) -> u64 {
        self.now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Get current time as Unix timestamp (milliseconds since epoch).
    pub fn unix_timestamp_millis(&self) -> u64 {
        self.now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Get current time as Unix timestamp (nanoseconds since epoch).
    pub fn unix_timestamp_nanos(&self) -> u64 {
        self.now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    /// Get the duration elapsed since the anchor was created.
    ///
    /// This is always safe and never fails, unlike SystemTime::elapsed().
    pub fn elapsed(&self) -> std::time::Duration {
        Instant::now().duration_since(self.instant)
    }

    /// Get the underlying SystemTime anchor.
    pub fn system_time(&self) -> SystemTime {
        self.system_time
    }

    /// Get the underlying Instant anchor.
    pub fn instant(&self) -> Instant {
        self.instant
    }
}

impl Default for TimeAnchor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn anchor_produces_monotonic_times() {
        let anchor = TimeAnchor::new();

        let t1 = anchor.now();
        thread::sleep(Duration::from_millis(10));
        let t2 = anchor.now();
        thread::sleep(Duration::from_millis(10));
        let t3 = anchor.now();

        assert!(t1 <= t2);
        assert!(t2 <= t3);
        assert!(t1 <= t3);
    }

    #[test]
    fn unix_timestamps_monotonic() {
        let anchor = TimeAnchor::new();

        let ts1 = anchor.unix_timestamp_secs();
        thread::sleep(Duration::from_millis(100));
        let ts2 = anchor.unix_timestamp_secs();

        assert!(ts2 >= ts1);
    }

    #[test]
    fn convert_instant_to_system_time() {
        let anchor = TimeAnchor::new();
        let instant = Instant::now();

        let st = anchor.to_system_time(instant);

        // Should be close to current time
        let now = SystemTime::now();
        let diff = now.duration_since(st).ok()
            .or_else(|_| st.duration_since(now))
            .unwrap();

        // Should be within 100ms (generous tolerance)
        assert!(diff < Duration::from_millis(100));
    }

    #[test]
    fn future_instant_produces_future_system_time() {
        let anchor = TimeAnchor::new();
        let future_instant = Instant::now() + Duration::from_secs(100);

        let future_time = anchor.to_system_time(future_instant);
        let now = anchor.now();

        // The future time should be about 100 seconds later
        match future_time.duration_since(now) {
            Ok(diff) => {
                // Allow 100ms tolerance for execution time
                let expected = Duration::from_secs(100);
                assert!(diff > expected - Duration::from_millis(100));
                assert!(diff < expected + Duration::from_millis(100));
            }
            Err(_) => panic!("Future time should be after now"),
        }
    }

    #[test]
    fn elapsed_increases() {
        let anchor = TimeAnchor::new();

        let e1 = anchor.elapsed();
        thread::sleep(Duration::from_millis(10));
        let e2 = anchor.elapsed();

        assert!(e2 > e1);
    }

    #[test]
    fn unix_timestamp_millis_greater_precision() {
        let anchor = TimeAnchor::new();

        let secs = anchor.unix_timestamp_secs();
        let millis = anchor.unix_timestamp_millis();

        // Millis should be about 1000x larger
        assert!(millis / secs > 500); // Account for rounding
    }

    #[test]
    fn default_creates_valid_anchor() {
        let anchor = TimeAnchor::default();
        let _ts = anchor.now();
        assert!(anchor.elapsed() < Duration::from_secs(10));
    }
}
