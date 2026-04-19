//! Schedule parsing module.
//!
//! Parses string schedule formats into [`Schedule`] enum variants with validation.
//! Supports cron expressions, named schedules, ISO timestamps, relative times,
//! and intervals.
//!
//! # Supported Formats
//!
//! | Format | Syntax | Example |
//! |--------|--------|---------|
//! | Raw cron | `<expr>` | `0 0 * * *` |
//! | Prefixed cron | `cron:<expr>` | `cron:*/5 * * * *` |
//! | Named | `@name` | `@daily`, `@hourly` |
//! | ISO timestamp | `at:<timestamp>` | `at:2024-01-15T09:00:00Z` |
//! | Relative delay | `in:<duration>` | `in:1h30m`, `in:PT5M` |
//! | Interval | `every:<duration>` | `every:5m`, `every:PT1H` |

use std::str::FromStr;
use std::time::Duration;

use chrono::DateTime;
use chrono::Utc;

use crate::error::JobError;
use crate::types::Schedule;

/// Result type for parsing operations.
pub type Result<T> = std::result::Result<T, JobError>;

/// Tiger Style: Maximum schedule string length to prevent resource exhaustion.
const MAX_SCHEDULE_STRING_LENGTH: usize = 256;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE.saturating_mul(60);
const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR.saturating_mul(24);
const SECONDS_PER_WEEK: u64 = SECONDS_PER_DAY.saturating_mul(7);

/// Tiger Style: Maximum interval duration (30 days) to prevent unbounded scheduling.
const MAX_INTERVAL_SECONDS: u64 = 30u64.saturating_mul(SECONDS_PER_DAY);

#[allow(unknown_lints)]
#[allow(ambient_clock, reason = "schedule parsing needs an explicit UTC boundary helper")]
fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Clone, Copy)]
struct RawInput<'a>(&'a str);

impl RawInput<'_> {
    fn as_str(&self) -> &str {
        self.0
    }
}

struct SecondsAddInput {
    total_secs: u64,
    next_secs: u64,
}

fn checked_add_secs(input: SecondsAddInput, raw_input: RawInput<'_>) -> Result<u64> {
    input.total_secs.checked_add(input.next_secs).ok_or_else(|| JobError::InvalidJobSpec {
        reason: format!("duration '{}' exceeds supported size", raw_input.as_str()),
    })
}

/// Parse a schedule string into a [`Schedule`] enum.
///
/// # Supported Formats
///
/// - Cron expressions: `"0 0 * * *"` or `"cron:0 0 * * *"`
/// - Named schedules: `"@daily"`, `"@hourly"`, `"@weekly"`, `"@monthly"`, `"@yearly"`
/// - ISO 8601 timestamp: `"at:2024-01-15T09:00:00Z"`
/// - Relative time: `"in:1h"`, `"in:30m"`, `"in:2d"`
/// - Interval: `"every:5m"`, `"every:1h"`, `"every:PT5M"`
///
/// # Errors
///
/// Returns [`JobError::InvalidJobSpec`] for:
/// - Empty or whitespace-only input
/// - Input exceeding `MAX_SCHEDULE_STRING_LENGTH` (256 characters)
/// - Invalid cron syntax
/// - Invalid timestamp format
/// - Invalid duration format
/// - Duration exceeding `MAX_INTERVAL_SECONDS` (30 days)
///
/// # Examples
///
/// ```ignore
/// use aspen_jobs::parse_schedule;
///
/// // Cron expression
/// let schedule = parse_schedule("0 0 * * *")?;
///
/// // Named schedule
/// let schedule = parse_schedule("@daily")?;
///
/// // Execute in 1 hour
/// let schedule = parse_schedule("in:1h")?;
///
/// // Execute at specific time
/// let schedule = parse_schedule("at:2024-01-15T09:00:00Z")?;
///
/// // Execute every 5 minutes
/// let schedule = parse_schedule("every:5m")?;
/// ```
pub fn parse_schedule(input: &str) -> Result<Schedule> {
    let input = input.trim();

    // Tiger Style: Fail fast on empty input
    if input.is_empty() {
        return Err(JobError::InvalidJobSpec {
            reason: "schedule string cannot be empty".to_string(),
        });
    }

    // Tiger Style: Bound input length
    if input.len() > MAX_SCHEDULE_STRING_LENGTH {
        return Err(JobError::InvalidJobSpec {
            reason: format!("schedule string exceeds maximum length of {} characters", MAX_SCHEDULE_STRING_LENGTH),
        });
    }

    if input.starts_with('@') {
        let schedule = parse_named_schedule(input)?;
        debug_assert!(matches!(schedule, Schedule::Recurring(_)));
        return Ok(schedule);
    }
    if let Some(cron_expr) = input.strip_prefix("cron:") {
        let schedule = parse_cron_schedule(cron_expr)?;
        debug_assert!(matches!(schedule, Schedule::Recurring(_)));
        return Ok(schedule);
    }
    if let Some(timestamp) = input.strip_prefix("at:") {
        let schedule = parse_at_schedule(timestamp, utc_now())?;
        debug_assert!(matches!(schedule, Schedule::Once(_)));
        return Ok(schedule);
    }
    if let Some(delay_input) = input.strip_prefix("in:") {
        let schedule = parse_in_schedule(delay_input)?;
        debug_assert!(matches!(schedule, Schedule::Once(_)));
        return Ok(schedule);
    }
    if let Some(interval_input) = input.strip_prefix("every:") {
        let schedule = parse_every_schedule(interval_input)?;
        debug_assert!(matches!(schedule, Schedule::Interval { .. }));
        return Ok(schedule);
    }

    // Try to parse as raw cron expression (auto-detect)
    let schedule = parse_cron_schedule(input)?;
    debug_assert!(matches!(schedule, Schedule::Recurring(_)));
    Ok(schedule)
}

/// Parse named schedule (@daily, @hourly, etc.)
/// Uses 7-field cron format: sec min hour day_of_month month day_of_week year
fn parse_named_schedule(input: &str) -> Result<Schedule> {
    match input.to_lowercase().as_str() {
        // sec=0 min=0 hour=0 dom=1 mon=1 dow=* year=*
        "@yearly" | "@annually" => Ok(Schedule::cron("0 0 0 1 1 * *")),
        // sec=0 min=0 hour=0 dom=1 mon=* dow=* year=*
        "@monthly" => Ok(Schedule::cron("0 0 0 1 * * *")),
        // sec=0 min=0 hour=0 dom=* mon=* dow=0 (Sunday) year=*
        "@weekly" => Ok(Schedule::cron("0 0 0 * * 0 *")),
        // sec=0 min=0 hour=0 dom=* mon=* dow=* year=*
        "@daily" | "@midnight" => Ok(Schedule::cron("0 0 0 * * * *")),
        // sec=0 min=0 hour=* dom=* mon=* dow=* year=*
        "@hourly" => Ok(Schedule::cron("0 0 * * * * *")),
        // sec=0 min=* hour=* dom=* mon=* dow=* year=*
        "@minutely" => Ok(Schedule::cron("0 * * * * * *")),
        _ => Err(JobError::InvalidJobSpec {
            reason: format!(
                "unknown named schedule '{}'. Valid options: @yearly, @monthly, @weekly, @daily, @hourly, @minutely",
                input
            ),
        }),
    }
}

/// Parse cron expression with validation.
fn parse_cron_schedule(cron_expr: &str) -> Result<Schedule> {
    let cron_expr = cron_expr.trim();

    if cron_expr.is_empty() {
        return Err(JobError::InvalidJobSpec {
            reason: "cron expression cannot be empty".to_string(),
        });
    }

    // Validate cron expression (fail-fast)
    cron::Schedule::from_str(cron_expr).map_err(|e| JobError::InvalidJobSpec {
        reason: format!("invalid cron expression '{}': {}", cron_expr, e),
    })?;

    Ok(Schedule::Recurring(cron_expr.to_string()))
}

/// Parse absolute timestamp (at:TIMESTAMP).
fn parse_at_schedule(timestamp: &str, current_time: DateTime<Utc>) -> Result<Schedule> {
    let timestamp = timestamp.trim();

    // Parse ISO 8601 / RFC 3339 timestamp
    let dt = DateTime::parse_from_rfc3339(timestamp).map_err(|e| JobError::InvalidJobSpec {
        reason: format!(
            "invalid timestamp '{}': expected ISO 8601 format (e.g., 2024-01-15T09:00:00Z): {}",
            timestamp, e
        ),
    })?;

    let utc_time = dt.with_timezone(&Utc);

    // Validate that the time is in the future
    if utc_time <= current_time {
        return Err(JobError::InvalidJobSpec {
            reason: format!(
                "scheduled time '{}' is in the past (current time: {})",
                utc_time.to_rfc3339(),
                current_time.to_rfc3339()
            ),
        });
    }

    debug_assert!(utc_time > current_time);
    debug_assert_eq!(utc_time.timezone(), Utc);
    Ok(Schedule::Once(utc_time))
}

/// Parse relative time (in:DURATION).
fn parse_in_schedule(duration_str: &str) -> Result<Schedule> {
    let relative_spec = parse_duration(duration_str)?;

    // Tiger Style: Validate duration bounds
    if relative_spec.as_secs() > MAX_INTERVAL_SECONDS {
        return Err(JobError::InvalidJobSpec {
            reason: format!(
                "relative delay {}s exceeds maximum of {} seconds (30 days)",
                relative_spec.as_secs(),
                MAX_INTERVAL_SECONDS
            ),
        });
    }

    Schedule::after(relative_spec)
}

/// Parse interval schedule (every:DURATION).
fn parse_every_schedule(duration_str: &str) -> Result<Schedule> {
    let cadence_spec = parse_duration(duration_str)?;

    // Tiger Style: Validate minimum interval (1 second)
    if cadence_spec.as_secs() == 0 && cadence_spec.subsec_nanos() == 0 {
        return Err(JobError::InvalidJobSpec {
            reason: "interval duration must be at least 1 second".to_string(),
        });
    }

    // Tiger Style: Validate maximum interval
    if cadence_spec.as_secs() > MAX_INTERVAL_SECONDS {
        return Err(JobError::InvalidJobSpec {
            reason: format!(
                "interval {}s exceeds maximum of {} seconds (30 days)",
                cadence_spec.as_secs(),
                MAX_INTERVAL_SECONDS
            ),
        });
    }

    Ok(Schedule::interval(cadence_spec))
}

/// Parse duration string.
///
/// Supports:
/// - Human-readable: "5m", "1h", "30s", "2d", "1h30m"
/// - ISO 8601: "PT5M", "PT1H", "P1D"
fn parse_duration(input: &str) -> Result<Duration> {
    let input = input.trim();

    if input.is_empty() {
        return Err(JobError::InvalidJobSpec {
            reason: "duration cannot be empty".to_string(),
        });
    }

    // Try ISO 8601 duration (starts with P)
    if input.starts_with('P') || input.starts_with('p') {
        return parse_iso8601_duration(input);
    }

    // Parse human-readable duration
    parse_human_duration(input)
}

/// Parse a single ISO 8601 duration unit and return seconds.
fn parse_iso8601_duration_unit(
    unit: char,
    digits: &str,
    is_in_time_part: bool,
    raw_input: RawInput<'_>,
) -> Result<u64> {
    let value: f64 = digits.parse().map_err(|_| JobError::InvalidJobSpec {
        reason: format!("invalid {} value in duration: '{}'", unit, raw_input.as_str()),
    })?;

    let multiplier_secs = match unit {
        'Y' => 365.0 * 24.0 * 60.0 * 60.0,                    // Years (approximate)
        'M' if !is_in_time_part => 30.0 * 24.0 * 60.0 * 60.0, // Months (approximate)
        'D' => 24.0 * 60.0 * 60.0,                            // Days
        'H' => 60.0 * 60.0,                                   // Hours
        'M' if is_in_time_part => 60.0,                       // Minutes
        'S' => 1.0,                                           // Seconds
        _ => {
            return Err(JobError::InvalidJobSpec {
                reason: format!("invalid character '{}' in ISO 8601 duration: '{}'", unit, raw_input.as_str()),
            });
        }
    };
    let total_secs = value * multiplier_secs;
    if !total_secs.is_finite() || !(0.0..=(u64::MAX as f64)).contains(&total_secs) {
        return Err(JobError::InvalidJobSpec {
            reason: format!("duration '{}' exceeds supported size", raw_input.as_str()),
        });
    }

    let total_secs = total_secs as u64;
    debug_assert!(total_secs > 0);
    Ok(total_secs)
}

/// Parse ISO 8601 duration (P[n]Y[n]M[n]DT[n]H[n]M[n]S).
fn parse_iso8601_duration(input: &str) -> Result<Duration> {
    let input = input.to_uppercase();
    let raw_input = RawInput(&input);
    let mut total_secs: u64 = 0;
    let mut chars = input.chars().peekable();

    // Skip 'P'
    if chars.next() != Some('P') {
        return Err(JobError::InvalidJobSpec {
            reason: format!("ISO 8601 duration must start with 'P': '{}'", input),
        });
    }

    let mut is_in_time_part = false;
    let mut current_digits = String::new();

    for c in chars {
        match c {
            'T' => {
                is_in_time_part = true;
            }
            '0'..='9' | '.' => {
                current_digits.push(c);
            }
            'Y' | 'M' | 'D' | 'H' | 'S' => {
                if current_digits.is_empty() {
                    return Err(JobError::InvalidJobSpec {
                        reason: format!("missing number before '{}' in ISO 8601 duration: '{}'", c, input),
                    });
                }
                let unit_secs = parse_iso8601_duration_unit(c, &current_digits, is_in_time_part, raw_input)?;
                total_secs = checked_add_secs(
                    SecondsAddInput {
                        total_secs,
                        next_secs: unit_secs,
                    },
                    raw_input,
                )?;
                current_digits.clear();
            }
            _ => {
                return Err(JobError::InvalidJobSpec {
                    reason: format!("invalid character '{}' in ISO 8601 duration: '{}'", c, input),
                });
            }
        }
    }

    if total_secs == 0 {
        return Err(JobError::InvalidJobSpec {
            reason: format!("ISO 8601 duration '{}' evaluates to zero", input),
        });
    }

    debug_assert!(current_digits.is_empty());
    debug_assert!(total_secs > 0);
    Ok(Duration::from_secs(total_secs))
}

/// Parse a single human duration unit and return its value in seconds.
///
/// Returns Ok(multiplier) for valid units, or Err for invalid units.
fn parse_human_duration_unit(unit: char, digits: &str, raw_input: RawInput<'_>) -> Result<u64> {
    if digits.is_empty() {
        return Err(JobError::InvalidJobSpec {
            reason: format!("missing number before '{}' in duration: '{}'", unit, raw_input.as_str()),
        });
    }

    let value: u64 = digits.parse().map_err(|_| JobError::InvalidJobSpec {
        reason: format!("invalid {} value in duration: '{}'", unit, raw_input.as_str()),
    })?;

    let seconds_per_unit = match unit {
        's' | 'S' => 1,
        'm' => SECONDS_PER_MINUTE,
        'h' | 'H' => SECONDS_PER_HOUR,
        'd' | 'D' => SECONDS_PER_DAY,
        'w' | 'W' => SECONDS_PER_WEEK,
        _ => {
            return Err(JobError::InvalidJobSpec {
                reason: format!(
                    "invalid unit '{}' in duration '{}'. Valid units: s, m, h, d, w",
                    unit,
                    raw_input.as_str()
                ),
            });
        }
    };

    let unit_secs = value.checked_mul(seconds_per_unit).ok_or_else(|| JobError::InvalidJobSpec {
        reason: format!("duration '{}' exceeds supported size", raw_input.as_str()),
    })?;
    debug_assert!(unit_secs > 0);
    Ok(unit_secs)
}

/// Parse human-readable duration (e.g., "5m", "1h30m", "2d").
fn parse_human_duration(input: &str) -> Result<Duration> {
    let raw_input = RawInput(input);
    let mut total_secs: u64 = 0;
    let mut current_digits = String::new();

    for c in input.chars() {
        match c {
            '0'..='9' => {
                current_digits.push(c);
            }
            's' | 'S' | 'm' | 'h' | 'H' | 'd' | 'D' | 'w' | 'W' => {
                let unit_secs = parse_human_duration_unit(c, &current_digits, raw_input)?;
                total_secs = checked_add_secs(
                    SecondsAddInput {
                        total_secs,
                        next_secs: unit_secs,
                    },
                    raw_input,
                )?;
                current_digits.clear();
            }
            ' ' => {
                // Allow spaces between units
            }
            _ => {
                return Err(JobError::InvalidJobSpec {
                    reason: format!("invalid character '{}' in duration '{}'. Valid units: s, m, h, d, w", c, input),
                });
            }
        }
    }

    // Handle trailing number without unit (assume seconds)
    if !current_digits.is_empty() {
        return Err(JobError::InvalidJobSpec {
            reason: format!("duration '{}' ends with number without unit. Use s, m, h, d, or w", input),
        });
    }

    if total_secs == 0 {
        return Err(JobError::InvalidJobSpec {
            reason: format!("duration '{}' evaluates to zero", input),
        });
    }

    debug_assert!(current_digits.is_empty());
    debug_assert!(total_secs > 0);
    Ok(Duration::from_secs(total_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Named schedule tests ===

    #[test]
    fn test_parse_named_daily() {
        let schedule = parse_schedule("@daily").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 * * * *"));
    }

    #[test]
    fn test_parse_named_hourly() {
        let schedule = parse_schedule("@hourly").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 * * * * *"));
    }

    #[test]
    fn test_parse_named_weekly() {
        let schedule = parse_schedule("@weekly").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 * * 0 *"));
    }

    #[test]
    fn test_parse_named_monthly() {
        let schedule = parse_schedule("@monthly").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 1 * * *"));
    }

    #[test]
    fn test_parse_named_yearly() {
        let schedule = parse_schedule("@yearly").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 1 1 * *"));
    }

    #[test]
    fn test_parse_named_annually() {
        let schedule = parse_schedule("@annually").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 1 1 * *"));
    }

    #[test]
    fn test_parse_named_minutely() {
        let schedule = parse_schedule("@minutely").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 * * * * * *"));
    }

    #[test]
    fn test_parse_named_midnight() {
        let schedule = parse_schedule("@midnight").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 * * * *"));
    }

    #[test]
    fn test_parse_named_case_insensitive() {
        let schedule = parse_schedule("@DAILY").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(_)));
    }

    #[test]
    fn test_parse_named_invalid() {
        let result = parse_schedule("@invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown named schedule"));
    }

    // === Cron expression tests ===
    // Note: The cron crate uses 7-field format: sec min hour day_of_month month day_of_week year

    #[test]
    fn test_parse_cron_raw() {
        // Daily at midnight (sec=0 min=0 hour=0 dom=* mon=* dow=* year=*)
        let schedule = parse_schedule("0 0 0 * * * *").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 * * * *"));
    }

    #[test]
    fn test_parse_cron_with_prefix() {
        let schedule = parse_schedule("cron:0 0 0 * * * *").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 0 * * * *"));
    }

    #[test]
    fn test_parse_cron_every_5_minutes() {
        // Every 5 minutes (sec=0 min=*/5 hour=* dom=* mon=* dow=* year=*)
        let schedule = parse_schedule("0 */5 * * * * *").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 */5 * * * * *"));
    }

    #[test]
    fn test_parse_cron_complex() {
        // Every weekday at 9am (7-field format: sec min hour dom month dow year)
        let schedule = parse_schedule("0 0 9 * * Mon-Fri *").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(ref s) if s == "0 0 9 * * Mon-Fri *"));
    }

    #[test]
    fn test_parse_cron_with_seconds() {
        // At specific second/minute/hour on weekdays
        let schedule = parse_schedule("30 15 9 * * Mon-Fri *").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(_)));
    }

    #[test]
    fn test_parse_cron_invalid() {
        let result = parse_schedule("invalid cron");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid cron expression"));
    }

    #[test]
    fn test_parse_cron_empty_after_prefix() {
        let result = parse_schedule("cron:");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cron expression cannot be empty"));
    }

    // === At (timestamp) tests ===

    #[test]
    fn test_parse_at_valid() {
        // Use a far future date to avoid flaky tests
        let schedule = parse_schedule("at:2099-12-31T23:59:59Z").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_at_with_positive_offset() {
        let schedule = parse_schedule("at:2099-12-31T23:59:59+05:00").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_at_with_negative_offset() {
        let schedule = parse_schedule("at:2099-12-31T23:59:59-08:00").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_at_invalid_format() {
        let result = parse_schedule("at:not-a-timestamp");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid timestamp"));
    }

    #[test]
    fn test_parse_at_past_time() {
        let result = parse_schedule("at:2020-01-01T00:00:00Z");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("in the past"));
    }

    // === In (relative) tests ===

    #[test]
    fn test_parse_in_seconds() {
        let schedule = parse_schedule("in:30s").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_minutes() {
        let schedule = parse_schedule("in:5m").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_hours() {
        let schedule = parse_schedule("in:2h").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_days() {
        let schedule = parse_schedule("in:1d").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_weeks() {
        let schedule = parse_schedule("in:1w").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_combined() {
        let schedule = parse_schedule("in:1h30m").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_with_spaces() {
        let schedule = parse_schedule("in:1h 30m").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_iso8601() {
        let schedule = parse_schedule("in:PT5M").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_iso8601_hours() {
        let schedule = parse_schedule("in:PT2H").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_iso8601_complex() {
        let schedule = parse_schedule("in:P1DT2H30M").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    #[test]
    fn test_parse_in_exceeds_max() {
        let result = parse_schedule("in:60d");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    // === Every (interval) tests ===

    #[test]
    fn test_parse_every_seconds() {
        let schedule = parse_schedule("every:30s").unwrap();
        match schedule {
            Schedule::Interval { every, start_at } => {
                assert_eq!(every.as_secs(), 30);
                assert!(start_at.is_none());
            }
            _ => panic!("expected Interval schedule"),
        }
    }

    #[test]
    fn test_parse_every_minutes() {
        let schedule = parse_schedule("every:5m").unwrap();
        match schedule {
            Schedule::Interval { every, start_at } => {
                assert_eq!(every.as_secs(), 300);
                assert!(start_at.is_none());
            }
            _ => panic!("expected Interval schedule"),
        }
    }

    #[test]
    fn test_parse_every_hours() {
        let schedule = parse_schedule("every:1h").unwrap();
        match schedule {
            Schedule::Interval { every, .. } => {
                assert_eq!(every.as_secs(), 3600);
            }
            _ => panic!("expected Interval schedule"),
        }
    }

    #[test]
    fn test_parse_every_days() {
        let schedule = parse_schedule("every:1d").unwrap();
        match schedule {
            Schedule::Interval { every, .. } => {
                assert_eq!(every.as_secs(), 86400);
            }
            _ => panic!("expected Interval schedule"),
        }
    }

    #[test]
    fn test_parse_every_iso8601() {
        let schedule = parse_schedule("every:PT30M").unwrap();
        match schedule {
            Schedule::Interval { every, .. } => {
                assert_eq!(every.as_secs(), 1800);
            }
            _ => panic!("expected Interval schedule"),
        }
    }

    #[test]
    fn test_parse_every_zero() {
        // A duration that parses but is zero - need a valid format that equals 0
        // We can't easily make "0s" work since the parser rejects zero duration
        let result = parse_schedule("every:0s");
        assert!(result.is_err());
        // It will fail with "evaluates to zero" from parse_human_duration
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[test]
    fn test_parse_every_exceeds_max() {
        let result = parse_schedule("every:60d");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    // === Edge cases ===

    #[test]
    fn test_parse_empty() {
        let result = parse_schedule("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_parse_whitespace() {
        let result = parse_schedule("   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_parse_trims_whitespace() {
        let schedule = parse_schedule("  @daily  ").unwrap();
        assert!(matches!(schedule, Schedule::Recurring(_)));
    }

    #[test]
    fn test_parse_exceeds_max_length() {
        let long_input = "a".repeat(300);
        let result = parse_schedule(&long_input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum length"));
    }

    #[test]
    fn test_parse_duration_missing_unit() {
        let result = parse_schedule("in:30");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ends with number without unit"));
    }

    #[test]
    fn test_parse_duration_invalid_char() {
        let result = parse_schedule("in:5x");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid character"));
    }

    #[test]
    fn test_parse_iso8601_lowercase() {
        let schedule = parse_schedule("in:pt5m").unwrap();
        assert!(matches!(schedule, Schedule::Once(_)));
    }

    // === Duration calculation verification ===

    #[test]
    fn test_duration_calculation_human() {
        let duration = parse_duration("1h30m45s").unwrap();
        assert_eq!(duration.as_secs(), 3600 + 1800 + 45);
    }

    #[test]
    fn test_duration_calculation_iso8601() {
        let duration = parse_iso8601_duration("P1DT2H30M").unwrap();
        assert_eq!(duration.as_secs(), 86400 + 7200 + 1800);
    }

    #[test]
    fn test_duration_calculation_iso8601_days_only() {
        let duration = parse_iso8601_duration("P7D").unwrap();
        assert_eq!(duration.as_secs(), 7 * 24 * 60 * 60);
    }
}
