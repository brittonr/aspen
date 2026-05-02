//! Supporting types for the job system.

pub use aspen_jobs_core::DLQStats;
pub use aspen_jobs_core::Priority;
pub use aspen_jobs_core::QueueStats;
pub use aspen_jobs_core::RetryPolicy;
pub use aspen_jobs_core::Schedule;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Timelike;
use chrono::Utc;

/// Runtime scheduling helpers for [`Schedule`].
pub trait ScheduleExt {
    /// Calculate the next execution time.
    fn next_execution(&self) -> Option<DateTime<Utc>>;
}

impl ScheduleExt for Schedule {
    #[allow(unknown_lints)]
    #[allow(ambient_clock, reason = "runtime schedule evaluation needs current UTC time")]
    fn next_execution(&self) -> Option<DateTime<Utc>> {
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
                    let elapsed = now - start;
                    let elapsed_ms = elapsed.num_milliseconds() as u64;
                    let interval_ms = every.as_millis() as u64;
                    if interval_ms == 0 {
                        return Some(now);
                    }
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
                let current_hour_start = now.with_minute(0)?.with_second(0)?.with_nanosecond(0)?;

                if current_hour.is_none_or(|h| h < current_hour_start) {
                    Some(now)
                } else if *current_hour_count < *max_per_hour {
                    Some(now + chrono::Duration::seconds(1))
                } else {
                    Some(current_hour_start + chrono::Duration::hours(1))
                }
            }
            Self::BusinessHours {
                days,
                start_hour,
                end_hour,
                timezone: _,
            } => {
                let weekday = now.weekday().num_days_from_monday() + 1;
                let hour = now.hour() as u8;

                if days.contains(&weekday) {
                    if hour < *start_hour {
                        Some(now.with_hour(*start_hour as u32)?.with_minute(0)?.with_second(0)?)
                    } else if hour < *end_hour {
                        Some(now + chrono::Duration::minutes(1))
                    } else {
                        next_business_day(now, days, *start_hour)
                    }
                } else {
                    next_business_day(now, days, *start_hour)
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
}

fn next_business_day(from: DateTime<Utc>, days: &[u32], start_hour: u8) -> Option<DateTime<Utc>> {
    for i in 1..=7 {
        let next_day = from + chrono::Duration::days(i);
        let weekday = next_day.weekday().num_days_from_monday() + 1;
        if days.contains(&weekday) {
            return next_day.with_hour(start_hour as u32)?.with_minute(0)?.with_second(0);
        }
    }
    None
}

/// Statistics for a job type.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
