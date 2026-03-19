//! WebSocket connection handler for NIP-01 protocol messages.
//!
//! Each connection runs as a separate tokio task, reading client messages
//! and dispatching to EVENT/REQ/CLOSE/AUTH handlers. A broadcast receiver
//! delivers real-time events from other connections.
//!
//! NIP-42 authentication is handled per-connection: the relay sends an
//! AUTH challenge on connect, and the client may respond with a signed
//! kind 22242 event. Write access is gated by the configured `WritePolicy`.

use std::borrow::Cow;
use std::sync::Arc;

use futures::SinkExt;
use futures::StreamExt;
use futures::stream::SplitSink;
use nostr::filter::MatchEventOptions;
use nostr::prelude::*;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::auth::AuthState;
use crate::config::WritePolicy;
use crate::constants::MAX_EVENT_SIZE;
use crate::storage::NostrEventStore;
use crate::subscriptions::ConnectionId;
use crate::subscriptions::SubscriptionRegistry;

/// Handle a single WebSocket connection.
///
/// Reads NIP-01 client messages, processes them, and sends relay responses.
/// Also listens on the broadcast channel for real-time event push.
///
/// On connect, sends an AUTH challenge per NIP-42. The `write_policy`
/// determines whether EVENT submissions require authentication.
#[allow(clippy::too_many_arguments)]
pub async fn handle_connection<S: NostrEventStore>(
    ws: WebSocketStream<TcpStream>,
    conn_id: ConnectionId,
    store: Arc<S>,
    registry: Arc<SubscriptionRegistry>,
    mut event_rx: broadcast::Receiver<Arc<Event>>,
    cancel: tokio_util::sync::CancellationToken,
    write_policy: WritePolicy,
    relay_url: Option<String>,
) {
    let (mut ws_tx, mut ws_rx) = ws.split();

    info!(conn_id, "nostr client connected");

    // NIP-42: send AUTH challenge immediately on connect
    let mut auth_state = AuthState::new();
    let auth_msg = RelayMessage::Auth {
        challenge: Cow::Borrowed(auth_state.challenge_hex()),
    };
    if let Err(e) = ws_tx.send(Message::Text(auth_msg.as_json().into())).await {
        debug!(conn_id, error = %e, "failed to send auth challenge");
        return;
    }

    loop {
        tokio::select! {
            // Client messages
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if text.len() > MAX_EVENT_SIZE as usize {
                            let _ = send_notice(&mut ws_tx, "message too large").await;
                            continue;
                        }
                        if let Err(e) = handle_message(
                            &text, conn_id, &store, &registry, &mut ws_tx,
                            &mut auth_state, write_policy, relay_url.as_deref(),
                        ).await {
                            warn!(conn_id, error = %e, "error handling message");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        debug!(conn_id, "client disconnected");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_tx.send(Message::Pong(data)).await;
                    }
                    Some(Ok(_)) => {} // Binary, Pong, Frame — ignore
                    Some(Err(e)) => {
                        debug!(conn_id, error = %e, "websocket error");
                        break;
                    }
                }
            }

            // Real-time broadcast events
            event = event_rx.recv() => {
                match event {
                    Ok(event) => {
                        if let Err(e) = push_matching_event(
                            conn_id, &event, &registry, &mut ws_tx,
                        ).await {
                            debug!(conn_id, error = %e, "error pushing event");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!(conn_id, skipped = n, "broadcast lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            // Cancellation
            _ = cancel.cancelled() => {
                debug!(conn_id, "connection cancelled");
                break;
            }
        }
    }

    // Cleanup
    registry.remove_connection(conn_id).await;
    let _ = ws_tx.close().await;
    info!(conn_id, "nostr client session ended");
}

/// Process a single NIP-01 client message.
#[allow(clippy::too_many_arguments)]
async fn handle_message<S: NostrEventStore>(
    text: &str,
    conn_id: ConnectionId,
    store: &Arc<S>,
    registry: &Arc<SubscriptionRegistry>,
    ws_tx: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    auth_state: &mut AuthState,
    write_policy: WritePolicy,
    relay_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_msg: ClientMessage<'_> = ClientMessage::from_json(text)?;

    match client_msg {
        ClientMessage::Event(event) => {
            handle_event(event.into_owned(), store, registry, ws_tx, auth_state, write_policy).await?;
        }
        ClientMessage::Req {
            subscription_id,
            filters,
        } => {
            let sub_id = subscription_id.into_owned();
            let filters: Vec<Filter> = filters.into_iter().map(|f| f.into_owned()).collect();
            handle_req(conn_id, sub_id, filters, store, registry, ws_tx).await?;
        }
        ClientMessage::Close(sub_id) => {
            handle_close(conn_id, &sub_id, registry).await;
        }
        ClientMessage::Auth(event) => {
            handle_auth(event.into_owned(), auth_state, relay_url, ws_tx).await?;
        }
        _ => {
            // negentropy, count — not supported yet
            send_notice(ws_tx, "unsupported message type").await?;
        }
    }

    Ok(())
}

/// Handle AUTH: verify kind 22242 event and mark connection as authenticated.
async fn handle_auth(
    event: Event,
    auth_state: &mut AuthState,
    relay_url: Option<&str>,
    ws_tx: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_id = event.id;
    let now_secs = Timestamp::now().as_secs();

    match auth_state.verify_and_authenticate(&event, relay_url, now_secs) {
        Ok(pubkey) => {
            debug!(pubkey = %pubkey.to_hex(), "client authenticated via NIP-42");
            let msg = RelayMessage::Ok {
                event_id,
                status: true,
                message: Cow::Borrowed(""),
            };
            ws_tx.send(Message::Text(msg.as_json().into())).await?;
        }
        Err(e) => {
            debug!(error = %e, "NIP-42 auth failed");
            let msg = RelayMessage::Ok {
                event_id,
                status: false,
                message: Cow::Owned(format!("auth-required: {e}")),
            };
            ws_tx.send(Message::Text(msg.as_json().into())).await?;
        }
    }

    Ok(())
}

/// Handle EVENT: check write policy, validate, store, broadcast, respond with OK.
async fn handle_event<S: NostrEventStore>(
    event: Event,
    store: &Arc<S>,
    registry: &Arc<SubscriptionRegistry>,
    ws_tx: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    auth_state: &AuthState,
    write_policy: WritePolicy,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_id = event.id;

    // Write policy check
    match write_policy {
        WritePolicy::ReadOnly => {
            let msg = RelayMessage::Ok {
                event_id,
                status: false,
                message: Cow::Borrowed("blocked: relay is read-only"),
            };
            ws_tx.send(Message::Text(msg.as_json().into())).await?;
            return Ok(());
        }
        WritePolicy::AuthRequired if !auth_state.is_authenticated() => {
            let msg = RelayMessage::Ok {
                event_id,
                status: false,
                message: Cow::Borrowed("auth-required: please authenticate"),
            };
            ws_tx.send(Message::Text(msg.as_json().into())).await?;
            return Ok(());
        }
        WritePolicy::AuthRequired | WritePolicy::Open => {
            // Allowed — proceed to validation
        }
    }

    // Validate signature
    if let Err(e) = event.verify() {
        let msg = RelayMessage::Ok {
            event_id,
            status: false,
            message: Cow::Owned(format!("invalid: {e}")),
        };
        ws_tx.send(Message::Text(msg.as_json().into())).await?;
        return Ok(());
    }

    // Store
    match store.store_event(&event).await {
        Ok(is_new) => {
            if is_new {
                // Broadcast to subscribers
                registry.broadcast_event(Arc::new(event));
            }
            let msg = RelayMessage::Ok {
                event_id,
                status: true,
                message: Cow::Borrowed(""),
            };
            ws_tx.send(Message::Text(msg.as_json().into())).await?;
        }
        Err(e) => {
            let msg = RelayMessage::Ok {
                event_id,
                status: false,
                message: Cow::Owned(format!("error: {e}")),
            };
            ws_tx.send(Message::Text(msg.as_json().into())).await?;
        }
    }

    Ok(())
}

/// Handle REQ: register subscription, send stored events, send EOSE.
async fn handle_req<S: NostrEventStore>(
    conn_id: ConnectionId,
    sub_id: SubscriptionId,
    filters: Vec<Filter>,
    store: &Arc<S>,
    registry: &Arc<SubscriptionRegistry>,
    ws_tx: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Register subscription (replaces existing with same ID)
    if let Err(e) = registry.subscribe(conn_id, sub_id.clone(), filters.clone()).await {
        let msg = RelayMessage::Closed {
            subscription_id: Cow::Borrowed(&sub_id),
            message: Cow::Owned(format!("error: {e}")),
        };
        ws_tx.send(Message::Text(msg.as_json().into())).await?;
        return Ok(());
    }

    // Query stored events
    let events = store.query_events(&filters).await.unwrap_or_default();

    // Send matching stored events
    for event in &events {
        let msg = RelayMessage::Event {
            subscription_id: Cow::Borrowed(&sub_id),
            event: Cow::Borrowed(event),
        };
        ws_tx.send(Message::Text(msg.as_json().into())).await?;
    }

    // Send EOSE
    let eose = RelayMessage::EndOfStoredEvents(Cow::Borrowed(&sub_id));
    ws_tx.send(Message::Text(eose.as_json().into())).await?;

    debug!(conn_id, sub_id = %sub_id, stored = events.len(), "subscription registered");
    Ok(())
}

/// Handle CLOSE: remove subscription.
async fn handle_close(conn_id: ConnectionId, sub_id: &SubscriptionId, registry: &Arc<SubscriptionRegistry>) {
    registry.unsubscribe(conn_id, sub_id).await;
    debug!(conn_id, sub_id = %sub_id, "subscription closed");
}

/// Push a broadcast event to a connection if it matches any active subscription.
async fn push_matching_event(
    conn_id: ConnectionId,
    event: &Event,
    registry: &Arc<SubscriptionRegistry>,
    ws_tx: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subs = registry.get_connection_subscriptions(conn_id).await;
    for (sub_id, filters) in &subs {
        let matches = filters.iter().any(|f| f.match_event(event, MatchEventOptions::default()));
        if matches {
            let msg = RelayMessage::Event {
                subscription_id: Cow::Borrowed(sub_id),
                event: Cow::Borrowed(event),
            };
            ws_tx.send(Message::Text(msg.as_json().into())).await?;
        }
    }
    Ok(())
}

/// Send a NOTICE message to the client.
async fn send_notice(
    ws_tx: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let msg = RelayMessage::Notice(Cow::Borrowed(message));
    ws_tx.send(Message::Text(msg.as_json().into())).await?;
    Ok(())
}
