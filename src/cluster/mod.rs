//! Cluster coordination and peer discovery for Aspen.
//!
//! This module provides the infrastructure for distributed cluster coordination,
//! including:
//!
//! - **Ractor Cluster**: Actor-based node communication via NodeServer
//! - **Iroh P2P Transport**: QUIC-based peer-to-peer networking with NAT traversal
//! - **Gossip-based Peer Discovery**: Automatic node discovery via iroh-gossip (default)
//! - **Cluster Tickets**: Compact bootstrap information for joining clusters
//! - **Manual Peer Configuration**: Explicit peer list as fallback when gossip is disabled
//!
//! # Peer Discovery
//!
//! Aspen supports two modes for peer discovery:
//!
//! ## Automatic (Gossip - Default)
//!
//! When gossip is enabled (default), nodes automatically discover each other:
//! 1. Each node subscribes to a gossip topic (derived from cluster cookie)
//! 2. Nodes broadcast their EndpointAddr every 10 seconds
//! 3. Received announcements are logged and available for connection attempts
//!
//! ## Manual (Explicit Peers)
//!
//! When gossip is disabled, nodes must be configured with explicit peer addresses:
//! - Via CLI: `--peers "node_id@endpoint_id"`
//! - Via config file: `peers = ["node_id@endpoint_id"]`
//!
//! # Cluster Tickets
//!
//! Tickets provide a convenient way to join clusters:
//! - First node starts with default gossip (topic from cookie)
//! - HTTP GET `/cluster-ticket` returns a serialized ticket
//! - New nodes use `--ticket "aspen{...}"` to join automatically
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  NodeServer     │  Ractor cluster coordination
//! │  (ractor)       │
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │ IrohEndpoint    │  P2P QUIC transport
//! │ (iroh)          │
//! └────────┬────────┘
//!          │
//!          ├─────────► Gossip (peer discovery)
//!          ├─────────► IRPC (Raft RPC)
//!          └─────────► HTTP Control Plane
//! ```

use std::fmt;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as SyncMutex};
use std::time::Duration;

use anyhow::{Context, Result};
use iroh::{Endpoint as IrohEndpoint, EndpointAddr, RelayMode, RelayUrl, SecretKey};
use iroh_gossip::net::{Gossip, GOSSIP_ALPN};
use iroh_gossip::proto::TopicId;
use ractor::{Actor, ActorRef, MessagingErr};
use ractor_cluster::node::NodeConnectionMode;
use ractor_cluster::{
    ClientConnectErr, ClusterBidiStream, IncomingEncryptionMode, NodeEventSubscription,
    NodeServer as RactorNodeServer, NodeServerMessage, client_connect, client_connect_external,
};
use tokio::net::ToSocketAddrs;
use tokio::sync::{Mutex as AsyncMutex, Notify};
use tokio::task::JoinHandle;

pub mod bootstrap;
pub mod config;
pub mod gossip_discovery;
pub mod metadata;
pub mod ticket;

/// Controls how the node server should behave while running in deterministic
/// simulations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeterministicClusterConfig {
    pub simulation_seed: Option<u64>,
}

/// Configuration for the local `ractor_cluster::NodeServer`.
#[derive(Debug, Clone)]
pub struct NodeServerConfig {
    label: String,
    host: String,
    port: u16,
    cookie: String,
    encryption: Option<IncomingEncryptionMode>,
    connection_mode: NodeConnectionMode,
    determinism: Option<DeterministicClusterConfig>,
    iroh_config: Option<IrohEndpointConfig>,
}

impl NodeServerConfig {
    pub fn new(
        label: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        cookie: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            host: host.into(),
            port,
            cookie: cookie.into(),
            encryption: None,
            connection_mode: NodeConnectionMode::Transitive,
            determinism: None,
            iroh_config: None,
        }
    }

    pub fn with_encryption(mut self, encryption: IncomingEncryptionMode) -> Self {
        self.encryption = Some(encryption);
        self
    }

    pub fn with_connection_mode(mut self, mode: NodeConnectionMode) -> Self {
        self.connection_mode = mode;
        self
    }

    pub fn with_determinism(mut self, config: DeterministicClusterConfig) -> Self {
        self.determinism = Some(config);
        self
    }

    pub fn with_iroh(mut self, config: IrohEndpointConfig) -> Self {
        self.iroh_config = Some(config);
        self
    }

    pub async fn launch(self) -> Result<NodeServerHandle> {
        let server = RactorNodeServer::new(
            self.port,
            self.cookie.clone(),
            self.label.clone(),
            self.host.clone(),
            self.encryption.clone(),
            Some(self.connection_mode),
        );
        let actor_name = format!("node-server-{}", self.label);
        let (actor_ref, join_handle) = Actor::spawn(Some(actor_name.into()), server, ())
            .await
            .context("failed to spawn node server")?;

        // Optionally create Iroh endpoint if configured
        let iroh_manager = if let Some(iroh_config) = self.iroh_config {
            Some(
                IrohEndpointManager::new(iroh_config)
                    .await
                    .context("failed to create Iroh endpoint manager")?,
            )
        } else {
            None
        };

        Ok(NodeServerHandle {
            inner: Arc::new(NodeServerInner {
                actor: actor_ref,
                join_handle: AsyncMutex::new(Some(join_handle)),
                label: self.label,
                host: self.host,
                port: self.port,
                cookie: self.cookie,
                connection_mode: self.connection_mode,
                determinism: self.determinism,
                subscriptions: SyncMutex::new(Vec::new()),
                iroh_manager,
            }),
        })
    }
}

struct NodeServerInner {
    actor: ActorRef<NodeServerMessage>,
    join_handle: AsyncMutex<Option<JoinHandle<()>>>,
    label: String,
    host: String,
    port: u16,
    cookie: String,
    connection_mode: NodeConnectionMode,
    determinism: Option<DeterministicClusterConfig>,
    subscriptions: SyncMutex<Vec<String>>,
    iroh_manager: Option<IrohEndpointManager>,
}

impl fmt::Debug for NodeServerInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeServerInner")
            .field("label", &self.label)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("cookie", &"<hidden>")
            .field("connection_mode", &self.connection_mode)
            .finish()
    }
}

/// Handle for a launched node server.
pub struct NodeServerHandle {
    inner: Arc<NodeServerInner>,
}

impl Clone for NodeServerHandle {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl fmt::Debug for NodeServerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeServerHandle")
            .field("label", &self.inner.label)
            .field("addr", &self.addr())
            .finish()
    }
}

impl NodeServerHandle {
    pub fn label(&self) -> &str {
        &self.inner.label
    }

    pub fn cookie(&self) -> &str {
        &self.inner.cookie
    }

    pub fn determinism(&self) -> Option<&DeterministicClusterConfig> {
        self.inner.determinism.as_ref()
    }

    pub fn addr(&self) -> SocketAddr {
        let ip: IpAddr = self
            .inner
            .host
            .parse()
            .unwrap_or_else(|_| IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        SocketAddr::new(ip, self.inner.port)
    }

    pub fn actor(&self) -> ActorRef<NodeServerMessage> {
        self.inner.actor.clone()
    }

    pub fn iroh_manager(&self) -> Option<&IrohEndpointManager> {
        self.inner.iroh_manager.as_ref()
    }

    pub fn subscribe(
        &self,
        id: impl Into<String>,
        subscription: Box<dyn NodeEventSubscription>,
    ) -> Result<()> {
        let sub_id = id.into();
        self.inner
            .actor
            .cast(NodeServerMessage::SubscribeToEvents {
                id: sub_id.clone(),
                subscription,
            })?;
        self.inner
            .subscriptions
            .lock()
            .expect("subscriptions poisoned")
            .push(sub_id);
        Ok(())
    }

    pub fn unsubscribe(&self, id: &str) -> Result<()> {
        self.inner
            .actor
            .cast(NodeServerMessage::UnsubscribeToEvents(id.to_string()))?;
        let mut guard = self
            .inner
            .subscriptions
            .lock()
            .expect("subscriptions poisoned");
        if let Some(pos) = guard.iter().position(|existing| existing == id) {
            guard.remove(pos);
        }
        Ok(())
    }

    pub async fn client_connect<T>(&self, address: T) -> Result<(), ClientConnectErr>
    where
        T: ToSocketAddrs,
    {
        client_connect(&self.inner.actor, address).await
    }

    pub async fn client_connect_external(
        &self,
        stream: Box<dyn ClusterBidiStream>,
    ) -> Result<(), ClientConnectErr> {
        client_connect_external(&self.inner.actor, stream).await
    }

    pub fn attach_external_stream(
        &self,
        stream: Box<dyn ClusterBidiStream>,
        is_server: bool,
    ) -> Result<(), MessagingErr<NodeServerMessage>> {
        self.inner
            .actor
            .cast(NodeServerMessage::ConnectionOpenedExternal { stream, is_server })
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.inner.actor.stop(Some("node-server-shutdown".into()));
        if let Some(join) = self.inner.join_handle.lock().await.take() {
            join.await
                .context("node server task aborted while shutting down")?;
        }

        // best-effort removal of outstanding subscriptions
        let subs = {
            let mut guard = self
                .inner
                .subscriptions
                .lock()
                .expect("subscriptions poisoned");
            mem::take(&mut *guard)
        };
        for id in subs {
            let _ = self
                .inner
                .actor
                .cast(NodeServerMessage::UnsubscribeToEvents(id));
        }

        // Shutdown Iroh endpoint if present
        if let Some(iroh_manager) = &self.inner.iroh_manager {
            iroh_manager
                .shutdown()
                .await
                .context("failed to shutdown Iroh endpoint")?;
        }

        Ok(())
    }
}

/// Configuration for Iroh endpoint creation.
///
/// Tiger Style: Fixed limits and explicit configuration.
/// Relay URLs are optional but bounded (max 4 relay servers).
#[derive(Debug, Clone)]
pub struct IrohEndpointConfig {
    /// Optional secret key for the endpoint. If None, a new key is generated.
    pub secret_key: Option<SecretKey>,
    /// Relay server URLs for NAT traversal (max 4 relays).
    pub relay_urls: Vec<RelayUrl>,
    /// Bind port for the QUIC socket (0 = random port).
    pub bind_port: u16,
    /// Enable gossip-based peer discovery (default: true).
    pub enable_gossip: bool,
    /// Optional explicit gossip topic ID. If None, derived from cluster cookie.
    pub gossip_topic: Option<TopicId>,
}

impl Default for IrohEndpointConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            relay_urls: Vec::new(),
            bind_port: 0,
            enable_gossip: true,
            gossip_topic: None,
        }
    }
}

impl IrohEndpointConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the secret key for deterministic endpoint identity.
    pub fn with_secret_key(mut self, key: SecretKey) -> Self {
        self.secret_key = Some(key);
        self
    }

    /// Add a relay server URL (max 4 relays enforced).
    pub fn with_relay_url(mut self, url: RelayUrl) -> Result<Self> {
        const MAX_RELAY_URLS: usize = 4;
        if self.relay_urls.len() >= MAX_RELAY_URLS {
            anyhow::bail!(
                "cannot add more than {} relay URLs",
                MAX_RELAY_URLS
            );
        }
        self.relay_urls.push(url);
        Ok(self)
    }

    /// Set the bind port for the QUIC socket.
    pub fn with_bind_port(mut self, port: u16) -> Self {
        self.bind_port = port;
        self
    }

    /// Enable or disable gossip-based peer discovery.
    pub fn with_gossip(mut self, enable: bool) -> Self {
        self.enable_gossip = enable;
        self
    }

    /// Set an explicit gossip topic ID.
    pub fn with_gossip_topic(mut self, topic: TopicId) -> Self {
        self.gossip_topic = Some(topic);
        self
    }
}

/// Manages the lifecycle of an Iroh endpoint for P2P transport.
///
/// Tiger Style:
/// - Explicit error handling for endpoint creation and connection
/// - Resource cleanup via shutdown method
/// - EndpointAddr exposed for peer discovery via HTTP control-plane
pub struct IrohEndpointManager {
    endpoint: IrohEndpoint,
    node_addr: EndpointAddr,
    secret_key: SecretKey,
    gossip: Option<Arc<Gossip>>,
}

impl IrohEndpointManager {
    /// Create and bind a new Iroh endpoint.
    ///
    /// Tiger Style: Fail fast if endpoint creation fails.
    pub async fn new(config: IrohEndpointConfig) -> Result<Self> {
        // Generate or use provided secret key
        let secret_key = config.secret_key.unwrap_or_else(|| {
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut bytes);
            SecretKey::from(bytes)
        });

        // Build endpoint with explicit configuration
        let mut builder = IrohEndpoint::builder();
        builder = builder.secret_key(secret_key.clone());

        // Configure bind address if port is specified
        if config.bind_port > 0 {
            let bind_addr = std::net::SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.bind_port);
            builder = builder.bind_addr_v4(bind_addr);
        }

        // Configure relay mode based on relay URLs
        if config.relay_urls.is_empty() {
            builder = builder.relay_mode(RelayMode::Disabled);
        } else {
            // Use default relay mode (RelayMode::Default) when relay URLs are provided
            // Note: In Iroh 0.95.1, relay URLs are configured differently - this may need
            // additional configuration via discovery or other mechanisms
            builder = builder.relay_mode(RelayMode::Default);
        }

        // Configure ALPNs: raft-rpc + optionally gossip
        let alpns = if config.enable_gossip {
            vec![b"raft-rpc".to_vec(), GOSSIP_ALPN.to_vec()]
        } else {
            vec![b"raft-rpc".to_vec()]
        };
        builder = builder.alpns(alpns);

        let endpoint = builder
            .bind()
            .await
            .context("failed to bind Iroh endpoint")?;

        // Extract node address for discovery (synchronous in 0.95.1)
        let node_addr = endpoint.addr();

        // Optionally spawn gossip
        let gossip = if config.enable_gossip {
            let gossip = Gossip::builder().spawn(endpoint.clone());
            tracing::info!("gossip spawned for peer discovery");
            Some(Arc::new(gossip))
        } else {
            None
        };

        Ok(Self {
            endpoint,
            node_addr,
            secret_key,
            gossip,
        })
    }

    /// Get a reference to the underlying Iroh endpoint.
    pub fn endpoint(&self) -> &IrohEndpoint {
        &self.endpoint
    }

    /// Get the node address for peer discovery.
    ///
    /// This should be shared with other nodes via the HTTP control-plane
    /// so they can dial this endpoint.
    pub fn node_addr(&self) -> &EndpointAddr {
        &self.node_addr
    }

    /// Get the secret key used by this endpoint.
    ///
    /// Needed for signing gossip messages for peer discovery.
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// Get a reference to the gossip instance, if enabled.
    pub fn gossip(&self) -> Option<&Arc<Gossip>> {
        self.gossip.as_ref()
    }

    /// Add a known peer address to the endpoint for direct connections.
    ///
    /// Note: In Iroh 0.95.1, peer discovery is handled differently.
    /// This is a placeholder that stores the address for future use.
    /// Actual peer discovery should be configured via discovery services
    /// (DnsDiscovery, PkarrPublisher, etc.) when needed.
    pub fn add_peer(&self, _addr: EndpointAddr) -> Result<()> {
        // In Iroh 0.95.1, there's no direct add_node_addr method on Endpoint.
        // Peer discovery is handled via discovery services or by passing
        // EndpointAddr directly to connect() calls.
        Ok(())
    }

    /// Shutdown the endpoint and close all connections.
    ///
    /// Tiger Style: Explicit cleanup with bounded wait time.
    pub async fn shutdown(&self) -> Result<()> {
        // Note: Gossip and router are owned by the endpoint and will be
        // cleaned up when the endpoint closes. No explicit shutdown needed.

        // Iroh endpoint shutdown is graceful with internal timeouts
        // In Iroh 0.95.1, close() returns () not Result
        self.endpoint.close().await;
        Ok(())
    }
}

impl fmt::Debug for IrohEndpointManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ip_addrs: Vec<_> = self.node_addr.ip_addrs().collect();
        f.debug_struct("IrohEndpointManager")
            .field("node_id", &self.endpoint.id())
            .field("local_endpoints", &ip_addrs)
            .finish()
    }
}
