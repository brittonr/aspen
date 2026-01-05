//! Supporting types for the job system.

use std::time::Duration;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Timelike;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// Priority level for job execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Lowest priority.
    Low = 0,
    /// Normal priority (default).
    #[default]
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical priority (highest).
    Critical = 3,
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
        Self::Fixed { max_attempts, delay }
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
            Self::Custom { max_attempts, delays } => max_attempts.unwrap_or(delays.len() as u32),
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
    /// Execute at fixed intervals.
    Interval {
        /// Duration between executions.
        every: Duration,
        /// Optional start time (defaults to now).
        start_at: Option<DateTime<Utc>>,
    },
    /// Rate-limited execution.
    RateLimit {
        /// Maximum executions per hour.
        max_per_hour: u32,
        /// Current hour's execution count (internal).
        #[serde(skip)]
        current_hour_count: u32,
        /// Current hour timestamp (internal).
        #[serde(skip)]
        current_hour: Option<DateTime<Utc>>,
    },
    /// Business hours execution.
    BusinessHours {
        /// Days of week to run (Monday = 1, Sunday = 7).
        days: Vec<u32>,
        /// Start hour (0-23).
        start_hour: u8,
        /// End hour (0-23).
        end_hour: u8,
        /// Timezone name (e.g., "America/New_York").
        timezone: Option<String>,
    },
    /// Exponential backoff schedule.
    Exponential {
        /// Base delay between executions.
        base_delay: Duration,
        /// Maximum delay between executions.
        max_delay: Duration,
        /// Current multiplier (internal).
        #[serde(skip)]
        current_multiplier: f64,
    },
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

    /// Create an interval schedule.
    pub fn interval(every: Duration) -> Self {
        Self::Interval { every, start_at: None }
    }

    /// Create an interval schedule with specific start time.
    pub fn interval_starting_at(every: Duration, start: DateTime<Utc>) -> Self {
        Self::Interval {
            every,
            start_at: Some(start),
        }
    }

    /// Create a rate-limited schedule.
    pub fn rate_limited(max_per_hour: u32) -> Self {
        Self::RateLimit {
            max_per_hour,
            current_hour_count: 0,
            current_hour: None,
        }
    }

    /// Create a business hours schedule.
    pub fn business_hours(days: Vec<u32>, start_hour: u8, end_hour: u8) -> Self {
        Self::BusinessHours {
            days,
            start_hour,
            end_hour,
            timezone: None,
        }
    }

    /// Create an exponential backoff schedule.
    pub fn exponential_backoff(base_delay: Duration, max_delay: Duration) -> Self {
        Self::Exponential {
            base_delay,
            max_delay,
            current_multiplier: 1.0,
        }
    }

    /// Calculate the next execution time.
    pub fn next_execution(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();

        match self {
            Self::Once(time) => {
                if *time > now {
                    Some(*time)
                } else {
                    None
                }
            }
            Self::Recurring(cron_expr) => {
                use std::str::FromStr;
                let schedule = cron::Schedule::from_str(cron_expr).ok()?;
                schedule.upcoming(Utc).next()
            }
            Self::Interval { every, start_at } => {
                let start = start_at.unwrap_or(now);
                if start > now {
                    Some(start)
                } else {
                    // Calculate next interval from start
                    let elapsed = now - start;
                    let elapsed_ms = elapsed.num_milliseconds() as u64;
                    let interval_ms = every.as_millis() as u64;
                    let periods = (elapsed_ms / interval_ms) + 1;
                    let next_ms = periods * interval_ms;
                    Some(start + chrono::Duration::milliseconds(next_ms as i64))
                }
            }
            Self::RateLimit {
                max_per_hour,
                current_hour_count,
                current_hour,
            } => {
                // Check if we're in a new hour
                let current_hour_start =
                    now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();

                if current_hour.is_none_or(|h| h < current_hour_start) {
                    // New hour, can execute immediately
                    Some(now)
                } else if *current_hour_count < *max_per_hour {
                    // Still have capacity in current hour
                    Some(now + chrono::Duration::seconds(1))
                } else {
                    // Wait until next hour
                    Some(current_hour_start + chrono::Duration::hours(1))
                }
            }
            Self::BusinessHours {
                days,
                start_hour,
                end_hour,
                timezone: _,
            } => {
                // Simple implementation without timezone support for now
                let weekday = now.weekday().num_days_from_monday() + 1;
                let hour = now.hour() as u8;

                // Check if today is a business day
                if days.contains(&weekday) {
                    if hour < *start_hour {
                        // Before business hours today
                        Some(now.with_hour(*start_hour as u32).unwrap().with_minute(0).unwrap().with_second(0).unwrap())
                    } else if hour < *end_hour {
                        // During business hours
                        Some(now + chrono::Duration::minutes(1))
                    } else {
                        // After business hours, find next business day
                        self.next_business_day(now, days, *start_hour)
                    }
                } else {
                    // Not a business day, find next one
                    self.next_business_day(now, days, *start_hour)
                }
            }
            Self::Exponential {
                base_delay,
                max_delay,
                current_multiplier,
            } => {
                let delay_ms = (base_delay.as_millis() as f64 * current_multiplier) as u64;
                let delay_ms = delay_ms.min(max_delay.as_millis() as u64);
                Some(now + chrono::Duration::milliseconds(delay_ms as i64))
            }
        }
    }

    /// Helper to find next business day.
    fn next_business_day(&self, from: DateTime<Utc>, days: &[u32], start_hour: u8) -> Option<DateTime<Utc>> {
        for i in 1..=7 {
            let next_day = from + chrono::Duration::days(i);
            let weekday = next_day.weekday().num_days_from_monday() + 1;
            if days.contains(&weekday) {
                return Some(
                    next_day.with_hour(start_hour as u32).unwrap().with_minute(0).unwrap().with_second(0).unwrap(),
                );
            }
        }
        None
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
