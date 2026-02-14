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
    pub max_schedule_per_tick: usize,
    /// Scheduler tick interval.
    pub scheduler_interval: Duration,
}

impl Default for JobManagerConfig {
    fn default() -> Self {
        Self {
            default_visibility_timeout: Duration::from_secs(300), // 5 minutes
            default_job_timeout: Duration::from_secs(300),        // 5 minutes
            enable_deduplication: true,
            deduplication_ttl: Duration::from_secs(3600), // 1 hour
            max_schedule_per_tick: 100,
            scheduler_interval: Duration::from_secs(60), // 1 minute
        }
    }
}
