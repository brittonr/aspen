// Health checker configuration with validation
//
// Configuration for health checking intervals, thresholds, and circuit breaker behavior.

use serde::{Deserialize, Serialize};

/// Configuration for health checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval between health checks (seconds)
    pub check_interval_secs: u64,

    /// Timeout for health check response (seconds)
    pub check_timeout_secs: u64,

    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,

    /// Number of consecutive successes needed to recover from degraded state
    pub recovery_threshold: u32,

    /// Enable circuit breaker behavior
    pub enable_circuit_breaker: bool,

    /// Time to wait before retrying unhealthy VM (seconds)
    pub circuit_break_duration_secs: u64,

    /// Enable automatic termination of unhealthy VMs
    pub enable_auto_termination: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            check_timeout_secs: 5,
            failure_threshold: 3,
            recovery_threshold: 2,
            enable_circuit_breaker: true,
            circuit_break_duration_secs: 60,
            enable_auto_termination: false,
        }
    }
}

impl HealthCheckConfig {
    /// Validate configuration values
    ///
    /// Ensures all configuration values are within acceptable ranges
    /// and internally consistent.
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=10).contains(&self.failure_threshold) {
            return Err("failure_threshold must be between 1 and 10".to_string());
        }

        if self.recovery_threshold < 1 || self.recovery_threshold > self.failure_threshold {
            return Err(
                "recovery_threshold must be between 1 and failure_threshold".to_string()
            );
        }

        if !(1..=300).contains(&self.check_interval_secs) {
            return Err("check_interval_secs must be between 1 and 300".to_string());
        }

        if self.check_timeout_secs < 1 || self.check_timeout_secs >= self.check_interval_secs {
            return Err(
                "check_timeout_secs must be >= 1 and < check_interval_secs".to_string()
            );
        }

        if !(10..=600).contains(&self.circuit_break_duration_secs) {
            return Err(
                "circuit_break_duration_secs must be between 10 and 600".to_string()
            );
        }

        Ok(())
    }

    /// Create a new configuration with custom values
    pub fn new(
        check_interval_secs: u64,
        check_timeout_secs: u64,
        failure_threshold: u32,
        recovery_threshold: u32,
    ) -> Result<Self, String> {
        let config = Self {
            check_interval_secs,
            check_timeout_secs,
            failure_threshold,
            recovery_threshold,
            enable_circuit_breaker: true,
            circuit_break_duration_secs: 60,
            enable_auto_termination: false,
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = HealthCheckConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_failure_threshold_validation() {
        let mut config = HealthCheckConfig::default();

        config.failure_threshold = 0;
        assert!(config.validate().is_err());

        config.failure_threshold = 11;
        assert!(config.validate().is_err());

        config.failure_threshold = 5;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_recovery_threshold_validation() {
        let mut config = HealthCheckConfig::default();

        config.recovery_threshold = 0;
        assert!(config.validate().is_err());

        config.failure_threshold = 3;
        config.recovery_threshold = 4;
        assert!(config.validate().is_err());

        config.recovery_threshold = 2;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_timeout_validation() {
        let mut config = HealthCheckConfig::default();

        config.check_timeout_secs = 0;
        assert!(config.validate().is_err());

        config.check_interval_secs = 10;
        config.check_timeout_secs = 10;
        assert!(config.validate().is_err());

        config.check_timeout_secs = 5;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_circuit_breaker_duration_validation() {
        let mut config = HealthCheckConfig::default();

        config.circuit_break_duration_secs = 5;
        assert!(config.validate().is_err());

        config.circuit_break_duration_secs = 601;
        assert!(config.validate().is_err());

        config.circuit_break_duration_secs = 60;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_new_with_custom_values() {
        let config = HealthCheckConfig::new(10, 5, 5, 3);
        assert!(config.is_ok());

        let config = HealthCheckConfig::new(10, 15, 5, 3); // timeout >= interval
        assert!(config.is_err());
    }
}
