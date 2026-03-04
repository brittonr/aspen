//! Contacts error types.

use snafu::Snafu;

/// Errors from contacts operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ContactsError {
    /// Failed to parse vCard data.
    #[snafu(display("failed to parse vCard: {reason}"))]
    ParseVcard { reason: String },

    /// Contact not found.
    #[snafu(display("contact not found: {id}"))]
    NotFound { id: String },

    /// Address book not found.
    #[snafu(display("address book not found: {id}"))]
    BookNotFound { id: String },

    /// Group not found.
    #[snafu(display("group not found: {id}"))]
    GroupNotFound { id: String },

    /// Invalid input.
    #[snafu(display("invalid input: {reason}"))]
    InvalidInput { reason: String },

    /// Storage error.
    #[snafu(display("storage error: {reason}"))]
    StorageError { reason: String },
}
