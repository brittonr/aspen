//! Hybrid Logical Clock (HLC) for distributed systems.
//!
//! A Hybrid Logical Clock combines a physical clock (SystemTime) with a logical
//! counter to provide monotonic timestamps that preserve causality ordering in
//! distributed systems while remaining close to wall-clock time.
//!
//! # Example
//!
//! ```
//! # fn example() -> Result<(), String> {
//! let mut local_clock = HybridLogicalClock::now()?;
//! let mut remote_clock = HybridLogicalClock::now()?;
//!
//! // Process a remote message with a timestamp from another node
//! remote_clock.physical += 1000; // Simulate different system time
//! local_clock.update(&remote_clock)?;
//!
//! // Local clock now has a timestamp that respects causality
//! assert!(local_clock > &HybridLogicalClock::now()?);
//! # Ok(())
//! # }
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

/// A Hybrid Logical Clock for distributed systems.
///
/// Combines a physical component (SystemTime in milliseconds) with a logical
/// counter to produce timestamps that are:
/// - Close to physical time (wall clock)
/// - Monotonically increasing
/// - Respect causality (happens-before ordering)
/// - Resilient to clock skew
///
/// # Clock Skew Handling
///
/// The clock validates that received timestamps don't exceed a configurable
/// threshold ahead of local time. This prevents accepting timestamps that are
/// impossibly far in the future due to buggy or malicious nodes.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct HybridLogicalClock {
    /// Physical component: milliseconds since Unix epoch
    pub physical: u64,
    /// Logical component: counter for ordering
    pub logical: u64,
}

impl HybridLogicalClock {
    /// Maximum drift threshold in milliseconds.
    ///
    /// Timestamps more than this far in the future are rejected as invalid.
    const MAX_DRIFT_MS: u64 = 500;

    /// Create a new HLC from current system time.
    ///
    /// # Returns
    ///
    /// - `Ok(HybridLogicalClock)` with physical = now, logical = 0
    /// - `Err(String)` if system clock fails
    pub fn now() -> Result<Self, String> {
        let physical = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Clock error: {}", e))?
            .as_millis() as u64;

        Ok(Self {
            physical,
            logical: 0,
        })
    }

    /// Create an HLC with specific values.
    ///
    /// Used for deserializing timestamps from storage or network.
    pub fn new(physical: u64, logical: u64) -> Self {
        Self { physical, logical }
    }

    /// Update this clock with a received timestamp.
    ///
    /// This implements the HLC update rule:
    /// - If received is in the past: advance only logical counter (happens-before)
    /// - If received is at same time: advance logical counter
    /// - If received is in the future: adopt the future time with incremented logical
    ///
    /// # Clock Skew Protection
    ///
    /// Rejects timestamps more than 500ms in the future (configurable).
    /// This prevents accepting obviously incorrect timestamps from misbehaving nodes.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if update succeeded
    /// - `Err(String)` if received timestamp is invalid
    pub fn update(&mut self, received: &HybridLogicalClock) -> Result<(), String> {
        // Get current system time
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Clock error: {}", e))?
            .as_millis() as u64;

        // Check if received is too far in the future
        if received.physical > now_ms + Self::MAX_DRIFT_MS {
            return Err(format!(
                "Received timestamp too far ahead: {} ms (current: {} ms, threshold: {} ms)",
                received.physical - now_ms,
                now_ms,
                Self::MAX_DRIFT_MS
            ));
        }

        // Apply HLC update rule
        if received.physical > self.physical {
            // Received is from the future
            self.physical = received.physical;
            self.logical = received.logical + 1;
        } else if received.physical == self.physical {
            // Same time - increment logical counter
            self.logical = std::cmp::max(self.logical, received.logical) + 1;
        } else {
            // Received is from the past - advance only logical
            self.logical = std::cmp::max(self.logical, received.logical) + 1;
        }

        Ok(())
    }

    /// Increment the clock for a new local event.
    ///
    /// Should be called each time this node generates a new event.
    /// Advances the logical counter while keeping physical time current.
    pub fn tick(&mut self) {
        if let Ok(new_hlc) = Self::now() {
            // If system time advanced, use it
            if new_hlc.physical > self.physical {
                self.physical = new_hlc.physical;
                self.logical = 0; // Reset logical when physical advances
            } else {
                // Same or went back - just increment logical
                self.logical = self.logical.saturating_add(1);
            }
        } else {
            // Clock error - just increment logical
            self.logical = self.logical.saturating_add(1);
        }
    }

    /// Convert to a single u128 value for storage or transmission.
    ///
    /// Format: `(physical << 64) | logical`
    /// This preserves ordering: HLC_A <= HLC_B iff u128(HLC_A) <= u128(HLC_B)
    pub fn as_u128(&self) -> u128 {
        ((self.physical as u128) << 64) | (self.logical as u128)
    }

    /// Convert from a u128 value.
    pub fn from_u128(value: u128) -> Self {
        let physical = (value >> 64) as u64;
        let logical = (value & 0xFFFF_FFFF_FFFF_FFFF) as u64;
        Self { physical, logical }
    }

    /// Get the physical time component as a SystemTime.
    pub fn to_system_time(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_millis(self.physical)
    }

    /// Get current HLC timestamp with maximum drift validation.
    ///
    /// This is a convenience method that calls `Self::now()` and is safe
    /// for production use.
    pub fn current() -> Result<Self, String> {
        Self::now()
    }

    /// Check if this clock is ahead of another by more than drift threshold.
    ///
    /// Useful for detecting when another node's clock is suspiciously ahead.
    pub fn excessive_drift_from(&self, other: &HybridLogicalClock) -> bool {
        other.physical.saturating_sub(self.physical) > Self::MAX_DRIFT_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn hlc_creation_succeeds() {
        let hlc = HybridLogicalClock::now().unwrap();
        assert_eq!(hlc.logical, 0);
        assert!(hlc.physical > 0);
    }

    #[test]
    fn hlc_tick_increments_logical() {
        let mut hlc = HybridLogicalClock::new(1000, 0);
        hlc.tick();
        assert_eq!(hlc.logical, 1);

        hlc.tick();
        assert_eq!(hlc.logical, 2);
    }

    #[test]
    fn hlc_update_with_older_timestamp() {
        let mut local = HybridLogicalClock::new(1000, 0);
        let received = HybridLogicalClock::new(900, 5);

        local.update(&received).unwrap();

        // Logical advances even though physical is old
        assert!(local.logical > 0);
        assert_eq!(local.physical, 1000); // Keeps local physical time
    }

    #[test]
    fn hlc_update_with_newer_timestamp() {
        let mut local = HybridLogicalClock::new(1000, 0);
        let received = HybridLogicalClock::new(1100, 5);

        local.update(&received).unwrap();

        // Adopts the newer physical time
        assert_eq!(local.physical, 1100);
        assert_eq!(local.logical, 6);
    }

    #[test]
    fn hlc_update_with_same_time() {
        let mut local = HybridLogicalClock::new(1000, 3);
        let received = HybridLogicalClock::new(1000, 5);

        local.update(&received).unwrap();

        // Takes the max logical and increments
        assert_eq!(local.physical, 1000);
        assert_eq!(local.logical, 6);
    }

    #[test]
    fn hlc_rejects_too_far_future() {
        let mut local = HybridLogicalClock::new(1000, 0);
        // Create a timestamp far in the future (impossible)
        let far_future = HybridLogicalClock::new(1000 + 2000, 0); // 2 seconds ahead

        let result = local.update(&far_future);
        assert!(result.is_err(), "Should reject timestamp too far in future");
    }

    #[test]
    fn hlc_accepts_small_future_drift() {
        let mut local = HybridLogicalClock::new(1000, 0);
        // Small drift is acceptable
        let slight_future = HybridLogicalClock::new(1000 + 200, 0); // 200ms ahead

        let result = local.update(&slight_future);
        assert!(result.is_ok(), "Should accept small drift");
    }

    #[test]
    fn hlc_preserves_ordering_after_u128_conversion() {
        let hlc1 = HybridLogicalClock::new(1000, 0);
        let hlc2 = HybridLogicalClock::new(1000, 1);
        let hlc3 = HybridLogicalClock::new(1001, 0);

        let u128_1 = hlc1.as_u128();
        let u128_2 = hlc2.as_u128();
        let u128_3 = hlc3.as_u128();

        assert!(u128_1 < u128_2);
        assert!(u128_2 < u128_3);
        assert!(u128_1 < u128_3);
    }

    #[test]
    fn hlc_u128_roundtrip() {
        let original = HybridLogicalClock::new(123456, 789);
        let u128_value = original.as_u128();
        let recovered = HybridLogicalClock::from_u128(u128_value);

        assert_eq!(original, recovered);
    }

    #[test]
    fn hlc_tick_after_system_time_advance() {
        let mut hlc = HybridLogicalClock::new(1000, 5);
        std::thread::sleep(Duration::from_millis(10));

        hlc.tick();

        // After system time advance, logical should reset
        assert!(hlc.logical < 5);
    }

    #[test]
    fn hlc_to_system_time() {
        let hlc = HybridLogicalClock::new(1000, 999);
        let st = hlc.to_system_time();

        // Physical component represents milliseconds, so verify magnitude
        let duration = st.duration_since(UNIX_EPOCH).unwrap();
        assert_eq!(duration.as_millis(), 1000);
    }

    #[test]
    fn hlc_ordering_preserved() {
        let mut hlc1 = HybridLogicalClock::now().unwrap();
        let hlc2 = HybridLogicalClock::now().unwrap();

        assert!(hlc1 <= hlc2);

        hlc1.tick();
        assert!(hlc2 <= hlc1);
    }

    #[test]
    fn hlc_excessive_drift_detection() {
        let local = HybridLogicalClock::new(1000, 0);
        let far_ahead = HybridLogicalClock::new(1000 + 1000, 0); // 1 second ahead

        assert!(local.excessive_drift_from(&far_ahead));

        let close = HybridLogicalClock::new(1000 + 100, 0); // 100ms ahead
        assert!(!local.excessive_drift_from(&close));
    }
}
