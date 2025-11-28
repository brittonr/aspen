use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimeError {
    #[error("Clock skew detected: system time before UNIX epoch")]
    ClockSkew {
        #[source]
        source: std::time::SystemTimeError,
    },

    #[error("Time overflow when adding duration")]
    TimeOverflow,

    #[error("Invalid timestamp: {reason}")]
    InvalidTimestamp { reason: String },
}

pub type TimeResult<T> = Result<T, TimeError>;

/// Safe wrapper for SystemTime operations that handles errors properly
#[derive(Debug, Clone, Copy)]
pub struct SafeTimestamp {
    inner: SystemTime,
}

impl SafeTimestamp {
    /// Create a new timestamp for the current time
    pub fn now() -> Self {
        SafeTimestamp {
            inner: SystemTime::now(),
        }
    }

    /// Get the Unix timestamp in seconds
    pub fn unix_timestamp(&self) -> TimeResult<u64> {
        self.inner
            .duration_since(UNIX_EPOCH)
            .map_err(|e| TimeError::ClockSkew { source: e })
            .map(|d| d.as_secs())
    }

    /// Get the Unix timestamp in seconds, with fallback to current time on error
    /// Use this only when error handling is not critical (e.g., metrics, logging)
    pub fn unix_timestamp_or_now(&self) -> u64 {
        self.unix_timestamp().unwrap_or_else(|e| {
            tracing::warn!("Failed to get timestamp, using current time as fallback: {}", e);
            // Try once more with current time
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
    }

    /// Add duration to this timestamp
    pub fn add(&self, duration: std::time::Duration) -> TimeResult<SafeTimestamp> {
        self.inner
            .checked_add(duration)
            .map(|inner| SafeTimestamp { inner })
            .ok_or(TimeError::TimeOverflow)
    }

    /// Get elapsed time since this timestamp
    pub fn elapsed(&self) -> TimeResult<std::time::Duration> {
        self.inner.elapsed().map_err(|e| TimeError::ClockSkew { source: e })
    }

    /// Get the inner SystemTime
    pub fn as_system_time(&self) -> SystemTime {
        self.inner
    }
}

impl From<SystemTime> for SafeTimestamp {
    fn from(time: SystemTime) -> Self {
        SafeTimestamp { inner: time }
    }
}

impl Default for SafeTimestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// Convenience function to get current Unix timestamp with proper error handling
pub fn get_unix_timestamp() -> TimeResult<u64> {
    SafeTimestamp::now().unix_timestamp()
}

/// Convenience function that logs errors and returns 0 on failure
/// WARNING: Only use this for non-critical paths where 0 is an acceptable fallback
pub fn get_unix_timestamp_or_zero() -> u64 {
    get_unix_timestamp().unwrap_or_else(|e| {
        tracing::error!("Failed to get current timestamp: {}. Using 0 as fallback.", e);
        0
    })
}

/// Get current Unix timestamp as i64 (used throughout domain layer)
pub fn current_timestamp() -> TimeResult<i64> {
    SafeTimestamp::now().unix_timestamp().map(|ts| ts as i64)
}

/// Get current Unix timestamp as i64 with fallback to 0
/// WARNING: Only use for non-critical paths
pub fn current_timestamp_or_zero() -> i64 {
    current_timestamp().unwrap_or_else(|e| {
        tracing::warn!("Failed to get current timestamp: {}. Using 0 as fallback.", e);
        0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_timestamp_now() {
        let ts = SafeTimestamp::now();
        let unix = ts.unix_timestamp();
        assert!(unix.is_ok());
        assert!(unix.unwrap() > 0);
    }

    #[test]
    fn test_add_duration() {
        let ts = SafeTimestamp::now();
        let future = ts.add(std::time::Duration::from_secs(60));
        assert!(future.is_ok());
    }

    #[test]
    fn test_elapsed() {
        let ts = SafeTimestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = ts.elapsed();
        assert!(elapsed.is_ok());
        assert!(elapsed.unwrap().as_millis() >= 10);
    }
}