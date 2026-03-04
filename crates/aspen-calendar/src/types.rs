//! Calendar data model types.

use serde::Deserialize;
use serde::Serialize;

/// A calendar container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarMeta {
    /// Unique calendar ID.
    pub id: String,
    /// Calendar name.
    pub name: String,
    /// Display color (hex format).
    pub color: Option<String>,
    /// IANA timezone.
    pub timezone: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Owner identifier.
    pub owner: Option<String>,
    /// Creation timestamp (Unix ms).
    pub created_at_ms: u64,
}

/// A calendar event with iCalendar fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Unique event ID.
    pub id: String,
    /// Calendar this event belongs to.
    pub calendar_id: String,
    /// iCalendar UID.
    pub uid: String,
    /// Event summary/title (SUMMARY).
    pub summary: String,
    /// Description (DESCRIPTION).
    pub description: Option<String>,
    /// Location (LOCATION).
    pub location: Option<String>,
    /// Start time as Unix ms (DTSTART).
    pub dtstart_ms: u64,
    /// End time as Unix ms (DTEND).
    pub dtend_ms: Option<u64>,
    /// Whether this is an all-day event (DATE vs DATE-TIME).
    pub is_all_day: bool,
    /// Recurrence rule string (RRULE).
    pub rrule: Option<String>,
    /// Excluded dates as Unix ms (EXDATE).
    pub exdates: Vec<u64>,
    /// Attendees.
    pub attendees: Vec<EventAttendee>,
    /// Alarms/reminders.
    pub alarms: Vec<EventAlarm>,
    /// Event status.
    pub status: EventStatus,
    /// Categories/tags.
    pub categories: Vec<String>,
    /// URL.
    pub url: Option<String>,
    /// Creation timestamp (Unix ms).
    pub created_at_ms: u64,
    /// Last modified timestamp (Unix ms).
    pub updated_at_ms: u64,
    /// iCalendar SEQUENCE number.
    pub sequence: u32,
}

/// Event status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventStatus {
    /// Confirmed event.
    Confirmed,
    /// Tentative event.
    Tentative,
    /// Cancelled event.
    Cancelled,
}

impl Default for EventStatus {
    fn default() -> Self {
        Self::Confirmed
    }
}

/// Event attendee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAttendee {
    /// Attendee email.
    pub email: String,
    /// Attendee display name.
    pub name: Option<String>,
    /// Attendee role.
    pub role: AttendeeRole,
    /// Participation status.
    pub status: AttendeeStatus,
}

/// Attendee role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttendeeRole {
    /// Required participant.
    Required,
    /// Optional participant.
    Optional,
    /// Meeting chair.
    Chair,
}

impl Default for AttendeeRole {
    fn default() -> Self {
        Self::Required
    }
}

/// Attendee participation status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttendeeStatus {
    /// Needs to respond.
    NeedsAction,
    /// Accepted.
    Accepted,
    /// Declined.
    Declined,
    /// Tentatively accepted.
    Tentative,
}

impl Default for AttendeeStatus {
    fn default() -> Self {
        Self::NeedsAction
    }
}

/// Event alarm/reminder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAlarm {
    /// Alarm ID.
    pub id: String,
    /// Trigger: minutes before event start (negative = before).
    pub trigger_minutes_before: i32,
    /// Alarm action type.
    pub action: AlarmAction,
    /// Alarm description text.
    pub description: Option<String>,
}

/// Alarm action type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlarmAction {
    /// Display a notification.
    Display,
    /// Send an email.
    Email,
}

impl Default for AlarmAction {
    fn default() -> Self {
        Self::Display
    }
}
