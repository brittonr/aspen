//! Integration test: start relay, connect via WebSocket, publish, subscribe, receive.

use std::sync::Arc;
use std::time::Duration;

use aspen_nostr_relay::NostrIdentity;
use aspen_nostr_relay::NostrRelayConfig;
use aspen_nostr_relay::NostrRelayService;
use aspen_testing_core::DeterministicKeyValueStore;
use futures::SinkExt;
use futures::StreamExt;
use nostr::prelude::*;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Find a free port by binding to :0 and releasing.
async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

/// Read and discard the NIP-42 AUTH challenge sent on connect.
async fn drain_auth_challenge(
    ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
) {
    let msg = timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("timeout waiting for AUTH")
        .expect("stream ended")
        .expect("ws error");
    if let Message::Text(text) = msg {
        let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
        assert!(matches!(relay_msg, RelayMessage::Auth { .. }), "expected AUTH challenge, got: {relay_msg:?}");
    } else {
        panic!("expected text message for AUTH, got: {msg:?}");
    }
}

async fn start_relay() -> (
    NostrRelayConfig,
    NostrIdentity,
    Arc<NostrRelayService<aspen_testing_core::DeterministicKeyValueStore>>,
) {
    let port = free_port().await;
    let config = NostrRelayConfig {
        enabled: true,
        bind_addr: "127.0.0.1".into(),
        bind_port: port,
        ..Default::default()
    };
    let identity = NostrIdentity::generate();
    let kv = DeterministicKeyValueStore::new();
    let service = Arc::new(NostrRelayService::new(config.clone(), identity.clone(), kv));

    let svc = Arc::clone(&service);
    tokio::spawn(async move {
        let _ = svc.run().await;
    });

    // Wait for listener to be ready
    tokio::time::sleep(Duration::from_millis(50)).await;

    (config, identity, service)
}

#[tokio::test]
async fn test_publish_and_query() {
    let (config, _identity, service) = start_relay().await;
    let url = format!("ws://127.0.0.1:{}", config.bind_port);

    // Connect client
    let (mut ws, _) = connect_async(&url).await.unwrap();
    drain_auth_challenge(&mut ws).await;

    // Create and publish an event
    let keys = Keys::generate();
    let event = EventBuilder::text_note("integration test event").sign_with_keys(&keys).unwrap();

    let event_msg = ClientMessage::Event(std::borrow::Cow::Borrowed(&event));
    ws.send(Message::Text(event_msg.as_json().into())).await.unwrap();

    // Read OK response
    let resp = timeout(Duration::from_secs(5), ws.next()).await.unwrap().unwrap().unwrap();

    if let Message::Text(text) = resp {
        let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
        match relay_msg {
            RelayMessage::Ok { event_id, status, .. } => {
                assert!(status, "event should be accepted");
                assert_eq!(event_id, event.id);
            }
            other => panic!("expected OK, got: {other:?}"),
        }
    }

    // Subscribe and query the event back
    let sub_id = SubscriptionId::new("test-sub");
    let filter = Filter::new().kind(Kind::TextNote);
    let req = ClientMessage::Req {
        subscription_id: std::borrow::Cow::Borrowed(&sub_id),
        filters: vec![std::borrow::Cow::Borrowed(&filter)],
    };
    ws.send(Message::Text(req.as_json().into())).await.unwrap();

    // Read EVENT response
    let resp = timeout(Duration::from_secs(5), ws.next()).await.unwrap().unwrap().unwrap();

    if let Message::Text(text) = resp {
        let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
        match relay_msg {
            RelayMessage::Event {
                subscription_id,
                event: returned_event,
            } => {
                assert_eq!(*subscription_id, sub_id);
                assert_eq!(returned_event.id, event.id);
            }
            other => panic!("expected EVENT, got: {other:?}"),
        }
    }

    // Read EOSE
    let resp = timeout(Duration::from_secs(5), ws.next()).await.unwrap().unwrap().unwrap();

    if let Message::Text(text) = resp {
        let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
        assert!(matches!(relay_msg, RelayMessage::EndOfStoredEvents(_)), "expected EOSE");
    }

    assert_eq!(service.active_connections(), 1);

    // Close
    let close = ClientMessage::Close(std::borrow::Cow::Borrowed(&sub_id));
    ws.send(Message::Text(close.as_json().into())).await.unwrap();

    ws.close(None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    service.shutdown();
}

#[tokio::test]
async fn test_invalid_signature_rejected() {
    let (config, _identity, _service) = start_relay().await;
    let url = format!("ws://127.0.0.1:{}", config.bind_port);

    let (mut ws, _) = connect_async(&url).await.unwrap();
    drain_auth_challenge(&mut ws).await;

    // Craft an event with wrong signature (sign with one key, set pubkey of another)
    let keys1 = Keys::generate();
    let event = EventBuilder::text_note("tampered").sign_with_keys(&keys1).unwrap();

    // Tamper with content to invalidate ID/sig
    let json = event.as_json();
    let tampered = json.replace("tampered", "modified");

    // Send raw tampered JSON as EVENT message
    let raw = format!(r#"["EVENT",{}]"#, tampered);
    ws.send(Message::Text(raw.into())).await.unwrap();

    let resp = timeout(Duration::from_secs(5), ws.next()).await.unwrap().unwrap().unwrap();

    if let Message::Text(text) = resp {
        let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
        match relay_msg {
            RelayMessage::Ok { status, .. } => {
                assert!(!status, "tampered event should be rejected");
            }
            other => panic!("expected OK(false), got: {other:?}"),
        }
    }

    _service.shutdown();
}

#[tokio::test]
async fn test_real_time_subscription_push() {
    let (config, _identity, service) = start_relay().await;
    let url = format!("ws://127.0.0.1:{}", config.bind_port);

    // Client 1: subscriber
    let (mut ws1, _) = connect_async(&url).await.unwrap();
    drain_auth_challenge(&mut ws1).await;
    let sub_id = SubscriptionId::new("realtime");
    let filter = Filter::new().kind(Kind::TextNote);
    let req = ClientMessage::Req {
        subscription_id: std::borrow::Cow::Borrowed(&sub_id),
        filters: vec![std::borrow::Cow::Borrowed(&filter)],
    };
    ws1.send(Message::Text(req.as_json().into())).await.unwrap();

    // Read EOSE (no stored events)
    let resp = timeout(Duration::from_secs(5), ws1.next()).await.unwrap().unwrap().unwrap();
    if let Message::Text(text) = resp {
        assert!(text.contains("EOSE"));
    }

    // Client 2: publisher
    let (mut ws2, _) = connect_async(&url).await.unwrap();
    drain_auth_challenge(&mut ws2).await;
    let keys = Keys::generate();
    let event = EventBuilder::text_note("real-time push test").sign_with_keys(&keys).unwrap();
    let event_msg = ClientMessage::Event(std::borrow::Cow::Borrowed(&event));
    ws2.send(Message::Text(event_msg.as_json().into())).await.unwrap();

    // Client 2 reads OK
    let _ = timeout(Duration::from_secs(5), ws2.next()).await;

    // Client 1 should receive the event via real-time push
    let resp = timeout(Duration::from_secs(5), ws1.next()).await.unwrap().unwrap().unwrap();

    if let Message::Text(text) = resp {
        let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
        match relay_msg {
            RelayMessage::Event {
                event: pushed_event, ..
            } => {
                assert_eq!(pushed_event.id, event.id);
            }
            other => panic!("expected pushed EVENT, got: {other:?}"),
        }
    }

    service.shutdown();
}

#[tokio::test]
async fn test_connection_count() {
    let (config, _identity, service) = start_relay().await;
    let url = format!("ws://127.0.0.1:{}", config.bind_port);

    assert_eq!(service.active_connections(), 0);

    let (ws1, _) = connect_async(&url).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(service.active_connections(), 1);

    let (ws2, _) = connect_async(&url).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(service.active_connections(), 2);

    drop(ws1);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(service.active_connections(), 1);

    drop(ws2);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(service.active_connections(), 0);

    service.shutdown();
}

#[tokio::test]
async fn test_relay_info_json() {
    let identity = NostrIdentity::generate();
    let kv = DeterministicKeyValueStore::new();
    let config = NostrRelayConfig::default();
    let service = NostrRelayService::new(config, identity.clone(), kv);

    let info = service.relay_info_json();
    let parsed: serde_json::Value = serde_json::from_str(&info).unwrap();

    assert_eq!(parsed["name"], "aspen-nostr-relay");
    assert_eq!(parsed["pubkey"], identity.public_key_hex());
    assert_eq!(parsed["supported_nips"], serde_json::json!([1, 11, 34, 42]));
    assert_eq!(parsed["software"], "aspen");
}
