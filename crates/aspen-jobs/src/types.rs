//! Supporting types for the job system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Priority level for job execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Lowest priority.
    Low = 0,
    /// Normal priority (default).
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical priority (highest).
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

impl Priority {
    /// Get the queue name for this priority level.
    pub fn queue_name(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Get all priority levels in order from highest to lowest.
    pub fn all_ordered() -> Vec<Self> {
        vec![Self::Critical, Self::High, Self::Normal, Self::Low]
    }
}

/// Retry policy for failed jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryPolicy {
    /// No retries.
    None,
    /// Fixed delay between retries.
    Fixed {
        /// Maximum number of retry attempts.
        max_attempts: u32,
        /// Delay between retries.
        delay: Duration,
    },
    /// Exponential backoff.
    Exponential {
        /// Maximum number of retry attempts.
        max_attempts: u32,
        /// Initial delay.
        initial_delay: Duration,
        /// Multiplier for each retry (e.g., 2.0 for doubling).
        multiplier: f64,
        /// Maximum delay between retries.
        max_delay: Option<Duration>,
    },
    /// Custom retry delays.
    Custom {
        /// List of delays for each retry attempt.
        delays: Vec<Duration>,
        /// Maximum number of attempts (if None, uses delays.len()).
        max_attempts: Option<u32>,
    },
}

impl Default for RetryPolicy {
    fn default() -> Self {
        // Default to exponential backoff with 3 retries
        Self::exponential(3)
    }
}

impl RetryPolicy {
    /// Create a no-retry policy.
    pub fn none() -> Self {
        Self::None
    }

    /// Create a fixed retry policy.
    pub fn fixed(max_attempts: u32, delay: Duration) -> Self {
        Self::Fixed {
            max_attempts,
            delay,
        }
    }

    /// Create an exponential backoff policy with defaults.
    pub fn exponential(max_attempts: u32) -> Self {
        Self::Exponential {
            max_attempts,
            initial_delay: Duration::from_secs(1),
            multiplier: 2.0,
            max_delay: Some(Duration::from_secs(300)), // 5 minutes max
        }
    }

    /// Create an exponential backoff policy with custom parameters.
    pub fn exponential_custom(
        max_attempts: u32,
        initial_delay: Duration,
        multiplier: f64,
        max_delay: Option<Duration>,
    ) -> Self {
        Self::Exponential {
            max_attempts,
            initial_delay,
            multiplier,
            max_delay,
        }
    }

    /// Create a custom retry policy with specific delays.
    pub fn custom(delays: Vec<Duration>) -> Self {
        let max_attempts = delays.len() as u32;
        Self::Custom {
            delays,
            max_attempts: Some(max_attempts),
        }
    }

    /// Get the maximum number of attempts for this policy.
    pub fn max_attempts(&self) -> u32 {
        match self {
            Self::None => 1,
            Self::Fixed { max_attempts, .. } => *max_attempts,
            Self::Exponential { max_attempts, .. } => *max_attempts,
            Self::Custom {
                max_attempts,
                delays,
            } => max_attempts.unwrap_or(delays.len() as u32),
        }
    }
}

/// Schedule for job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    /// Execute once at a specific time.
    Once(DateTime<Utc>),
    /// Execute on a recurring schedule (cron expression).
    Recurring(String),
}

impl Schedule {
    /// Create a one-time schedule.
    pub fn once(time: DateTime<Utc>) -> Self {
        Self::Once(time)
    }

    /// Create a one-time schedule after a delay.
    pub fn after(delay: Duration) -> Self {
        let time = Utc::now() + chrono::Duration::from_std(delay).unwrap();
        Self::Once(time)
    }

    /// Create a recurring schedule with a cron expression.
    ///
    /// # Examples
    ///
    /// - "0 0 * * *" - Daily at midnight
    /// - "*/5 * * * *" - Every 5 minutes
    /// - "0 9-17 * * MON-FRI" - Every hour from 9am to 5pm on weekdays
    pub fn cron<S: Into<String>>(expression: S) -> Self {
        Self::Recurring(expression.into())
    }

    /// Common schedule: every minute.
    pub fn every_minute() -> Self {
        Self::cron("* * * * *")
    }

    /// Common schedule: every hour.
    pub fn every_hour() -> Self {
        Self::cron("0 * * * *")
    }

    /// Common schedule: daily at midnight.
    pub fn daily() -> Self {
        Self::cron("0 0 * * *")
    }

    /// Common schedule: weekly on Sunday at midnight.
    pub fn weekly() -> Self {
        Self::cron("0 0 * * SUN")
    }

    /// Common schedule: monthly on the first day at midnight.
    pub fn monthly() -> Self {
        Self::cron("0 0 1 * *")
    }

    /// Calculate the next execution time.
    pub fn next_execution(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Once(time) => {
                if *time > Utc::now() {
                    Some(*time)
                } else {
                    None
                }
            }
            Self::Recurring(cron_expr) => {
                // Parse cron expression and calculate next time
                // This would use the cron crate in a real implementation
                use std::str::FromStr;
                let schedule = cron::Schedule::from_str(cron_expr).ok()?;
                schedule.upcoming(Utc).next()
            }
        }
    }
}

/// Statistics for a job type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobTypeStats {
    /// Total jobs submitted.
    pub total_submitted: u64,
    /// Jobs currently pending.
    pub pending: u64,
    /// Jobs currently running.
    pub running: u64,
    /// Jobs completed successfully.
    pub completed: u64,
    /// Jobs failed.
    pub failed: u64,
    /// Jobs cancelled.
    pub cancelled: u64,
    /// Average execution time in milliseconds.
    pub avg_execution_time_ms: u64,
    /// Success rate (0.0 to 1.0).
    pub success_rate: f64,
}

/// Queue statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    /// Jobs by priority.
    pub by_priority: std::collections::HashMap<Priority, u64>,
    /// Total jobs in queue.
    pub total_queued: u64,
    /// Jobs being processed.
    pub processing: u64,
    /// Queue depth (waiting jobs).
    pub depth: u64,
    /// Oldest job age in seconds.
    pub oldest_job_age_sec: Option<u64>,
    /// Average wait time in seconds.
    pub avg_wait_time_sec: Option<u64>,
    /// DLQ statistics.
    pub dlq_stats: DLQStats,
}

/// Dead Letter Queue statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DLQStats {
    /// Total jobs in DLQ across all priorities.
    pub total_count: u64,
    /// Jobs in DLQ by priority.
    pub by_priority: std::collections::HashMap<Priority, u64>,
    /// Jobs in DLQ by job type.
    pub by_job_type: std::collections::HashMap<String, u64>,
    /// Oldest DLQ entry age in seconds.
    pub oldest_entry_age_sec: Option<u64>,
    /// Total jobs redriven from DLQ.
    pub total_redriven: u64,
}

