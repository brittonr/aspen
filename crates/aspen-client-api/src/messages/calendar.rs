//! Calendar operation types.
//!
//! Response types for distributed calendar operations including calendars,
//! events, recurrence expansion, free/busy queries, and iCalendar import/export.

use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

/// Lightweight event summary for listing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    /// Unique event ID.
    pub id: String,
    /// Calendar ID this event belongs to.
    pub calendar_id: String,
    /// Event summary/title.
    pub summary: String,
    /// Start time (Unix timestamp in milliseconds).
    pub dtstart_ms: u64,
    /// End time (Unix timestamp in milliseconds).
    pub dtend_ms: Option<u64>,
    /// Whether this is an all-day event.
    pub is_all_day: bool,
    /// Event location.
    pub location: Option<String>,
}

/// Calendar operation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Calendar ID.
    pub calendar_id: Option<String>,
    /// Calendar name.
    pub name: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Calendar listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarListResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of calendars.
    pub calendars: Vec<CalendarInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Calendar information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarInfo {
    /// Unique calendar ID.
    pub id: String,
    /// Calendar name.
    pub name: String,
    /// Calendar color (hex format).
    pub color: Option<String>,
    /// Calendar timezone.
    pub timezone: Option<String>,
    /// Number of events in this calendar.
    pub event_count: u32,
}

/// Calendar event operation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEventResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Event ID.
    pub event_id: Option<String>,
    /// Full iCalendar data.
    pub ical_data: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Calendar event listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarListEventsResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of event summaries.
    pub events: Vec<EventSummary>,
    /// Continuation token for paginated results.
    pub continuation_token: Option<String>,
    /// Total number of events.
    pub total: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Calendar event search response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarSearchResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of matching event summaries.
    pub events: Vec<EventSummary>,
    /// Total number of matches.
    pub total: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Free/busy query response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarFreeBusyResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of busy time periods.
    pub busy_periods: Vec<BusyPeriod>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Busy time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusyPeriod {
    /// Start time (Unix timestamp in milliseconds).
    pub start_ms: u64,
    /// End time (Unix timestamp in milliseconds).
    pub end_ms: u64,
    /// Event summary (if available).
    pub summary: Option<String>,
}

/// Recurrence expansion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarExpandResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of expanded event instances.
    pub instances: Vec<EventInstance>,
    /// Total number of instances.
    pub total: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Individual recurring event instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInstance {
    /// Instance start time (Unix timestamp in milliseconds).
    pub dtstart_ms: u64,
    /// Instance end time (Unix timestamp in milliseconds).
    pub dtend_ms: Option<u64>,
    /// Whether this is an exception to the recurrence rule.
    pub is_exception: bool,
}

/// Calendar export response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarExportResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Exported iCalendar data (multiple events in VCALENDAR format).
    pub ical_data: Option<String>,
    /// Number of events exported.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}
