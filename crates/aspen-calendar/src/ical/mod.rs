//! iCalendar parser, serializer, and RRULE expansion (RFC 5545).

pub mod parse;
pub mod rrule;
pub mod serialize;

pub use parse::parse_vcalendar;
pub use parse::parse_vevent;
pub use rrule::RecurrenceInstance;
pub use rrule::expand_rrule;
pub use serialize::serialize_vcalendar;
pub use serialize::serialize_vevent;
