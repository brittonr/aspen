//! Contact data model types.

use serde::Deserialize;
use serde::Serialize;

/// A contact entry with vCard fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Unique contact ID.
    pub id: String,
    /// Address book this contact belongs to.
    pub book_id: String,
    /// vCard UID.
    pub uid: String,
    /// Formatted display name (FN).
    pub display_name: String,
    /// Family name (N component).
    pub family_name: Option<String>,
    /// Given name (N component).
    pub given_name: Option<String>,
    /// Email addresses.
    pub emails: Vec<ContactEmail>,
    /// Phone numbers.
    pub phones: Vec<ContactPhone>,
    /// Postal addresses.
    pub addresses: Vec<ContactAddress>,
    /// Organization name.
    pub organization: Option<String>,
    /// Job title.
    pub title: Option<String>,
    /// Birthday (ISO 8601 date, e.g. "1990-01-15").
    pub birthday: Option<String>,
    /// Notes.
    pub notes: Option<String>,
    /// Blob hash for photo.
    pub photo_blob_hash: Option<String>,
    /// Categories/tags.
    pub categories: Vec<String>,
    /// URL.
    pub url: Option<String>,
    /// Custom X- properties.
    pub custom_fields: Vec<(String, String)>,
    /// Creation timestamp (Unix ms).
    pub created_at_ms: u64,
    /// Last modified timestamp (Unix ms).
    pub updated_at_ms: u64,
}

/// Email address entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactEmail {
    /// Email address.
    pub address: String,
    /// Label ("work", "home", "other").
    pub label: Option<String>,
    /// Whether this is the primary email.
    pub is_primary: bool,
}

/// Phone number entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactPhone {
    /// Phone number.
    pub number: String,
    /// Label ("work", "home", "cell").
    pub label: Option<String>,
    /// Whether this is the primary phone.
    pub is_primary: bool,
}

/// Postal address entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactAddress {
    /// Street address.
    pub street: Option<String>,
    /// City.
    pub city: Option<String>,
    /// State or province.
    pub region: Option<String>,
    /// Postal/ZIP code.
    pub postal_code: Option<String>,
    /// Country.
    pub country: Option<String>,
    /// Label ("work", "home").
    pub label: Option<String>,
}

/// An address book container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactBook {
    /// Unique book ID.
    pub id: String,
    /// Book name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Owner identifier.
    pub owner: Option<String>,
    /// Creation timestamp (Unix ms).
    pub created_at_ms: u64,
}

/// A contact group within an address book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactGroup {
    /// Unique group ID.
    pub id: String,
    /// Address book this group belongs to.
    pub book_id: String,
    /// Group name.
    pub name: String,
    /// Member contact IDs.
    pub member_ids: Vec<String>,
}
