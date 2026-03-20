//! Integration test for ephemeral pub/sub over real Iroh QUIC connections.
//!
//! Spawns two Iroh endpoints, registers the EphemeralProtocolHandler on one,
//! connects from the other, and verifies end-to-end event delivery.

use std::sync::Arc;
use std::time::Duration;

use aspen_hooks::pubsub::Cursor;
use aspen_hooks::pubsub::ephemeral::broker::EphemeralBroker;
use aspen_hooks::pubsub::ephemeral::handler::EPHEMERAL_ALPN;
use aspen_hooks::pubsub::ephemeral::handler::EphemeralProtocolHandler;
use aspen_hooks::pubsub::ephemeral::wire::EphemeralSubscribeRequest;
use aspen_hooks::pubsub::ephemeral::wire::EphemeralWireEvent;
use aspen_hooks::pubsub::ephemeral::wire::MAX_EPHEMERAL_WIRE_MESSAGE_SIZE;
use aspen_hooks::pubsub::ephemeral::wire::read_frame;
use aspen_hooks::pubsub::ephemeral::wire::write_frame;
use aspen_hooks::pubsub::event::Event;
use aspen_hooks::pubsub::topic::Topic;
use iroh::SecretKey;

/// Create an isolated Iroh endpoint for testing.
async fn make_endpoint(seed: u64) -> iroh::Endpoint {
    let mut key_bytes = [0u8; 32];
    key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
    let secret_key = SecretKey::from_bytes(&key_bytes);

    let addr = "127.0.0.1:0".parse().unwrap();
    iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret_key)
        .bind_addr(addr)
        .bind()
        .await
        .expect("failed to create endpoint")
}

#[tokio::test]
async fn test_ephemeral_pubsub_end_to_end() {
    // === Server side ===
    let broker = Arc::new(EphemeralBroker::new());
    let handler = EphemeralProtocolHandler::new(Arc::clone(&broker));

    let server_ep = make_endpoint(1).await;
    let _router = iroh::protocol::Router::builder(server_ep.clone()).accept(EPHEMERAL_ALPN, handler).spawn();

    let server_addr = server_ep.addr();

    // === Client side ===
    let client_ep = make_endpoint(2).await;

    // Connect to server with ephemeral ALPN
    let conn = client_ep.connect(server_addr, EPHEMERAL_ALPN).await.expect("failed to connect");

    // Open bi-stream and send subscribe request
    let (mut send, mut recv) = conn.open_bi().await.expect("failed to open bi-stream");

    let request = EphemeralSubscribeRequest {
        pattern: "orders.*".to_string(),
        buffer_size: 64,
    };
    write_frame(&mut send, &request).await.expect("failed to send subscribe request");

    // Give the server a moment to register the subscription
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify subscription registered
    assert_eq!(broker.subscription_count().await, 1);

    // === Publish an event on the server side ===
    let topic = Topic::new("orders.created").unwrap();
    let event = Event {
        topic: topic.clone(),
        cursor: Cursor::EPHEMERAL,
        timestamp_ms: 1704067200000,
        payload: b"order-123".to_vec(),
        headers: vec![("trace-id".to_string(), b"xyz".to_vec())],
    };
    broker.publish(&topic, event).await;

    // === Read event on client side ===
    let wire_event: EphemeralWireEvent =
        tokio::time::timeout(Duration::from_secs(5), read_frame(&mut recv, MAX_EPHEMERAL_WIRE_MESSAGE_SIZE))
            .await
            .expect("timed out waiting for event")
            .expect("failed to read event");

    assert_eq!(wire_event.topic, "orders.created");
    assert_eq!(wire_event.timestamp_ms, 1704067200000);
    assert_eq!(wire_event.payload, b"order-123");
    assert_eq!(wire_event.headers.len(), 1);
    assert_eq!(wire_event.headers[0].0, "trace-id");
    assert_eq!(wire_event.headers[0].1, b"xyz");

    // === Publish non-matching event (should NOT be delivered) ===
    let other_topic = Topic::new("users.created").unwrap();
    let other_event = Event {
        topic: other_topic.clone(),
        cursor: Cursor::EPHEMERAL,
        timestamp_ms: 1704067200001,
        payload: b"user-456".to_vec(),
        headers: vec![],
    };
    broker.publish(&other_topic, other_event).await;

    // === Publish another matching event ===
    let event2 = Event {
        topic: topic.clone(),
        cursor: Cursor::EPHEMERAL,
        timestamp_ms: 1704067200002,
        payload: b"order-456".to_vec(),
        headers: vec![],
    };
    broker.publish(&topic, event2).await;

    let wire_event2: EphemeralWireEvent =
        tokio::time::timeout(Duration::from_secs(5), read_frame(&mut recv, MAX_EPHEMERAL_WIRE_MESSAGE_SIZE))
            .await
            .expect("timed out waiting for second event")
            .expect("failed to read second event");

    assert_eq!(wire_event2.payload, b"order-456");

    // === Disconnect and verify cleanup ===
    drop(send);
    drop(recv);
    drop(conn);

    // Wait for server to notice disconnection and clean up
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(broker.subscription_count().await, 0);
}

#[tokio::test]
async fn test_ephemeral_pubsub_multiple_subscribers() {
    let broker = Arc::new(EphemeralBroker::new());
    let handler = EphemeralProtocolHandler::new(Arc::clone(&broker));

    let server_ep = make_endpoint(10).await;
    let _router = iroh::protocol::Router::builder(server_ep.clone()).accept(EPHEMERAL_ALPN, handler).spawn();

    let server_addr = server_ep.addr();

    // Connect two clients
    let client1_ep = make_endpoint(11).await;
    let conn1 = client1_ep.connect(server_addr.clone(), EPHEMERAL_ALPN).await.unwrap();
    let (mut send1, mut recv1) = conn1.open_bi().await.unwrap();

    let client2_ep = make_endpoint(12).await;
    let conn2 = client2_ep.connect(server_addr, EPHEMERAL_ALPN).await.unwrap();
    let (mut send2, mut recv2) = conn2.open_bi().await.unwrap();

    // Both subscribe to the same pattern
    let req = EphemeralSubscribeRequest {
        pattern: "events.>".to_string(),
        buffer_size: 32,
    };
    write_frame(&mut send1, &req).await.unwrap();
    write_frame(&mut send2, &req).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(broker.subscription_count().await, 2);

    // Publish one event
    let topic = Topic::new("events.important").unwrap();
    let event = Event {
        topic: topic.clone(),
        cursor: Cursor::EPHEMERAL,
        timestamp_ms: 100,
        payload: b"data".to_vec(),
        headers: vec![],
    };
    broker.publish(&topic, event).await;

    // Both should receive it
    let e1: EphemeralWireEvent =
        tokio::time::timeout(Duration::from_secs(5), read_frame(&mut recv1, MAX_EPHEMERAL_WIRE_MESSAGE_SIZE))
            .await
            .unwrap()
            .unwrap();

    let e2: EphemeralWireEvent =
        tokio::time::timeout(Duration::from_secs(5), read_frame(&mut recv2, MAX_EPHEMERAL_WIRE_MESSAGE_SIZE))
            .await
            .unwrap()
            .unwrap();

    assert_eq!(e1.payload, b"data");
    assert_eq!(e2.payload, b"data");

    // Disconnect client 1, client 2 should still work
    drop(send1);
    drop(recv1);
    drop(conn1);

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(broker.subscription_count().await, 1);
}
