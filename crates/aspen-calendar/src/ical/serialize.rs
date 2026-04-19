//! iCalendar serializer.
//!
//! Serializes CalendarEvent structs to iCalendar text format (RFC 5545).

use crate::types::AlarmAction;
use crate::types::AttendeeRole;
use crate::types::AttendeeStatus;
use crate::types::CalendarEvent;
use crate::types::EventAlarm;
use crate::types::EventAttendee;
use crate::types::EventStatus;

const SECONDS_PER_DAY_I64: i64 = 86_400;

/// Serialize a CalendarEvent to iCalendar VEVENT format.
pub fn serialize_vevent(event: &CalendarEvent) -> String {
    debug_assert!(!event.summary.is_empty(), "VEVENT summary should not be empty when serializing");
    debug_assert!(event.dtend_ms.is_none() || event.dtend_ms >= Some(event.dtstart_ms));
    let mut lines = Vec::with_capacity(20);

    lines.push("BEGIN:VEVENT".to_string());
    if !event.uid.is_empty() {
        lines.push(format!("UID:{}", event.uid));
    }
    lines.push(format!("DTSTART:{}", unix_ms_to_ical(event.dtstart_ms, event.is_all_day)));
    if let Some(end_ms) = event.dtend_ms {
        lines.push(format!("DTEND:{}", unix_ms_to_ical(end_ms, event.is_all_day)));
    }
    lines.push(format!("SUMMARY:{}", event.summary));

    push_optional_line(&mut lines, "DESCRIPTION", event.description.as_deref());
    push_optional_line(&mut lines, "LOCATION", event.location.as_deref());
    push_optional_line(&mut lines, "RRULE", event.rrule.as_deref());
    for exdate_ms in &event.exdates {
        lines.push(format!("EXDATE:{}", unix_ms_to_ical(*exdate_ms, false)));
    }
    push_status_line(&mut lines, &event.status);
    push_categories_line(&mut lines, &event.categories);
    push_optional_line(&mut lines, "URL", event.url.as_deref());
    if event.sequence > 0 {
        lines.push(format!("SEQUENCE:{}", event.sequence));
    }

    for attendee in &event.attendees {
        lines.push(serialize_attendee(attendee));
    }
    for alarm in &event.alarms {
        push_alarm_lines(&mut lines, alarm);
    }

    lines.push("END:VEVENT".to_string());
    lines.join("\r\n") + "\r\n"
}

fn push_optional_line(lines: &mut Vec<String>, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{name}:{value}"));
    }
}

fn push_status_line(lines: &mut Vec<String>, status: &EventStatus) {
    match status {
        EventStatus::Tentative => lines.push("STATUS:TENTATIVE".to_string()),
        EventStatus::Cancelled => lines.push("STATUS:CANCELLED".to_string()),
        EventStatus::Confirmed => {}
    }
}

fn push_categories_line(lines: &mut Vec<String>, categories: &[String]) {
    if !categories.is_empty() {
        lines.push(format!("CATEGORIES:{}", categories.join(",")));
    }
}

fn serialize_attendee(attendee: &EventAttendee) -> String {
    debug_assert!(!attendee.email.is_empty(), "attendee email must not be empty");
    let mut params = Vec::with_capacity(3);
    if let Some(ref name) = attendee.name {
        params.push(format!("CN=\"{name}\""));
    }
    let role_str = match attendee.role {
        AttendeeRole::Required => "REQ-PARTICIPANT",
        AttendeeRole::Optional => "OPT-PARTICIPANT",
        AttendeeRole::Chair => "CHAIR",
    };
    params.push(format!("ROLE={role_str}"));
    let status_str = match attendee.status {
        AttendeeStatus::Accepted => "ACCEPTED",
        AttendeeStatus::Declined => "DECLINED",
        AttendeeStatus::Tentative => "TENTATIVE",
        AttendeeStatus::NeedsAction => "NEEDS-ACTION",
    };
    params.push(format!("PARTSTAT={status_str}"));
    debug_assert!(params.len() >= 2, "attendee serialization must include role and participation status");
    let param_str = params.join(";");
    format!("ATTENDEE;{param_str}:mailto:{}", attendee.email)
}

fn push_alarm_lines(lines: &mut Vec<String>, alarm: &EventAlarm) {
    lines.push("BEGIN:VALARM".to_string());
    let trigger = format_trigger(alarm.trigger_minutes_before);
    lines.push(format!("TRIGGER:{trigger}"));
    let action = match alarm.action {
        AlarmAction::Display => "DISPLAY",
        AlarmAction::Email => "EMAIL",
    };
    lines.push(format!("ACTION:{action}"));
    push_optional_line(lines, "DESCRIPTION", alarm.description.as_deref());
    lines.push("END:VALARM".to_string());
}

/// Wrap events in a VCALENDAR block.
pub fn serialize_vcalendar(events: &[CalendarEvent]) -> String {
    let mut result = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Aspen//Calendar//EN\r\n");
    for event in events {
        result.push_str(&serialize_vevent(event));
    }
    result.push_str("END:VCALENDAR\r\n");
    result
}

/// Convert Unix ms to iCalendar datetime string.
fn unix_ms_to_ical(ms: u64, is_all_day: bool) -> String {
    let secs = (ms / 1000) as i64;
    let (year, month, day, hour, min, sec) = unix_secs_to_date(secs);

    if is_all_day {
        format!("{year:04}{month:02}{day:02}")
    } else {
        format!("{year:04}{month:02}{day:02}T{hour:02}{min:02}{sec:02}Z")
    }
}

/// Convert Unix seconds to (year, month, day, hour, min, sec).
///
/// Simplified Gregorian calendar. Accurate for 1970–2099.
fn unix_secs_to_date(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    debug_assert!(secs >= 0, "serializer expects non-negative unix seconds");
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

fn saturating_u32_from_i64(value: i64) -> u32 {
    if value <= 0 {
        0
    } else {
        u32::try_from(value).unwrap_or(u32::MAX)
    }
}

/// Format trigger duration (minutes before → iCal TRIGGER string).
fn format_trigger(minutes_before: i32) -> String {
    if minutes_before <= 0 {
        return format!("PT{}M", -minutes_before);
    }

    let total_mins = minutes_before;
    if total_mins >= 1440 && total_mins % 1440 == 0 {
        format!("-P{}D", total_mins / 1440)
    } else if total_mins >= 60 && total_mins % 60 == 0 {
        format!("-PT{}H", total_mins / 60)
    } else {
        format!("-PT{total_mins}M")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ical::parse::parse_vevent;

    #[test]
    fn test_serialize_roundtrip() {
        let ical_input = "\
BEGIN:VEVENT\r\n\
UID:roundtrip-001\r\n\
DTSTART:20260315T100000Z\r\n\
DTEND:20260315T110000Z\r\n\
SUMMARY:Team Meeting\r\n\
LOCATION:Room 42\r\n\
END:VEVENT";

        let event = parse_vevent(ical_input).unwrap();
        let serialized = serialize_vevent(&event);
        let reparsed = parse_vevent(&serialized).unwrap();

        assert_eq!(event.summary, reparsed.summary);
        assert_eq!(event.uid, reparsed.uid);
        assert_eq!(event.dtstart_ms, reparsed.dtstart_ms);
        assert_eq!(event.dtend_ms, reparsed.dtend_ms);
        assert_eq!(event.location, reparsed.location);
    }

    #[test]
    fn test_unix_ms_to_ical_datetime() {
        // Roundtrip: parse a known date, serialize, verify format
        let ical = unix_ms_to_ical(0, false);
        assert_eq!(ical, "19700101T000000Z"); // Unix epoch

        // Test that serialized dates match expected format
        let ms = 86_400_000; // 1 day after epoch
        let ical = unix_ms_to_ical(ms, false);
        assert_eq!(ical, "19700102T000000Z");
    }

    #[test]
    fn test_unix_ms_to_ical_date() {
        let ical = unix_ms_to_ical(1773720000000, true);
        // Should produce a date-only string
        assert_eq!(ical.len(), 8); // YYYYMMDD
    }

    #[test]
    fn test_format_trigger() {
        assert_eq!(format_trigger(15), "-PT15M");
        assert_eq!(format_trigger(60), "-PT1H");
        assert_eq!(format_trigger(1440), "-P1D");
    }
}
