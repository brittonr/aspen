//! End-to-end integration tests for the contacts executor.
//!
//! Tests the full flow: ClientRpcRequest → ContactsServiceExecutor → ContactStore → KV →
//! ClientRpcResponse

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ServiceExecutor;
use aspen_testing_core::DeterministicKeyValueStore;

fn make_executor() -> aspen_contacts_handler::ContactsServiceExecutor {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    aspen_contacts_handler::ContactsServiceExecutor::new(kv)
}

// ============================================================================
// Task 8.1: End-to-end contacts test
// ============================================================================

#[tokio::test]
async fn test_e2e_create_book_add_contacts_list_search_export() {
    let exec = make_executor();

    // 1. Create a book
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateBook {
            name: "Work".to_string(),
            description: Some("Work contacts".to_string()),
        })
        .await
        .unwrap();

    let book_id = match &resp {
        ClientRpcResponse::ContactsBookResult(r) => {
            assert!(r.is_success, "create book failed: {:?}", r.error);
            r.book_id.clone().unwrap()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    // 2. Add contacts via vCard
    let vcard1 =
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alice Smith\r\nEMAIL:alice@example.com\r\nTEL:+1-555-0101\r\nEND:VCARD";
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateContact {
            book_id: book_id.clone(),
            vcard_data: vcard1.to_string(),
        })
        .await
        .unwrap();

    let alice_id = match &resp {
        ClientRpcResponse::ContactsResult(r) => {
            assert!(r.is_success, "create contact failed: {:?}", r.error);
            let id = r.contact_id.clone().unwrap();
            // Verify vCard data is returned
            assert!(r.vcard_data.as_ref().unwrap().contains("Alice Smith"));
            id
        }
        other => panic!("unexpected response: {other:?}"),
    };

    let vcard2 = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bob Jones\r\nEMAIL:bob@example.com\r\nTEL:+1-555-0102\r\nEND:VCARD";
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateContact {
            book_id: book_id.clone(),
            vcard_data: vcard2.to_string(),
        })
        .await
        .unwrap();

    let bob_id = match &resp {
        ClientRpcResponse::ContactsResult(r) => {
            assert!(r.is_success, "create bob failed: {:?}", r.error);
            r.contact_id.clone().unwrap()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    // 3. List contacts
    let resp = exec
        .execute(ClientRpcRequest::ContactsListContacts {
            book_id: book_id.clone(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::ContactsListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.total, 2);
            let names: Vec<&str> = r.contacts.iter().map(|c| c.display_name.as_str()).collect();
            assert!(names.contains(&"Alice Smith"), "missing Alice in {names:?}");
            assert!(names.contains(&"Bob Jones"), "missing Bob in {names:?}");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // 4. Search by email
    let resp = exec
        .execute(ClientRpcRequest::ContactsSearchContacts {
            query: "alice@example".to_string(),
            book_id: None,
            limit: None,
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::ContactsSearchResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.total, 1);
            assert_eq!(r.contacts[0].display_name, "Alice Smith");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // 5. Export vCard
    let resp = exec
        .execute(ClientRpcRequest::ContactsExportVcard {
            book_id: book_id.clone(),
        })
        .await
        .unwrap();

    let exported_vcard = match &resp {
        ClientRpcResponse::ContactsExportResult(r) => {
            assert!(r.is_success, "export failed: {:?}", r.error);
            assert_eq!(r.count, 2);
            let data = r.vcard_data.clone().unwrap();
            assert!(data.contains("Alice Smith"));
            assert!(data.contains("Bob Jones"));
            data
        }
        other => panic!("unexpected response: {other:?}"),
    };

    // 6. Delete all contacts, then re-import from exported vCard
    exec.execute(ClientRpcRequest::ContactsDeleteContact { contact_id: alice_id }).await.unwrap();
    exec.execute(ClientRpcRequest::ContactsDeleteContact { contact_id: bob_id }).await.unwrap();

    // Verify list is empty
    let resp = exec
        .execute(ClientRpcRequest::ContactsListContacts {
            book_id: book_id.clone(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.total, 0);
        }
        other => panic!("unexpected: {other:?}"),
    }

    // 7. Import the exported vCard back
    let resp = exec
        .execute(ClientRpcRequest::ContactsImportVcard {
            book_id: book_id.clone(),
            vcard_data: exported_vcard,
        })
        .await
        .unwrap();

    match &resp {
        ClientRpcResponse::ContactsExportResult(r) => {
            assert!(r.is_success, "import failed: {:?}", r.error);
            assert_eq!(r.count, 2);
        }
        other => panic!("unexpected: {other:?}"),
    }

    // 8. Verify roundtrip: contacts are back
    let resp = exec
        .execute(ClientRpcRequest::ContactsListContacts {
            book_id: book_id.clone(),
            limit: None,
            continuation_token: None,
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.total, 2);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

// ============================================================================
// Task 8.4: Contact groups test
// ============================================================================

#[tokio::test]
async fn test_e2e_contact_groups() {
    let exec = make_executor();

    // Create a book
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateBook {
            name: "Personal".to_string(),
            description: None,
        })
        .await
        .unwrap();
    let book_id = match &resp {
        ClientRpcResponse::ContactsBookResult(r) => r.book_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Add two contacts
    let vcard1 = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Carol Davis\r\nEMAIL:carol@example.com\r\nEND:VCARD";
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateContact {
            book_id: book_id.clone(),
            vcard_data: vcard1.to_string(),
        })
        .await
        .unwrap();
    let carol_id = match &resp {
        ClientRpcResponse::ContactsResult(r) => r.contact_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    let vcard2 = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Dan Evans\r\nEMAIL:dan@example.com\r\nEND:VCARD";
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateContact {
            book_id: book_id.clone(),
            vcard_data: vcard2.to_string(),
        })
        .await
        .unwrap();
    let dan_id = match &resp {
        ClientRpcResponse::ContactsResult(r) => r.contact_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Create group with Carol as initial member
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateGroup {
            book_id: book_id.clone(),
            name: "Friends".to_string(),
            member_ids: vec![carol_id.clone()],
        })
        .await
        .unwrap();
    let group_id = match &resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            assert!(r.is_success, "create group failed: {:?}", r.error);
            assert_eq!(r.name.as_deref(), Some("Friends"));
            assert_eq!(r.member_ids.len(), 1);
            assert!(r.member_ids.contains(&carol_id));
            r.group_id.clone().unwrap()
        }
        other => panic!("unexpected: {other:?}"),
    };

    // Add Dan to group
    let resp = exec
        .execute(ClientRpcRequest::ContactsAddToGroup {
            group_id: group_id.clone(),
            contact_id: dan_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.member_ids.len(), 2);
            assert!(r.member_ids.contains(&carol_id));
            assert!(r.member_ids.contains(&dan_id));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // List groups
    let resp = exec
        .execute(ClientRpcRequest::ContactsListGroups {
            book_id: book_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsGroupListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.groups.len(), 1);
            assert_eq!(r.groups[0].name, "Friends");
            assert_eq!(r.groups[0].member_count, 2);
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Remove Carol from group
    let resp = exec
        .execute(ClientRpcRequest::ContactsRemoveFromGroup {
            group_id: group_id.clone(),
            contact_id: carol_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.member_ids.len(), 1);
            assert!(r.member_ids.contains(&dan_id));
            assert!(!r.member_ids.contains(&carol_id));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Delete group
    let resp = exec
        .execute(ClientRpcRequest::ContactsDeleteGroup {
            group_id: group_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsGroupResult(r) => {
            assert!(r.is_success);
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Verify group is gone
    let resp = exec
        .execute(ClientRpcRequest::ContactsListGroups {
            book_id: book_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsGroupListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.groups.len(), 0);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn test_e2e_get_and_update_contact() {
    let exec = make_executor();

    // Create book
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateBook {
            name: "Test".to_string(),
            description: None,
        })
        .await
        .unwrap();
    let book_id = match &resp {
        ClientRpcResponse::ContactsBookResult(r) => r.book_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Create contact
    let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Eve Fox\r\nEMAIL:eve@example.com\r\nEND:VCARD";
    let resp = exec
        .execute(ClientRpcRequest::ContactsCreateContact {
            book_id: book_id.clone(),
            vcard_data: vcard.to_string(),
        })
        .await
        .unwrap();
    let contact_id = match &resp {
        ClientRpcResponse::ContactsResult(r) => r.contact_id.clone().unwrap(),
        other => panic!("unexpected: {other:?}"),
    };

    // Get contact
    let resp = exec
        .execute(ClientRpcRequest::ContactsGetContact {
            contact_id: contact_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsResult(r) => {
            assert!(r.is_success);
            let vcard_data = r.vcard_data.as_ref().unwrap();
            assert!(vcard_data.contains("Eve Fox"));
            assert!(vcard_data.contains("eve@example.com"));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Update contact — change email
    let updated_vcard =
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Eve Fox-Grant\r\nEMAIL:eve.grant@example.com\r\nTEL:+1-555-0999\r\nEND:VCARD";
    let resp = exec
        .execute(ClientRpcRequest::ContactsUpdateContact {
            contact_id: contact_id.clone(),
            vcard_data: updated_vcard.to_string(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsResult(r) => {
            assert!(r.is_success, "update failed: {:?}", r.error);
            let vcard_data = r.vcard_data.as_ref().unwrap();
            assert!(vcard_data.contains("Eve Fox-Grant"));
            assert!(vcard_data.contains("eve.grant@example.com"));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Verify updated data persists
    let resp = exec
        .execute(ClientRpcRequest::ContactsGetContact {
            contact_id: contact_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsResult(r) => {
            assert!(r.is_success);
            let vcard_data = r.vcard_data.as_ref().unwrap();
            assert!(vcard_data.contains("Eve Fox-Grant"));
            assert!(!vcard_data.contains("Eve Fox\r\n")); // Old name should be gone
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn test_e2e_list_books_and_delete() {
    let exec = make_executor();

    // Create multiple books
    exec.execute(ClientRpcRequest::ContactsCreateBook {
        name: "Work".to_string(),
        description: Some("Work contacts".to_string()),
    })
    .await
    .unwrap();

    exec.execute(ClientRpcRequest::ContactsCreateBook {
        name: "Personal".to_string(),
        description: None,
    })
    .await
    .unwrap();

    // List books
    let resp = exec.execute(ClientRpcRequest::ContactsListBooks { limit: None }).await.unwrap();
    let (book1_id, book2_id) = match &resp {
        ClientRpcResponse::ContactsBookListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.books.len(), 2);
            (r.books[0].id.clone(), r.books[1].id.clone())
        }
        other => panic!("unexpected: {other:?}"),
    };

    // Delete first book
    let resp = exec
        .execute(ClientRpcRequest::ContactsDeleteBook {
            book_id: book1_id.clone(),
        })
        .await
        .unwrap();
    match &resp {
        ClientRpcResponse::ContactsBookResult(r) => assert!(r.is_success),
        other => panic!("unexpected: {other:?}"),
    }

    // Verify only one book remains
    let resp = exec.execute(ClientRpcRequest::ContactsListBooks { limit: None }).await.unwrap();
    match &resp {
        ClientRpcResponse::ContactsBookListResult(r) => {
            assert!(r.is_success);
            assert_eq!(r.books.len(), 1);
            assert_eq!(r.books[0].id, book2_id);
        }
        other => panic!("unexpected: {other:?}"),
    }
}
