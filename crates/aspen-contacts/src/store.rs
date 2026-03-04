//! Contact store — CRUD operations backed by `KeyValueStore`.
//!
//! All contacts, books, and groups are stored as JSON in the distributed KV store
//! with namespaced key prefixes:
//! - `contacts:book:{book_id}` — address books
//! - `contacts:entry:{book_id}:{contact_id}` — contacts
//! - `contacts:group:{book_id}:{group_id}` — contact groups

use std::sync::Arc;

use aspen_kv_types::DeleteRequest;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use crate::ContactBook;
use crate::ContactGroup;
use crate::ContactsError;
use crate::types::Contact;
use crate::vcard::parse_vcard;
use crate::vcard::parse_vcards;
use crate::vcard::serialize_vcards;

/// Key prefix for address books.
const BOOK_PREFIX: &str = "contacts:book:";
/// Key prefix for contact entries.
const ENTRY_PREFIX: &str = "contacts:entry:";
/// Key prefix for contact groups.
const GROUP_PREFIX: &str = "contacts:group:";

/// Maximum number of contacts to return from a list/search.
const MAX_LIST_LIMIT: u32 = 1_000;
/// Default list limit.
const DEFAULT_LIST_LIMIT: u32 = 100;

/// Contact store backed by a distributed `KeyValueStore`.
pub struct ContactStore<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> ContactStore<S> {
    /// Create a new contact store.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    // ========================================================================
    // Book operations
    // ========================================================================

    /// Create an address book.
    pub async fn create_book(
        &self,
        name: &str,
        description: Option<&str>,
        now_ms: u64,
    ) -> Result<ContactBook, ContactsError> {
        if name.is_empty() {
            return Err(ContactsError::InvalidInput {
                reason: "book name cannot be empty".into(),
            });
        }

        let id = generate_id("book", name);
        let book = ContactBook {
            id: id.clone(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            owner: None,
            created_at_ms: now_ms,
        };

        let key = format!("{BOOK_PREFIX}{id}");
        let value = serde_json::to_string(&book).map_err(|e| ContactsError::StorageError {
            reason: format!("serialize book: {e}"),
        })?;

        self.store
            .write(WriteRequest::set(&key, &value))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        debug!(book_id = %id, name, "created address book");
        Ok(book)
    }

    /// Delete an address book and all its contacts and groups.
    pub async fn delete_book(&self, book_id: &str) -> Result<(), ContactsError> {
        // Verify the book exists.
        self.get_book(book_id).await?;

        // Delete all contacts in the book.
        let contact_prefix = format!("{ENTRY_PREFIX}{book_id}:");
        let contacts = self.scan_all(&contact_prefix).await?;
        for entry in &contacts {
            self.store
                .delete(DeleteRequest::new(&entry.key))
                .await
                .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;
        }

        // Delete all groups in the book.
        let group_prefix = format!("{GROUP_PREFIX}{book_id}:");
        let groups = self.scan_all(&group_prefix).await?;
        for entry in &groups {
            self.store
                .delete(DeleteRequest::new(&entry.key))
                .await
                .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;
        }

        // Delete the book itself.
        let key = format!("{BOOK_PREFIX}{book_id}");
        self.store
            .delete(DeleteRequest::new(&key))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        debug!(book_id, "deleted address book");
        Ok(())
    }

    /// Get an address book by ID.
    pub async fn get_book(&self, book_id: &str) -> Result<ContactBook, ContactsError> {
        let key = format!("{BOOK_PREFIX}{book_id}");
        let result = self.kv_read(&key).await?;
        match result {
            Some(value) => serde_json::from_str(&value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize book: {e}"),
            }),
            None => Err(ContactsError::BookNotFound {
                id: book_id.to_string(),
            }),
        }
    }

    /// List all address books.
    pub async fn list_books(&self, limit: Option<u32>) -> Result<Vec<ContactBook>, ContactsError> {
        let limit = clamp_limit(limit);
        let scan = self
            .store
            .scan(ScanRequest {
                prefix: BOOK_PREFIX.to_string(),
                limit_results: Some(limit),
                continuation_token: None,
            })
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        let mut books = Vec::with_capacity(scan.entries.len());
        for entry in &scan.entries {
            let book: ContactBook = serde_json::from_str(&entry.value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize book: {e}"),
            })?;
            books.push(book);
        }
        Ok(books)
    }

    // ========================================================================
    // Contact CRUD
    // ========================================================================

    /// Create a contact from vCard data.
    pub async fn create_contact(&self, book_id: &str, vcard_data: &str, now_ms: u64) -> Result<Contact, ContactsError> {
        // Verify book exists.
        self.get_book(book_id).await?;

        let mut contact = parse_vcard(vcard_data).map_err(|e| ContactsError::ParseVcard { reason: e.to_string() })?;

        // Assign book and generate ID.
        contact.book_id = book_id.to_string();
        if contact.id.is_empty() {
            let seed = if contact.uid.is_empty() {
                format!("{}:{}:{}", book_id, contact.display_name, now_ms)
            } else {
                contact.uid.clone()
            };
            contact.id = generate_id("contact", &seed);
        }
        contact.created_at_ms = now_ms;
        contact.updated_at_ms = now_ms;

        self.write_contact(&contact).await?;
        debug!(contact_id = %contact.id, book_id, "created contact");
        Ok(contact)
    }

    /// Get a contact by ID.
    pub async fn get_contact(&self, contact_id: &str) -> Result<Contact, ContactsError> {
        // We need to scan for the contact since we don't know the book_id.
        let scan = self
            .store
            .scan(ScanRequest {
                prefix: ENTRY_PREFIX.to_string(),
                limit_results: Some(MAX_LIST_LIMIT),
                continuation_token: None,
            })
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        for entry in &scan.entries {
            let contact: Contact = serde_json::from_str(&entry.value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize contact: {e}"),
            })?;
            if contact.id == contact_id {
                return Ok(contact);
            }
        }

        Err(ContactsError::NotFound {
            id: contact_id.to_string(),
        })
    }

    /// Update a contact with new vCard data.
    pub async fn update_contact(
        &self,
        contact_id: &str,
        vcard_data: &str,
        now_ms: u64,
    ) -> Result<Contact, ContactsError> {
        let existing = self.get_contact(contact_id).await?;

        let mut contact = parse_vcard(vcard_data).map_err(|e| ContactsError::ParseVcard { reason: e.to_string() })?;

        // Preserve identity fields.
        contact.id = existing.id;
        contact.book_id = existing.book_id;
        contact.created_at_ms = existing.created_at_ms;
        contact.updated_at_ms = now_ms;

        self.write_contact(&contact).await?;
        debug!(contact_id, "updated contact");
        Ok(contact)
    }

    /// Delete a contact by ID.
    pub async fn delete_contact(&self, contact_id: &str) -> Result<(), ContactsError> {
        let contact = self.get_contact(contact_id).await?;
        let key = contact_key(&contact.book_id, &contact.id);
        self.store
            .delete(DeleteRequest::new(&key))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;
        debug!(contact_id, "deleted contact");
        Ok(())
    }

    /// List contacts in a book with pagination.
    pub async fn list_contacts(
        &self,
        book_id: &str,
        limit: Option<u32>,
        continuation_token: Option<&str>,
    ) -> Result<(Vec<Contact>, Option<String>), ContactsError> {
        let limit = clamp_limit(limit);
        let prefix = format!("{ENTRY_PREFIX}{book_id}:");
        let scan = self
            .store
            .scan(ScanRequest {
                prefix,
                limit_results: Some(limit),
                continuation_token: continuation_token.map(|s| s.to_string()),
            })
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        let mut contacts = Vec::with_capacity(scan.entries.len());
        for entry in &scan.entries {
            let contact: Contact = serde_json::from_str(&entry.value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize contact: {e}"),
            })?;
            contacts.push(contact);
        }

        Ok((contacts, scan.continuation_token))
    }

    /// Search contacts by name, email, or phone substring.
    pub async fn search_contacts(
        &self,
        query: &str,
        book_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Contact>, ContactsError> {
        let limit = clamp_limit(limit);
        let prefix = match book_id {
            Some(bid) => format!("{ENTRY_PREFIX}{bid}:"),
            None => ENTRY_PREFIX.to_string(),
        };

        let scan = self
            .store
            .scan(ScanRequest {
                prefix,
                limit_results: Some(MAX_LIST_LIMIT),
                continuation_token: None,
            })
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for entry in &scan.entries {
            if results.len() as u32 >= limit {
                break;
            }
            let contact: Contact = serde_json::from_str(&entry.value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize contact: {e}"),
            })?;

            if contact_matches_query(&contact, &query_lower) {
                results.push(contact);
            }
        }

        Ok(results)
    }

    /// Bulk import contacts from a multi-entry vCard string.
    pub async fn import_vcard(
        &self,
        book_id: &str,
        vcard_data: &str,
        now_ms: u64,
    ) -> Result<Vec<Contact>, ContactsError> {
        // Verify book exists.
        self.get_book(book_id).await?;

        let mut contacts = parse_vcards(vcard_data).map_err(|e| ContactsError::ParseVcard { reason: e.to_string() })?;

        for (idx, contact) in contacts.iter_mut().enumerate() {
            contact.book_id = book_id.to_string();
            if contact.id.is_empty() {
                let seed = if contact.uid.is_empty() {
                    format!("{}:{}:{}:{}", book_id, contact.display_name, now_ms, idx)
                } else {
                    contact.uid.clone()
                };
                contact.id = generate_id("contact", &seed);
            }
            contact.created_at_ms = now_ms;
            contact.updated_at_ms = now_ms;
        }

        // Write all contacts.
        let pairs: Vec<(String, String)> = contacts
            .iter()
            .map(|c| {
                let key = contact_key(&c.book_id, &c.id);
                let value = serde_json::to_string(c).unwrap_or_default();
                (key, value)
            })
            .collect();

        if !pairs.is_empty() {
            self.store
                .write(WriteRequest::from_command(WriteCommand::SetMulti { pairs }))
                .await
                .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;
        }

        debug!(book_id, count = contacts.len(), "imported contacts from vCard");
        Ok(contacts)
    }

    /// Export all contacts in a book as a vCard string.
    pub async fn export_vcard(&self, book_id: &str) -> Result<(String, u32), ContactsError> {
        // Verify book exists.
        self.get_book(book_id).await?;

        let prefix = format!("{ENTRY_PREFIX}{book_id}:");
        let entries = self.scan_all(&prefix).await?;

        let mut contacts = Vec::with_capacity(entries.len());
        for entry in &entries {
            let contact: Contact = serde_json::from_str(&entry.value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize contact: {e}"),
            })?;
            contacts.push(contact);
        }

        let count = contacts.len() as u32;
        let vcard_data = serialize_vcards(&contacts);
        Ok((vcard_data, count))
    }

    // ========================================================================
    // Group operations
    // ========================================================================

    /// Create a contact group.
    pub async fn create_group(
        &self,
        book_id: &str,
        name: &str,
        member_ids: Vec<String>,
    ) -> Result<ContactGroup, ContactsError> {
        // Verify book exists.
        self.get_book(book_id).await?;

        if name.is_empty() {
            return Err(ContactsError::InvalidInput {
                reason: "group name cannot be empty".into(),
            });
        }

        let id = generate_id("group", name);
        let group = ContactGroup {
            id: id.clone(),
            book_id: book_id.to_string(),
            name: name.to_string(),
            member_ids,
        };

        let key = group_key(book_id, &id);
        let value = serde_json::to_string(&group).map_err(|e| ContactsError::StorageError {
            reason: format!("serialize group: {e}"),
        })?;

        self.store
            .write(WriteRequest::set(&key, &value))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        debug!(group_id = %id, book_id, name, "created contact group");
        Ok(group)
    }

    /// Delete a contact group.
    pub async fn delete_group(&self, group_id: &str) -> Result<(), ContactsError> {
        let group = self.find_group(group_id).await?;
        let key = group_key(&group.book_id, &group.id);
        self.store
            .delete(DeleteRequest::new(&key))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;
        debug!(group_id, "deleted contact group");
        Ok(())
    }

    /// List groups in a book.
    pub async fn list_groups(&self, book_id: &str) -> Result<Vec<ContactGroup>, ContactsError> {
        let prefix = format!("{GROUP_PREFIX}{book_id}:");
        let entries = self.scan_all(&prefix).await?;

        let mut groups = Vec::with_capacity(entries.len());
        for entry in &entries {
            let group: ContactGroup = serde_json::from_str(&entry.value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize group: {e}"),
            })?;
            groups.push(group);
        }
        Ok(groups)
    }

    /// Add a contact to a group.
    pub async fn add_to_group(&self, group_id: &str, contact_id: &str) -> Result<ContactGroup, ContactsError> {
        let mut group = self.find_group(group_id).await?;

        if !group.member_ids.contains(&contact_id.to_string()) {
            group.member_ids.push(contact_id.to_string());
        }

        let key = group_key(&group.book_id, &group.id);
        let value = serde_json::to_string(&group).map_err(|e| ContactsError::StorageError {
            reason: format!("serialize group: {e}"),
        })?;
        self.store
            .write(WriteRequest::set(&key, &value))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        Ok(group)
    }

    /// Remove a contact from a group.
    pub async fn remove_from_group(&self, group_id: &str, contact_id: &str) -> Result<ContactGroup, ContactsError> {
        let mut group = self.find_group(group_id).await?;
        group.member_ids.retain(|id| id != contact_id);

        let key = group_key(&group.book_id, &group.id);
        let value = serde_json::to_string(&group).map_err(|e| ContactsError::StorageError {
            reason: format!("serialize group: {e}"),
        })?;
        self.store
            .write(WriteRequest::set(&key, &value))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

        Ok(group)
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Write a contact to the KV store.
    async fn write_contact(&self, contact: &Contact) -> Result<(), ContactsError> {
        let key = contact_key(&contact.book_id, &contact.id);
        let value = serde_json::to_string(contact).map_err(|e| ContactsError::StorageError {
            reason: format!("serialize contact: {e}"),
        })?;
        self.store
            .write(WriteRequest::set(&key, &value))
            .await
            .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;
        Ok(())
    }

    /// Read a single value from the KV store, returning None if not found.
    async fn kv_read(&self, key: &str) -> Result<Option<String>, ContactsError> {
        match self.store.read(ReadRequest::new(key)).await {
            Ok(result) => Ok(result.kv.map(|kv| kv.value)),
            Err(aspen_kv_types::KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(ContactsError::StorageError { reason: e.to_string() }),
        }
    }

    /// Scan all entries matching a prefix (paginating through all results).
    async fn scan_all(&self, prefix: &str) -> Result<Vec<aspen_kv_types::KeyValueWithRevision>, ContactsError> {
        let mut all_entries = Vec::new();
        let mut continuation_token = None;

        loop {
            let scan = self
                .store
                .scan(ScanRequest {
                    prefix: prefix.to_string(),
                    limit_results: Some(MAX_LIST_LIMIT),
                    continuation_token,
                })
                .await
                .map_err(|e| ContactsError::StorageError { reason: e.to_string() })?;

            all_entries.extend(scan.entries);

            if scan.is_truncated {
                continuation_token = scan.continuation_token;
            } else {
                break;
            }
        }

        Ok(all_entries)
    }

    /// Find a group by ID, scanning all books.
    async fn find_group(&self, group_id: &str) -> Result<ContactGroup, ContactsError> {
        let entries = self.scan_all(GROUP_PREFIX).await?;

        for entry in &entries {
            let group: ContactGroup = serde_json::from_str(&entry.value).map_err(|e| ContactsError::StorageError {
                reason: format!("deserialize group: {e}"),
            })?;
            if group.id == group_id {
                return Ok(group);
            }
        }

        Err(ContactsError::GroupNotFound {
            id: group_id.to_string(),
        })
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Build the KV key for a contact entry.
fn contact_key(book_id: &str, contact_id: &str) -> String {
    format!("{ENTRY_PREFIX}{book_id}:{contact_id}")
}

/// Build the KV key for a group.
fn group_key(book_id: &str, group_id: &str) -> String {
    format!("{GROUP_PREFIX}{book_id}:{group_id}")
}

/// Generate a deterministic ID from a seed.
fn generate_id(prefix: &str, seed: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::new();
    prefix.hash(&mut hasher);
    seed.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Clamp a limit to the valid range.
fn clamp_limit(limit: Option<u32>) -> u32 {
    limit.unwrap_or(DEFAULT_LIST_LIMIT).min(MAX_LIST_LIMIT)
}

/// Check if a contact matches a search query (name, email, phone substring match).
fn contact_matches_query(contact: &Contact, query_lower: &str) -> bool {
    if contact.display_name.to_lowercase().contains(query_lower) {
        return true;
    }
    if let Some(family) = &contact.family_name
        && family.to_lowercase().contains(query_lower)
    {
        return true;
    }
    if let Some(given) = &contact.given_name
        && given.to_lowercase().contains(query_lower)
    {
        return true;
    }
    for email in &contact.emails {
        if email.address.to_lowercase().contains(query_lower) {
            return true;
        }
    }
    for phone in &contact.phones {
        if phone.number.contains(query_lower) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;
    use crate::ContactEmail;
    use crate::ContactPhone;

    fn make_store() -> ContactStore<DeterministicKeyValueStore> {
        ContactStore::new(DeterministicKeyValueStore::new())
    }

    #[tokio::test]
    async fn test_create_and_get_book() {
        let store = make_store();
        let book = store.create_book("Personal", Some("My contacts"), 1000).await.unwrap();
        assert_eq!(book.name, "Personal");
        assert_eq!(book.description.as_deref(), Some("My contacts"));
        assert_eq!(book.created_at_ms, 1000);

        let fetched = store.get_book(&book.id).await.unwrap();
        assert_eq!(fetched.name, "Personal");
    }

    #[tokio::test]
    async fn test_create_book_empty_name_fails() {
        let store = make_store();
        let result = store.create_book("", None, 1000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_books() {
        let store = make_store();
        store.create_book("Personal", None, 1000).await.unwrap();
        store.create_book("Work", None, 2000).await.unwrap();

        let books = store.list_books(None).await.unwrap();
        assert_eq!(books.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_book_cascades() {
        let store = make_store();
        let book = store.create_book("Temp", None, 1000).await.unwrap();

        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Test User\r\nUID:u1\r\nEND:VCARD\r\n";
        store.create_contact(&book.id, vcard, 2000).await.unwrap();
        store.create_group(&book.id, "Friends", vec![]).await.unwrap();

        store.delete_book(&book.id).await.unwrap();

        assert!(store.get_book(&book.id).await.is_err());
        let (contacts, _) = store.list_contacts(&book.id, None, None).await.unwrap();
        assert!(contacts.is_empty());
        let groups = store.list_groups(&book.id).await.unwrap();
        assert!(groups.is_empty());
    }

    #[tokio::test]
    async fn test_create_and_get_contact() {
        let store = make_store();
        let book = store.create_book("Personal", None, 1000).await.unwrap();

        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:John Doe\r\nN:Doe;John;;;\r\nEMAIL;TYPE=work:john@example.com\r\nTEL;TYPE=cell:+1-555-0123\r\nUID:john-001\r\nEND:VCARD\r\n";
        let contact = store.create_contact(&book.id, vcard, 2000).await.unwrap();

        assert_eq!(contact.display_name, "John Doe");
        assert_eq!(contact.family_name.as_deref(), Some("Doe"));
        assert_eq!(contact.given_name.as_deref(), Some("John"));
        assert_eq!(contact.emails.len(), 1);
        assert_eq!(contact.phones.len(), 1);
        assert_eq!(contact.created_at_ms, 2000);

        let fetched = store.get_contact(&contact.id).await.unwrap();
        assert_eq!(fetched.display_name, "John Doe");
    }

    #[tokio::test]
    async fn test_update_contact() {
        let store = make_store();
        let book = store.create_book("Personal", None, 1000).await.unwrap();

        let vcard1 = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:John Doe\r\nUID:john-001\r\nEND:VCARD\r\n";
        let contact = store.create_contact(&book.id, vcard1, 2000).await.unwrap();

        let vcard2 = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:John Smith\r\nUID:john-001\r\nEND:VCARD\r\n";
        let updated = store.update_contact(&contact.id, vcard2, 3000).await.unwrap();

        assert_eq!(updated.display_name, "John Smith");
        assert_eq!(updated.id, contact.id);
        assert_eq!(updated.book_id, book.id);
        assert_eq!(updated.created_at_ms, 2000);
        assert_eq!(updated.updated_at_ms, 3000);
    }

    #[tokio::test]
    async fn test_delete_contact() {
        let store = make_store();
        let book = store.create_book("Personal", None, 1000).await.unwrap();

        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Temp\r\nUID:t1\r\nEND:VCARD\r\n";
        let contact = store.create_contact(&book.id, vcard, 2000).await.unwrap();

        store.delete_contact(&contact.id).await.unwrap();
        assert!(store.get_contact(&contact.id).await.is_err());
    }

    #[tokio::test]
    async fn test_list_contacts_with_pagination() {
        let store = make_store();
        let book = store.create_book("Personal", None, 1000).await.unwrap();

        for i in 0..5 {
            let vcard = format!("BEGIN:VCARD\r\nVERSION:4.0\r\nFN:User {i}\r\nUID:user-{i}\r\nEND:VCARD\r\n");
            store.create_contact(&book.id, &vcard, 2000 + i).await.unwrap();
        }

        let (contacts, _) = store.list_contacts(&book.id, Some(3), None).await.unwrap();
        assert!(contacts.len() <= 3);
    }

    #[tokio::test]
    async fn test_search_contacts() {
        let store = make_store();
        let book = store.create_book("Personal", None, 1000).await.unwrap();

        let vcard1 =
            "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alice Smith\r\nEMAIL:alice@example.com\r\nUID:a1\r\nEND:VCARD\r\n";
        let vcard2 = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bob Jones\r\nEMAIL:bob@example.com\r\nUID:b1\r\nEND:VCARD\r\n";
        store.create_contact(&book.id, vcard1, 2000).await.unwrap();
        store.create_contact(&book.id, vcard2, 2001).await.unwrap();

        let results = store.search_contacts("alice", Some(&book.id), None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, "Alice Smith");

        // Search by email.
        let results = store.search_contacts("bob@", Some(&book.id), None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].display_name, "Bob Jones");
    }

    #[tokio::test]
    async fn test_import_export_vcard() {
        let store = make_store();
        let book = store.create_book("Personal", None, 1000).await.unwrap();

        let vcards = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alice\r\nUID:a1\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bob\r\nUID:b1\r\nEND:VCARD\r\n";
        let imported = store.import_vcard(&book.id, vcards, 2000).await.unwrap();
        assert_eq!(imported.len(), 2);

        let (exported, count) = store.export_vcard(&book.id).await.unwrap();
        assert_eq!(count, 2);
        assert!(exported.contains("Alice"));
        assert!(exported.contains("Bob"));
    }

    #[tokio::test]
    async fn test_group_operations() {
        let store = make_store();
        let book = store.create_book("Personal", None, 1000).await.unwrap();

        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alice\r\nUID:a1\r\nEND:VCARD\r\n";
        let contact = store.create_contact(&book.id, vcard, 2000).await.unwrap();

        // Create group.
        let group = store.create_group(&book.id, "Friends", vec![]).await.unwrap();
        assert_eq!(group.name, "Friends");
        assert!(group.member_ids.is_empty());

        // Add member.
        let group = store.add_to_group(&group.id, &contact.id).await.unwrap();
        assert_eq!(group.member_ids.len(), 1);
        assert_eq!(group.member_ids[0], contact.id);

        // Add same member is idempotent.
        let group = store.add_to_group(&group.id, &contact.id).await.unwrap();
        assert_eq!(group.member_ids.len(), 1);

        // Remove member.
        let group = store.remove_from_group(&group.id, &contact.id).await.unwrap();
        assert!(group.member_ids.is_empty());

        // List groups.
        let groups = store.list_groups(&book.id).await.unwrap();
        assert_eq!(groups.len(), 1);

        // Delete group.
        store.delete_group(&group.id).await.unwrap();
        let groups = store.list_groups(&book.id).await.unwrap();
        assert!(groups.is_empty());
    }

    #[tokio::test]
    async fn test_get_nonexistent_contact() {
        let store = make_store();
        assert!(store.get_contact("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_get_nonexistent_book() {
        let store = make_store();
        assert!(store.get_book("nonexistent").await.is_err());
    }

    #[test]
    fn test_contact_matches_query() {
        let contact = Contact {
            id: "c1".into(),
            book_id: "b1".into(),
            uid: "u1".into(),
            display_name: "John Doe".into(),
            family_name: Some("Doe".into()),
            given_name: Some("John".into()),
            emails: vec![ContactEmail {
                address: "john@example.com".into(),
                label: None,
                is_primary: true,
            }],
            phones: vec![ContactPhone {
                number: "+1-555-0123".into(),
                label: None,
                is_primary: true,
            }],
            addresses: vec![],
            organization: None,
            title: None,
            birthday: None,
            notes: None,
            photo_blob_hash: None,
            categories: vec![],
            url: None,
            custom_fields: vec![],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        assert!(contact_matches_query(&contact, "john"));
        assert!(contact_matches_query(&contact, "doe"));
        assert!(contact_matches_query(&contact, "john@"));
        assert!(contact_matches_query(&contact, "555"));
        assert!(!contact_matches_query(&contact, "alice"));
    }
}
