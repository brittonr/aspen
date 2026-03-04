//! iCalendar serializer.
//!
//! Serializes CalendarEvent structs to iCalendar text format (RFC 5545).

use crate::types::AlarmAction;
use crate::types::AttendeeRole;
use crate::types::AttendeeStatus;
use crate::types::CalendarEvent;
use crate::types::EventStatus;

/// Serialize a CalendarEvent to iCalendar VEVENT format.
pub fn serialize_vevent(event: &CalendarEvent) -> String {
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

    if let Some(ref desc) = event.description {
        lines.push(format!("DESCRIPTION:{desc}"));
    }
    if let Some(ref loc) = event.location {
        lines.push(format!("LOCATION:{loc}"));
    }
    if let Some(ref rrule) = event.rrule {
        lines.push(format!("RRULE:{rrule}"));
    }
    for exdate_ms in &event.exdates {
        lines.push(format!("EXDATE:{}", unix_ms_to_ical(*exdate_ms, false)));
    }

    match event.status {
        EventStatus::Tentative => lines.push("STATUS:TENTATIVE".to_string()),
        EventStatus::Cancelled => lines.push("STATUS:CANCELLED".to_string()),
        EventStatus::Confirmed => {} // default, omit
    }

    if !event.categories.is_empty() {
        lines.push(format!("CATEGORIES:{}", event.categories.join(",")));
    }
    if let Some(ref url) = event.url {
        lines.push(format!("URL:{url}"));
    }
    if event.sequence > 0 {
        lines.push(format!("SEQUENCE:{}", event.sequence));
    }

    for attendee in &event.attendees {
        let mut params = Vec::new();
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
        let param_str = params.join(";");
        lines.push(format!("ATTENDEE;{param_str}:mailto:{}", attendee.email));
    }

    for alarm in &event.alarms {
        lines.push("BEGIN:VALARM".to_string());
        let trigger = format_trigger(alarm.trigger_minutes_before);
        lines.push(format!("TRIGGER:{trigger}"));
        let action = match alarm.action {
            AlarmAction::Display => "DISPLAY",
            AlarmAction::Email => "EMAIL",
        };
        lines.push(format!("ACTION:{action}"));
        if let Some(ref desc) = alarm.description {
            lines.push(format!("DESCRIPTION:{desc}"));
        }
        lines.push("END:VALARM".to_string());
    }

    lines.push("END:VEVENT".to_string());

    lines.join("\r\n") + "\r\n"
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
    let sec_of_day = secs.rem_euclid(86400);
    let hour = (sec_of_day / 3600) as u32;
    let min = ((sec_of_day % 3600) / 60) as u32;
    let sec = (sec_of_day % 60) as u32;

    let mut days = secs / 86400;
    if secs < 0 && secs % 86400 != 0 {
        days -= 1;
    }
    // days from 1970-01-01
    let days = days + 719468; // shift to 0000-03-01 epoch

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
