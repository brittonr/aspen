//! End-to-end integration tests for contacts.
//!
//! Tests the full contacts workflow: create book → add contacts → list →
//! search → export/import vCard roundtrip → groups.

use std::sync::Arc;

use aspen_contacts::ContactStore;
use aspen_contacts::parse_vcards;
use aspen_testing_core::DeterministicKeyValueStore;

async fn store() -> (ContactStore<DeterministicKeyValueStore>, Arc<DeterministicKeyValueStore>) {
    let kv = DeterministicKeyValueStore::new();
    let cs = ContactStore::new(kv.clone());
    (cs, kv)
}

// =========================================================================
// 8.1  End-to-end contacts
// =========================================================================

#[tokio::test]
async fn e2e_contacts_create_list_search_export_roundtrip() {
    let (cs, _kv) = store().await;
    let now = 1_700_000_000_000u64;

    // 1. Create an address book.
    let book = cs.create_book("Personal", Some("My contacts"), now).await.unwrap();
    assert!(!book.id.is_empty());
    assert_eq!(book.name, "Personal");

    // 2. Add contacts via vCard data.
    let vcard_alice =
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alice Smith\r\nEMAIL:alice@example.com\r\nTEL:+1-555-0100\r\nEND:VCARD\r\n";
    let vcard_bob = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bob Jones\r\nEMAIL:bob@example.com\r\nTEL:+1-555-0200\r\nORG:Acme Inc\r\nEND:VCARD\r\n";
    let vcard_carol = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Carol White\r\nEMAIL:carol@work.com\r\nEND:VCARD\r\n";

    let alice = cs.create_contact(&book.id, vcard_alice, now).await.unwrap();
    let bob = cs.create_contact(&book.id, vcard_bob, now + 1).await.unwrap();
    let carol = cs.create_contact(&book.id, vcard_carol, now + 2).await.unwrap();

    assert_eq!(alice.display_name, "Alice Smith");
    assert_eq!(bob.display_name, "Bob Jones");
    assert_eq!(carol.display_name, "Carol White");

    // 3. List contacts in the book.
    let (contacts, _token) = cs.list_contacts(&book.id, None, None).await.unwrap();
    assert_eq!(contacts.len(), 3);

    // 4. Search by email.
    let results = cs.search_contacts("alice@example", None, None).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].display_name, "Alice Smith");

    // 5. Search by phone number.
    let results = cs.search_contacts("555-0200", None, None).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].display_name, "Bob Jones");

    // 6. Search by name fragment.
    let results = cs.search_contacts("carol", None, None).await.unwrap();
    assert_eq!(results.len(), 1);

    // 7. Get single contact by ID.
    let got = cs.get_contact(&alice.id).await.unwrap();
    assert_eq!(got.display_name, "Alice Smith");

    // 8. Export to vCard, re-parse, verify roundtrip.
    let (exported, count) = cs.export_vcard(&book.id).await.unwrap();
    assert_eq!(count, 3);
    assert!(exported.contains("Alice Smith"));
    assert!(exported.contains("Bob Jones"));
    assert!(exported.contains("Carol White"));

    // Parse the exported vCard stream back.
    let parsed = parse_vcards(&exported).unwrap();
    assert_eq!(parsed.len(), 3);

    // 9. Delete a contact and verify count.
    cs.delete_contact(&bob.id).await.unwrap();
    let (contacts, _) = cs.list_contacts(&book.id, None, None).await.unwrap();
    assert_eq!(contacts.len(), 2);

    // 10. Update a contact.
    let updated_vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alice Jones\r\nEMAIL:alice.new@example.com\r\nEND:VCARD\r\n";
    let updated = cs.update_contact(&alice.id, updated_vcard, now + 10).await.unwrap();
    assert_eq!(updated.display_name, "Alice Jones");
}

#[tokio::test]
async fn e2e_contacts_import_vcard_bulk() {
    let (cs, _) = store().await;
    let now = 1_700_000_000_000u64;
    let book = cs.create_book("Imported", None, now).await.unwrap();

    // Build a multi-vCard string.
    let vcards = vec![
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Import One\r\nEMAIL:one@test.com\r\nEND:VCARD\r\n",
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Import Two\r\nEMAIL:two@test.com\r\nEND:VCARD\r\n",
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Import Three\r\nEMAIL:three@test.com\r\nEND:VCARD\r\n",
    ];
    let data = vcards.join("");

    let imported = cs.import_vcard(&book.id, &data, now).await.unwrap();
    assert_eq!(imported.len(), 3);

    let (contacts, _) = cs.list_contacts(&book.id, None, None).await.unwrap();
    assert_eq!(contacts.len(), 3);
}

#[tokio::test]
async fn e2e_contacts_multiple_books_search_isolation() {
    let (cs, _) = store().await;
    let now = 1_700_000_000_000u64;

    let work = cs.create_book("Work", None, now).await.unwrap();
    let personal = cs.create_book("Personal", None, now).await.unwrap();

    cs.create_contact(&work.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Dave Work\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();
    cs.create_contact(&personal.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Dave Personal\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();

    // Search across all books.
    let all = cs.search_contacts("Dave", None, None).await.unwrap();
    assert_eq!(all.len(), 2);

    // Search within specific book.
    let work_only = cs.search_contacts("Dave", Some(&work.id), None).await.unwrap();
    assert_eq!(work_only.len(), 1);
    assert_eq!(work_only[0].display_name, "Dave Work");

    // List books.
    let books = cs.list_books(None).await.unwrap();
    assert_eq!(books.len(), 2);
}

// =========================================================================
// 8.4  Contact groups
// =========================================================================

#[tokio::test]
async fn e2e_contact_groups_full_lifecycle() {
    let (cs, _) = store().await;
    let now = 1_700_000_000_000u64;

    // Setup book and contacts.
    let book = cs.create_book("Groups Test", None, now).await.unwrap();
    let alice = cs
        .create_contact(&book.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alice\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();
    let bob = cs
        .create_contact(&book.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bob\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();
    let carol = cs
        .create_contact(&book.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Carol\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();

    // 1. Create a group.
    let group = cs.create_group(&book.id, "Friends", vec![]).await.unwrap();
    assert_eq!(group.name, "Friends");
    assert!(group.member_ids.is_empty());

    // 2. Add members to the group.
    let g = cs.add_to_group(&group.id, &alice.id).await.unwrap();
    assert_eq!(g.member_ids.len(), 1);

    let g = cs.add_to_group(&group.id, &bob.id).await.unwrap();
    assert_eq!(g.member_ids.len(), 2);

    let g = cs.add_to_group(&group.id, &carol.id).await.unwrap();
    assert_eq!(g.member_ids.len(), 3);

    // 3. List groups.
    let groups = cs.list_groups(&book.id).await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "Friends");
    assert_eq!(groups[0].member_ids.len(), 3);

    // 4. Remove a member.
    let g = cs.remove_from_group(&group.id, &bob.id).await.unwrap();
    assert_eq!(g.member_ids.len(), 2);
    assert!(!g.member_ids.contains(&bob.id));
    assert!(g.member_ids.contains(&alice.id));
    assert!(g.member_ids.contains(&carol.id));

    // 5. Delete group.
    cs.delete_group(&group.id).await.unwrap();
    let groups = cs.list_groups(&book.id).await.unwrap();
    assert!(groups.is_empty());
}

#[tokio::test]
async fn e2e_contact_group_create_with_initial_members() {
    let (cs, _) = store().await;
    let now = 1_700_000_000_000u64;

    let book = cs.create_book("Init Members", None, now).await.unwrap();
    let a = cs
        .create_contact(&book.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alpha\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();
    let b = cs
        .create_contact(&book.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bravo\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();

    let group = cs.create_group(&book.id, "Crew", vec![a.id.clone(), b.id.clone()]).await.unwrap();
    assert_eq!(group.member_ids.len(), 2);
    assert!(group.member_ids.contains(&a.id));
    assert!(group.member_ids.contains(&b.id));
}

#[tokio::test]
async fn e2e_delete_book_cascade() {
    let (cs, _) = store().await;
    let now = 1_700_000_000_000u64;

    let book = cs.create_book("Ephemeral", None, now).await.unwrap();
    cs.create_contact(&book.id, "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Gone\r\nEND:VCARD\r\n", now)
        .await
        .unwrap();
    cs.create_group(&book.id, "G1", vec![]).await.unwrap();

    // Delete book should cascade.
    cs.delete_book(&book.id).await.unwrap();

    // Book should be gone.
    assert!(cs.get_book(&book.id).await.is_err());
    // List should be empty.
    let books = cs.list_books(None).await.unwrap();
    assert!(books.is_empty());
}
