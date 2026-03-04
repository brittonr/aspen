use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Book operations
        ClientRpcRequest::ContactsCreateBook { .. } => Some(Some(Operation::Write {
            key: "contacts:book:".to_string(),
            value: vec![],
        })),
        ClientRpcRequest::ContactsDeleteBook { book_id } => Some(Some(Operation::Write {
            key: format!("contacts:book:{}", book_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsListBooks { .. } => Some(Some(Operation::Read {
            key: "contacts:book:".to_string(),
        })),

        // Contact CRUD
        ClientRpcRequest::ContactsCreateContact { book_id, .. } => Some(Some(Operation::Write {
            key: format!("contacts:entry:{}:", book_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsGetContact { contact_id } => Some(Some(Operation::Read {
            key: format!("contacts:entry:{}", contact_id),
        })),
        ClientRpcRequest::ContactsUpdateContact { contact_id, .. } => Some(Some(Operation::Write {
            key: format!("contacts:entry:{}", contact_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsDeleteContact { contact_id } => Some(Some(Operation::Write {
            key: format!("contacts:entry:{}", contact_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsListContacts { book_id, .. } => Some(Some(Operation::Read {
            key: format!("contacts:entry:{}:", book_id),
        })),
        ClientRpcRequest::ContactsSearchContacts { .. } => Some(Some(Operation::Read {
            key: "contacts:entry:".to_string(),
        })),

        // Import/export
        ClientRpcRequest::ContactsImportVcard { book_id, .. } => Some(Some(Operation::Write {
            key: format!("contacts:entry:{}:", book_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsExportVcard { book_id } => Some(Some(Operation::Read {
            key: format!("contacts:entry:{}:", book_id),
        })),

        // Group operations
        ClientRpcRequest::ContactsCreateGroup { book_id, .. } => Some(Some(Operation::Write {
            key: format!("contacts:group:{}:", book_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsDeleteGroup { group_id } => Some(Some(Operation::Write {
            key: format!("contacts:group:{}", group_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsListGroups { book_id } => Some(Some(Operation::Read {
            key: format!("contacts:group:{}:", book_id),
        })),
        ClientRpcRequest::ContactsAddToGroup { group_id, .. } => Some(Some(Operation::Write {
            key: format!("contacts:group:{}", group_id),
            value: vec![],
        })),
        ClientRpcRequest::ContactsRemoveFromGroup { group_id, .. } => Some(Some(Operation::Write {
            key: format!("contacts:group:{}", group_id),
            value: vec![],
        })),

        _ => None,
    }
}
