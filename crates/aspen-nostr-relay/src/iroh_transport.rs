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
use crate::rate_limit::RateLimiter;
use crate::storage::KvEventStore;
use crate::storage::NostrEventStore;
use crate::subscriptions::SubscriptionRegistry;

/// Maximum concurrent iroh connections to the Nostr relay.
const MAX_IROH_CONNECTIONS: usize = 128;

/// Protocol handler for Nostr over iroh QUIC.
pub struct NostrProtocolHandler<S: ?Sized + 'static> {
    store: Arc<KvEventStore<S>>,
    registry: Arc<SubscriptionRegistry>,
    rate_limiter: Arc<RateLimiter>,
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
        rate_limiter: Arc<RateLimiter>,
        write_policy: WritePolicy,
        relay_url: Option<String>,
    ) -> Self {
        Self {
            store,
            registry,
            rate_limiter,
            write_policy,
            relay_url,
            connection_semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_IROH_CONNECTIONS)),
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
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let write_policy = self.write_policy;
        let relay_url = self.relay_url.clone();
        let event_rx = registry.add_connection(conn_id).await;

        debug!(conn_id, "accepted nostr iroh connection");

        tokio::spawn(async move {
            let result = handle_iroh_connection(
                connection,
                conn_id,
                store,
                registry.clone(),
                event_rx,
                rate_limiter,
                write_policy,
                relay_url,
            )
            .await;
            if let Err(e) = result {
                debug!(conn_id, error = %e, "nostr iroh connection ended with error");
            }
            registry.remove_connection(conn_id).await;
            drop(permit);
        });

        Ok(())
    }
}

/// Handle a single iroh connection: accept a bidirectional stream and
/// process length-prefixed Nostr messages.
#[allow(clippy::too_many_arguments)]
async fn handle_iroh_connection<S: NostrEventStore>(
    connection: Connection,
    conn_id: u64,
    store: Arc<S>,
    registry: Arc<SubscriptionRegistry>,
    mut event_rx: broadcast::Receiver<Arc<Event>>,
    rate_limiter: Arc<RateLimiter>,
    write_policy: WritePolicy,
    relay_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

                        let responses = process_message(
                            &text,
                            conn_id,
                            &store,
                            &registry,
                            &mut auth_state,
                            write_policy,
                            relay_url.as_deref(),
                            &rate_limiter,
                        )
                        .await;

                        for resp in responses {
                            write_frame(&mut send, resp.as_bytes()).await?;
                        }
                    }
                    Ok(None) => {
                        debug!(conn_id, "iroh client disconnected");
                        break;
                    }
                    Err(e) => {
                        debug!(conn_id, error = %e, "iroh frame read error");
                        break;
                    }
                }
            }

            event = event_rx.recv() => {
                match event {
                    Ok(event) => {
                        let subs = registry.get_connection_subscriptions(conn_id).await;
                        for (sub_id, filters) in &subs {
                            let matches = filters.iter().any(|f| f.match_event(&event, MatchEventOptions::default()));
                            if matches {
                                let msg = RelayMessage::Event {
                                    subscription_id: Cow::Borrowed(sub_id),
                                    event: Cow::Borrowed(&event),
                                };
                                write_frame(&mut send, msg.as_json().as_bytes()).await?;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!(conn_id, skipped = n, "iroh broadcast lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    Ok(())
}

/// Process a single Nostr message and return response strings.
///
/// Reuses the same logic as the WebSocket connection handler but returns
/// response strings instead of writing to a WebSocket sink.
#[allow(clippy::too_many_arguments)]
async fn process_message<S: NostrEventStore>(
    text: &str,
    conn_id: u64,
    store: &Arc<S>,
    registry: &Arc<SubscriptionRegistry>,
    auth_state: &mut AuthState,
    write_policy: WritePolicy,
    relay_url: Option<&str>,
    rate_limiter: &RateLimiter,
) -> Vec<String> {
    let mut responses = Vec::new();

    let client_msg = match ClientMessage::from_json(text) {
        Ok(msg) => msg,
        Err(e) => {
            let notice = RelayMessage::Notice(Cow::Owned(format!("error: {e}")));
            responses.push(notice.as_json());
            return responses;
        }
    };

    match client_msg {
        ClientMessage::Event(event) => {
            let event_id = event.id;

            // IP rate limiting not applicable for iroh (no raw IP)
            // Pubkey rate check after signature

            // Write policy
            match write_policy {
                WritePolicy::ReadOnly => {
                    let msg = RelayMessage::Ok {
                        event_id,
                        status: false,
                        message: Cow::Borrowed("blocked: relay is read-only"),
                    };
                    responses.push(msg.as_json());
                    return responses;
                }
                WritePolicy::AuthRequired if !auth_state.is_authenticated() => {
                    let msg = RelayMessage::Ok {
                        event_id,
                        status: false,
                        message: Cow::Borrowed("auth-required: please authenticate"),
                    };
                    responses.push(msg.as_json());
                    return responses;
                }
                _ => {}
            }

            let event = event.into_owned();

            // Validate signature
            if let Err(e) = event.verify() {
                let msg = RelayMessage::Ok {
                    event_id,
                    status: false,
                    message: Cow::Owned(format!("invalid: {e}")),
                };
                responses.push(msg.as_json());
                return responses;
            }

            // Pubkey rate limit
            if rate_limiter.is_enabled() && !rate_limiter.check_pubkey(&event.pubkey.to_hex()) {
                let msg = RelayMessage::Ok {
                    event_id,
                    status: false,
                    message: Cow::Borrowed("rate-limited: too many events from this author"),
                };
                responses.push(msg.as_json());
                return responses;
            }

            match store.store_event(&event).await {
                Ok(is_new) => {
                    if is_new {
                        registry.broadcast_event(Arc::new(event));
                    }
                    let msg = RelayMessage::Ok {
                        event_id,
                        status: true,
                        message: Cow::Borrowed(""),
                    };
                    responses.push(msg.as_json());
                }
                Err(e) => {
                    let msg = RelayMessage::Ok {
                        event_id,
                        status: false,
                        message: Cow::Owned(format!("error: {e}")),
                    };
                    responses.push(msg.as_json());
                }
            }
        }

        ClientMessage::Req {
            subscription_id,
            filters,
        } => {
            let sub_id = subscription_id.into_owned();
            let filters: Vec<Filter> = filters.into_iter().map(|f| f.into_owned()).collect();

            if let Err(e) = registry.subscribe(conn_id, sub_id.clone(), filters.clone()).await {
                let msg = RelayMessage::Closed {
                    subscription_id: Cow::Borrowed(&sub_id),
                    message: Cow::Owned(format!("error: {e}")),
                };
                responses.push(msg.as_json());
                return responses;
            }

            let events = store.query_events(&filters).await.unwrap_or_default();
            for event in &events {
                let msg = RelayMessage::Event {
                    subscription_id: Cow::Borrowed(&sub_id),
                    event: Cow::Borrowed(event),
                };
                responses.push(msg.as_json());
            }

            let eose = RelayMessage::EndOfStoredEvents(Cow::Borrowed(&sub_id));
            responses.push(eose.as_json());
        }

        ClientMessage::Close(sub_id) => {
            registry.unsubscribe(conn_id, &sub_id).await;
        }

        ClientMessage::Auth(event) => {
            let event_id = event.id;
            let now_secs = Timestamp::now().as_secs();
            match auth_state.verify_and_authenticate(&event, relay_url, now_secs) {
                Ok(_pubkey) => {
                    let msg = RelayMessage::Ok {
                        event_id,
                        status: true,
                        message: Cow::Borrowed(""),
                    };
                    responses.push(msg.as_json());
                }
                Err(e) => {
                    let msg = RelayMessage::Ok {
                        event_id,
                        status: false,
                        message: Cow::Owned(format!("auth-required: {e}")),
                    };
                    responses.push(msg.as_json());
                }
            }
        }

        _ => {
            let notice = RelayMessage::Notice(Cow::Borrowed("unsupported message type"));
            responses.push(notice.as_json());
        }
    }

    responses
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
    let mut buf = vec![0u8; len as usize];
    recv.read_exact(&mut buf).await?;
    Ok(Some(buf))
}
