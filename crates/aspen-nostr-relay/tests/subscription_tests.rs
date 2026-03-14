//! Tests for the subscription registry.

use std::sync::Arc;

use aspen_nostr_relay::SubscriptionRegistry;
use nostr::prelude::*;

#[tokio::test]
async fn test_add_and_remove_connection() {
    let registry = SubscriptionRegistry::new(16);
    let _rx = registry.add_connection(1).await;

    // Should be able to subscribe
    let result = registry.subscribe(1, SubscriptionId::new("sub1"), vec![Filter::new()]).await;
    assert!(result.is_ok());

    // Remove connection clears subscriptions
    registry.remove_connection(1).await;

    let subs = registry.get_connection_subscriptions(1).await;
    assert!(subs.is_empty());
}

#[tokio::test]
async fn test_subscribe_and_unsubscribe() {
    let registry = SubscriptionRegistry::new(16);
    let _rx = registry.add_connection(1).await;

    registry.subscribe(1, SubscriptionId::new("sub1"), vec![Filter::new()]).await.unwrap();

    let filters = registry.get_filters(1, &SubscriptionId::new("sub1")).await;
    assert!(filters.is_some());

    registry.unsubscribe(1, &SubscriptionId::new("sub1")).await;

    let filters = registry.get_filters(1, &SubscriptionId::new("sub1")).await;
    assert!(filters.is_none());
}

#[tokio::test]
async fn test_subscription_replacement() {
    let registry = SubscriptionRegistry::new(16);
    let _rx = registry.add_connection(1).await;

    // Subscribe with kind filter
    registry
        .subscribe(1, SubscriptionId::new("sub1"), vec![Filter::new().kind(Kind::TextNote)])
        .await
        .unwrap();

    // Replace with different filter
    registry
        .subscribe(1, SubscriptionId::new("sub1"), vec![Filter::new().kind(Kind::Custom(30617))])
        .await
        .unwrap();

    // Should still have exactly 1 subscription
    let subs = registry.get_connection_subscriptions(1).await;
    assert_eq!(subs.len(), 1);
}

#[tokio::test]
async fn test_subscription_limit_enforced() {
    let registry = SubscriptionRegistry::new(2);
    let _rx = registry.add_connection(1).await;

    // First two subscriptions should work
    registry.subscribe(1, SubscriptionId::new("sub1"), vec![Filter::new()]).await.unwrap();
    registry.subscribe(1, SubscriptionId::new("sub2"), vec![Filter::new()]).await.unwrap();

    // Third should fail
    let result = registry.subscribe(1, SubscriptionId::new("sub3"), vec![Filter::new()]).await;
    assert!(result.is_err());

    // Replacement of existing should still work at limit
    let result = registry.subscribe(1, SubscriptionId::new("sub1"), vec![Filter::new()]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_broadcast_delivers_to_receiver() {
    let registry = SubscriptionRegistry::new(16);
    let mut rx = registry.add_connection(1).await;

    let keys = Keys::generate();
    let event = EventBuilder::text_note("broadcast test").sign_with_keys(&keys).unwrap();

    registry.broadcast_event(Arc::new(event.clone()));

    let received = rx.recv().await.unwrap();
    assert_eq!(received.id, event.id);
}

#[tokio::test]
async fn test_get_connection_subscriptions() {
    let registry = SubscriptionRegistry::new(16);
    let _rx = registry.add_connection(1).await;

    registry.subscribe(1, SubscriptionId::new("a"), vec![Filter::new()]).await.unwrap();
    registry.subscribe(1, SubscriptionId::new("b"), vec![Filter::new()]).await.unwrap();

    let subs = registry.get_connection_subscriptions(1).await;
    assert_eq!(subs.len(), 2);
}
