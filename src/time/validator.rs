//! Timestamp validation utilities for production systems.
//!
//! Provides validators for:
//! - Standalone timestamp validation (within reasonable bounds)
//! - Sequence validation (monotonic ordering with clock skew tolerance)
//! - Future timestamp detection

use std::time::{SystemTime, Duration};

/// Validates individual timestamps to detect clock skew and anomalies.
///
/// # Example
///
/// ```
/// use std::time::SystemTime;
/// # fn example() {
/// let validator = TimestampValidator::new(3600); // 1 hour tolerance
/// let now = SystemTime::now();
/// match validator.validate(now) {
///     Ok(()) => println!("Timestamp is valid"),
///     Err(e) => println!("Invalid timestamp: {}", e),
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TimestampValidator {
    /// Maximum allowed drift from current time (seconds)
    max_drift_secs: u64,
    /// Maximum age of timestamp (seconds)
    max_age_secs: u64,
}

impl TimestampValidator {
    /// Create a new validator with maximum drift tolerance.
    ///
    /// # Arguments
    ///
    /// * `max_drift_secs` - Maximum acceptable future drift (handles clock skew)
    /// * Default max_age is 24 hours
    pub fn new(max_drift_secs: u64) -> Self {
        Self {
            max_drift_secs,
            max_age_secs: 86400, // 24 hours
        }
    }

    /// Create a new validator with both drift and age limits.
    pub fn with_max_age(max_drift_secs: u64, max_age_secs: u64) -> Self {
        Self {
            max_drift_secs,
            max_age_secs,
        }
    }

    /// Validate a timestamp against current time.
    ///
    /// # Checks
    ///
    /// - **Too far in the future**: Rejected if > max_drift_secs in future
    /// - **Too old**: Rejected if > max_age_secs in past
    /// - **Recent**: Accepted if recent (within tolerances)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if timestamp is valid
    /// - `Err(String)` if timestamp violates any constraint
    pub fn validate(&self, ts: SystemTime) -> Result<(), String> {
        let now = SystemTime::now();

        // Check if timestamp is in the past
        match now.duration_since(ts) {
            Ok(age) => {
                // Timestamp is in the past - check age
                if age.as_secs() > self.max_age_secs {
                    return Err(format!(
                        "Timestamp too old: {} seconds ago (max: {} seconds)",
                        age.as_secs(),
                        self.max_age_secs
                    ));
                }
                Ok(())
            }
            Err(_) => {
                // Timestamp is in the future - check how far
                if let Ok(future) = ts.duration_since(now) {
                    if future.as_secs() > self.max_drift_secs {
                        return Err(format!(
                            "Timestamp too far in future: {} seconds (max: {} seconds)",
                            future.as_secs(),
                            self.max_drift_secs
                        ));
                    }
                    // Small future drift - accept but log concern
                    Ok(())
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Validate and return the timestamp if valid, else use current time.
    pub fn validate_or_now(&self, ts: SystemTime) -> SystemTime {
        match self.validate(ts) {
            Ok(()) => ts,
            Err(_) => SystemTime::now(),
        }
    }
}

impl Default for TimestampValidator {
    fn default() -> Self {
        Self::new(300) // 5 minutes drift tolerance
    }
}

/// Validates a sequence of timestamps to ensure monotonic ordering with tolerance.
///
/// # Example
///
/// ```
/// use std::time::SystemTime;
/// # fn example() {
/// let mut validator = SequenceValidator::new(5); // 5 second skew tolerance
/// let now = SystemTime::now();
///
/// assert!(validator.validate_next(now).is_ok());
///
/// // Small backwards jump is tolerated
/// let slightly_past = now - std::time::Duration::from_secs(2);
/// let result = validator.validate_next(slightly_past);
/// assert!(result.is_ok() && !result.unwrap(), "Time went back 2s");
///
/// // Large backwards jump is rejected
/// let far_past = now - std::time::Duration::from_secs(10);
/// assert!(validator.validate_next(far_past).is_err(), "Too much skew");
/// # }
/// ```
#[derive(Debug)]
pub struct SequenceValidator {
    /// Last seen timestamp
    last_timestamp: Option<SystemTime>,
    /// Tolerance for backwards jumps (seconds)
    skew_tolerance_secs: u64,
}

impl SequenceValidator {
    /// Create a new sequence validator.
    ///
    /// # Arguments
    ///
    /// * `skew_tolerance_secs` - Maximum backwards jump to tolerate (in seconds)
    ///   This handles small clock skew adjustments. Larger jumps are rejected.
    pub fn new(skew_tolerance_secs: u64) -> Self {
        Self {
            last_timestamp: None,
            skew_tolerance_secs,
        }
    }

    /// Validate that a new timestamp maintains sequence ordering.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - Timestamp is forward (normal case)
    /// - `Ok(false)` - Timestamp went backward but within tolerance (warning)
    /// - `Err(String)` - Timestamp violates constraints (reject)
    ///
    /// # Behavior
    ///
    /// On first call, any timestamp is accepted. On subsequent calls:
    /// - Timestamps > last are always accepted
    /// - Timestamps < last are accepted if difference <= tolerance
    /// - Timestamps much < last are rejected with error
    pub fn validate_next(&mut self, ts: SystemTime) -> Result<bool, String> {
        match self.last_timestamp {
            None => {
                self.last_timestamp = Some(ts);
                Ok(true)
            }
            Some(last) => {
                match ts.duration_since(last) {
                    Ok(_) => {
                        // Normal case: time went forward
                        self.last_timestamp = Some(ts);
                        Ok(true)
                    }
                    Err(e) => {
                        // Time went backward
                        let skew = e.duration().as_secs();
                        if skew <= self.skew_tolerance_secs {
                            // Small skew - accept and adjust
                            self.last_timestamp = Some(ts);
                            Ok(false) // Warning: time went backwards
                        } else {
                            // Large skew - reject
                            Err(format!(
                                "Sequence violation: timestamp went back {}s (tolerance: {}s)",
                                skew, self.skew_tolerance_secs
                            ))
                        }
                    }
                }
            }
        }
    }

    /// Get the last validated timestamp.
    pub fn last(&self) -> Option<SystemTime> {
        self.last_timestamp
    }

    /// Reset the validator to initial state.
    pub fn reset(&mut self) {
        self.last_timestamp = None;
    }
}

impl Default for SequenceValidator {
    fn default() -> Self {
        Self::new(5) // 5 second tolerance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_validator_accepts_recent() {
        let validator = TimestampValidator::new(300);
        let now = SystemTime::now();
        assert!(validator.validate(now).is_ok());
    }

    #[test]
    fn timestamp_validator_rejects_too_old() {
        let validator = TimestampValidator::new(300);
        let ancient = SystemTime::now() - Duration::from_secs(100000);
        assert!(validator.validate(ancient).is_err());
    }

    #[test]
    fn timestamp_validator_accepts_small_future() {
        let validator = TimestampValidator::new(300);
        let near_future = SystemTime::now() + Duration::from_secs(100);
        assert!(validator.validate(near_future).is_ok());
    }

    #[test]
    fn timestamp_validator_rejects_far_future() {
        let validator = TimestampValidator::new(300);
        let far_future = SystemTime::now() + Duration::from_secs(1000);
        assert!(validator.validate(far_future).is_err());
    }

    #[test]
    fn sequence_validator_detects_forward_time() {
        let mut validator = SequenceValidator::new(5);
        let now = SystemTime::now();

        let result = validator.validate_next(now).unwrap();
        assert!(result, "First timestamp should return true");

        let later = now + Duration::from_secs(1);
        let result = validator.validate_next(later).unwrap();
        assert!(result, "Forward time should return true");
    }

    #[test]
    fn sequence_validator_detects_small_skew() {
        let mut validator = SequenceValidator::new(5);
        let now = SystemTime::now();

        validator.validate_next(now).unwrap();

        let past = now - Duration::from_secs(2);
        let result = validator.validate_next(past).unwrap();

        assert!(!result, "Should detect the backwards jump");
    }

    #[test]
    fn sequence_validator_rejects_large_skew() {
        let mut validator = SequenceValidator::new(5);
        let now = SystemTime::now();

        validator.validate_next(now).unwrap();

        let far_past = now - Duration::from_secs(10);
        let result = validator.validate_next(far_past);

        assert!(result.is_err(), "Should reject large backwards jump");
    }

    #[test]
    fn sequence_validator_reset() {
        let mut validator = SequenceValidator::new(5);
        let now = SystemTime::now();

        validator.validate_next(now).unwrap();
        assert!(validator.last().is_some());

        validator.reset();
        assert!(validator.last().is_none());
    }

    #[test]
    fn validator_default_creation() {
        let _default = TimestampValidator::default();
        let _seq_default = SequenceValidator::default();
    }
}
