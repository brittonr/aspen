//! iCalendar parser (RFC 5545).
//!
//! Parses iCalendar VEVENT components into CalendarEvent structs.

use crate::error::CalendarError;
use crate::types::AlarmAction;
use crate::types::AttendeeRole;
use crate::types::AttendeeStatus;
use crate::types::CalendarEvent;
use crate::types::EventAlarm;
use crate::types::EventAttendee;
use crate::types::EventStatus;

const SECONDS_PER_DAY_I64: i64 = 86_400;
const MILLIS_PER_SECOND_U64: u64 = 1_000;
const DATE_SHIFT_DAYS: i64 = 719_469;

struct PendingAlarm {
    trigger_minutes_before: Option<i32>,
    action: AlarmAction,
    description: Option<String>,
    next_alarm_id: u32,
}

impl PendingAlarm {
    fn new() -> Self {
        Self {
            trigger_minutes_before: None,
            action: AlarmAction::Display,
            description: None,
            next_alarm_id: 0,
        }
    }

    fn reset(&mut self) {
        self.trigger_minutes_before = None;
        self.action = AlarmAction::Display;
        self.description = None;
    }
}

struct EventParseState {
    is_inside_event: bool,
    is_inside_alarm: bool,
    event: CalendarEvent,
    pending_alarm: PendingAlarm,
}

impl EventParseState {
    fn new(line_count: usize) -> Self {
        Self {
            is_inside_event: false,
            is_inside_alarm: false,
            event: empty_calendar_event(line_count),
            pending_alarm: PendingAlarm::new(),
        }
    }
}

struct DateTimeParts {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

struct AlarmProperty<'a> {
    name_upper: &'a str,
    value: &'a str,
}

/// Parse a single VEVENT from iCalendar text.
///
/// Returns a partially-filled CalendarEvent (caller must set `id`,
/// `calendar_id`, `created_at_ms`, `updated_at_ms`).
pub fn parse_vevent(input: &str) -> Result<CalendarEvent, CalendarError> {
    let lines = unfold_lines(input);
    let mut parse_state = EventParseState::new(lines.len());

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if handle_component_boundary(trimmed, &mut parse_state) {
            continue;
        }
        if !parse_state.is_inside_event {
            continue;
        }
        handle_event_line(trimmed, &mut parse_state);
    }

    validate_parsed_event(parse_state.event)
}

/// Parse multiple VEVENTs from a VCALENDAR block.
pub fn parse_vcalendar(input: &str) -> Result<Vec<CalendarEvent>, CalendarError> {
    let max_event_count = input.match_indices("BEGIN:VEVENT").count();
    let mut events = Vec::with_capacity(max_event_count);
    let mut current_event = String::new();
    let mut is_inside_event = false;
    let mut nesting_depth: u32 = 0;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("BEGIN:VEVENT") && !is_inside_event {
            is_inside_event = true;
            nesting_depth = 0;
            current_event.clear();
        }
        if !is_inside_event {
            continue;
        }

        current_event.push_str(line);
        current_event.push('\n');
        nesting_depth = update_component_depth(trimmed, nesting_depth);

        if trimmed.eq_ignore_ascii_case("END:VEVENT") && nesting_depth == 0 {
            is_inside_event = false;
            events.push(parse_vevent(&current_event)?);
        }
    }

    debug_assert!(events.len() <= max_event_count);
    Ok(events)
}

// ============================================================================
// Internal helpers
// ============================================================================

fn empty_calendar_event(line_count: usize) -> CalendarEvent {
    CalendarEvent {
        id: String::new(),
        calendar_id: String::new(),
        uid: String::new(),
        summary: String::new(),
        description: None,
        location: None,
        dtstart_ms: 0,
        dtend_ms: None,
        is_all_day: false,
        rrule: None,
        exdates: Vec::with_capacity(line_count.min(4)),
        attendees: Vec::with_capacity(line_count.min(4)),
        alarms: Vec::with_capacity(line_count.min(2)),
        status: EventStatus::Confirmed,
        categories: Vec::new(),
        url: None,
        created_at_ms: 0,
        updated_at_ms: 0,
        sequence: 0,
    }
}

fn handle_component_boundary(trimmed: &str, parse_state: &mut EventParseState) -> bool {
    debug_assert!(!trimmed.is_empty());
    if trimmed.eq_ignore_ascii_case("BEGIN:VEVENT") {
        parse_state.is_inside_event = true;
        debug_assert!(parse_state.is_inside_event);
        return true;
    }
    if trimmed.eq_ignore_ascii_case("END:VEVENT") {
        if parse_state.is_inside_alarm {
            flush_alarm(&mut parse_state.event, &mut parse_state.pending_alarm);
            parse_state.is_inside_alarm = false;
        }
        parse_state.is_inside_event = false;
        debug_assert!(!parse_state.is_inside_event);
        return true;
    }
    if trimmed.eq_ignore_ascii_case("BEGIN:VALARM") {
        parse_state.is_inside_alarm = true;
        parse_state.pending_alarm.reset();
        debug_assert!(parse_state.is_inside_alarm);
        return true;
    }
    if trimmed.eq_ignore_ascii_case("END:VALARM") {
        flush_alarm(&mut parse_state.event, &mut parse_state.pending_alarm);
        parse_state.is_inside_alarm = false;
        debug_assert!(!parse_state.is_inside_alarm);
        return true;
    }
    false
}

fn handle_event_line(trimmed: &str, parse_state: &mut EventParseState) {
    let (name, _params, value) = parse_property(trimmed);
    let name_upper = name.to_ascii_uppercase();
    debug_assert!(!name_upper.is_empty());

    if parse_state.is_inside_alarm {
        handle_alarm_line(
            AlarmProperty {
                name_upper: name_upper.as_str(),
                value,
            },
            &mut parse_state.pending_alarm,
        );
        return;
    }

    match name_upper.as_str() {
        "UID" => parse_state.event.uid = value.to_string(),
        "SUMMARY" => parse_state.event.summary = value.to_string(),
        "DESCRIPTION" => parse_state.event.description = Some(value.to_string()),
        "LOCATION" => parse_state.event.location = Some(value.to_string()),
        "DTSTART" => {
            let (timestamp_ms, is_all_day) = parse_datetime(name, value);
            parse_state.event.dtstart_ms = timestamp_ms;
            parse_state.event.is_all_day = is_all_day;
        }
        "DTEND" => {
            let (timestamp_ms, _) = parse_datetime(name, value);
            parse_state.event.dtend_ms = Some(timestamp_ms);
        }
        "RRULE" => parse_state.event.rrule = Some(value.to_string()),
        "EXDATE" => append_exdates(&mut parse_state.event.exdates, value),
        "ATTENDEE" => {
            if let Some(attendee) = parse_attendee(trimmed) {
                parse_state.event.attendees.push(attendee);
            }
        }
        "STATUS" => parse_state.event.status = parse_status(value),
        "CATEGORIES" => parse_state.event.categories = parse_categories(value),
        "URL" => parse_state.event.url = Some(value.to_string()),
        "SEQUENCE" => parse_state.event.sequence = value.parse::<u32>().unwrap_or(0),
        _ => {}
    }

    debug_assert!(
        parse_state.event.dtend_ms.is_none() || parse_state.event.dtend_ms >= Some(parse_state.event.dtstart_ms)
    );
}

fn handle_alarm_line(alarm_property: AlarmProperty<'_>, pending_alarm: &mut PendingAlarm) {
    match alarm_property.name_upper {
        "TRIGGER" => pending_alarm.trigger_minutes_before = Some(parse_trigger(alarm_property.value)),
        "ACTION" => {
            pending_alarm.action = if alarm_property.value.eq_ignore_ascii_case("EMAIL") {
                AlarmAction::Email
            } else {
                AlarmAction::Display
            };
        }
        "DESCRIPTION" => pending_alarm.description = Some(alarm_property.value.to_string()),
        _ => {}
    }
}

fn append_exdates(exdates: &mut Vec<u64>, value: &str) {
    let max_exdates = value.split(',').count();
    exdates.reserve(max_exdates);
    for date_value in value.split(',') {
        let (timestamp_ms, _) = parse_datetime("EXDATE", date_value.trim());
        exdates.push(timestamp_ms);
    }
}

fn parse_status(value: &str) -> EventStatus {
    match value.to_ascii_uppercase().as_str() {
        "TENTATIVE" => EventStatus::Tentative,
        "CANCELLED" => EventStatus::Cancelled,
        _ => EventStatus::Confirmed,
    }
}

fn parse_categories(value: &str) -> Vec<String> {
    let max_category_count = value.split(',').count();
    let mut categories = Vec::with_capacity(max_category_count);
    for category in value.split(',') {
        categories.push(category.trim().to_string());
    }
    categories
}

fn validate_parsed_event(event: CalendarEvent) -> Result<CalendarEvent, CalendarError> {
    if event.summary.is_empty() && event.uid.is_empty() {
        return Err(CalendarError::ParseIcal {
            reason: "missing SUMMARY or UID property".to_string(),
        });
    }
    debug_assert!(event.dtend_ms.is_none() || event.dtend_ms >= Some(event.dtstart_ms));
    Ok(event)
}

fn update_component_depth(trimmed: &str, current_depth: u32) -> u32 {
    let upper = trimmed.to_ascii_uppercase();
    if upper.starts_with("BEGIN:") {
        return current_depth.saturating_add(1);
    }
    if upper.starts_with("END:") {
        return current_depth.saturating_sub(1);
    }
    current_depth
}

/// Unfold continuation lines (RFC 5545 §3.1).
fn unfold_lines(input: &str) -> Vec<String> {
    let max_line_count = input.lines().count();
    let mut result: Vec<String> = Vec::with_capacity(max_line_count);
    for line in input.lines() {
        if (line.starts_with(' ') || line.starts_with('\t')) && !result.is_empty() {
            if let Some(last) = result.last_mut() {
                last.push_str(line.get(1..).unwrap_or_default());
            }
        } else {
            result.push(line.to_string());
        }
    }
    debug_assert!(result.len() <= max_line_count);
    result
}

/// Parse a property line into (name, params, value).
fn parse_property(line: &str) -> (&str, Vec<(&str, &str)>, &str) {
    let (name_and_params, value) = split_property_name_and_value(line);
    let Some(semicolon_pos) = name_and_params.find(';') else {
        debug_assert!(!name_and_params.is_empty() || value.is_empty());
        return (name_and_params, Vec::new(), value);
    };

    let name = &name_and_params[..semicolon_pos];
    let params_input = name_and_params.get(semicolon_pos.saturating_add(1)..).unwrap_or_default();
    let mut params = Vec::with_capacity(params_input.split(';').count());
    for param in params_input.split(';') {
        if let Some(eq_pos) = param.find('=') {
            let param_value = param.get(eq_pos.saturating_add(1)..).unwrap_or_default();
            params.push((&param[..eq_pos], param_value));
        }
    }
    debug_assert!(!name.is_empty() || params.is_empty());
    (name, params, value)
}

fn split_property_name_and_value(line: &str) -> (&str, &str) {
    if let Some(colon_pos) = find_value_colon(line) {
        let value = line.get(colon_pos.saturating_add(1)..).unwrap_or_default();
        (&line[..colon_pos], value)
    } else {
        (line, "")
    }
}

fn find_value_colon(line: &str) -> Option<usize> {
    let mut is_in_quotes = false;
    for (i, ch) in line.char_indices() {
        match ch {
            '"' => is_in_quotes = !is_in_quotes,
            ':' if !is_in_quotes => return Some(i),
            _ => {}
        }
    }
    None
}

/// Parse iCalendar date/datetime string to Unix ms.
///
/// Supports:
/// - `YYYYMMDD` (DATE, all-day) → midnight UTC
/// - `YYYYMMDDTHHMMSS` (local datetime) → treated as UTC
/// - `YYYYMMDDTHHMMSSZ` (UTC datetime)
fn parse_datetime(property_name: impl AsRef<str>, value: &str) -> (u64, bool) {
    let property_name = property_name.as_ref();
    let clean = value.trim().trim_end_matches('Z');
    let is_date_only = clean.len() == 8 || property_name.contains("VALUE=DATE");
    debug_assert!(clean.is_empty() || clean.len() == 8 || clean.len() >= 15);

    if is_date_only && clean.len() >= 8 {
        return (date_time_parts_to_unix_ms(parse_date_parts(clean)), true);
    }
    if clean.len() >= 15 {
        return (date_time_parts_to_unix_ms(parse_timestamp_parts(clean)), false);
    }
    (0, false)
}

fn parse_date_parts(value: &str) -> DateTimeParts {
    DateTimeParts {
        year: parse_i64_part(value, 0..4, 2000),
        month: parse_u32_part(value, 4..6, 1),
        day: parse_u32_part(value, 6..8, 1),
        hour: 0,
        minute: 0,
        second: 0,
    }
}

fn parse_timestamp_parts(value: &str) -> DateTimeParts {
    DateTimeParts {
        year: parse_i64_part(value, 0..4, 2000),
        month: parse_u32_part(value, 4..6, 1),
        day: parse_u32_part(value, 6..8, 1),
        hour: parse_u32_part(value, 9..11, 0),
        minute: parse_u32_part(value, 11..13, 0),
        second: parse_u32_part(value, 13..15, 0),
    }
}

fn parse_i64_part(value: &str, range: std::ops::Range<usize>, fallback: i64) -> i64 {
    value.get(range).and_then(|part| part.parse::<i64>().ok()).unwrap_or(fallback)
}

fn parse_u32_part(value: &str, range: std::ops::Range<usize>, fallback: u32) -> u32 {
    value.get(range).and_then(|part| part.parse::<u32>().ok()).unwrap_or(fallback)
}

/// Simple date → Unix timestamp in milliseconds.
///
/// Handles years 1970–2099. Not a full calendar implementation.
fn date_time_parts_to_unix_ms(date_time: DateTimeParts) -> u64 {
    debug_assert!((1..=12).contains(&date_time.month));
    debug_assert!((1..=31).contains(&date_time.day));

    let adjusted_year = if date_time.month <= 2 {
        date_time.year.saturating_sub(1)
    } else {
        date_time.year
    };
    let adjusted_month = if date_time.month <= 2 {
        date_time.month.saturating_add(12)
    } else {
        date_time.month
    };
    let month_index = i64::from(adjusted_month).saturating_sub(3);
    let days = 365_i64
        .saturating_mul(adjusted_year)
        .saturating_add(adjusted_year / 4)
        .saturating_sub(adjusted_year / 100)
        .saturating_add(adjusted_year / 400)
        .saturating_add((153_i64.saturating_mul(month_index).saturating_add(2)) / 5)
        .saturating_add(i64::from(date_time.day))
        .saturating_sub(DATE_SHIFT_DAYS);
    let total_seconds = days
        .saturating_mul(SECONDS_PER_DAY_I64)
        .saturating_add(i64::from(date_time.hour).saturating_mul(3_600))
        .saturating_add(i64::from(date_time.minute).saturating_mul(60))
        .saturating_add(i64::from(date_time.second));

    if total_seconds < 0 {
        0
    } else {
        saturating_u64_from_i64(total_seconds).saturating_mul(MILLIS_PER_SECOND_U64)
    }
}

fn saturating_u64_from_i64(value: i64) -> u64 {
    if value <= 0 {
        0
    } else {
        match u64::try_from(value) {
            Ok(converted_value) => converted_value,
            Err(_) => u64::MAX,
        }
    }
}

/// Parse TRIGGER duration (e.g., "-PT15M" → 15, "-PT1H" → 60).
///
/// Returns minutes before event (always positive).
fn parse_trigger(value: &str) -> i32 {
    let is_negative = value.starts_with('-');
    let clean = value.trim_start_matches('-').trim_start_matches('+');
    let clean = clean.trim_start_matches("PT").trim_start_matches("P");

    let mut minutes_before = 0_i32;
    let mut number_buffer = String::new();
    for ch in clean.chars() {
        if ch.is_ascii_digit() {
            number_buffer.push(ch);
            continue;
        }
        let amount = number_buffer.parse().unwrap_or(0);
        number_buffer.clear();
        minutes_before = minutes_before.saturating_add(trigger_unit_minutes(ch, amount));
    }

    debug_assert!(minutes_before >= 0);
    if is_negative { minutes_before } else { -minutes_before }
}

fn trigger_unit_minutes(unit: char, amount: i32) -> i32 {
    match unit {
        'W' => amount.saturating_mul(7 * 24 * 60),
        'D' => amount.saturating_mul(24 * 60),
        'H' => amount.saturating_mul(60),
        'M' => amount,
        'S' => amount / 60,
        _ => 0,
    }
}

/// Parse ATTENDEE property line.
fn parse_attendee(line: &str) -> Option<EventAttendee> {
    let (_, params, value) = parse_property(line);
    let email = value.strip_prefix("mailto:").unwrap_or(value).to_string();
    if email.is_empty() {
        return None;
    }

    let mut name = None;
    let mut role = AttendeeRole::Required;
    let mut status = AttendeeStatus::NeedsAction;
    for (key, value) in &params {
        match key.to_ascii_uppercase().as_str() {
            "CN" => name = Some(value.trim_matches('"').to_string()),
            "ROLE" => role = parse_attendee_role(value),
            "PARTSTAT" => status = parse_attendee_status(value),
            _ => {}
        }
    }

    debug_assert!(!email.is_empty());
    debug_assert!(name.as_deref().is_none_or(|attendee_name| !attendee_name.is_empty()));
    Some(EventAttendee {
        email,
        name,
        role,
        status,
    })
}

fn parse_attendee_role(value: &str) -> AttendeeRole {
    match value.to_ascii_uppercase().as_str() {
        "OPT-PARTICIPANT" => AttendeeRole::Optional,
        "CHAIR" => AttendeeRole::Chair,
        _ => AttendeeRole::Required,
    }
}

fn parse_attendee_status(value: &str) -> AttendeeStatus {
    match value.to_ascii_uppercase().as_str() {
        "ACCEPTED" => AttendeeStatus::Accepted,
        "DECLINED" => AttendeeStatus::Declined,
        "TENTATIVE" => AttendeeStatus::Tentative,
        _ => AttendeeStatus::NeedsAction,
    }
}

/// Flush a pending alarm into the event.
fn flush_alarm(event: &mut CalendarEvent, pending_alarm: &mut PendingAlarm) {
    let Some(trigger_mins) = pending_alarm.trigger_minutes_before.take() else {
        return;
    };
    pending_alarm.next_alarm_id = pending_alarm.next_alarm_id.saturating_add(1);
    event.alarms.push(EventAlarm {
        id: format!("alarm-{}", pending_alarm.next_alarm_id),
        trigger_minutes_before: trigger_mins,
        action: pending_alarm.action.clone(),
        description: pending_alarm.description.take(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_event() {
        let ical = "\
BEGIN:VEVENT\r\n\
UID:event-001@example.com\r\n\
DTSTART:20260315T100000Z\r\n\
DTEND:20260315T110000Z\r\n\
SUMMARY:Team Meeting\r\n\
END:VEVENT";

        let event = parse_vevent(ical).unwrap();
        assert_eq!(event.summary, "Team Meeting");
        assert_eq!(event.uid, "event-001@example.com");
        assert!(!event.is_all_day);
        assert!(event.dtstart_ms > 0);
        assert!(event.dtend_ms.is_some());
        assert!(event.dtend_ms.unwrap() > event.dtstart_ms);
    }

    #[test]
    fn test_parse_all_day_event() {
        let ical = "\
BEGIN:VEVENT\r\n\
UID:allday-001\r\n\
DTSTART;VALUE=DATE:20260315\r\n\
DTEND;VALUE=DATE:20260316\r\n\
SUMMARY:Holiday\r\n\
END:VEVENT";

        let event = parse_vevent(ical).unwrap();
        assert!(event.is_all_day);
        assert_eq!(event.summary, "Holiday");
    }

    #[test]
    fn test_parse_event_with_alarm() {
        let ical = "\
BEGIN:VEVENT\r\n\
UID:alarm-001\r\n\
DTSTART:20260315T100000Z\r\n\
SUMMARY:Meeting\r\n\
BEGIN:VALARM\r\n\
TRIGGER:-PT15M\r\n\
ACTION:DISPLAY\r\n\
DESCRIPTION:Reminder\r\n\
END:VALARM\r\n\
END:VEVENT";

        let event = parse_vevent(ical).unwrap();
        assert_eq!(event.alarms.len(), 1);
        assert_eq!(event.alarms[0].trigger_minutes_before, 15);
        assert_eq!(event.alarms[0].action, AlarmAction::Display);
        assert_eq!(event.alarms[0].description.as_deref(), Some("Reminder"));
    }

    #[test]
    fn test_parse_event_with_attendees() {
        let ical = "\
BEGIN:VEVENT\r\n\
UID:attendee-001\r\n\
DTSTART:20260315T100000Z\r\n\
SUMMARY:Meeting\r\n\
ATTENDEE;CN=\"John Doe\";ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED:mailto:john@example.com\r\n\
ATTENDEE;CN=\"Jane\";ROLE=OPT-PARTICIPANT;PARTSTAT=TENTATIVE:mailto:jane@example.com\r\n\
END:VEVENT";

        let event = parse_vevent(ical).unwrap();
        assert_eq!(event.attendees.len(), 2);
        assert_eq!(event.attendees[0].email, "john@example.com");
        assert_eq!(event.attendees[0].name.as_deref(), Some("John Doe"));
        assert_eq!(event.attendees[0].status, AttendeeStatus::Accepted);
        assert_eq!(event.attendees[1].role, AttendeeRole::Optional);
        assert_eq!(event.attendees[1].status, AttendeeStatus::Tentative);
    }

    #[test]
    fn test_parse_event_with_rrule() {
        let ical = "\
BEGIN:VEVENT\r\n\
UID:rrule-001\r\n\
DTSTART:20260315T100000Z\r\n\
SUMMARY:Weekly Standup\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20260601T000000Z\r\n\
EXDATE:20260318T100000Z\r\n\
END:VEVENT";

        let event = parse_vevent(ical).unwrap();
        assert_eq!(event.rrule.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20260601T000000Z"));
        assert_eq!(event.exdates.len(), 1);
    }

    #[test]
    fn test_parse_vcalendar_multiple_events() {
        let ical = "\
BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:multi-001\r\n\
DTSTART:20260315T100000Z\r\n\
SUMMARY:First\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:multi-002\r\n\
DTSTART:20260316T100000Z\r\n\
SUMMARY:Second\r\n\
END:VEVENT\r\n\
END:VCALENDAR";

        let events = parse_vcalendar(ical).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].summary, "First");
        assert_eq!(events[1].summary, "Second");
    }

    #[test]
    fn test_parse_trigger_durations() {
        assert_eq!(parse_trigger("-PT15M"), 15);
        assert_eq!(parse_trigger("-PT1H"), 60);
        assert_eq!(parse_trigger("-PT1H30M"), 90);
        assert_eq!(parse_trigger("-P1D"), 1440);
        assert_eq!(parse_trigger("-P1W"), 10080);
    }

    #[test]
    fn test_parse_datetime_formats() {
        // UTC datetime
        let (ms, all_day) = parse_datetime("DTSTART", "20260315T100000Z");
        assert!(!all_day);
        assert!(ms > 0);

        // Date only
        let (ms2, all_day2) = parse_datetime("DTSTART;VALUE=DATE", "20260315");
        assert!(all_day2);
        assert!(ms2 > 0);
    }
}
