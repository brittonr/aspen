//! Cluster coordination and peer discovery for Aspen.
//!
//! This module provides the infrastructure for distributed cluster coordination,
//! including:
//!
//! - **Iroh P2P Transport**: QUIC-based peer-to-peer networking with NAT traversal (exclusive inter-node transport)
//! - **Gossip-based Peer Discovery**: Automatic node discovery via iroh-gossip (default)
//! - **Cluster Tickets**: Compact bootstrap information for joining clusters
//! - **Manual Peer Configuration**: Explicit peer list as fallback when gossip is disabled
//!
//! # Test Coverage
//!
//! TODO: Add unit tests for IrohEndpointManager:
//!       - Endpoint creation with various IrohEndpointConfig options
//!       - Discovery service integration (mDNS, DNS, Pkarr)
//!       - Router spawning and ALPN protocol registration
//!       - Graceful shutdown sequence
//!       Coverage: 0% line coverage (requires Iroh endpoint mocking)
//!
//! # Peer Discovery
//!
//! Aspen provides multiple automatic discovery mechanisms (all can work simultaneously):
//!
//! ## Iroh Discovery Services (Establish Connectivity)
//!
//! 1. **mDNS** (enabled by default): Discovers peers on the same LAN
//!    - Works automatically for multi-machine testing on same network
//!    - Does NOT work on localhost/127.0.0.1 (multicast limitation)
//!
//! 2. **DNS Discovery** (opt-in): Production peer discovery via DNS service
//!    - Query DNS service for initial bootstrap peers
//!    - Recommended for cloud/multi-region deployments
//!
//! 3. **Pkarr** (opt-in): DHT-based distributed discovery
//!    - Publish node addresses to DHT relay
//!    - Provides resilience against DNS failures
//!
//! ## Gossip (Broadcasts Raft Metadata - Default)
//!
//! Once Iroh connectivity is established (via mDNS, DNS, Pkarr, or manual):
//! 1. Each node subscribes to a gossip topic (derived from cluster cookie)
//! 2. Nodes broadcast their `node_id` + `EndpointAddr` every 10 seconds
//! 3. Received announcements are automatically added to the Raft network factory
//! 4. Raft RPCs can then flow to discovered peers
//!
//! ## Manual Peers (Fallback)
//!
//! When all discovery is disabled, nodes must be configured with explicit peer addresses:
//! - Via CLI: `--peers "node_id@endpoint_id"`
//! - Via config file: `peers = ["node_id@endpoint_id"]`
//! - Use for: single-host testing, airgapped deployments, custom discovery logic
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
//! │  RaftNode       │  Direct async API for Raft consensus
//! │  (openraft)     │
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
use std::net::Ipv4Addr;
use std::sync::Arc;

use anyhow::{Context, Result};
use iroh::protocol::Router;
use iroh::{Endpoint as IrohEndpoint, EndpointAddr, RelayMode, SecretKey};
use iroh_gossip::net::{GOSSIP_ALPN, Gossip};
use iroh_gossip::proto::TopicId;

pub mod bootstrap;
pub mod config;
pub mod gossip_discovery;
pub mod metadata;
pub mod ticket;
pub mod validation;

/// Controls how the node server should behave while running in deterministic
/// simulations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeterministicClusterConfig {
    pub simulation_seed: Option<u64>,
}

/// Configuration for Iroh endpoint creation.
///
/// Tiger Style: Fixed limits and explicit configuration.
/// Relay URLs are optional but bounded (max 4 relay servers).
#[derive(Debug, Clone)]
pub struct IrohEndpointConfig {
    /// Optional secret key for the endpoint. If None, a new key is generated.
    pub secret_key: Option<SecretKey>,
    /// Bind port for the QUIC socket (0 = random port).
    /// Applied to both IPv4 and IPv6 if enabled.
    pub bind_port: u16,
    /// Enable IPv6 binding in addition to IPv4 (default: true).
    ///
    /// When true, the endpoint will bind to both IPv4 (0.0.0.0) and IPv6 (::)
    /// on the specified port. This enables dual-stack operation for better
    /// connectivity in IPv6-enabled environments.
    ///
    /// Set to false if:
    /// - Running on IPv4-only infrastructure
    /// - IPv6 causes connectivity issues in your environment
    /// - You need to bind to a specific IPv4-only interface
    pub enable_ipv6: bool,
    /// Enable gossip-based peer discovery (default: true).
    pub enable_gossip: bool,
    /// Optional explicit gossip topic ID. If None, derived from cluster cookie.
    pub gossip_topic: Option<TopicId>,
    /// Enable mDNS discovery for local network (default: true).
    pub enable_mdns: bool,
    /// Enable DNS discovery for production (default: false).
    pub enable_dns_discovery: bool,
    /// Custom DNS discovery URL (None = use n0's service).
    pub dns_discovery_url: Option<String>,
    /// Enable Pkarr publisher (default: false).
    pub enable_pkarr: bool,
    /// ALPNs (Application-Layer Protocol Negotiation) to accept on this endpoint.
    ///
    /// # When to Set ALPNs
    ///
    /// **Using Iroh Router (recommended, bootstrap.rs):**
    /// - ALPNs are set AUTOMATICALLY via `Router::builder().accept(ALPN, handler)` calls
    /// - DO NOT set `alpns` here when using Router - they will be configured by Router
    /// - The Router handles ALPN-based protocol dispatching for Raft RPC, Gossip, etc.
    ///
    /// **NOT using Router (legacy/testing):**
    /// - Set `alpns` here to the list of protocols this endpoint should accept
    /// - Required for `endpoint.accept()` to work with specific protocols
    /// - Example: `vec![RAFT_ALPN.to_vec(), GOSSIP_ALPN.to_vec()]`
    ///
    /// # Common ALPNs in Aspen
    ///
    /// - `RAFT_ALPN` ("raft-rpc"): Raft consensus RPC between cluster nodes
    /// - `CLIENT_ALPN` ("aspen-client"): Client RPC connections
    /// - `GOSSIP_ALPN` ("iroh-gossip/0"): Peer discovery via iroh-gossip
    ///
    /// # Architecture Note
    ///
    /// In production (aspen-node), the Router is spawned AFTER bootstrap via
    /// `iroh_manager.spawn_router()`, which registers all protocol handlers and
    /// sets ALPNs automatically. The `alpns` field here is primarily for:
    /// - Testing scenarios without Router
    /// - Custom endpoint configurations
    pub alpns: Vec<Vec<u8>>,
}

impl Default for IrohEndpointConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            bind_port: 0,
            enable_ipv6: true, // Enable dual-stack by default for better connectivity
            enable_gossip: true,
            gossip_topic: None,
            enable_mdns: true,
            enable_dns_discovery: false,
            dns_discovery_url: None,
            enable_pkarr: false,
            alpns: Vec::new(),
        }
    }
}

impl IrohEndpointConfig {
    /// Create a new endpoint configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the secret key for deterministic endpoint identity.
    pub fn with_secret_key(mut self, key: SecretKey) -> Self {
        self.secret_key = Some(key);
        self
    }

    /// Set the bind port for the QUIC socket.
    pub fn with_bind_port(mut self, port: u16) -> Self {
        self.bind_port = port;
        self
    }

    /// Enable or disable IPv6 dual-stack binding.
    ///
    /// When enabled (default), the endpoint binds to both IPv4 and IPv6.
    /// Disable for IPv4-only environments or when IPv6 causes issues.
    pub fn with_ipv6(mut self, enable: bool) -> Self {
        self.enable_ipv6 = enable;
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

    /// Enable or disable mDNS discovery.
    pub fn with_mdns(mut self, enable: bool) -> Self {
        self.enable_mdns = enable;
        self
    }

    /// Enable or disable DNS discovery.
    pub fn with_dns_discovery(mut self, enable: bool) -> Self {
        self.enable_dns_discovery = enable;
        self
    }

    /// Set custom DNS discovery URL.
    pub fn with_dns_discovery_url(mut self, url: String) -> Self {
        self.dns_discovery_url = Some(url);
        self
    }

    /// Enable or disable Pkarr publisher.
    pub fn with_pkarr(mut self, enable: bool) -> Self {
        self.enable_pkarr = enable;
        self
    }

    /// Set ALPNs to accept on this endpoint.
    ///
    /// Required when NOT using an Iroh Router for ALPN dispatching.
    /// The endpoint must have at least one ALPN configured to accept incoming connections.
    pub fn with_alpns(mut self, alpns: Vec<Vec<u8>>) -> Self {
        self.alpns = alpns;
        self
    }

    /// Add a single ALPN to accept on this endpoint.
    pub fn with_alpn(mut self, alpn: Vec<u8>) -> Self {
        self.alpns.push(alpn);
        self
    }
}

/// Manages the lifecycle of an Iroh endpoint for P2P transport.
///
/// Uses Iroh Router for proper ALPN-based protocol dispatching.
/// Protocol handlers are registered with the Router during initialization.
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
    /// The Iroh Router for ALPN-based protocol dispatching.
    /// If None, protocol handlers must be registered separately.
    router: Option<Router>,
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
            rand::rng().fill_bytes(&mut bytes);
            SecretKey::from(bytes)
        });

        // Build endpoint with explicit configuration
        let mut builder = IrohEndpoint::builder();
        builder = builder.secret_key(secret_key.clone());

        // Configure bind addresses if port is specified
        // Tiger Style: Support both IPv4 and IPv6 for dual-stack connectivity
        if config.bind_port > 0 {
            // Always bind IPv4
            let bind_addr_v4 = std::net::SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.bind_port);
            builder = builder.bind_addr_v4(bind_addr_v4);
            tracing::info!(port = config.bind_port, "bound IPv4 address");

            // Optionally bind IPv6 for dual-stack
            if config.enable_ipv6 {
                let bind_addr_v6 = std::net::SocketAddrV6::new(
                    std::net::Ipv6Addr::UNSPECIFIED,
                    config.bind_port,
                    0,
                    0,
                );
                builder = builder.bind_addr_v6(bind_addr_v6);
                tracing::info!(port = config.bind_port, "bound IPv6 address (dual-stack)");
            }
        }

        // Configure relay mode based on relay URLs
        // Use default relay mode for NAT traversal
        builder = builder.relay_mode(RelayMode::Default);

        // Configure ALPNs if provided.
        // When using Router (spawn_router()), ALPNs are set automatically via .accept() calls.
        // When NOT using Router (use_router=false), ALPNs must be set here for the endpoint
        // to accept incoming connections.
        if !config.alpns.is_empty() {
            builder = builder.alpns(config.alpns.clone());
            tracing::info!(
                alpns = ?config.alpns.iter().map(|a| String::from_utf8_lossy(a).to_string()).collect::<Vec<_>>(),
                "configured endpoint to accept ALPNs"
            );
        }

        // Configure discovery services for bootstrapping network connectivity
        // mDNS: Local network discovery (default enabled for dev/testing)
        if config.enable_mdns {
            builder = builder.discovery(iroh::discovery::mdns::MdnsDiscovery::builder());
            tracing::info!("mDNS discovery enabled for local network peer discovery");
        }

        // DNS Discovery: Production peer discovery via DNS lookups
        if config.enable_dns_discovery {
            let dns_discovery_builder = if let Some(ref url) = config.dns_discovery_url {
                iroh::discovery::dns::DnsDiscovery::builder(url.clone())
            } else {
                iroh::discovery::dns::DnsDiscovery::n0_dns()
            };
            builder = builder.discovery(dns_discovery_builder);
            tracing::info!(
                "DNS discovery enabled with URL: {}",
                config
                    .dns_discovery_url
                    .as_deref()
                    .unwrap_or("n0 DNS service (iroh.link)")
            );
        }

        // Pkarr Publisher: Publish node addresses to DHT-based relay
        if config.enable_pkarr {
            let pkarr_builder = iroh::discovery::pkarr::PkarrPublisher::n0_dns();
            builder = builder.discovery(pkarr_builder);
            tracing::info!("Pkarr publisher enabled with n0 Pkarr service");
        }

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
            router: None, // Router is created later via spawn_router()
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

    /// Get a reference to the router, if spawned.
    pub fn router(&self) -> Option<&Router> {
        self.router.as_ref()
    }

    /// Spawn the Iroh Router with protocol handlers.
    ///
    /// This method creates a Router that properly dispatches incoming connections
    /// based on their ALPN. This eliminates the race condition that occurred when
    /// multiple servers were accepting from the same endpoint.
    ///
    /// # Arguments
    /// * `raft_handler` - Protocol handler for Raft RPC (ALPN: `raft-rpc`)
    /// * `tui_handler` - Optional protocol handler for TUI RPC (ALPN: `aspen-tui`)
    ///
    /// # Tiger Style
    /// - Must be called before any server starts accepting connections
    /// - ALPNs are set automatically by the Router
    pub fn spawn_router<R, T>(&mut self, raft_handler: R, tui_handler: Option<T>)
    where
        R: iroh::protocol::ProtocolHandler,
        T: iroh::protocol::ProtocolHandler,
    {
        use crate::protocol_handlers::{CLIENT_ALPN, RAFT_ALPN};

        let mut builder = Router::builder(self.endpoint.clone());

        // Register Raft RPC handler
        builder = builder.accept(RAFT_ALPN, raft_handler);
        tracing::info!("registered Raft RPC protocol handler (ALPN: raft-rpc)");

        // Register Client RPC handler if provided
        if let Some(handler) = tui_handler {
            builder = builder.accept(CLIENT_ALPN, handler);
            tracing::info!("registered Client RPC protocol handler (ALPN: aspen-tui)");
        }

        // Register Gossip handler if enabled
        if let Some(gossip) = &self.gossip {
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
            tracing::info!("registered Gossip protocol handler (ALPN: gossip)");
        }

        // Spawn the router (sets ALPNs and starts accept loop)
        self.router = Some(builder.spawn());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Spawn the Iroh Router with extended protocol handlers.
    ///
    /// This method creates a Router with support for:
    /// - Legacy unauthenticated Raft RPC (ALPN: `raft-rpc`) - required
    /// - Authenticated Raft RPC (ALPN: `raft-auth`) - optional
    /// - Log subscription (ALPN: `aspen-logs`) - optional
    /// - Client RPC (ALPN: `aspen-tui`) - optional
    /// - Gossip (ALPN: `iroh-gossip/0`) - automatic if enabled
    ///
    /// # Arguments
    /// * `raft_handler` - Protocol handler for unauthenticated Raft RPC
    /// * `auth_raft_handler` - Optional handler for authenticated Raft RPC
    /// * `log_subscriber_handler` - Optional handler for log subscription
    /// * `client_handler` - Optional handler for Client RPC
    ///
    /// # Tiger Style
    /// - Must be called before any server starts accepting connections
    /// - ALPNs are set automatically by the Router
    pub fn spawn_router_extended<R, A, L, C>(
        &mut self,
        raft_handler: R,
        auth_raft_handler: Option<A>,
        log_subscriber_handler: Option<L>,
        client_handler: Option<C>,
    ) where
        R: iroh::protocol::ProtocolHandler,
        A: iroh::protocol::ProtocolHandler,
        L: iroh::protocol::ProtocolHandler,
        C: iroh::protocol::ProtocolHandler,
    {
        use crate::protocol_handlers::{
            CLIENT_ALPN, LOG_SUBSCRIBER_ALPN, RAFT_ALPN, RAFT_AUTH_ALPN,
        };

        let mut builder = Router::builder(self.endpoint.clone());

        // Register legacy Raft RPC handler (unauthenticated)
        builder = builder.accept(RAFT_ALPN, raft_handler);
        tracing::info!("registered Raft RPC protocol handler (ALPN: raft-rpc)");

        // Register authenticated Raft RPC handler
        if let Some(handler) = auth_raft_handler {
            builder = builder.accept(RAFT_AUTH_ALPN, handler);
            tracing::info!("registered authenticated Raft RPC protocol handler (ALPN: raft-auth)");
        }

        // Register log subscriber handler
        if let Some(handler) = log_subscriber_handler {
            builder = builder.accept(LOG_SUBSCRIBER_ALPN, handler);
            tracing::info!("registered log subscriber protocol handler (ALPN: aspen-logs)");
        }

        // Register Client RPC handler if provided
        if let Some(handler) = client_handler {
            builder = builder.accept(CLIENT_ALPN, handler);
            tracing::info!("registered Client RPC protocol handler (ALPN: aspen-tui)");
        }

        // Register Gossip handler if enabled
        if let Some(gossip) = &self.gossip {
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
            tracing::info!("registered Gossip protocol handler (ALPN: gossip)");
        }

        // Spawn the router (sets ALPNs and starts accept loop)
        self.router = Some(builder.spawn());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Spawn the Iroh Router with full protocol support including blobs.
    ///
    /// This method creates a Router with support for:
    /// - Legacy unauthenticated Raft RPC (ALPN: `raft-rpc`) - required
    /// - Authenticated Raft RPC (ALPN: `raft-auth`) - optional
    /// - Log subscription (ALPN: `aspen-logs`) - optional
    /// - Client RPC (ALPN: `aspen-tui`) - optional
    /// - Blobs (ALPN: `iroh-blobs/0`) - optional
    /// - Gossip (ALPN: `iroh-gossip/0`) - automatic if enabled
    ///
    /// # Arguments
    /// * `raft_handler` - Protocol handler for unauthenticated Raft RPC
    /// * `auth_raft_handler` - Optional handler for authenticated Raft RPC
    /// * `log_subscriber_handler` - Optional handler for log subscription
    /// * `client_handler` - Optional handler for Client RPC
    /// * `blobs_handler` - Optional handler for blob transfers
    ///
    /// # Tiger Style
    /// - Must be called before any server starts accepting connections
    /// - ALPNs are set automatically by the Router
    pub fn spawn_router_full<R, A, L, C, B>(
        &mut self,
        raft_handler: R,
        auth_raft_handler: Option<A>,
        log_subscriber_handler: Option<L>,
        client_handler: Option<C>,
        blobs_handler: Option<B>,
    ) where
        R: iroh::protocol::ProtocolHandler,
        A: iroh::protocol::ProtocolHandler,
        L: iroh::protocol::ProtocolHandler,
        C: iroh::protocol::ProtocolHandler,
        B: iroh::protocol::ProtocolHandler,
    {
        use crate::protocol_handlers::{
            CLIENT_ALPN, LOG_SUBSCRIBER_ALPN, RAFT_ALPN, RAFT_AUTH_ALPN,
        };

        let mut builder = Router::builder(self.endpoint.clone());

        // Register legacy Raft RPC handler (unauthenticated)
        builder = builder.accept(RAFT_ALPN, raft_handler);
        tracing::info!("registered Raft RPC protocol handler (ALPN: raft-rpc)");

        // Register authenticated Raft RPC handler
        if let Some(handler) = auth_raft_handler {
            builder = builder.accept(RAFT_AUTH_ALPN, handler);
            tracing::info!("registered authenticated Raft RPC protocol handler (ALPN: raft-auth)");
        }

        // Register log subscriber handler
        if let Some(handler) = log_subscriber_handler {
            builder = builder.accept(LOG_SUBSCRIBER_ALPN, handler);
            tracing::info!("registered log subscriber protocol handler (ALPN: aspen-logs)");
        }

        // Register Client RPC handler if provided
        if let Some(handler) = client_handler {
            builder = builder.accept(CLIENT_ALPN, handler);
            tracing::info!("registered Client RPC protocol handler (ALPN: aspen-tui)");
        }

        // Register Blobs handler if provided
        if let Some(handler) = blobs_handler {
            builder = builder.accept(iroh_blobs::ALPN, handler);
            tracing::info!("registered Blobs protocol handler (ALPN: iroh-blobs/0)");
        }

        // Register Gossip handler if enabled
        if let Some(gossip) = &self.gossip {
            builder = builder.accept(GOSSIP_ALPN, gossip.clone());
            tracing::info!("registered Gossip protocol handler (ALPN: gossip)");
        }

        // Spawn the router (sets ALPNs and starts accept loop)
        self.router = Some(builder.spawn());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Add a known peer address to the endpoint for direct connections.
    ///
    /// # Iroh 0.95.1 Behavior
    ///
    /// In Iroh 0.95.1, peer addresses are added implicitly when calling `connect()`.
    /// The endpoint's discovery services (mDNS, DNS, Pkarr) handle address resolution.
    ///
    /// This method logs the address for debugging but does NOT store it directly.
    /// For Raft peer management, use `IrpcRaftNetworkFactory::add_peer()` instead,
    /// which stores addresses in the network factory's peer map.
    ///
    /// # Arguments
    /// * `addr` - The EndpointAddr to add (logged but not stored)
    ///
    /// # Returns
    /// Always returns `Ok(())`. Use `IrpcRaftNetworkFactory::add_peer()` for actual
    /// peer management in Raft clusters.
    pub fn add_peer(&self, addr: EndpointAddr) -> Result<()> {
        // In Iroh 0.95.1, peer addresses are added implicitly via connect() or
        // through discovery services. This method cannot directly store addresses.
        //
        // For Raft peer management, use IrpcRaftNetworkFactory::add_peer() instead.
        // That method stores addresses in the network factory's peer map, which
        // is then used when creating connections via new_client().
        tracing::debug!(
            peer_id = %addr.id,
            direct_addrs = addr.ip_addrs().count(),
            "add_peer called but Iroh 0.95.1 doesn't support direct address storage; \
             use IrpcRaftNetworkFactory::add_peer() for Raft peer management"
        );
        Ok(())
    }

    /// Shutdown the endpoint and close all connections.
    ///
    /// Tiger Style: Explicit cleanup with bounded wait time.
    pub async fn shutdown(&self) -> Result<()> {
        // Shutdown the Router first (this stops accepting new connections)
        if let Some(router) = &self.router {
            router
                .shutdown()
                .await
                .context("failed to shutdown Iroh Router")?;
            tracing::info!("Iroh Router shutdown complete");
        }

        // Iroh endpoint shutdown is graceful with internal timeouts
        // In Iroh 0.95.1, close() returns () not Result
        self.endpoint.close().await;
        tracing::info!("Iroh endpoint closed");

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
