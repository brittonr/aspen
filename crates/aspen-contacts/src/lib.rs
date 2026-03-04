//! Contacts domain logic for Aspen.
//!
//! Provides contact management with vCard 4.0 (RFC 6350) support:
//! - Address book CRUD
//! - Contact CRUD with vCard parsing/serialization
//! - Contact groups
//! - Search by name, email, phone
//! - Bulk import/export in vCard format
//!
//! All state is stored in the distributed KV store with
//! namespaced key prefixes (`contacts:book:`, `contacts:entry:`, `contacts:group:`).

pub mod error;
pub mod store;
pub mod types;
pub mod vcard;

pub use error::ContactsError;
pub use store::ContactStore;
pub use types::Contact;
pub use types::ContactAddress;
pub use types::ContactBook;
pub use types::ContactEmail;
pub use types::ContactGroup;
pub use types::ContactPhone;
pub use vcard::parse_vcard;
pub use vcard::parse_vcards;
pub use vcard::serialize_vcard;
pub use vcard::serialize_vcards;
