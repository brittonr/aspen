//! Tests for KV-backed Nostr event storage.

use aspen_nostr_relay::storage::KvEventStore;
use aspen_nostr_relay::storage::NostrEventStore;
use aspen_testing_core::DeterministicKeyValueStore;
use nostr::prelude::*;

fn make_store() -> KvEventStore<aspen_testing_core::DeterministicKeyValueStore> {
    let kv = DeterministicKeyValueStore::new();
    KvEventStore::new(kv)
}

fn sign_event(keys: &Keys, builder: EventBuilder) -> Event {
    builder.sign_with_keys(keys).unwrap()
}

#[tokio::test]
async fn test_store_and_retrieve() {
    let store = make_store();
    let keys = Keys::generate();
    let event = sign_event(&keys, EventBuilder::text_note("hello"));

    let is_new = store.store_event(&event).await.unwrap();
    assert!(is_new);

    let retrieved = store.get_event(&event.id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, event.id);
    assert_eq!(retrieved.content, "hello");
}

#[tokio::test]
async fn test_duplicate_event_returns_false() {
    let store = make_store();
    let keys = Keys::generate();
    let event = sign_event(&keys, EventBuilder::text_note("dup test"));

    assert!(store.store_event(&event).await.unwrap());
    assert!(!store.store_event(&event).await.unwrap());
}

#[tokio::test]
async fn test_event_count() {
    let store = make_store();
    let keys = Keys::generate();

    assert_eq!(store.event_count().await.unwrap(), 0);

    let e1 = sign_event(&keys, EventBuilder::text_note("one"));
    store.store_event(&e1).await.unwrap();
    assert_eq!(store.event_count().await.unwrap(), 1);

    let e2 = sign_event(&keys, EventBuilder::text_note("two"));
    store.store_event(&e2).await.unwrap();
    assert_eq!(store.event_count().await.unwrap(), 2);
}

#[tokio::test]
async fn test_delete_event() {
    let store = make_store();
    let keys = Keys::generate();
    let event = sign_event(&keys, EventBuilder::text_note("delete me"));

    store.store_event(&event).await.unwrap();
    assert_eq!(store.event_count().await.unwrap(), 1);

    store.delete_event(&event.id).await.unwrap();
    assert_eq!(store.event_count().await.unwrap(), 0);

    let retrieved = store.get_event(&event.id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_get_nonexistent_event() {
    let store = make_store();
    let fake_id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    let result = store.get_event(&fake_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_query_by_kind() {
    let store = make_store();
    let keys = Keys::generate();

    // Store a text note (kind 1)
    let note = sign_event(&keys, EventBuilder::text_note("a note"));
    store.store_event(&note).await.unwrap();

    // Store a kind 30617 event (repo announcement)
    let repo = sign_event(&keys, EventBuilder::new(Kind::Custom(30617), "repo data"));
    store.store_event(&repo).await.unwrap();

    // Query for kind 30617 only
    let results = store.query_events(&[Filter::new().kind(Kind::Custom(30617))]).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, repo.id);
}

#[tokio::test]
async fn test_query_by_author() {
    let store = make_store();
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();

    let e1 = sign_event(&keys1, EventBuilder::text_note("from author 1"));
    let e2 = sign_event(&keys2, EventBuilder::text_note("from author 2"));
    store.store_event(&e1).await.unwrap();
    store.store_event(&e2).await.unwrap();

    let results = store.query_events(&[Filter::new().author(keys1.public_key())]).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].pubkey, keys1.public_key());
}

#[tokio::test]
async fn test_query_by_event_ids() {
    let store = make_store();
    let keys = Keys::generate();

    let e1 = sign_event(&keys, EventBuilder::text_note("one"));
    let e2 = sign_event(&keys, EventBuilder::text_note("two"));
    let e3 = sign_event(&keys, EventBuilder::text_note("three"));
    store.store_event(&e1).await.unwrap();
    store.store_event(&e2).await.unwrap();
    store.store_event(&e3).await.unwrap();

    let results = store.query_events(&[Filter::new().id(e1.id).id(e3.id)]).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_query_results_sorted_newest_first() {
    let store = make_store();
    let keys = Keys::generate();

    // Store multiple events (timestamps auto-assigned by nostr crate)
    let mut events = Vec::new();
    for i in 0..5 {
        let e = sign_event(&keys, EventBuilder::text_note(format!("event {i}")));
        store.store_event(&e).await.unwrap();
        events.push(e);
        // Small delay to ensure different timestamps
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let results = store.query_events(&[Filter::new().author(keys.public_key())]).await.unwrap();

    // Verify descending order
    for i in 1..results.len() {
        assert!(results[i - 1].created_at >= results[i].created_at);
    }
}

#[tokio::test]
async fn test_query_with_limit() {
    let store = make_store();
    let keys = Keys::generate();

    for i in 0..10 {
        let e = sign_event(&keys, EventBuilder::text_note(format!("event {i}")));
        store.store_event(&e).await.unwrap();
    }

    let results = store.query_events(&[Filter::new().author(keys.public_key()).limit(3)]).await.unwrap();
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_query_or_across_filters() {
    let store = make_store();
    let keys1 = Keys::generate();
    let keys2 = Keys::generate();

    let e1 = sign_event(&keys1, EventBuilder::text_note("from 1"));
    let e2 = sign_event(&keys2, EventBuilder::text_note("from 2"));
    store.store_event(&e1).await.unwrap();
    store.store_event(&e2).await.unwrap();

    // OR: author1 OR author2
    let results = store
        .query_events(&[
            Filter::new().author(keys1.public_key()),
            Filter::new().author(keys2.public_key()),
        ])
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_replaceable_event_kind_0() {
    let store = make_store();
    let keys = Keys::generate();

    // Kind 0 (metadata) is replaceable per pubkey+kind
    let e1 = sign_event(&keys, EventBuilder::new(Kind::Metadata, r#"{"name":"old"}"#));
    store.store_event(&e1).await.unwrap();
    assert_eq!(store.event_count().await.unwrap(), 1);

    let e2 = sign_event(&keys, EventBuilder::new(Kind::Metadata, r#"{"name":"new"}"#));
    store.store_event(&e2).await.unwrap();

    // Should have replaced the old one
    assert_eq!(store.event_count().await.unwrap(), 1);

    let old = store.get_event(&e1.id).await.unwrap();
    assert!(old.is_none(), "old event should be deleted");

    let new = store.get_event(&e2.id).await.unwrap();
    assert!(new.is_some(), "new event should exist");
}

#[tokio::test]
async fn test_parameterized_replaceable_event() {
    let store = make_store();
    let keys = Keys::generate();

    // Kind 30617 is parameterized replaceable — keyed by (pubkey, kind, d-tag)
    let e1 = sign_event(
        &keys,
        EventBuilder::new(Kind::Custom(30617), "repo v1")
            .tag(Tag::custom(TagKind::SingleLetter(SingleLetterTag::lowercase(Alphabet::D)), ["my-repo"])),
    );
    store.store_event(&e1).await.unwrap();

    let e2 = sign_event(
        &keys,
        EventBuilder::new(Kind::Custom(30617), "repo v2")
            .tag(Tag::custom(TagKind::SingleLetter(SingleLetterTag::lowercase(Alphabet::D)), ["my-repo"])),
    );
    store.store_event(&e2).await.unwrap();

    // Should have replaced e1
    assert_eq!(store.event_count().await.unwrap(), 1);
    let old = store.get_event(&e1.id).await.unwrap();
    assert!(old.is_none());
    let new = store.get_event(&e2.id).await.unwrap();
    assert!(new.is_some());

    // Different d-tag should coexist
    let e3 = sign_event(
        &keys,
        EventBuilder::new(Kind::Custom(30617), "other repo")
            .tag(Tag::custom(TagKind::SingleLetter(SingleLetterTag::lowercase(Alphabet::D)), ["other-repo"])),
    );
    store.store_event(&e3).await.unwrap();
    assert_eq!(store.event_count().await.unwrap(), 2);
}

#[tokio::test]
async fn test_delete_nonexistent_is_ok() {
    let store = make_store();
    let fake_id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    // Should not error
    store.delete_event(&fake_id).await.unwrap();
}
