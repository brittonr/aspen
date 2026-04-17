//! Nostr relay service — owns the TCP listener, event store, and subscriptions.
//!
//! Implements NIP-11 relay info document and manages the lifecycle of
//! WebSocket connections.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use aspen_traits::KeyValueStore;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::config::NostrRelayConfig;
use crate::config::WritePolicy;
use crate::connection::handle_connection;
use crate::keys::NostrIdentity;
use crate::rate_limit::RateLimiter;
use crate::storage::KvEventStore;
use crate::storage::NostrEventStore;
use crate::subscriptions::SubscriptionRegistry;

/// Shared relay state wrapped in Arc for spawned tasks.
struct RelayInner<S: ?Sized> {
    store: Arc<KvEventStore<S>>,
    registry: Arc<SubscriptionRegistry>,
    throttle: Arc<RateLimiter>,
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
        let (inner, cancel) = build_relay_inner(store, &config);

        Self {
            config,
            identity,
            inner,
            conn_counter: AtomicU32::new(0),
            cancel,
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
    let (inner, cancel) = build_relay_inner(store, &config);

    NostrRelayService {
        config,
        identity,
        inner,
        conn_counter: AtomicU32::new(0),
        cancel,
    }
}

/// Build the shared relay inner state from a store and config.
fn build_relay_inner<S: ?Sized + 'static>(
    store: Arc<KvEventStore<S>>,
    config: &NostrRelayConfig,
) -> (Arc<RelayInner<S>>, tokio_util::sync::CancellationToken) {
    let registry = Arc::new(SubscriptionRegistry::new(config.max_subscriptions_per_connection));
    let throttle = Arc::new(RateLimiter::new(
        config.events_per_second_per_ip,
        config.events_burst_per_ip,
        config.events_per_second_per_pubkey,
        config.events_burst_per_pubkey,
    ));
    let cancel = tokio_util::sync::CancellationToken::new();
    throttle.start_cleanup_task(cancel.child_token());

    let inner = Arc::new(RelayInner {
        store,
        registry,
        throttle,
        active_connections: AtomicU32::new(0),
    });
    (inner, cancel)
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
                        Ok((mut stream, peer)) => {
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

                            // Peek at the HTTP request to decide: NIP-11 info vs WebSocket upgrade
                            let mut buf = [0u8; 4096];
                            let n = match stream.peek(&mut buf).await {
                                Ok(n) if n > 0 => n,
                                _ => { drop(stream); continue; }
                            };
                            let request_bytes = &buf[..n];

                            // Check for NIP-11 info request: Accept header contains
                            // "application/nostr+json" AND no Upgrade: websocket
                            if let Some(action) = classify_http_request(request_bytes) {
                                match action {
                                    HttpAction::Nip11 => {
                                        let info = self.relay_info_json();
                                        let response = format!(
                                            "HTTP/1.1 200 OK\r\n\
                                             Content-Type: application/nostr+json\r\n\
                                             Access-Control-Allow-Origin: *\r\n\
                                             Content-Length: {}\r\n\
                                             Connection: close\r\n\r\n{}",
                                            info.len(),
                                            info
                                        );
                                        // Consume the peeked bytes
                                        drop(stream.read(&mut buf).await);
                                        drop(stream.write_all(response.as_bytes()).await);
                                        drop(stream.shutdown().await);
                                        continue;
                                    }
                                    HttpAction::Reject => {
                                        let response = "HTTP/1.1 400 Bad Request\r\n\
                                             Content-Length: 0\r\n\
                                             Connection: close\r\n\r\n";
                                        drop(stream.read(&mut buf).await);
                                        drop(stream.write_all(response.as_bytes()).await);
                                        drop(stream.shutdown().await);
                                        continue;
                                    }
                                    HttpAction::WebSocket => {
                                        // Fall through to WebSocket upgrade
                                    }
                                }
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
                            let conn_throttle = Arc::clone(&inner.throttle);

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
                                    Some((conn_throttle, peer.ip())),
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

    /// Access the subscription registry (for iroh transport sharing).
    pub fn registry(&self) -> &Arc<SubscriptionRegistry> {
        &self.inner.registry
    }

    /// Access the rate limiter (for iroh transport sharing).
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.inner.throttle
    }

    /// Access the write policy.
    pub fn write_policy(&self) -> WritePolicy {
        self.config.write_policy
    }

    /// Access the relay URL.
    pub fn relay_url(&self) -> Option<&str> {
        self.config.relay_url.as_deref()
    }
}

/// Classification of an incoming HTTP request.
enum HttpAction {
    /// NIP-11 relay info request — serve JSON and close.
    Nip11,
    /// WebSocket upgrade — proceed with handshake.
    WebSocket,
    /// Neither NIP-11 nor WebSocket — reject with 400.
    Reject,
}

/// Inspect raw HTTP request bytes to decide how to handle the connection.
///
/// Returns `None` if the bytes don't look like HTTP (binary garbage).
fn classify_http_request(buf: &[u8]) -> Option<HttpAction> {
    let text = std::str::from_utf8(buf).ok()?;
    let lower = text.to_ascii_lowercase();

    // Must start with an HTTP method
    if !text.starts_with("GET ") && !text.starts_with("HEAD ") {
        return None;
    }

    let has_ws_upgrade = lower.contains("upgrade: websocket") || lower.contains("upgrade:websocket");
    let has_nostr_accept = lower.contains("application/nostr+json");

    if has_nostr_accept && !has_ws_upgrade {
        Some(HttpAction::Nip11)
    } else if has_ws_upgrade {
        Some(HttpAction::WebSocket)
    } else {
        Some(HttpAction::Reject)
    }
}
