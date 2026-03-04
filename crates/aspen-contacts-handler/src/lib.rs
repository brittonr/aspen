//! Contacts RPC handler for Aspen.
//!
//! Handles all contacts operations (address books, contacts, groups, vCard import/export)
//! via the `ContactStore` backed by the distributed KV store.

pub mod executor;

use std::sync::Arc;

pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use aspen_rpc_core::ServiceHandler;
pub use executor::ContactsServiceExecutor;

/// Factory for creating `ContactsHandler` instances.
///
/// Priority 350 (application layer).
pub struct ContactsHandlerFactory;

impl ContactsHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ContactsHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for ContactsHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        let kv_store = ctx.kv_store.clone();
        let executor = Arc::new(ContactsServiceExecutor::new(kv_store));
        Some(Arc::new(ServiceHandler::new(executor)))
    }

    fn name(&self) -> &'static str {
        "ContactsHandler"
    }

    fn priority(&self) -> u32 {
        350
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("contacts")
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(ContactsHandlerFactory);

#[cfg(test)]
mod tests {
    use aspen_rpc_core::ServiceExecutor;

    use super::*;

    #[test]
    fn test_can_handle_all_contacts_variants() {
        let executor = ContactsServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        let handles = executor.handles();

        // All 16 contacts request variants should be handled.
        assert_eq!(handles.len(), 16);
        assert!(handles.contains(&"ContactsCreateBook"));
        assert!(handles.contains(&"ContactsDeleteBook"));
        assert!(handles.contains(&"ContactsListBooks"));
        assert!(handles.contains(&"ContactsCreateContact"));
        assert!(handles.contains(&"ContactsGetContact"));
        assert!(handles.contains(&"ContactsUpdateContact"));
        assert!(handles.contains(&"ContactsDeleteContact"));
        assert!(handles.contains(&"ContactsListContacts"));
        assert!(handles.contains(&"ContactsSearchContacts"));
        assert!(handles.contains(&"ContactsImportVcard"));
        assert!(handles.contains(&"ContactsExportVcard"));
        assert!(handles.contains(&"ContactsCreateGroup"));
        assert!(handles.contains(&"ContactsDeleteGroup"));
        assert!(handles.contains(&"ContactsListGroups"));
        assert!(handles.contains(&"ContactsAddToGroup"));
        assert!(handles.contains(&"ContactsRemoveFromGroup"));
    }

    #[test]
    fn test_service_name() {
        let executor = ContactsServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        assert_eq!(executor.service_name(), "contacts");
    }

    #[test]
    fn test_priority() {
        let executor = ContactsServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        assert_eq!(executor.priority(), 350);
    }

    #[test]
    fn test_app_id() {
        let executor = ContactsServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        assert_eq!(executor.app_id(), Some("contacts"));
    }
}
