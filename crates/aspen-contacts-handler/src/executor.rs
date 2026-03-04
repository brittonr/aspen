//! Contacts service executor — dispatches RPC requests to `ContactStore`.

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::contacts::ContactSummary;
use aspen_client_api::contacts::ContactsBookInfo;
use aspen_client_api::contacts::ContactsBookListResponse;
use aspen_client_api::contacts::ContactsBookResponse;
use aspen_client_api::contacts::ContactsExportResponse;
use aspen_client_api::contacts::ContactsGroupInfo;
use aspen_client_api::contacts::ContactsGroupListResponse;
use aspen_client_api::contacts::ContactsGroupResponse;
use aspen_client_api::contacts::ContactsListResponse;
use aspen_client_api::contacts::ContactsResponse;
use aspen_client_api::contacts::ContactsSearchResponse;
use aspen_contacts::Contact;
use aspen_contacts::ContactStore;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use tracing::warn;

/// Handles list for contacts operations.
const HANDLES: &[&str] = &[
    "ContactsCreateBook",
    "ContactsDeleteBook",
    "ContactsListBooks",
    "ContactsCreateContact",
    "ContactsGetContact",
    "ContactsUpdateContact",
    "ContactsDeleteContact",
    "ContactsListContacts",
    "ContactsSearchContacts",
    "ContactsImportVcard",
    "ContactsExportVcard",
    "ContactsCreateGroup",
    "ContactsDeleteGroup",
    "ContactsListGroups",
    "ContactsAddToGroup",
    "ContactsRemoveFromGroup",
];

/// Contacts service executor backed by `ContactStore`.
pub struct ContactsServiceExecutor {
    store: ContactStore<dyn KeyValueStore>,
}

impl ContactsServiceExecutor {
    /// Create a new executor.
    pub fn new(kv_store: Arc<dyn KeyValueStore>) -> Self {
        Self {
            store: ContactStore::new(kv_store),
        }
    }
}

#[async_trait]
impl aspen_rpc_core::ServiceExecutor for ContactsServiceExecutor {
    fn service_name(&self) -> &'static str {
        "contacts"
    }

    fn handles(&self) -> &'static [&'static str] {
        HANDLES
    }

    fn priority(&self) -> u32 {
        350
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("contacts")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ContactsCreateBook { name, description } => {
                self.handle_create_book(&name, description.as_deref()).await
            }
            ClientRpcRequest::ContactsDeleteBook { book_id } => self.handle_delete_book(&book_id).await,
            ClientRpcRequest::ContactsListBooks { limit } => self.handle_list_books(limit).await,
            ClientRpcRequest::ContactsCreateContact { book_id, vcard_data } => {
                self.handle_create_contact(&book_id, &vcard_data).await
            }
            ClientRpcRequest::ContactsGetContact { contact_id } => self.handle_get_contact(&contact_id).await,
            ClientRpcRequest::ContactsUpdateContact { contact_id, vcard_data } => {
                self.handle_update_contact(&contact_id, &vcard_data).await
            }
            ClientRpcRequest::ContactsDeleteContact { contact_id } => self.handle_delete_contact(&contact_id).await,
            ClientRpcRequest::ContactsListContacts {
                book_id,
                limit,
                continuation_token,
            } => self.handle_list_contacts(&book_id, limit, continuation_token.as_deref()).await,
            ClientRpcRequest::ContactsSearchContacts { query, book_id, limit } => {
                self.handle_search_contacts(&query, book_id.as_deref(), limit).await
            }
            ClientRpcRequest::ContactsImportVcard { book_id, vcard_data } => {
                self.handle_import_vcard(&book_id, &vcard_data).await
            }
            ClientRpcRequest::ContactsExportVcard { book_id } => self.handle_export_vcard(&book_id).await,
            ClientRpcRequest::ContactsCreateGroup {
                book_id,
                name,
                member_ids,
            } => self.handle_create_group(&book_id, &name, member_ids).await,
            ClientRpcRequest::ContactsDeleteGroup { group_id } => self.handle_delete_group(&group_id).await,
            ClientRpcRequest::ContactsListGroups { book_id } => self.handle_list_groups(&book_id).await,
            ClientRpcRequest::ContactsAddToGroup { group_id, contact_id } => {
                self.handle_add_to_group(&group_id, &contact_id).await
            }
            ClientRpcRequest::ContactsRemoveFromGroup { group_id, contact_id } => {
                self.handle_remove_from_group(&group_id, &contact_id).await
            }
            other => {
                warn!(variant = %other.variant_name(), "unexpected request in contacts executor");
                Ok(ClientRpcResponse::error("INTERNAL_ERROR", "unexpected request"))
            }
        }
    }
}

impl ContactsServiceExecutor {
    fn now_ms() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    async fn handle_create_book(&self, name: &str, description: Option<&str>) -> Result<ClientRpcResponse> {
        match self.store.create_book(name, description, Self::now_ms()).await {
            Ok(book) => Ok(ClientRpcResponse::ContactsBookResult(ContactsBookResponse {
                is_success: true,
                book_id: Some(book.id),
                name: Some(book.name),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsBookResult(ContactsBookResponse {
                is_success: false,
                book_id: None,
                name: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_delete_book(&self, book_id: &str) -> Result<ClientRpcResponse> {
        match self.store.delete_book(book_id).await {
            Ok(()) => Ok(ClientRpcResponse::ContactsBookResult(ContactsBookResponse {
                is_success: true,
                book_id: Some(book_id.to_string()),
                name: None,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsBookResult(ContactsBookResponse {
                is_success: false,
                book_id: Some(book_id.to_string()),
                name: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_list_books(&self, limit: Option<u32>) -> Result<ClientRpcResponse> {
        match self.store.list_books(limit).await {
            Ok(books) => Ok(ClientRpcResponse::ContactsBookListResult(ContactsBookListResponse {
                is_success: true,
                books: books
                    .into_iter()
                    .map(|b| ContactsBookInfo {
                        id: b.id,
                        name: b.name,
                        description: b.description,
                        contact_count: 0, // Would need a separate count query
                    })
                    .collect(),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsBookListResult(ContactsBookListResponse {
                is_success: false,
                books: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_create_contact(&self, book_id: &str, vcard_data: &str) -> Result<ClientRpcResponse> {
        match self.store.create_contact(book_id, vcard_data, Self::now_ms()).await {
            Ok(contact) => {
                let vcard = aspen_contacts::serialize_vcard(&contact);
                Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                    is_success: true,
                    contact_id: Some(contact.id),
                    vcard_data: Some(vcard),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                is_success: false,
                contact_id: None,
                vcard_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_get_contact(&self, contact_id: &str) -> Result<ClientRpcResponse> {
        match self.store.get_contact(contact_id).await {
            Ok(contact) => {
                let vcard = aspen_contacts::serialize_vcard(&contact);
                Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                    is_success: true,
                    contact_id: Some(contact.id),
                    vcard_data: Some(vcard),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                is_success: false,
                contact_id: Some(contact_id.to_string()),
                vcard_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_update_contact(&self, contact_id: &str, vcard_data: &str) -> Result<ClientRpcResponse> {
        match self.store.update_contact(contact_id, vcard_data, Self::now_ms()).await {
            Ok(contact) => {
                let vcard = aspen_contacts::serialize_vcard(&contact);
                Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                    is_success: true,
                    contact_id: Some(contact.id),
                    vcard_data: Some(vcard),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                is_success: false,
                contact_id: Some(contact_id.to_string()),
                vcard_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_delete_contact(&self, contact_id: &str) -> Result<ClientRpcResponse> {
        match self.store.delete_contact(contact_id).await {
            Ok(()) => Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                is_success: true,
                contact_id: Some(contact_id.to_string()),
                vcard_data: None,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsResult(ContactsResponse {
                is_success: false,
                contact_id: Some(contact_id.to_string()),
                vcard_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_list_contacts(
        &self,
        book_id: &str,
        limit: Option<u32>,
        continuation_token: Option<&str>,
    ) -> Result<ClientRpcResponse> {
        match self.store.list_contacts(book_id, limit, continuation_token).await {
            Ok((contacts, token)) => {
                let total = contacts.len() as u32;
                Ok(ClientRpcResponse::ContactsListResult(ContactsListResponse {
                    is_success: true,
                    contacts: contacts.into_iter().map(to_contact_summary).collect(),
                    continuation_token: token,
                    total,
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ContactsListResult(ContactsListResponse {
                is_success: false,
                contacts: vec![],
                continuation_token: None,
                total: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_search_contacts(
        &self,
        query: &str,
        book_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        match self.store.search_contacts(query, book_id, limit).await {
            Ok(contacts) => {
                let total = contacts.len() as u32;
                Ok(ClientRpcResponse::ContactsSearchResult(ContactsSearchResponse {
                    is_success: true,
                    contacts: contacts.into_iter().map(to_contact_summary).collect(),
                    total,
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ContactsSearchResult(ContactsSearchResponse {
                is_success: false,
                contacts: vec![],
                total: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_import_vcard(&self, book_id: &str, vcard_data: &str) -> Result<ClientRpcResponse> {
        match self.store.import_vcard(book_id, vcard_data, Self::now_ms()).await {
            Ok(contacts) => {
                let count = contacts.len() as u32;
                Ok(ClientRpcResponse::ContactsExportResult(ContactsExportResponse {
                    is_success: true,
                    vcard_data: None,
                    count,
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ContactsExportResult(ContactsExportResponse {
                is_success: false,
                vcard_data: None,
                count: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_export_vcard(&self, book_id: &str) -> Result<ClientRpcResponse> {
        match self.store.export_vcard(book_id).await {
            Ok((data, count)) => Ok(ClientRpcResponse::ContactsExportResult(ContactsExportResponse {
                is_success: true,
                vcard_data: Some(data),
                count,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsExportResult(ContactsExportResponse {
                is_success: false,
                vcard_data: None,
                count: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_create_group(
        &self,
        book_id: &str,
        name: &str,
        member_ids: Vec<String>,
    ) -> Result<ClientRpcResponse> {
        match self.store.create_group(book_id, name, member_ids).await {
            Ok(group) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: true,
                group_id: Some(group.id),
                name: Some(group.name),
                member_ids: group.member_ids,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: false,
                group_id: None,
                name: None,
                member_ids: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_delete_group(&self, group_id: &str) -> Result<ClientRpcResponse> {
        match self.store.delete_group(group_id).await {
            Ok(()) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: true,
                group_id: Some(group_id.to_string()),
                name: None,
                member_ids: vec![],
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: false,
                group_id: Some(group_id.to_string()),
                name: None,
                member_ids: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_list_groups(&self, book_id: &str) -> Result<ClientRpcResponse> {
        match self.store.list_groups(book_id).await {
            Ok(groups) => Ok(ClientRpcResponse::ContactsGroupListResult(ContactsGroupListResponse {
                is_success: true,
                groups: groups
                    .into_iter()
                    .map(|g| ContactsGroupInfo {
                        id: g.id,
                        name: g.name,
                        member_count: g.member_ids.len() as u32,
                    })
                    .collect(),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsGroupListResult(ContactsGroupListResponse {
                is_success: false,
                groups: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_add_to_group(&self, group_id: &str, contact_id: &str) -> Result<ClientRpcResponse> {
        match self.store.add_to_group(group_id, contact_id).await {
            Ok(group) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: true,
                group_id: Some(group.id),
                name: Some(group.name),
                member_ids: group.member_ids,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: false,
                group_id: Some(group_id.to_string()),
                name: None,
                member_ids: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_remove_from_group(&self, group_id: &str, contact_id: &str) -> Result<ClientRpcResponse> {
        match self.store.remove_from_group(group_id, contact_id).await {
            Ok(group) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: true,
                group_id: Some(group.id),
                name: Some(group.name),
                member_ids: group.member_ids,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ContactsGroupResult(ContactsGroupResponse {
                is_success: false,
                group_id: Some(group_id.to_string()),
                name: None,
                member_ids: vec![],
                error: Some(e.to_string()),
            })),
        }
    }
}

fn to_contact_summary(contact: Contact) -> ContactSummary {
    let primary_email =
        contact.emails.iter().find(|e| e.is_primary).or(contact.emails.first()).map(|e| e.address.clone());

    let primary_phone =
        contact.phones.iter().find(|p| p.is_primary).or(contact.phones.first()).map(|p| p.number.clone());

    ContactSummary {
        id: contact.id,
        display_name: contact.display_name,
        primary_email,
        primary_phone,
    }
}
