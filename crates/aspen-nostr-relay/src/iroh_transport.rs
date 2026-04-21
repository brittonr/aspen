//! Iroh QUIC transport for the Nostr relay.
//!
//! Registers a protocol handler on `NOSTR_WS_ALPN` that accepts iroh
//! connections and processes Nostr messages using length-prefixed framing.
//! Shares the event store, subscription registry, and broadcast channel
//! with the TCP WebSocket listener.

use std::borrow::Cow;
use std::sync::Arc;

use aspen_traits::KeyValueStore;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use nostr::filter::MatchEventOptions;
use nostr::prelude::*;
use tokio::sync::broadcast;
use tracing::debug;

use crate::auth::AuthState;
use crate::config::WritePolicy;
use crate::constants::MAX_EVENT_SIZE;
use crate::constants::MAX_SUBSCRIPTIONS_PER_CONNECTION;
use crate::rate_limit::RateLimiter;
use crate::storage::KvEventStore;
use crate::storage::NostrEventStore;
use crate::subscriptions::SubscriptionRegistry;

/// Maximum concurrent iroh connections to the Nostr relay.
const MAX_IROH_CONNECTIONS: u32 = 128;

/// Protocol handler for Nostr over iroh QUIC.
pub struct NostrProtocolHandler<S: ?Sized + 'static> {
    store: Arc<KvEventStore<S>>,
    registry: Arc<SubscriptionRegistry>,
    throttle: Arc<RateLimiter>,
    write_policy: WritePolicy,
    relay_url: Option<String>,
    connection_semaphore: Arc<tokio::sync::Semaphore>,
    conn_counter: std::sync::atomic::AtomicU32,
}

impl<S: ?Sized> NostrProtocolHandler<S> {
    /// Create a new protocol handler sharing state with the TCP relay.
    pub fn new(
        store: Arc<KvEventStore<S>>,
        registry: Arc<SubscriptionRegistry>,
        throttle: Arc<RateLimiter>,
        write_policy: WritePolicy,
        relay_url: Option<String>,
    ) -> Self {
        Self {
            store,
            registry,
            throttle,
            write_policy,
            relay_url,
            connection_semaphore: Arc::new(tokio::sync::Semaphore::new(
                usize::try_from(MAX_IROH_CONNECTIONS).unwrap_or(usize::MAX),
            )),
            conn_counter: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

impl<S: ?Sized + 'static> std::fmt::Debug for NostrProtocolHandler<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NostrProtocolHandler").finish()
    }
}

impl<S: KeyValueStore + 'static> ProtocolHandler for NostrProtocolHandler<S> {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let permit = self
            .connection_semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| AcceptError::from_err(std::io::Error::other("nostr iroh connection limit reached")))?;

        let conn_id = self.conn_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u64;
        let store = Arc::clone(&self.store);
        let registry = Arc::clone(&self.registry);
        let conn_throttle = Arc::clone(&self.throttle);
        let write_policy = self.write_policy;
        let relay_url = self.relay_url.clone();
        let event_rx = registry.add_connection(conn_id).await;

        debug!(conn_id, "accepted nostr iroh connection");

        tokio::spawn(async move {
            let ctx = IrohConnectionContext {
                conn_id,
                store,
                registry: registry.clone(),
                throttle: conn_throttle,
                write_policy,
                relay_url,
            };
            let result = handle_iroh_connection(connection, ctx, event_rx).await;
            if let Err(e) = result {
                debug!(conn_id, error = %e, "nostr iroh connection ended with error");
            }
            registry.remove_connection(conn_id).await;
            drop(permit);
        });

        Ok(())
    }
}

/// Connection-scoped context for iroh Nostr connections.
pub(crate) struct IrohConnectionContext<S: ?Sized> {
    conn_id: u64,
    store: Arc<S>,
    registry: Arc<SubscriptionRegistry>,
    throttle: Arc<RateLimiter>,
    write_policy: WritePolicy,
    relay_url: Option<String>,
}

/// Handle a single iroh connection: accept a bidirectional stream and
/// process length-prefixed Nostr messages.
async fn handle_iroh_connection<S: NostrEventStore>(
    connection: Connection,
    ctx: IrohConnectionContext<S>,
    mut event_rx: broadcast::Receiver<Arc<Event>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const { assert!(MAX_SUBSCRIPTIONS_PER_CONNECTION > 0, "max subscriptions per connection must be positive") };
    const { assert!(MAX_EVENT_SIZE > 0, "max event size must be positive") };

    let (mut send, mut recv) = connection.accept_bi().await?;
    let mut auth_state = AuthState::new();

    // Send AUTH challenge
    let auth_msg = RelayMessage::Auth {
        challenge: Cow::Borrowed(auth_state.challenge_hex()),
    };
    write_frame(&mut send, auth_msg.as_json().as_bytes()).await?;

    loop {
        tokio::select! {
            frame = read_frame(&mut recv) => {
                match frame {
                    Ok(Some(data)) => {
                        let text = String::from_utf8(data)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                        let responses = process_message(&text, &ctx, &mut auth_state).await;

                        for resp in responses {
                            write_frame(&mut send, resp.as_bytes()).await?;
                        }
                    }
                    Ok(None) => {
                        debug!(ctx.conn_id, "iroh client disconnected");
                        break;
                    }
                    Err(e) => {
                        debug!(ctx.conn_id, error = %e, "iroh frame read error");
                        break;
                    }
                }
            }

            event = event_rx.recv() => {
                match event {
                    Ok(event) => {
                        push_matching_iroh_event(ctx.conn_id, &event, &ctx.registry, &mut send).await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!(ctx.conn_id, skipped = n, "iroh broadcast lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    Ok(())
}

/// Push a broadcast event to matching subscriptions over an iroh stream.
async fn push_matching_iroh_event(
    conn_id: u64,
    event: &Event,
    registry: &Arc<SubscriptionRegistry>,
    send: &mut iroh::endpoint::SendStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subs = registry.get_connection_subscriptions(conn_id).await;
    for (sub_id, filters) in &subs {
        let is_match = filters.iter().any(|f| f.match_event(event, MatchEventOptions::new()));
        if is_match {
            let msg = RelayMessage::Event {
                subscription_id: Cow::Borrowed(sub_id),
                event: Cow::Borrowed(event),
            };
            write_frame(send, msg.as_json().as_bytes()).await?;
        }
    }
    Ok(())
}

/// Process a single Nostr message and return response strings.
async fn process_message<S: NostrEventStore>(
    text: &str,
    ctx: &IrohConnectionContext<S>,
    auth_state: &mut AuthState,
) -> Vec<String> {
    debug_assert!(!text.is_empty(), "empty messages should be filtered before dispatch");
    let client_msg = match ClientMessage::from_json(text) {
        Ok(msg) => msg,
        Err(e) => {
            let notice = RelayMessage::Notice(Cow::Owned(format!("error: {e}")));
            return vec![notice.as_json()];
        }
    };

    match client_msg {
        ClientMessage::Event(event) => process_event_message(event, ctx, auth_state).await,
        ClientMessage::Req {
            subscription_id,
            filters,
        } => process_req_message(subscription_id, filters, ctx).await,
        ClientMessage::Close(sub_id) => {
            ctx.registry.unsubscribe(ctx.conn_id, &sub_id).await;
            Vec::new()
        }
        ClientMessage::Auth(event) => process_auth_message(event, auth_state, ctx.relay_url.as_deref()),
        _ => {
            vec![RelayMessage::Notice(Cow::Borrowed("unsupported message type")).as_json()]
        }
    }
}

/// Process an EVENT client message over iroh.
async fn process_event_message<S: NostrEventStore>(
    event: Cow<'_, Event>,
    ctx: &IrohConnectionContext<S>,
    auth_state: &AuthState,
) -> Vec<String> {
    debug_assert!(!event.id.to_hex().is_empty(), "events must have an ID");
    let event_id = event.id;

    // Write policy check (reuses same logic as WebSocket path)
    match ctx.write_policy {
        WritePolicy::ReadOnly => {
            return vec![ok_response(event_id, false, "blocked: relay is read-only")];
        }
        WritePolicy::AuthRequired if !auth_state.is_authenticated() => {
            return vec![ok_response(event_id, false, "auth-required: please authenticate")];
        }
        _ => {}
    }

    let event = event.into_owned();

    if let Err(e) = event.verify() {
        return vec![ok_response(event_id, false, &format!("invalid: {e}"))];
    }

    if ctx.throttle.is_enabled() && !ctx.throttle.check_pubkey(&event.pubkey.to_hex()) {
        return vec![ok_response(
            event_id,
            false,
            "rate-limited: too many events from this author",
        )];
    }

    match ctx.store.store_event(&event).await {
        Ok(is_new) => {
            if is_new {
                ctx.registry.broadcast_event(Arc::new(event));
            }
            vec![ok_response(event_id, true, "")]
        }
        Err(e) => vec![ok_response(event_id, false, &format!("error: {e}"))],
    }
}

/// Process a REQ client message over iroh.
async fn process_req_message<S: NostrEventStore>(
    subscription_id: Cow<'_, SubscriptionId>,
    filters: Vec<Cow<'_, Filter>>,
    ctx: &IrohConnectionContext<S>,
) -> Vec<String> {
    debug_assert!(!filters.is_empty(), "REQ must include at least one filter per NIP-01");
    let sub_id = subscription_id.into_owned();
    let filters: Vec<Filter> = filters.into_iter().map(|f| f.into_owned()).collect();

    if let Err(e) = ctx.registry.subscribe(ctx.conn_id, sub_id.clone(), filters.clone()).await {
        let msg = RelayMessage::Closed {
            subscription_id: Cow::Borrowed(&sub_id),
            message: Cow::Owned(format!("error: {e}")),
        };
        return vec![msg.as_json()];
    }

    let events = ctx.store.query_events(&filters).await.unwrap_or_default();
    let mut responses = Vec::with_capacity(events.len().saturating_add(1));
    for event in &events {
        let msg = RelayMessage::Event {
            subscription_id: Cow::Borrowed(&sub_id),
            event: Cow::Borrowed(event),
        };
        responses.push(msg.as_json());
    }
    responses.push(RelayMessage::EndOfStoredEvents(Cow::Borrowed(&sub_id)).as_json());
    responses
}

/// Process an AUTH client message over iroh.
fn process_auth_message(event: Cow<'_, Event>, auth_state: &mut AuthState, relay_url: Option<&str>) -> Vec<String> {
    let event_id = event.id;
    let now_secs = Timestamp::now().as_secs();
    match auth_state.verify_and_authenticate(&event, relay_url, now_secs) {
        Ok(_pubkey) => vec![ok_response(event_id, true, "")],
        Err(e) => vec![ok_response(event_id, false, &format!("auth-required: {e}"))],
    }
}

/// Build a RelayMessage::Ok JSON string.
fn ok_response(event_id: EventId, status: bool, message: &str) -> String {
    let msg = RelayMessage::Ok {
        event_id,
        status,
        message: Cow::Borrowed(message),
    };
    msg.as_json()
}

/// Write a length-prefixed frame: 4-byte BE length + payload.
pub async fn write_frame(
    send: &mut iroh::endpoint::SendStream,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let len = data.len() as u32;
    send.write_all(&len.to_be_bytes()).await?;
    send.write_all(data).await?;
    Ok(())
}

/// Read a length-prefixed frame. Returns `None` on clean EOF.
///
/// Rejects frames exceeding `MAX_EVENT_SIZE`.
pub async fn read_frame(
    recv: &mut iroh::endpoint::RecvStream,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
    let mut len_buf = [0u8; 4];
    match recv.read_exact(&mut len_buf).await {
        Ok(()) => {}
        Err(iroh::endpoint::ReadExactError::FinishedEarly(_)) => return Ok(None),
        Err(e) => return Err(Box::new(e)),
    }
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_EVENT_SIZE {
        return Err(format!("frame too large: {len} > {MAX_EVENT_SIZE}").into());
    }
    let buf_len = usize::try_from(len).map_err(|_| std::io::Error::other("frame length exceeds platform capacity"))?;
    let mut buf = vec![0u8; buf_len];
    recv.read_exact(&mut buf).await?;
    Ok(Some(buf))
}
