//! Contacts operation types.
//!
//! Response types for distributed contacts operations including address books,
//! contacts, groups, and vCard import/export.

use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

/// Lightweight contact summary for listing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSummary {
    /// Unique contact ID.
    pub id: String,
    /// Display name.
    pub display_name: String,
    /// Primary email address.
    pub primary_email: Option<String>,
    /// Primary phone number.
    pub primary_phone: Option<String>,
}

/// Address book operation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsBookResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Address book ID.
    pub book_id: Option<String>,
    /// Address book name.
    pub name: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Address book listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsBookListResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of address books.
    pub books: Vec<ContactsBookInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Address book information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsBookInfo {
    /// Unique address book ID.
    pub id: String,
    /// Address book name.
    pub name: String,
    /// Address book description.
    pub description: Option<String>,
    /// Number of contacts in this book.
    pub contact_count: u32,
}

/// Contact operation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Contact ID.
    pub contact_id: Option<String>,
    /// Full vCard data.
    pub vcard_data: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Contact listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsListResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of contact summaries.
    pub contacts: Vec<ContactSummary>,
    /// Continuation token for paginated results.
    pub continuation_token: Option<String>,
    /// Total number of contacts.
    pub total: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Contact search response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsSearchResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of matching contact summaries.
    pub contacts: Vec<ContactSummary>,
    /// Total number of matches.
    pub total: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Contact group operation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsGroupResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Group ID.
    pub group_id: Option<String>,
    /// Group name.
    pub name: Option<String>,
    /// List of contact IDs in this group.
    pub member_ids: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Contact group listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsGroupListResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of contact groups.
    pub groups: Vec<ContactsGroupInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Contact group information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsGroupInfo {
    /// Unique group ID.
    pub id: String,
    /// Group name.
    pub name: String,
    /// Number of members in this group.
    pub member_count: u32,
}

/// Contacts export response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsExportResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Exported vCard data (multiple contacts concatenated).
    pub vcard_data: Option<String>,
    /// Number of contacts exported.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}
