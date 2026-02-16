//! Configuration for the job manager.

use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;

/// Configuration for the job manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobManagerConfig {
    /// Default visibility timeout for jobs.
    pub default_visibility_timeout: Duration,
    /// Default job timeout.
    pub default_job_timeout: Duration,
    /// Whether to enable job deduplication.
    pub enable_deduplication: bool,
    /// TTL for deduplication entries.
    pub deduplication_ttl: Duration,
    /// Maximum jobs to schedule per tick.
    pub max_schedule_per_tick: u32,
    /// Scheduler tick interval.
    pub scheduler_interval: Duration,
}

impl Default for JobManagerConfig {
    fn default() -> Self {
        let config = Self {
            default_visibility_timeout: Duration::from_secs(300), // 5 minutes
            default_job_timeout: Duration::from_secs(300),        // 5 minutes
            enable_deduplication: true,
            deduplication_ttl: Duration::from_secs(3600), // 1 hour
            max_schedule_per_tick: 100_u32,
            scheduler_interval: Duration::from_secs(60), // 1 minute
        };
        config.validate();
        config
    }
}

impl JobManagerConfig {
    /// Validate configuration invariants (Tiger Style).
    pub(crate) fn validate(&self) {
        assert!(
            self.max_schedule_per_tick > 0,
            "max_schedule_per_tick must be positive, got {}",
            self.max_schedule_per_tick
        );
        assert!(!self.default_visibility_timeout.is_zero(), "default_visibility_timeout must not be zero");
        assert!(!self.default_job_timeout.is_zero(), "default_job_timeout must not be zero");
    }
}
