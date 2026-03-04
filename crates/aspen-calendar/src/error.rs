//! Calendar error types.

use snafu::Snafu;

/// Errors from calendar operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CalendarError {
    /// Failed to parse iCalendar data.
    #[snafu(display("failed to parse iCalendar: {reason}"))]
    ParseIcal { reason: String },

    /// Invalid recurrence rule.
    #[snafu(display("invalid RRULE: {reason}"))]
    InvalidRrule { reason: String },

    /// Event not found.
    #[snafu(display("event not found: {id}"))]
    NotFound { id: String },

    /// Calendar not found.
    #[snafu(display("calendar not found: {id}"))]
    CalendarNotFound { id: String },

    /// Invalid input.
    #[snafu(display("invalid input: {reason}"))]
    InvalidInput { reason: String },

    /// Storage error.
    #[snafu(display("storage error: {reason}"))]
    StorageError { reason: String },
}
