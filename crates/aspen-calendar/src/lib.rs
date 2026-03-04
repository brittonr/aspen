//! Calendar domain logic for Aspen.
//!
//! Provides calendar and event management with iCalendar (RFC 5545) support:
//! - Calendar CRUD
//! - Event CRUD with iCalendar parsing/serialization
//! - Recurring event expansion (RRULE)
//! - Free/busy queries
//! - Bulk import/export in iCalendar format
//!
//! All state is stored in the distributed KV store with
//! namespaced key prefixes (`calendar:cal:`, `calendar:event:`).

pub mod error;
pub mod ical;
pub mod types;

pub use error::CalendarError;
pub use ical::RecurrenceInstance;
pub use ical::expand_rrule;
pub use ical::parse_vcalendar;
pub use ical::parse_vevent;
pub use ical::serialize_vcalendar;
pub use ical::serialize_vevent;
pub use types::AlarmAction;
pub use types::AttendeeRole;
pub use types::AttendeeStatus;
pub use types::CalendarEvent;
pub use types::CalendarMeta;
pub use types::EventAlarm;
pub use types::EventAttendee;
pub use types::EventStatus;
