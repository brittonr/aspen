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

/// Parse a single VEVENT from iCalendar text.
///
/// Returns a partially-filled CalendarEvent (caller must set `id`,
/// `calendar_id`, `created_at_ms`, `updated_at_ms`).
pub fn parse_vevent(input: &str) -> Result<CalendarEvent, CalendarError> {
    let lines = unfold_lines(input);
    let mut in_event = false;
    let mut in_alarm = false;
    let mut event = CalendarEvent {
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
        exdates: Vec::new(),
        attendees: Vec::new(),
        alarms: Vec::new(),
        status: EventStatus::Confirmed,
        categories: Vec::new(),
        url: None,
        created_at_ms: 0,
        updated_at_ms: 0,
        sequence: 0,
    };

    let mut alarm_trigger: Option<i32> = None;
    let mut alarm_action: AlarmAction = AlarmAction::Display;
    let mut alarm_desc: Option<String> = None;
    let mut alarm_count: u32 = 0;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("BEGIN:VEVENT") {
            in_event = true;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VEVENT") {
            // Flush any pending alarm
            if in_alarm {
                flush_alarm(&mut event, alarm_trigger.take(), &alarm_action, alarm_desc.take(), &mut alarm_count);
            }
            break;
        }
        if trimmed.eq_ignore_ascii_case("BEGIN:VALARM") {
            in_alarm = true;
            alarm_trigger = None;
            alarm_action = AlarmAction::Display;
            alarm_desc = None;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VALARM") {
            flush_alarm(&mut event, alarm_trigger.take(), &alarm_action, alarm_desc.take(), &mut alarm_count);
            in_alarm = false;
            continue;
        }

        if !in_event {
            // Skip VCALENDAR-level properties
            continue;
        }

        let (name, _params, value) = parse_property(trimmed);
        let name_upper = name.to_ascii_uppercase();

        if in_alarm {
            match name_upper.as_str() {
                "TRIGGER" => alarm_trigger = Some(parse_trigger(value)),
                "ACTION" => {
                    alarm_action = if value.eq_ignore_ascii_case("EMAIL") {
                        AlarmAction::Email
                    } else {
                        AlarmAction::Display
                    };
                }
                "DESCRIPTION" => alarm_desc = Some(value.to_string()),
                _ => {}
            }
            continue;
        }

        match name_upper.as_str() {
            "UID" => event.uid = value.to_string(),
            "SUMMARY" => event.summary = value.to_string(),
            "DESCRIPTION" => event.description = Some(value.to_string()),
            "LOCATION" => event.location = Some(value.to_string()),
            "DTSTART" => {
                let (ms, is_all_day) = parse_datetime(name, value);
                event.dtstart_ms = ms;
                event.is_all_day = is_all_day;
            }
            "DTEND" => {
                let (ms, _) = parse_datetime(name, value);
                event.dtend_ms = Some(ms);
            }
            "RRULE" => event.rrule = Some(value.to_string()),
            "EXDATE" => {
                for date_str in value.split(',') {
                    let (ms, _) = parse_datetime("EXDATE", date_str.trim());
                    event.exdates.push(ms);
                }
            }
            "ATTENDEE" => {
                if let Some(attendee) = parse_attendee(trimmed) {
                    event.attendees.push(attendee);
                }
            }
            "STATUS" => {
                event.status = match value.to_ascii_uppercase().as_str() {
                    "TENTATIVE" => EventStatus::Tentative,
                    "CANCELLED" => EventStatus::Cancelled,
                    _ => EventStatus::Confirmed,
                };
            }
            "CATEGORIES" => {
                event.categories = value.split(',').map(|s| s.trim().to_string()).collect();
            }
            "URL" => event.url = Some(value.to_string()),
            "SEQUENCE" => {
                event.sequence = value.parse::<u32>().unwrap_or(0);
            }
            _ => {}
        }
    }

    if event.summary.is_empty() && event.uid.is_empty() {
        return Err(CalendarError::ParseIcal {
            reason: "missing SUMMARY or UID property".to_string(),
        });
    }

    Ok(event)
}

/// Parse multiple VEVENTs from a VCALENDAR block.
pub fn parse_vcalendar(input: &str) -> Result<Vec<CalendarEvent>, CalendarError> {
    let mut events = Vec::new();
    let mut current = String::new();
    let mut in_event = false;
    let mut depth: u32 = 0;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("BEGIN:VEVENT") && !in_event {
            in_event = true;
            depth = 0;
            current.clear();
        }

        if in_event {
            current.push_str(line);
            current.push('\n');

            // Track nested BEGIN/END for VALARM etc.
            if trimmed.to_ascii_uppercase().starts_with("BEGIN:") {
                depth = depth.saturating_add(1);
            }
            if trimmed.to_ascii_uppercase().starts_with("END:") {
                depth = depth.saturating_sub(1);
            }

            if trimmed.eq_ignore_ascii_case("END:VEVENT") && depth == 0 {
                in_event = false;
                events.push(parse_vevent(&current)?);
            }
        }
    }

    Ok(events)
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Unfold continuation lines (RFC 5545 §3.1).
fn unfold_lines(input: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for line in input.lines() {
        if (line.starts_with(' ') || line.starts_with('\t')) && !result.is_empty() {
            let last = result.last_mut().expect("checked non-empty");
            last.push_str(&line[1..]);
        } else {
            result.push(line.to_string());
        }
    }
    result
}

/// Parse a property line into (name, params, value).
fn parse_property(line: &str) -> (&str, Vec<(&str, &str)>, &str) {
    let colon_pos = find_value_colon(line);
    let (name_params, value) = if let Some(pos) = colon_pos {
        (&line[..pos], &line[pos + 1..])
    } else {
        (line, "")
    };

    let mut params = Vec::new();
    let name;
    if let Some(semi_pos) = name_params.find(';') {
        name = &name_params[..semi_pos];
        for param in name_params[semi_pos + 1..].split(';') {
            if let Some(eq_pos) = param.find('=') {
                params.push((&param[..eq_pos], &param[eq_pos + 1..]));
            }
        }
    } else {
        name = name_params;
    }

    (name, params, value)
}

fn find_value_colon(line: &str) -> Option<usize> {
    let mut in_quotes = false;
    for (i, ch) in line.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ':' if !in_quotes => return Some(i),
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
fn parse_datetime(name: &str, value: &str) -> (u64, bool) {
    let clean = value.trim().trim_end_matches('Z');

    // Check for VALUE=DATE parameter in the name part
    let is_date_only = clean.len() == 8 || name.contains("VALUE=DATE");

    if is_date_only && clean.len() >= 8 {
        // DATE format: YYYYMMDD
        let year = clean[0..4].parse::<i64>().unwrap_or(2000);
        let month = clean[4..6].parse::<u32>().unwrap_or(1);
        let day = clean[6..8].parse::<u32>().unwrap_or(1);
        let ms = date_to_unix_ms(year, month, day, 0, 0, 0);
        return (ms, true);
    }

    if clean.len() >= 15 {
        // DATETIME format: YYYYMMDDTHHMMSS
        let year = clean[0..4].parse::<i64>().unwrap_or(2000);
        let month = clean[4..6].parse::<u32>().unwrap_or(1);
        let day = clean[6..8].parse::<u32>().unwrap_or(1);
        let hour = clean[9..11].parse::<u32>().unwrap_or(0);
        let min = clean[11..13].parse::<u32>().unwrap_or(0);
        let sec = clean[13..15].parse::<u32>().unwrap_or(0);
        let ms = date_to_unix_ms(year, month, day, hour, min, sec);
        return (ms, false);
    }

    (0, false)
}

/// Simple date → Unix timestamp in milliseconds.
///
/// Handles years 1970–2099. Not a full calendar implementation.
fn date_to_unix_ms(year: i64, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> u64 {
    // Days from epoch (1970-01-01) using a simplified calculation.
    // Accurate for dates 1970-2099.
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

/// Parse TRIGGER duration (e.g., "-PT15M" → 15, "-PT1H" → 60).
///
/// Returns minutes before event (always positive).
fn parse_trigger(value: &str) -> i32 {
    let is_negative = value.starts_with('-');
    let clean = value.trim_start_matches('-').trim_start_matches('+');
    let clean = clean.trim_start_matches("PT").trim_start_matches("P");

    let mut minutes: i32 = 0;
    let mut num_buf = String::new();

    for ch in clean.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            let n: i32 = num_buf.parse().unwrap_or(0);
            num_buf.clear();
            match ch {
                'W' => minutes += n * 7 * 24 * 60,
                'D' => minutes += n * 24 * 60,
                'H' => minutes += n * 60,
                'M' => minutes += n,
                'S' => minutes += n / 60, // round down
                _ => {}
            }
        }
    }

    if is_negative { minutes } else { -minutes }
}

/// Parse ATTENDEE property line.
fn parse_attendee(line: &str) -> Option<EventAttendee> {
    let (_, params, value) = parse_property(line);

    // Value is typically "mailto:user@example.com"
    let email = value.strip_prefix("mailto:").unwrap_or(value).to_string();
    if email.is_empty() {
        return None;
    }

    let mut name = None;
    let mut role = AttendeeRole::Required;
    let mut status = AttendeeStatus::NeedsAction;

    for (k, v) in &params {
        match k.to_ascii_uppercase().as_str() {
            "CN" => name = Some(v.trim_matches('"').to_string()),
            "ROLE" => {
                role = match v.to_ascii_uppercase().as_str() {
                    "OPT-PARTICIPANT" => AttendeeRole::Optional,
                    "CHAIR" => AttendeeRole::Chair,
                    _ => AttendeeRole::Required,
                };
            }
            "PARTSTAT" => {
                status = match v.to_ascii_uppercase().as_str() {
                    "ACCEPTED" => AttendeeStatus::Accepted,
                    "DECLINED" => AttendeeStatus::Declined,
                    "TENTATIVE" => AttendeeStatus::Tentative,
                    _ => AttendeeStatus::NeedsAction,
                };
            }
            _ => {}
        }
    }

    Some(EventAttendee {
        email,
        name,
        role,
        status,
    })
}

/// Flush a pending alarm into the event.
fn flush_alarm(
    event: &mut CalendarEvent,
    trigger: Option<i32>,
    action: &AlarmAction,
    description: Option<String>,
    count: &mut u32,
) {
    if let Some(trigger_mins) = trigger {
        *count = count.saturating_add(1);
        event.alarms.push(EventAlarm {
            id: format!("alarm-{}", count),
            trigger_minutes_before: trigger_mins,
            action: action.clone(),
            description,
        });
    }
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
