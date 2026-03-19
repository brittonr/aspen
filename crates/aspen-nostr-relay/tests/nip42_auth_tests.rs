//! NIP-42 integration tests — write policy enforcement and auth flows.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use aspen_nostr_relay::NostrEventStore;
use aspen_nostr_relay::NostrIdentity;
use aspen_nostr_relay::NostrRelayConfig;
use aspen_nostr_relay::NostrRelayService;
use aspen_nostr_relay::WritePolicy;
use aspen_testing_core::DeterministicKeyValueStore;
use futures::SinkExt;
use futures::StreamExt;
use nostr::prelude::*;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

async fn start_relay_with_policy(
    write_policy: WritePolicy,
    relay_url: Option<String>,
) -> (u16, Arc<NostrRelayService<DeterministicKeyValueStore>>) {
    let port = free_port().await;
    let config = NostrRelayConfig {
        enabled: true,
        bind_addr: "127.0.0.1".into(),
        bind_port: port,
        write_policy,
        relay_url,
        ..Default::default()
    };
    let identity = NostrIdentity::generate();
    let kv = DeterministicKeyValueStore::new();
    let service = Arc::new(NostrRelayService::new(config, identity, kv));

    let svc = Arc::clone(&service);
    tokio::spawn(async move {
        let _ = svc.run().await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    (port, service)
}

type WsStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Read the AUTH challenge from the relay and return the challenge string.
async fn read_auth_challenge(ws: &mut WsStream) -> String {
    let msg = timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("timeout")
        .expect("stream ended")
        .expect("ws error");
    if let Message::Text(text) = msg {
        let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
        match relay_msg {
            RelayMessage::Auth { challenge } => challenge.into_owned(),
            other => panic!("expected AUTH, got: {other:?}"),
        }
    } else {
        panic!("expected text message");
    }
}

/// Read the next relay text message, skipping pings.
async fn read_relay_msg(ws: &mut WsStream) -> RelayMessage<'static> {
    loop {
        let msg = timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timeout")
            .expect("stream ended")
            .expect("ws error");
        if let Message::Text(text) = msg {
            let relay_msg: RelayMessage<'_> = RelayMessage::from_json(&text).unwrap();
            // Convert to owned to avoid lifetime issues
            return match relay_msg {
                RelayMessage::Ok {
                    event_id,
                    status,
                    message,
                } => RelayMessage::Ok {
                    event_id,
                    status,
                    message: Cow::Owned(message.into_owned()),
                },
                RelayMessage::Auth { challenge } => RelayMessage::Auth {
                    challenge: Cow::Owned(challenge.into_owned()),
                },
                RelayMessage::EndOfStoredEvents(sub_id) => {
                    RelayMessage::EndOfStoredEvents(Cow::Owned(sub_id.into_owned()))
                }
                RelayMessage::Event { subscription_id, event } => RelayMessage::Event {
                    subscription_id: Cow::Owned(subscription_id.into_owned()),
                    event: Cow::Owned(event.into_owned()),
                },
                RelayMessage::Notice(msg) => RelayMessage::Notice(Cow::Owned(msg.into_owned())),
                RelayMessage::Closed {
                    subscription_id,
                    message,
                } => RelayMessage::Closed {
                    subscription_id: Cow::Owned(subscription_id.into_owned()),
                    message: Cow::Owned(message.into_owned()),
                },
                other => panic!("unexpected relay message type: {other:?}"),
            };
        }
        // Skip non-text messages (pings etc)
    }
}

/// Build a kind 22242 auth event for NIP-42.
fn build_nip42_auth_event(keys: &Keys, challenge: &str, relay_url: &str) -> Event {
    let tags = vec![
        Tag::custom(TagKind::custom("relay"), [relay_url]),
        Tag::custom(TagKind::custom("challenge"), [challenge]),
    ];
    EventBuilder::new(Kind::Custom(22242), "").tags(tags).sign_with_keys(keys).expect("signing failed")
}

/// Send an EVENT and return the OK response.
async fn send_event_and_read_ok(ws: &mut WsStream, event: &Event) -> (bool, String) {
    let msg = ClientMessage::Event(Cow::Borrowed(event));
    ws.send(Message::Text(msg.as_json().into())).await.unwrap();
    let relay_msg = read_relay_msg(ws).await;
    match relay_msg {
        RelayMessage::Ok { status, message, .. } => (status, message.into_owned()),
        other => panic!("expected OK, got: {other:?}"),
    }
}

// =========================================================================
// 5.1: Open policy allows unauthenticated writes
// =========================================================================

#[tokio::test]
async fn open_policy_accepts_unauthenticated_event() {
    let (port, service) = start_relay_with_policy(WritePolicy::Open, None).await;
    let url = format!("ws://127.0.0.1:{port}");

    let (mut ws, _) = connect_async(&url).await.unwrap();
    let _challenge = read_auth_challenge(&mut ws).await;

    // Send event without authenticating
    let keys = Keys::generate();
    let event = EventBuilder::text_note("open policy test").sign_with_keys(&keys).unwrap();
    let (status, _) = send_event_and_read_ok(&mut ws, &event).await;
    assert!(status, "open policy should accept unauthenticated events");

    service.shutdown();
}

// =========================================================================
// 5.2: AuthRequired rejects unauthenticated writes
// =========================================================================

#[tokio::test]
async fn auth_required_rejects_unauthenticated_event() {
    let (port, service) = start_relay_with_policy(WritePolicy::AuthRequired, None).await;
    let url = format!("ws://127.0.0.1:{port}");

    let (mut ws, _) = connect_async(&url).await.unwrap();
    let _challenge = read_auth_challenge(&mut ws).await;

    let keys = Keys::generate();
    let event = EventBuilder::text_note("should be rejected").sign_with_keys(&keys).unwrap();
    let (status, message) = send_event_and_read_ok(&mut ws, &event).await;
    assert!(!status, "auth_required should reject unauthenticated events");
    assert!(message.contains("auth-required"), "message should contain 'auth-required', got: {message}");

    service.shutdown();
}

// =========================================================================
// 5.3: AuthRequired accepts authenticated writes
// =========================================================================

#[tokio::test]
async fn auth_required_accepts_after_nip42_auth() {
    let relay_url = "wss://test.relay.example.com";
    let (port, service) = start_relay_with_policy(WritePolicy::AuthRequired, Some(relay_url.to_string())).await;
    let url = format!("ws://127.0.0.1:{port}");

    let (mut ws, _) = connect_async(&url).await.unwrap();
    let challenge = read_auth_challenge(&mut ws).await;

    // Complete NIP-42 auth
    let keys = Keys::generate();
    let auth_event = build_nip42_auth_event(&keys, &challenge, relay_url);
    let auth_msg = ClientMessage::Auth(Cow::Borrowed(&auth_event));
    ws.send(Message::Text(auth_msg.as_json().into())).await.unwrap();

    let auth_resp = read_relay_msg(&mut ws).await;
    match auth_resp {
        RelayMessage::Ok { status, .. } => assert!(status, "NIP-42 auth should succeed"),
        other => panic!("expected OK for auth, got: {other:?}"),
    }

    // Now send an event — should be accepted
    let event = EventBuilder::text_note("authed write").sign_with_keys(&keys).unwrap();
    let (status, _) = send_event_and_read_ok(&mut ws, &event).await;
    assert!(status, "authenticated client should be able to write");

    service.shutdown();
}

// =========================================================================
// 5.4: ReadOnly rejects writes even after auth
// =========================================================================

#[tokio::test]
async fn read_only_rejects_event_after_auth() {
    let relay_url = "wss://test.relay.example.com";
    let (port, service) = start_relay_with_policy(WritePolicy::ReadOnly, Some(relay_url.to_string())).await;
    let url = format!("ws://127.0.0.1:{port}");

    let (mut ws, _) = connect_async(&url).await.unwrap();
    let challenge = read_auth_challenge(&mut ws).await;

    // Authenticate
    let keys = Keys::generate();
    let auth_event = build_nip42_auth_event(&keys, &challenge, relay_url);
    let auth_msg = ClientMessage::Auth(Cow::Borrowed(&auth_event));
    ws.send(Message::Text(auth_msg.as_json().into())).await.unwrap();
    let _ = read_relay_msg(&mut ws).await; // OK for auth

    // Try to write — should be rejected
    let event = EventBuilder::text_note("should be blocked").sign_with_keys(&keys).unwrap();
    let (status, message) = send_event_and_read_ok(&mut ws, &event).await;
    assert!(!status, "read-only should reject all external writes");
    assert!(message.contains("read-only"), "message should contain 'read-only', got: {message}");

    service.shutdown();
}

// =========================================================================
// 5.5: ReadOnly allows reads (REQ/CLOSE)
// =========================================================================

#[tokio::test]
async fn read_only_allows_req_subscriptions() {
    let (port, service) = start_relay_with_policy(WritePolicy::ReadOnly, None).await;
    let url = format!("ws://127.0.0.1:{port}");

    // Publish an event via the internal API
    let keys = Keys::generate();
    let event = EventBuilder::text_note("internal event").sign_with_keys(&keys).unwrap();
    service.publish(&event).await.unwrap();

    // Connect and subscribe
    let (mut ws, _) = connect_async(&url).await.unwrap();
    let _challenge = read_auth_challenge(&mut ws).await;

    let sub_id = SubscriptionId::new("read-test");
    let filter = Filter::new().kind(Kind::TextNote);
    let req = ClientMessage::Req {
        subscription_id: Cow::Borrowed(&sub_id),
        filters: vec![Cow::Borrowed(&filter)],
    };
    ws.send(Message::Text(req.as_json().into())).await.unwrap();

    // Should receive the event
    let msg = read_relay_msg(&mut ws).await;
    match msg {
        RelayMessage::Event { event: e, .. } => {
            assert_eq!(e.id, event.id);
        }
        other => panic!("expected EVENT, got: {other:?}"),
    }

    // Should receive EOSE
    let msg = read_relay_msg(&mut ws).await;
    assert!(matches!(msg, RelayMessage::EndOfStoredEvents(_)), "expected EOSE, got: {msg:?}");

    service.shutdown();
}

// =========================================================================
// 5.6: publish() bypasses write policy
// =========================================================================

#[tokio::test]
async fn publish_api_bypasses_write_policy() {
    let (_, service) = start_relay_with_policy(WritePolicy::ReadOnly, None).await;

    let keys = Keys::generate();
    let event = EventBuilder::text_note("bridge event").sign_with_keys(&keys).unwrap();

    // publish() should work even in read-only mode
    let is_new = service.publish(&event).await.unwrap();
    assert!(is_new, "publish() should accept events regardless of write policy");

    // Verify it's stored
    let stored = service.store().get_event(&event.id).await.unwrap();
    assert!(stored.is_some());

    service.shutdown();
}

// =========================================================================
// 5.7: NIP-11 includes 42 and limitation
// =========================================================================

#[tokio::test]
async fn nip11_includes_42_and_limitation() {
    // Open policy — no limitation
    let kv = DeterministicKeyValueStore::new();
    let identity = NostrIdentity::generate();
    let config_open = NostrRelayConfig {
        write_policy: WritePolicy::Open,
        ..Default::default()
    };
    let service_open = NostrRelayService::new(config_open, identity.clone(), kv.clone());
    let info_open: serde_json::Value = serde_json::from_str(&service_open.relay_info_json()).unwrap();
    let nips = info_open["supported_nips"].as_array().unwrap();
    assert!(nips.contains(&serde_json::json!(42)), "should include NIP-42");
    assert!(info_open.get("limitation").is_none(), "open policy should not have limitation");

    // AuthRequired — auth_required limitation
    let config_auth = NostrRelayConfig {
        write_policy: WritePolicy::AuthRequired,
        ..Default::default()
    };
    let service_auth = NostrRelayService::new(config_auth, identity.clone(), kv.clone());
    let info_auth: serde_json::Value = serde_json::from_str(&service_auth.relay_info_json()).unwrap();
    assert_eq!(info_auth["limitation"]["auth_required"], true);
    assert!(info_auth["limitation"].get("read_only").is_none());

    // ReadOnly — both limitations
    let config_ro = NostrRelayConfig {
        write_policy: WritePolicy::ReadOnly,
        ..Default::default()
    };
    let service_ro = NostrRelayService::new(config_ro, identity, kv);
    let info_ro: serde_json::Value = serde_json::from_str(&service_ro.relay_info_json()).unwrap();
    assert_eq!(info_ro["limitation"]["auth_required"], true);
    assert_eq!(info_ro["limitation"]["read_only"], true);
}
