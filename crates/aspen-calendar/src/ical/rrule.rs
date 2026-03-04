//! RRULE recurrence rule expansion (RFC 5545 §3.3.10).
//!
//! Supports: FREQ (DAILY/WEEKLY/MONTHLY/YEARLY), INTERVAL, BYDAY,
//! BYMONTHDAY, COUNT, UNTIL, EXDATE filtering.
//!
//! Always bounded by `max_instances` (default 100, max 1000).

use crate::error::CalendarError;

/// Maximum instances to prevent unbounded expansion.
const MAX_INSTANCES_LIMIT: u32 = 1000;

/// A single recurrence instance.
#[derive(Debug, Clone)]
pub struct RecurrenceInstance {
    /// Start time (Unix ms).
    pub dtstart_ms: u64,
    /// End time (Unix ms), if original event had one.
    pub dtend_ms: Option<u64>,
}

/// Parsed RRULE components.
#[derive(Debug)]
struct ParsedRrule {
    freq: Frequency,
    interval: u32,
    count: Option<u32>,
    until_ms: Option<u64>,
    byday: Vec<Weekday>,
    bymonthday: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Weekday {
    Mo,
    Tu,
    We,
    Th,
    Fr,
    Sa,
    Su,
}

/// Expand a recurrence rule into instances within a time range.
///
/// # Arguments
/// - `dtstart_ms`: original event start (Unix ms)
/// - `duration_ms`: event duration (end - start), 0 if no end
/// - `rrule`: RRULE string (e.g., "FREQ=WEEKLY;BYDAY=MO,WE,FR")
/// - `exdates`: excluded dates (Unix ms)
/// - `range_start_ms`: expansion range start
/// - `range_end_ms`: expansion range end
/// - `max_instances`: cap on returned instances (capped at 1000)
pub fn expand_rrule(
    dtstart_ms: u64,
    duration_ms: u64,
    rrule: &str,
    exdates: &[u64],
    range_start_ms: u64,
    range_end_ms: u64,
    max_instances: u32,
) -> Result<Vec<RecurrenceInstance>, CalendarError> {
    let max = max_instances.min(MAX_INSTANCES_LIMIT);
    let parsed = parse_rrule(rrule)?;
    let mut instances = Vec::new();
    let mut current_ms = dtstart_ms;
    let mut count: u32 = 0;
    // Safety: cap total iterations to prevent runaway loops.
    let max_iterations: u32 = max.saturating_mul(10).min(100_000);
    let mut iterations: u32 = 0;

    loop {
        iterations = iterations.saturating_add(1);
        if iterations > max_iterations {
            break;
        }

        // Check termination conditions
        if current_ms > range_end_ms {
            break;
        }
        if let Some(until) = parsed.until_ms
            && current_ms > until
        {
            break;
        }
        if let Some(max_count) = parsed.count
            && count >= max_count
        {
            break;
        }
        if instances.len() as u32 >= max {
            break;
        }

        // For WEEKLY with BYDAY, expand each day in the week
        let candidate_times = if parsed.freq == Frequency::Weekly && !parsed.byday.is_empty() {
            expand_week_days(current_ms, &parsed.byday)
        } else if parsed.freq == Frequency::Monthly && !parsed.bymonthday.is_empty() {
            expand_month_days(current_ms, &parsed.bymonthday)
        } else {
            vec![current_ms]
        };

        for candidate_ms in &candidate_times {
            if *candidate_ms > range_end_ms {
                continue;
            }
            if let Some(until) = parsed.until_ms
                && *candidate_ms > until
            {
                continue;
            }

            count = count.saturating_add(1);

            // Skip if excluded
            if exdates.contains(candidate_ms) {
                continue;
            }

            // Only include if in range
            if *candidate_ms >= range_start_ms {
                let end = if duration_ms > 0 {
                    Some(candidate_ms.saturating_add(duration_ms))
                } else {
                    None
                };
                instances.push(RecurrenceInstance {
                    dtstart_ms: *candidate_ms,
                    dtend_ms: end,
                });

                if instances.len() as u32 >= max {
                    return Ok(instances);
                }
            }

            if let Some(max_count) = parsed.count
                && count >= max_count
            {
                return Ok(instances);
            }
        }

        // Advance to next period
        current_ms = advance(current_ms, &parsed);
    }

    Ok(instances)
}

/// Parse RRULE string into components.
fn parse_rrule(rrule: &str) -> Result<ParsedRrule, CalendarError> {
    let mut freq = None;
    let mut interval: u32 = 1;
    let mut count = None;
    let mut until_ms = None;
    let mut byday = Vec::new();
    let mut bymonthday = Vec::new();

    for part in rrule.split(';') {
        let (key, value) = part.split_once('=').ok_or(CalendarError::InvalidRrule {
            reason: format!("invalid RRULE part: {part}"),
        })?;

        match key.to_ascii_uppercase().as_str() {
            "FREQ" => {
                freq = Some(match value.to_ascii_uppercase().as_str() {
                    "DAILY" => Frequency::Daily,
                    "WEEKLY" => Frequency::Weekly,
                    "MONTHLY" => Frequency::Monthly,
                    "YEARLY" => Frequency::Yearly,
                    _ => {
                        return Err(CalendarError::InvalidRrule {
                            reason: format!("unsupported frequency: {value}"),
                        });
                    }
                });
            }
            "INTERVAL" => interval = value.parse::<u32>().unwrap_or(1).max(1),
            "COUNT" => count = Some(value.parse::<u32>().unwrap_or(1)),
            "UNTIL" => {
                let clean = value.trim().trim_end_matches('Z');
                if clean.len() >= 15 {
                    let year = clean[0..4].parse::<i64>().unwrap_or(2000);
                    let month = clean[4..6].parse::<u32>().unwrap_or(1);
                    let day = clean[6..8].parse::<u32>().unwrap_or(1);
                    let hour = clean[9..11].parse::<u32>().unwrap_or(0);
                    let min = clean[11..13].parse::<u32>().unwrap_or(0);
                    let sec = clean[13..15].parse::<u32>().unwrap_or(0);
                    until_ms = Some(date_to_unix_ms(year, month, day, hour, min, sec));
                } else if clean.len() >= 8 {
                    let year = clean[0..4].parse::<i64>().unwrap_or(2000);
                    let month = clean[4..6].parse::<u32>().unwrap_or(1);
                    let day = clean[6..8].parse::<u32>().unwrap_or(1);
                    until_ms = Some(date_to_unix_ms(year, month, day, 23, 59, 59));
                }
            }
            "BYDAY" => {
                for day_str in value.split(',') {
                    let d = day_str.trim().to_ascii_uppercase();
                    if let Some(wd) = parse_weekday(&d) {
                        byday.push(wd);
                    }
                }
            }
            "BYMONTHDAY" => {
                for d in value.split(',') {
                    if let Ok(day) = d.trim().parse::<u32>()
                        && (1..=31).contains(&day)
                    {
                        bymonthday.push(day);
                    }
                }
            }
            _ => {} // Ignore unsupported parts
        }
    }

    let freq = freq.ok_or(CalendarError::InvalidRrule {
        reason: "missing FREQ".to_string(),
    })?;

    Ok(ParsedRrule {
        freq,
        interval,
        count,
        until_ms,
        byday,
        bymonthday,
    })
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    // Strip any leading ordinal (e.g., "1MO" → "MO")
    let clean = s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '+');
    match clean {
        "MO" => Some(Weekday::Mo),
        "TU" => Some(Weekday::Tu),
        "WE" => Some(Weekday::We),
        "TH" => Some(Weekday::Th),
        "FR" => Some(Weekday::Fr),
        "SA" => Some(Weekday::Sa),
        "SU" => Some(Weekday::Su),
        _ => None,
    }
}

/// Advance timestamp by one period.
fn advance(current_ms: u64, rule: &ParsedRrule) -> u64 {
    let interval_ms = match rule.freq {
        Frequency::Daily => u64::from(rule.interval) * 86_400_000,
        Frequency::Weekly => u64::from(rule.interval) * 7 * 86_400_000,
        Frequency::Monthly => u64::from(rule.interval) * 30 * 86_400_000, // approximate
        Frequency::Yearly => u64::from(rule.interval) * 365 * 86_400_000, // approximate
    };
    current_ms.saturating_add(interval_ms)
}

/// Expand BYDAY within a week starting at `week_start_ms`.
fn expand_week_days(week_start_ms: u64, days: &[Weekday]) -> Vec<u64> {
    let start_day = weekday_from_ms(week_start_ms);
    let mut times = Vec::with_capacity(days.len());

    for target in days {
        let offset_days = weekday_offset(start_day, *target);
        let t = week_start_ms.saturating_add(u64::from(offset_days) * 86_400_000);
        times.push(t);
    }

    times.sort();
    times
}

/// Expand BYMONTHDAY within a month starting at `month_start_ms`.
fn expand_month_days(month_start_ms: u64, days: &[u32]) -> Vec<u64> {
    let secs = (month_start_ms / 1000) as i64;
    let (_, _, current_day, _, _, _) = unix_secs_to_date(secs);

    let mut times = Vec::with_capacity(days.len());
    for &target_day in days {
        let offset = target_day as i64 - current_day as i64;
        let t = if offset >= 0 {
            month_start_ms.saturating_add(offset as u64 * 86_400_000)
        } else {
            month_start_ms.saturating_sub((-offset) as u64 * 86_400_000)
        };
        times.push(t);
    }

    times.sort();
    times
}

/// Get weekday (0=Mon .. 6=Sun) from Unix ms.
fn weekday_from_ms(ms: u64) -> Weekday {
    let days = (ms / 86_400_000) as i64;
    // 1970-01-01 is a Thursday (3)
    let wd = ((days + 3) % 7 + 7) % 7;
    match wd {
        0 => Weekday::Mo,
        1 => Weekday::Tu,
        2 => Weekday::We,
        3 => Weekday::Th,
        4 => Weekday::Fr,
        5 => Weekday::Sa,
        6 => Weekday::Su,
        _ => Weekday::Mo,
    }
}

/// Days offset from `from` to `to` within a week (0..6).
fn weekday_offset(from: Weekday, to: Weekday) -> u32 {
    let from_n = weekday_num(from);
    let to_n = weekday_num(to);
    ((to_n as i32 - from_n as i32 + 7) % 7) as u32
}

fn weekday_num(wd: Weekday) -> u32 {
    match wd {
        Weekday::Mo => 0,
        Weekday::Tu => 1,
        Weekday::We => 2,
        Weekday::Th => 3,
        Weekday::Fr => 4,
        Weekday::Sa => 5,
        Weekday::Su => 6,
    }
}

fn date_to_unix_ms(year: i64, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> u64 {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 12 } else { month };
    let days = 365 * y + y / 4 - y / 100 + y / 400 + (153 * (m as i64 - 3) + 2) / 5 + day as i64 - 719469;
    let total_seconds = days * 86400 + hour as i64 * 3600 + min as i64 * 60 + sec as i64;
    if total_seconds < 0 {
        0
    } else {
        (total_seconds as u64).saturating_mul(1000)
    }
}

fn unix_secs_to_date(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let sec_of_day = secs.rem_euclid(86400);
    let hour = (sec_of_day / 3600) as u32;
    let min = ((sec_of_day % 3600) / 60) as u32;
    let sec = (sec_of_day % 60) as u32;

    let mut days = secs / 86400;
    if secs < 0 && secs % 86400 != 0 {
        days -= 1;
    }
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day, hour, min, sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_expansion() {
        let start = date_to_unix_ms(2026, 3, 15, 10, 0, 0);
        let end = date_to_unix_ms(2026, 3, 20, 10, 0, 0);
        let instances = expand_rrule(start, 3_600_000, "FREQ=DAILY", &[], start, end, 100).unwrap();
        // 15, 16, 17, 18, 19, 20 = 6 days
        assert_eq!(instances.len(), 6);
    }

    #[test]
    fn test_weekly_with_byday() {
        let start = date_to_unix_ms(2026, 3, 16, 10, 0, 0); // Monday
        let end = date_to_unix_ms(2026, 3, 30, 10, 0, 0);
        let instances = expand_rrule(start, 3_600_000, "FREQ=WEEKLY;BYDAY=MO,WE,FR", &[], start, end, 100).unwrap();
        // Week 1: Mon Mar 16, Wed Mar 18, Fri Mar 20
        // Week 2: Mon Mar 23, Wed Mar 25, Fri Mar 27
        // Week 3: Mon Mar 30
        assert!(instances.len() >= 6);
    }

    #[test]
    fn test_count_limit() {
        let start = date_to_unix_ms(2026, 3, 15, 10, 0, 0);
        let far_end = date_to_unix_ms(2030, 1, 1, 0, 0, 0);
        let instances = expand_rrule(start, 0, "FREQ=DAILY;COUNT=5", &[], start, far_end, 100).unwrap();
        assert_eq!(instances.len(), 5);
    }

    #[test]
    fn test_until_limit() {
        let start = date_to_unix_ms(2026, 3, 15, 10, 0, 0);
        let far_end = date_to_unix_ms(2030, 1, 1, 0, 0, 0);
        let until = "20260320T100000Z";
        let instances = expand_rrule(start, 0, &format!("FREQ=DAILY;UNTIL={until}"), &[], start, far_end, 100).unwrap();
        // Mar 15-20 = 6 days
        assert_eq!(instances.len(), 6);
    }

    #[test]
    fn test_exdate_exclusion() {
        let start = date_to_unix_ms(2026, 3, 15, 10, 0, 0);
        let end = date_to_unix_ms(2026, 3, 20, 10, 0, 0);
        let exclude = date_to_unix_ms(2026, 3, 17, 10, 0, 0);
        let instances = expand_rrule(start, 0, "FREQ=DAILY", &[exclude], start, end, 100).unwrap();
        // 6 days minus 1 excluded = 5
        assert_eq!(instances.len(), 5);
    }

    #[test]
    fn test_max_instances_cap() {
        let start = date_to_unix_ms(2026, 1, 1, 0, 0, 0);
        let far_end = date_to_unix_ms(2030, 1, 1, 0, 0, 0);
        let instances = expand_rrule(start, 0, "FREQ=DAILY", &[], start, far_end, 10).unwrap();
        assert_eq!(instances.len(), 10);
    }

    #[test]
    fn test_interval() {
        let start = date_to_unix_ms(2026, 3, 15, 10, 0, 0);
        let end = date_to_unix_ms(2026, 3, 30, 10, 0, 0);
        let instances = expand_rrule(start, 0, "FREQ=DAILY;INTERVAL=3", &[], start, end, 100).unwrap();
        // Mar 15, 18, 21, 24, 27, 30 = 6 days
        assert_eq!(instances.len(), 6);
    }

    #[test]
    fn test_invalid_rrule_missing_freq() {
        let start = date_to_unix_ms(2026, 3, 15, 10, 0, 0);
        let result = expand_rrule(start, 0, "INTERVAL=2", &[], start, start + 86_400_000, 100);
        assert!(result.is_err());
    }
}
