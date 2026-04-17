//! RRULE recurrence rule expansion (RFC 5545 §3.3.10).
//!
//! Supports: FREQ (DAILY/WEEKLY/MONTHLY/YEARLY), INTERVAL, BYDAY,
//! BYMONTHDAY, COUNT, UNTIL, EXDATE filtering.
//!
//! Always bounded by `max_instances` (default 100, max 1000).

use crate::error::CalendarError;

/// Maximum instances to prevent unbounded expansion.
const MAX_INSTANCES_LIMIT: u32 = 1000;
const MILLIS_PER_SECOND: u64 = 1_000;
const SECONDS_PER_DAY_I64: i64 = 86_400;
const MILLIS_PER_DAY: u64 = 86_400_000;
const DAYS_PER_WEEK: u64 = 7;

/// A single recurrence instance.
#[derive(Debug, Clone)]
pub struct RecurrenceInstance {
    /// Start time (Unix ms).
    pub dtstart_ms: u64,
    /// End time (Unix ms), if original event had one.
    pub dtend_ms: Option<u64>,
}

/// Input for RRULE expansion.
pub struct RecurrenceExpansionRequest<'a> {
    pub dtstart_ms: u64,
    pub duration_ms: u64,
    pub rrule: &'a str,
    pub exdates: &'a [u64],
    pub range_start_ms: u64,
    pub range_end_ms: u64,
    pub max_instances: u32,
}

struct ExpansionStopContext<'a> {
    parsed: &'a ParsedRrule,
    seen_instances: u32,
    max_instances: u32,
}

struct AppendCandidatesContext<'a> {
    parsed: &'a ParsedRrule,
    request: &'a RecurrenceExpansionRequest<'a>,
    max_instances: u32,
}

/// Parsed RRULE components.
#[derive(Debug)]
struct ParsedRrule {
    freq: Frequency,
    period_stride: u32,
    count: Option<u32>,
    until_ms: Option<u64>,
    byday: Vec<Weekday>,
    bymonthday: Vec<u32>,
}

#[derive(Clone, Copy)]
struct DateTimeParts {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
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
pub fn expand_rrule(request: RecurrenceExpansionRequest<'_>) -> Result<Vec<RecurrenceInstance>, CalendarError> {
    let max_instances = request.max_instances.min(MAX_INSTANCES_LIMIT);
    let parsed = parse_rrule(request.rrule)?;
    let mut instances = Vec::with_capacity(usize::try_from(max_instances).unwrap_or(usize::MAX));
    let mut current_ms = request.dtstart_ms;
    let mut seen_instances: u32 = 0;
    let max_iterations = max_instances.saturating_mul(10).min(100_000);
    debug_assert!(max_instances <= MAX_INSTANCES_LIMIT);
    debug_assert!(max_iterations <= 100_000);

    for _ in 0..max_iterations {
        if should_stop_expansion(current_ms, request.range_end_ms, ExpansionStopContext {
            parsed: &parsed,
            seen_instances,
            max_instances,
        }) {
            break;
        }

        let candidate_times = candidate_times_for_period(current_ms, &parsed);
        let is_complete = append_candidate_instances(
            &mut instances,
            &candidate_times,
            &mut seen_instances,
            AppendCandidatesContext {
                parsed: &parsed,
                request: &request,
                max_instances,
            },
        );
        if is_complete {
            return Ok(instances);
        }

        current_ms = advance(current_ms, &parsed);
    }

    Ok(instances)
}

fn should_stop_expansion(current_ms: u64, range_end_ms: u64, context: ExpansionStopContext<'_>) -> bool {
    debug_assert!(context.max_instances <= MAX_INSTANCES_LIMIT);
    debug_assert!(context.seen_instances <= context.max_instances || context.parsed.count.is_some());
    if current_ms > range_end_ms || context.seen_instances >= context.max_instances {
        return true;
    }
    if let Some(until_ms) = context.parsed.until_ms
        && current_ms > until_ms
    {
        return true;
    }
    if let Some(max_count) = context.parsed.count
        && context.seen_instances >= max_count
    {
        return true;
    }
    false
}

fn candidate_times_for_period(current_ms: u64, parsed: &ParsedRrule) -> Vec<u64> {
    debug_assert!(parsed.period_stride >= 1);
    debug_assert!(parsed.byday.len() <= 7);
    if parsed.freq == Frequency::Weekly && !parsed.byday.is_empty() {
        expand_week_days(current_ms, &parsed.byday)
    } else if parsed.freq == Frequency::Monthly && !parsed.bymonthday.is_empty() {
        expand_month_days(current_ms, &parsed.bymonthday)
    } else {
        vec![current_ms]
    }
}

fn append_candidate_instances(
    instances: &mut Vec<RecurrenceInstance>,
    candidate_times: &[u64],
    seen_instances: &mut u32,
    context: AppendCandidatesContext<'_>,
) -> bool {
    debug_assert!(context.max_instances <= MAX_INSTANCES_LIMIT);
    debug_assert!(*seen_instances <= context.max_instances || context.parsed.count.is_some());
    for &candidate_ms in candidate_times {
        if candidate_ms > context.request.range_end_ms {
            continue;
        }
        if let Some(until_ms) = context.parsed.until_ms
            && candidate_ms > until_ms
        {
            continue;
        }

        *seen_instances = seen_instances.saturating_add(1);
        if context.request.exdates.contains(&candidate_ms) {
            continue;
        }
        if candidate_ms >= context.request.range_start_ms {
            let dtend_ms = if context.request.duration_ms > 0 {
                Some(candidate_ms.saturating_add(context.request.duration_ms))
            } else {
                None
            };
            instances.push(RecurrenceInstance {
                dtstart_ms: candidate_ms,
                dtend_ms,
            });
            if instances.len() >= usize::try_from(context.max_instances).unwrap_or(usize::MAX) {
                return true;
            }
        }

        if let Some(max_count) = context.parsed.count
            && *seen_instances >= max_count
        {
            return true;
        }
    }
    false
}

/// Parse RRULE string into components.
fn parse_rrule(rrule: &str) -> Result<ParsedRrule, CalendarError> {
    let mut freq = None;
    let mut period_stride: u32 = 1;
    let mut count = None;
    let mut until_ms = None;
    let mut byday = Vec::new();
    let mut bymonthday = Vec::new();
    debug_assert!(!rrule.is_empty());
    debug_assert!(period_stride >= 1);

    for part in rrule.split(';') {
        let (key, value) = part.split_once('=').ok_or(CalendarError::InvalidRrule {
            reason: format!("invalid RRULE part: {part}"),
        })?;

        match key.to_ascii_uppercase().as_str() {
            "FREQ" => freq = Some(parse_frequency(value)?),
            "INTERVAL" => period_stride = value.parse::<u32>().unwrap_or(1).max(1),
            "COUNT" => count = Some(value.parse::<u32>().unwrap_or(1)),
            "UNTIL" => until_ms = parse_until_ms(value),
            "BYDAY" => byday = parse_byday_values(value),
            "BYMONTHDAY" => bymonthday = parse_bymonthday_values(value),
            _ => {}
        }
    }

    let freq = freq.ok_or(CalendarError::InvalidRrule {
        reason: "missing FREQ".to_string(),
    })?;

    Ok(ParsedRrule {
        freq,
        period_stride,
        count,
        until_ms,
        byday,
        bymonthday,
    })
}

fn parse_frequency(value: &str) -> Result<Frequency, CalendarError> {
    match value.to_ascii_uppercase().as_str() {
        "DAILY" => Ok(Frequency::Daily),
        "WEEKLY" => Ok(Frequency::Weekly),
        "MONTHLY" => Ok(Frequency::Monthly),
        "YEARLY" => Ok(Frequency::Yearly),
        _ => Err(CalendarError::InvalidRrule {
            reason: format!("unsupported frequency: {value}"),
        }),
    }
}

fn parse_until_ms(value: &str) -> Option<u64> {
    let clean = value.trim().trim_end_matches('Z');
    debug_assert!(clean.len() <= value.len());
    debug_assert!(clean.len() != 14);
    if clean.len() >= 15 {
        return Some(date_to_unix_ms(DateTimeParts {
            year: clean[0..4].parse::<i64>().unwrap_or(2000),
            month: clean[4..6].parse::<u32>().unwrap_or(1),
            day: clean[6..8].parse::<u32>().unwrap_or(1),
            hour: clean[9..11].parse::<u32>().unwrap_or(0),
            minute: clean[11..13].parse::<u32>().unwrap_or(0),
            second: clean[13..15].parse::<u32>().unwrap_or(0),
        }));
    }
    if clean.len() >= 8 {
        return Some(date_to_unix_ms(DateTimeParts {
            year: clean[0..4].parse::<i64>().unwrap_or(2000),
            month: clean[4..6].parse::<u32>().unwrap_or(1),
            day: clean[6..8].parse::<u32>().unwrap_or(1),
            hour: 23,
            minute: 59,
            second: 59,
        }));
    }
    None
}

fn parse_byday_values(value: &str) -> Vec<Weekday> {
    let mut weekdays = Vec::with_capacity(value.split(',').count());
    for day_str in value.split(',') {
        let normalized_day = day_str.trim().to_ascii_uppercase();
        if let Some(weekday) = parse_weekday(&normalized_day) {
            weekdays.push(weekday);
        }
    }
    weekdays
}

fn parse_bymonthday_values(value: &str) -> Vec<u32> {
    let mut month_days = Vec::with_capacity(value.split(',').count());
    for day_str in value.split(',') {
        if let Ok(month_day) = day_str.trim().parse::<u32>()
            && (1..=31).contains(&month_day)
        {
            month_days.push(month_day);
        }
    }
    month_days
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
    let period_days = match rule.freq {
        Frequency::Daily => u64::from(rule.period_stride),
        Frequency::Weekly => u64::from(rule.period_stride).saturating_mul(DAYS_PER_WEEK),
        Frequency::Monthly => u64::from(rule.period_stride).saturating_mul(30),
        Frequency::Yearly => u64::from(rule.period_stride).saturating_mul(365),
    };
    current_ms.saturating_add(saturating_millis_for_days(period_days))
}

/// Expand BYDAY within a week starting at `week_start_ms`.
fn expand_week_days(week_start_ms: u64, days: &[Weekday]) -> Vec<u64> {
    let start_day = weekday_from_ms(week_start_ms);
    let mut times = Vec::with_capacity(days.len());

    for target in days {
        let day_delta_days = weekday_offset(start_day, *target);
        let candidate_time = week_start_ms.saturating_add(saturating_millis_for_days(u64::from(day_delta_days)));
        times.push(candidate_time);
    }

    times.sort();
    times
}

/// Expand BYMONTHDAY within a month starting at `month_start_ms`.
fn expand_month_days(month_start_ms: u64, days: &[u32]) -> Vec<u64> {
    let secs = i64::try_from(month_start_ms / MILLIS_PER_SECOND).unwrap_or(i64::MAX);
    let (_, _, current_day, _, _, _) = unix_secs_to_date(secs);

    let mut times = Vec::with_capacity(days.len());
    for &target_day in days {
        let day_delta_days = i64::from(target_day).saturating_sub(i64::from(current_day));
        times.push(apply_day_offset(month_start_ms, day_delta_days));
    }

    times.sort();
    times
}

/// Get weekday (0=Mon .. 6=Sun) from Unix ms.
fn weekday_from_ms(ms: u64) -> Weekday {
    let elapsed_days = i64::try_from(ms / MILLIS_PER_DAY).unwrap_or(i64::MAX);
    let weekday_index = elapsed_days.saturating_add(3).rem_euclid(7);
    match weekday_index {
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
    let from_index = i32::try_from(weekday_num(from)).unwrap_or(0);
    let to_index = i32::try_from(weekday_num(to)).unwrap_or(0);
    let day_delta_days = (to_index - from_index + 7).rem_euclid(7);
    u32::try_from(day_delta_days).unwrap_or(0)
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

fn date_to_unix_ms(parts: DateTimeParts) -> u64 {
    debug_assert!((1..=12).contains(&parts.month));
    debug_assert!((1..=31).contains(&parts.day));
    let adjusted_year = if parts.month <= 2 {
        parts.year.saturating_sub(1)
    } else {
        parts.year
    };
    let adjusted_month = if parts.month <= 2 {
        parts.month.saturating_add(12)
    } else {
        parts.month
    };
    let day_count = 365_i64
        .saturating_mul(adjusted_year)
        .saturating_add(adjusted_year / 4)
        .saturating_sub(adjusted_year / 100)
        .saturating_add(adjusted_year / 400)
        .saturating_add((153_i64.saturating_mul(i64::from(adjusted_month).saturating_sub(3)).saturating_add(2)) / 5)
        .saturating_add(i64::from(parts.day))
        .saturating_sub(719_469);
    let total_seconds = day_count
        .saturating_mul(SECONDS_PER_DAY_I64)
        .saturating_add(i64::from(parts.hour).saturating_mul(3_600))
        .saturating_add(i64::from(parts.minute).saturating_mul(60))
        .saturating_add(i64::from(parts.second));
    if total_seconds <= 0 {
        0
    } else {
        u64::try_from(total_seconds).unwrap_or(u64::MAX).saturating_mul(MILLIS_PER_SECOND)
    }
}

fn unix_secs_to_date(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let sec_of_day = secs.rem_euclid(SECONDS_PER_DAY_I64);
    debug_assert!(sec_of_day >= 0);
    debug_assert!(sec_of_day < SECONDS_PER_DAY_I64);
    let hour = saturating_u32_from_i64(sec_of_day / 3_600);
    let minute = saturating_u32_from_i64((sec_of_day % 3_600) / 60);
    let second = saturating_u32_from_i64(sec_of_day % 60);

    let mut elapsed_days = secs / SECONDS_PER_DAY_I64;
    if secs < 0 && secs % SECONDS_PER_DAY_I64 != 0 {
        elapsed_days = elapsed_days.saturating_sub(1);
    }
    let shifted_days = elapsed_days.saturating_add(719_468);
    let era = if shifted_days >= 0 {
        shifted_days
    } else {
        shifted_days.saturating_sub(146_096)
    } / 146_097;
    let day_of_era = shifted_days.saturating_sub(era.saturating_mul(146_097));
    let year_of_era = day_of_era
        .saturating_sub(day_of_era / 1_460)
        .saturating_add(day_of_era / 36_524)
        .saturating_sub(day_of_era / 146_096)
        / 365;
    let year_base = year_of_era.saturating_add(era.saturating_mul(400));
    let day_of_year = day_of_era.saturating_sub(
        365_i64
            .saturating_mul(year_of_era)
            .saturating_add(year_of_era / 4)
            .saturating_sub(year_of_era / 100),
    );
    let month_phase = (5_i64.saturating_mul(day_of_year).saturating_add(2)) / 153;
    let day = saturating_u32_from_i64(
        day_of_year
            .saturating_sub((153_i64.saturating_mul(month_phase).saturating_add(2)) / 5)
            .saturating_add(1),
    );
    let month_i64 = if month_phase < 10 {
        month_phase.saturating_add(3)
    } else {
        month_phase.saturating_sub(9)
    };
    let year = if month_i64 <= 2 {
        year_base.saturating_add(1)
    } else {
        year_base
    };
    let month = saturating_u32_from_i64(month_i64);
    debug_assert!((1..=12).contains(&month));
    debug_assert!((1..=31).contains(&day));
    (year, month, day, hour, minute, second)
}

fn apply_day_offset(base_ms: u64, day_offset_delta: i64) -> u64 {
    let offset_ms = saturating_millis_for_days(day_offset_delta.unsigned_abs());
    if day_offset_delta >= 0 {
        base_ms.saturating_add(offset_ms)
    } else {
        base_ms.saturating_sub(offset_ms)
    }
}

fn saturating_millis_for_days(day_count: u64) -> u64 {
    day_count.saturating_mul(MILLIS_PER_DAY)
}

fn saturating_u32_from_i64(value: i64) -> u32 {
    if value <= 0 {
        0
    } else {
        u32::try_from(value).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(year: i64, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> u64 {
        date_to_unix_ms(DateTimeParts {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    fn request<'a>(
        dtstart_ms: u64,
        duration_ms: u64,
        rrule: &'a str,
        exdates: &'a [u64],
        range_start_ms: u64,
        range_end_ms: u64,
        max_instances: u32,
    ) -> RecurrenceExpansionRequest<'a> {
        RecurrenceExpansionRequest {
            dtstart_ms,
            duration_ms,
            rrule,
            exdates,
            range_start_ms,
            range_end_ms,
            max_instances,
        }
    }

    #[test]
    fn test_daily_expansion() {
        let start = dt(2026, 3, 15, 10, 0, 0);
        let end = dt(2026, 3, 20, 10, 0, 0);
        let instances = expand_rrule(request(start, 3_600_000, "FREQ=DAILY", &[], start, end, 100)).unwrap();
        assert_eq!(instances.len(), 6);
    }

    #[test]
    fn test_weekly_with_byday() {
        let start = dt(2026, 3, 16, 10, 0, 0);
        let end = dt(2026, 3, 30, 10, 0, 0);
        let instances =
            expand_rrule(request(start, 3_600_000, "FREQ=WEEKLY;BYDAY=MO,WE,FR", &[], start, end, 100)).unwrap();
        assert!(instances.len() >= 6);
    }

    #[test]
    fn test_count_limit() {
        let start = dt(2026, 3, 15, 10, 0, 0);
        let far_end = dt(2030, 1, 1, 0, 0, 0);
        let instances = expand_rrule(request(start, 0, "FREQ=DAILY;COUNT=5", &[], start, far_end, 100)).unwrap();
        assert_eq!(instances.len(), 5);
    }

    #[test]
    fn test_until_limit() {
        let start = dt(2026, 3, 15, 10, 0, 0);
        let far_end = dt(2030, 1, 1, 0, 0, 0);
        let until = "20260320T100000Z";
        let instances =
            expand_rrule(request(start, 0, &format!("FREQ=DAILY;UNTIL={until}"), &[], start, far_end, 100)).unwrap();
        assert_eq!(instances.len(), 6);
    }

    #[test]
    fn test_exdate_exclusion() {
        let start = dt(2026, 3, 15, 10, 0, 0);
        let end = dt(2026, 3, 20, 10, 0, 0);
        let exclude = dt(2026, 3, 17, 10, 0, 0);
        let instances = expand_rrule(request(start, 0, "FREQ=DAILY", &[exclude], start, end, 100)).unwrap();
        assert_eq!(instances.len(), 5);
    }

    #[test]
    fn test_max_instances_cap() {
        let start = dt(2026, 1, 1, 0, 0, 0);
        let far_end = dt(2030, 1, 1, 0, 0, 0);
        let instances = expand_rrule(request(start, 0, "FREQ=DAILY", &[], start, far_end, 10)).unwrap();
        assert_eq!(instances.len(), 10);
    }

    #[test]
    fn test_interval() {
        let start = dt(2026, 3, 15, 10, 0, 0);
        let end = dt(2026, 3, 30, 10, 0, 0);
        let instances = expand_rrule(request(start, 0, "FREQ=DAILY;INTERVAL=3", &[], start, end, 100)).unwrap();
        assert_eq!(instances.len(), 6);
    }

    #[test]
    fn test_invalid_rrule_missing_freq() {
        let start = dt(2026, 3, 15, 10, 0, 0);
        let result = expand_rrule(request(start, 0, "INTERVAL=2", &[], start, start + MILLIS_PER_DAY, 100));
        assert!(result.is_err());
    }
}
