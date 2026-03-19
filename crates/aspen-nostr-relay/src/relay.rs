//! Nostr relay service — owns the TCP listener, event store, and subscriptions.
//!
//! Implements NIP-11 relay info document and manages the lifecycle of
//! WebSocket connections.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use aspen_traits::KeyValueStore;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::config::NostrRelayConfig;
use crate::connection::handle_connection;
use crate::keys::NostrIdentity;
use crate::storage::KvEventStore;
use crate::storage::NostrEventStore;
use crate::subscriptions::SubscriptionRegistry;

/// Shared relay state wrapped in Arc for spawned tasks.
struct RelayInner<S: ?Sized> {
    store: Arc<KvEventStore<S>>,
    registry: Arc<SubscriptionRegistry>,
    active_connections: AtomicU32,
}

/// The Nostr relay service.
///
/// Binds a TCP listener, accepts WebSocket connections, and coordinates
/// event storage and subscription fan-out.
pub struct NostrRelayService<S: ?Sized> {
    config: NostrRelayConfig,
    identity: NostrIdentity,
    inner: Arc<RelayInner<S>>,
    conn_counter: AtomicU32,
    cancel: tokio_util::sync::CancellationToken,
}

impl<S: KeyValueStore + 'static> NostrRelayService<S> {
    /// Create a new relay service from a concrete KV store.
    pub fn new(config: NostrRelayConfig, identity: NostrIdentity, kv: Arc<S>) -> Self {
        let store = Arc::new(KvEventStore::new(kv));
        let registry = Arc::new(SubscriptionRegistry::new(config.max_subscriptions_per_connection));

        Self {
            config,
            identity,
            inner: Arc::new(RelayInner {
                store,
                registry,
                active_connections: AtomicU32::new(0),
            }),
            conn_counter: AtomicU32::new(0),
            cancel: tokio_util::sync::CancellationToken::new(),
        }
    }
}

/// Create a relay service from a trait-object KV store.
///
/// Returns a type-erased relay that works with `Arc<dyn KeyValueStore>`.
pub fn new_dyn_relay(
    config: NostrRelayConfig,
    identity: NostrIdentity,
    kv: Arc<dyn KeyValueStore>,
) -> NostrRelayService<dyn KeyValueStore> {
    let store = Arc::new(KvEventStore::from_arc(kv));
    let registry = Arc::new(SubscriptionRegistry::new(config.max_subscriptions_per_connection));

    NostrRelayService {
        config,
        identity,
        inner: Arc::new(RelayInner {
            store,
            registry,
            active_connections: AtomicU32::new(0),
        }),
        conn_counter: AtomicU32::new(0),
        cancel: tokio_util::sync::CancellationToken::new(),
    }
}

impl<S: KeyValueStore + ?Sized + 'static> NostrRelayService<S> {
    /// Run the relay — binds the listener and accepts connections until cancelled.
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.bind_addr, self.config.bind_port);
        let listener = TcpListener::bind(&addr).await?;

        info!(
            addr,
            pubkey = %self.identity.public_key_hex(),
            "nostr relay listening"
        );

        loop {
            tokio::select! {
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, peer)) => {
                            let active = self.inner.active_connections.load(Ordering::Relaxed);
                            if active >= self.config.max_connections {
                                warn!(
                                    active,
                                    max = self.config.max_connections,
                                    "connection limit reached, rejecting"
                                );
                                drop(stream);
                                continue;
                            }

                            let conn_id = self.conn_counter.fetch_add(1, Ordering::Relaxed) as u64;
                            self.inner.active_connections.fetch_add(1, Ordering::Relaxed);

                            let inner = Arc::clone(&self.inner);
                            let event_rx = inner.registry.add_connection(conn_id).await;
                            let cancel = self.cancel.child_token();

                            debug!(conn_id, peer = %peer, "accepting nostr connection");

                            let ws = match accept_async(stream).await {
                                Ok(ws) => ws,
                                Err(e) => {
                                    debug!(conn_id, error = %e, "websocket handshake failed");
                                    inner.active_connections.fetch_sub(1, Ordering::Relaxed);
                                    inner.registry.remove_connection(conn_id).await;
                                    continue;
                                }
                            };

                            let write_policy = self.config.write_policy;
                            let relay_url = self.config.relay_url.clone();

                            tokio::spawn(async move {
                                handle_connection(
                                    ws,
                                    conn_id,
                                    Arc::clone(&inner.store),
                                    Arc::clone(&inner.registry),
                                    event_rx,
                                    cancel,
                                    write_policy,
                                    relay_url,
                                ).await;
                                inner.active_connections.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        Err(e) => {
                            warn!(error = %e, "accept error");
                        }
                    }
                }
                _ = self.cancel.cancelled() => {
                    info!("nostr relay shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Publish an event into the relay (for use by bridge plugins).
    pub async fn publish(&self, event: &nostr::Event) -> Result<bool, crate::storage::StorageError> {
        let is_new = self.inner.store.store_event(event).await?;
        if is_new {
            self.inner.registry.broadcast_event(Arc::new(event.clone()));
        }
        Ok(is_new)
    }

    /// Get the NIP-11 relay information document as JSON.
    pub fn relay_info_json(&self) -> String {
        use crate::config::WritePolicy;

        let mut doc = serde_json::json!({
            "name": "aspen-nostr-relay",
            "description": "Aspen cluster Nostr relay",
            "pubkey": self.identity.public_key_hex(),
            "supported_nips": [1, 11, 34, 42],
            "software": "aspen",
            "version": env!("CARGO_PKG_VERSION"),
        });

        // NIP-11 limitation object for auth/read-only policies
        match self.config.write_policy {
            WritePolicy::AuthRequired => {
                doc["limitation"] = serde_json::json!({
                    "auth_required": true,
                });
            }
            WritePolicy::ReadOnly => {
                doc["limitation"] = serde_json::json!({
                    "auth_required": true,
                    "read_only": true,
                });
            }
            WritePolicy::Open => {}
        }

        doc.to_string()
    }

    /// Trigger graceful shutdown.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }

    /// Number of active WebSocket connections.
    pub fn active_connections(&self) -> u32 {
        self.inner.active_connections.load(Ordering::Relaxed)
    }

    /// Access the event store.
    pub fn store(&self) -> &Arc<KvEventStore<S>> {
        &self.inner.store
    }

    /// Access the Nostr identity.
    pub fn identity(&self) -> &NostrIdentity {
        &self.identity
    }
}
