//! Iroh endpoint manager for P2P transport lifecycle.

use std::fmt;
use std::net::Ipv4Addr;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use iroh::Endpoint as IrohEndpoint;
use iroh::EndpointAddr;
use iroh::RelayMode;
use iroh::SecretKey;
use iroh::protocol::Router;
use iroh_gossip::net::Gossip;

use crate::IrohEndpointConfig;
use crate::RouterBuilder;
use crate::config;
use crate::transport;

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
    ///
    /// # Key Persistence
    ///
    /// The secret key is resolved in the following order:
    /// 1. If `config.secret_key` is provided, use it (explicit config takes priority)
    /// 2. If `config.secret_key_path` is set and the file exists, load from file
    /// 3. Otherwise, generate a new key and save it to `secret_key_path` if set
    ///
    /// This ensures stable node identity across restarts when `secret_key_path` is configured.
    pub async fn new(config: IrohEndpointConfig) -> Result<Self> {
        // Resolve secret key with persistence support
        let secret_key = Self::resolve_secret_key(&config)?;

        // Configure transport to allow larger messages for git bridge operations
        let transport_config = Self::build_transport_config();

        // Build endpoint with explicit configuration
        let mut builder = IrohEndpoint::builder();
        builder = builder.secret_key(secret_key.clone());
        builder = builder.transport_config(transport_config);

        // Configure bind addresses if port is specified
        builder = Self::configure_bind_addresses(builder, &config);

        // Configure relay mode
        let relay_mode = Self::resolve_relay_mode(&config);
        builder = builder.relay_mode(relay_mode);

        // Configure ALPNs if provided
        builder = Self::configure_alpns(builder, &config);

        // Configure discovery services
        builder = Self::configure_discovery(builder, &config);

        let endpoint = builder.bind().await.context("failed to bind Iroh endpoint")?;

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

    /// Build transport config with larger windows for git bridge operations.
    fn build_transport_config() -> iroh::endpoint::TransportConfig {
        use iroh::endpoint::TransportConfig;
        use iroh::endpoint::VarInt;
        let mut transport_config = TransportConfig::default();

        // Tiger Style: validate transport config constants
        const STREAM_WINDOW_BYTES: u32 = 64 * 1024 * 1024; // 64MB
        const CONNECTION_WINDOW_BYTES: u32 = 256 * 1024 * 1024; // 256MB
        const _: () = assert!(
            STREAM_WINDOW_BYTES <= CONNECTION_WINDOW_BYTES,
            "ENDPOINT: stream window must not exceed connection window"
        );

        // Set stream receive window to 64MB to handle large git objects
        transport_config.stream_receive_window(VarInt::from_u32(STREAM_WINDOW_BYTES));
        // Set connection receive window to 256MB
        transport_config.receive_window(VarInt::from_u32(CONNECTION_WINDOW_BYTES));
        // Set idle timeout high enough for server-side processing of large git batches.
        // Processing 5000+ tree objects sequentially can take 30-60 seconds.
        // Default QUIC idle timeout is often 30-60s, which is too short.
        // 600 seconds is well within the valid range for QUIC idle timeout.
        // Use a match to satisfy Tiger Style (no .expect() in production).
        let idle_timeout = match std::time::Duration::from_secs(600).try_into() {
            Ok(timeout) => timeout,
            Err(_) => {
                debug_assert!(false, "600 seconds should be a valid idle timeout");
                // Fall back to default if conversion somehow fails
                return transport_config;
            }
        };
        transport_config.max_idle_timeout(Some(idle_timeout));
        transport_config
    }

    /// Configure bind addresses based on port and IPv6 settings.
    fn configure_bind_addresses(
        mut builder: iroh::endpoint::Builder,
        config: &IrohEndpointConfig,
    ) -> iroh::endpoint::Builder {
        // Tiger Style: Support both IPv4 and IPv6 for dual-stack connectivity
        if config.bind_port > 0 {
            // Always bind IPv4
            let bind_addr_v4 = std::net::SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.bind_port);
            builder = builder.bind_addr_v4(bind_addr_v4);
            tracing::info!(port = config.bind_port, "bound IPv4 address");

            // Optionally bind IPv6 for dual-stack
            if config.enable_ipv6 {
                let bind_addr_v6 = std::net::SocketAddrV6::new(std::net::Ipv6Addr::UNSPECIFIED, config.bind_port, 0, 0);
                builder = builder.bind_addr_v6(bind_addr_v6);
                tracing::info!(port = config.bind_port, "bound IPv6 address (dual-stack)");
            }
        }
        builder
    }

    /// Resolve relay mode from configuration.
    fn resolve_relay_mode(config: &IrohEndpointConfig) -> RelayMode {
        match config.relay_mode {
            config::RelayMode::Default => {
                tracing::info!("using default n0 relay infrastructure");
                RelayMode::Default
            }
            config::RelayMode::Disabled => {
                tracing::info!("relay servers disabled - direct connections only");
                RelayMode::Disabled
            }
            config::RelayMode::Custom => Self::resolve_custom_relay_mode(config),
        }
    }

    /// Resolve custom relay mode, parsing relay URLs.
    fn resolve_custom_relay_mode(config: &IrohEndpointConfig) -> RelayMode {
        if config.relay_urls.is_empty() {
            tracing::warn!("relay_mode is Custom but no relay_urls configured, falling back to default");
            return RelayMode::Default;
        }

        use iroh::RelayUrl;
        let mut relay_nodes = Vec::new();
        for url_str in &config.relay_urls {
            match url_str.parse::<RelayUrl>() {
                Ok(url) => {
                    relay_nodes.push(url);
                    tracing::info!(url = %url_str, "added custom relay server");
                }
                Err(e) => {
                    tracing::warn!(
                        url = %url_str,
                        error = %e,
                        "failed to parse relay URL, skipping"
                    );
                }
            }
        }

        if relay_nodes.is_empty() {
            tracing::warn!("no valid relay URLs parsed, falling back to default relays");
            RelayMode::Default
        } else {
            let relay_map: iroh::RelayMap = relay_nodes.into_iter().collect();
            tracing::info!(relay_count = relay_map.len(), "configured custom relay infrastructure");
            RelayMode::Custom(relay_map)
        }
    }

    /// Configure ALPNs on the endpoint builder if provided.
    fn configure_alpns(mut builder: iroh::endpoint::Builder, config: &IrohEndpointConfig) -> iroh::endpoint::Builder {
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
        builder
    }

    /// Configure discovery services on the endpoint builder.
    fn configure_discovery(
        mut builder: iroh::endpoint::Builder,
        config: &IrohEndpointConfig,
    ) -> iroh::endpoint::Builder {
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
                config.dns_discovery_url.as_deref().unwrap_or("n0 DNS service (iroh.link)")
            );
        }

        // Pkarr DHT Discovery
        if config.enable_pkarr {
            builder = Self::configure_pkarr_discovery(builder, config);
        }

        builder
    }

    /// Configure Pkarr DHT discovery on the endpoint builder.
    fn configure_pkarr_discovery(
        mut builder: iroh::endpoint::Builder,
        config: &IrohEndpointConfig,
    ) -> iroh::endpoint::Builder {
        use iroh::discovery::pkarr::dht::DhtDiscovery;

        let mut dht_builder = DhtDiscovery::builder()
            .dht(config.enable_pkarr_dht)
            .include_direct_addresses(config.include_pkarr_direct_addresses)
            .republish_delay(std::time::Duration::from_secs(config.pkarr_republish_delay_secs));

        // Add relay if enabled
        if config.enable_pkarr_relay {
            dht_builder = Self::configure_pkarr_relay(dht_builder, config);
        }

        builder = builder.discovery(dht_builder);
        tracing::info!(
            dht_enabled = config.enable_pkarr_dht,
            relay_enabled = config.enable_pkarr_relay,
            custom_relay = config.pkarr_relay_url.is_some(),
            include_direct_addrs = config.include_pkarr_direct_addresses,
            republish_delay_secs = config.pkarr_republish_delay_secs,
            "Pkarr DHT discovery enabled (publish + resolve)"
        );

        builder
    }

    /// Configure Pkarr relay settings on the DHT builder.
    fn configure_pkarr_relay(
        mut dht_builder: iroh::discovery::pkarr::dht::Builder,
        config: &IrohEndpointConfig,
    ) -> iroh::discovery::pkarr::dht::Builder {
        if let Some(ref relay_url) = config.pkarr_relay_url {
            // Use custom pkarr relay URL (private infrastructure)
            match relay_url.parse::<url::Url>() {
                Ok(url) => {
                    dht_builder = dht_builder.pkarr_relay(url);
                    tracing::info!(
                        relay_url = %relay_url,
                        "Pkarr using custom relay server (private infrastructure)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        relay_url = %relay_url,
                        error = %e,
                        "failed to parse custom pkarr relay URL, falling back to n0 relay"
                    );
                    dht_builder = dht_builder.n0_dns_pkarr_relay();
                }
            }
        } else {
            // Use n0's default relay at dns.iroh.link
            dht_builder = dht_builder.n0_dns_pkarr_relay();
            tracing::debug!("Pkarr using n0 default relay (dns.iroh.link)");
        }
        dht_builder
    }

    /// Resolve the secret key from config, file, or generate a new one.
    ///
    /// Priority order:
    /// 1. Explicit config.secret_key (takes precedence)
    /// 2. Load from config.secret_key_path if file exists
    /// 3. Generate new key and save to secret_key_path if set
    fn resolve_secret_key(config: &IrohEndpointConfig) -> Result<SecretKey> {
        // 1. Explicit key in config takes priority
        if let Some(ref key) = config.secret_key {
            tracing::info!("using explicitly configured secret key");
            return Ok(key.clone());
        }

        // 2. Try to load from file if path is configured
        if let Some(ref path) = config.secret_key_path {
            if path.exists() {
                match Self::load_secret_key_from_file(path) {
                    Ok(key) => {
                        tracing::info!(
                            path = %path.display(),
                            endpoint_id = %key.public().to_string(),
                            "loaded existing secret key from file"
                        );
                        return Ok(key);
                    }
                    Err(e) => {
                        // Log warning but continue to generate new key
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "failed to load secret key from file, generating new key"
                        );
                    }
                }
            }
        }

        // 3. Generate new key
        let secret_key = Self::generate_secret_key();

        // Save to file if path is configured
        Self::persist_secret_key(&secret_key, config);

        Ok(secret_key)
    }

    /// Generate a new random secret key.
    fn generate_secret_key() -> SecretKey {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        SecretKey::from(bytes)
    }

    /// Persist secret key to file if path is configured.
    fn persist_secret_key(secret_key: &SecretKey, config: &IrohEndpointConfig) {
        if let Some(ref path) = config.secret_key_path {
            match Self::save_secret_key_to_file(secret_key, path) {
                Ok(()) => {
                    tracing::info!(
                        path = %path.display(),
                        endpoint_id = %secret_key.public().to_string(),
                        "generated and saved new secret key to file"
                    );
                }
                Err(e) => {
                    // Log error but don't fail - node can still run with ephemeral key
                    tracing::error!(
                        path = %path.display(),
                        error = %e,
                        "failed to save secret key to file - identity will not persist across restarts"
                    );
                }
            }
        } else {
            tracing::info!(
                endpoint_id = %secret_key.public().to_string(),
                "generated ephemeral secret key (no persistence path configured)"
            );
        }
    }

    /// Load a secret key from a hex-encoded file.
    ///
    /// File format: 64 hex characters (32 bytes) with optional trailing newline.
    fn load_secret_key_from_file(path: &std::path::Path) -> Result<SecretKey> {
        // Tiger Style: path must exist for loading
        debug_assert!(path.exists(), "ENDPOINT: secret key path must exist for loading");

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read secret key file: {}", path.display()))?;

        let hex_str = contents.trim();

        // Validate length (64 hex chars = 32 bytes)
        if hex_str.len() != 64 {
            anyhow::bail!("invalid secret key file: expected 64 hex characters, got {}", hex_str.len());
        }

        let bytes = hex::decode(hex_str).with_context(|| "failed to decode secret key hex")?;

        // Tiger Style: decoded bytes must be exactly 32 bytes for Ed25519
        debug_assert!(bytes.len() == 32, "ENDPOINT: decoded secret key must be 32 bytes");

        let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("invalid secret key length"))?;

        Ok(SecretKey::from(bytes_array))
    }

    /// Save a secret key to a hex-encoded file with restrictive permissions.
    ///
    /// File format: 64 hex characters (32 bytes) with trailing newline.
    /// Permissions: 0600 (owner read/write only) on Unix.
    fn save_secret_key_to_file(key: &SecretKey, path: &std::path::Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        let hex_str = hex::encode(key.to_bytes());
        let contents = format!("{}\n", hex_str);

        // Write file with restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600) // Owner read/write only
                .open(path)
                .with_context(|| format!("failed to create secret key file: {}", path.display()))?;

            use std::io::Write;
            file.write_all(contents.as_bytes())
                .with_context(|| format!("failed to write secret key file: {}", path.display()))?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(path, contents)
                .with_context(|| format!("failed to write secret key file: {}", path.display()))?;
        }

        Ok(())
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

    /// Create a router builder for fluent protocol handler configuration.
    ///
    /// This is the preferred way to spawn a router with custom handlers.
    /// The builder automatically registers gossip if enabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// manager.spawn_router_with(|b| b
    ///     .raft(raft_handler)
    ///     .auth_raft(auth_handler)
    ///     .client(client_handler)
    ///     .blobs(blobs_handler));
    /// ```
    pub fn spawn_router_with<F>(&mut self, configure: F)
    where F: FnOnce(RouterBuilder) -> RouterBuilder {
        let builder = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        let configured = configure(builder);
        self.router = Some(configured.spawn_internal());
        tracing::info!("Iroh Router spawned with ALPN-based protocol dispatching");
    }

    /// Spawn the Iroh Router with protocol handlers.
    ///
    /// This method creates a Router that properly dispatches incoming connections
    /// based on their ALPN. This eliminates the race condition that occurred when
    /// multiple servers were accepting from the same endpoint.
    ///
    /// **Note**: Consider using `spawn_router_with()` for more flexible configuration.
    ///
    /// # Arguments
    /// * `raft_handler` - Protocol handler for Raft RPC (ALPN: `raft-rpc`)
    /// * `tui_handler` - Optional protocol handler for TUI RPC (ALPN: `aspen-tui`)
    ///
    /// # Tiger Style
    /// - Must be called before any server starts accepting connections
    /// - ALPNs are set automatically by the Router
    #[allow(deprecated)] // Legacy API for backward compatibility - use spawn_router_extended with auth_raft
    pub fn spawn_router<R, T>(&mut self, raft_handler: R, tui_handler: Option<T>)
    where
        R: iroh::protocol::ProtocolHandler,
        T: iroh::protocol::ProtocolHandler,
    {
        let mut rb = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        #[allow(deprecated)]
        {
            rb = rb.raft(raft_handler);
        }
        if let Some(handler) = tui_handler {
            rb = rb.client(handler);
        }
        self.router = Some(rb.spawn_internal());
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
    /// **Note**: Consider using `spawn_router_with()` for more flexible configuration.
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
    #[allow(deprecated)] // Legacy API supports both auth and non-auth raft handlers
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
        let mut rb = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        #[allow(deprecated)]
        {
            rb = rb.raft(raft_handler);
        }
        if let Some(handler) = auth_raft_handler {
            rb = rb.auth_raft(handler);
        }
        if let Some(handler) = log_subscriber_handler {
            rb = rb.log_subscriber(handler);
        }
        if let Some(handler) = client_handler {
            rb = rb.client(handler);
        }
        self.router = Some(rb.spawn_internal());
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
    /// **Note**: Consider using `spawn_router_with()` for more flexible configuration.
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
    #[allow(deprecated)] // Legacy API supports both auth and non-auth raft handlers
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
        let mut rb = RouterBuilder::new(Router::builder(self.endpoint.clone()), self.gossip.clone());
        #[allow(deprecated)]
        {
            rb = rb.raft(raft_handler);
        }
        if let Some(handler) = auth_raft_handler {
            rb = rb.auth_raft(handler);
        }
        if let Some(handler) = log_subscriber_handler {
            rb = rb.log_subscriber(handler);
        }
        if let Some(handler) = client_handler {
            rb = rb.client(handler);
        }
        #[cfg(feature = "blob")]
        if let Some(handler) = blobs_handler {
            rb = rb.blobs(handler);
        }
        #[cfg(not(feature = "blob"))]
        let _ = blobs_handler;
        self.router = Some(rb.spawn_internal());
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
            router.shutdown().await.context("failed to shutdown Iroh Router")?;
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

// ============================================================================
// NetworkTransport Implementation
// ============================================================================

#[async_trait::async_trait]
impl transport::NetworkTransport for IrohEndpointManager {
    type Endpoint = IrohEndpoint;
    type Address = EndpointAddr;
    type SecretKey = SecretKey;
    type Gossip = Gossip;

    fn node_addr(&self) -> &Self::Address {
        &self.node_addr
    }

    fn node_id_string(&self) -> String {
        self.endpoint.id().to_string()
    }

    fn secret_key(&self) -> &Self::SecretKey {
        &self.secret_key
    }

    fn endpoint(&self) -> &Self::Endpoint {
        &self.endpoint
    }

    fn gossip(&self) -> Option<&Arc<Self::Gossip>> {
        self.gossip.as_ref()
    }

    async fn shutdown(&self) -> Result<()> {
        // Delegate to the existing shutdown implementation
        IrohEndpointManager::shutdown(self).await
    }
}

impl transport::IrohTransportExt for IrohEndpointManager {
    fn router(&self) -> Option<&Router> {
        self.router.as_ref()
    }
}
