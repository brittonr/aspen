//! DNS protocol server for Aspen.
//!
//! This module provides a DNS server that serves records from the Aspen DNS layer
//! and optionally forwards unknown queries to upstream DNS servers.
//!
//! # Features
//!
//! - Serves DNS queries for configured zones from local cache
//! - Forwards unknown queries to upstream DNS servers (configurable)
//! - Supports UDP and TCP protocols
//! - Multiple zone support
//!
//! # Architecture
//!
//! ```text
//! DNS Queries (UDP/TCP :5353)
//!         ↓
//! DnsProtocolServer
//!         ↓
//!     ┌───┴───┐
//!     ↓       ↓
//! AspenDnsAuthority  ForwardingResolver
//! (local zones)      (upstream DNS)
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hickory_server::ServerFuture;
use hickory_server::authority::Authority;
use hickory_server::authority::AuthorityObject;
use hickory_server::authority::Catalog;
use tokio::net::TcpListener;
use tokio::net::UdpSocket;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::authority::AspenDnsAuthority;
use super::client::AspenDnsClient;
use super::config::DnsServerConfig;
use super::error::DnsError;
use super::error::DnsResult;

/// DNS protocol server for Aspen.
///
/// Serves DNS queries from the Aspen DNS layer's local cache for configured zones,
/// with optional forwarding of unknown queries to upstream DNS servers.
///
/// # Example
///
/// ```ignore
/// use aspen_dns::{DnsProtocolServer, AspenDnsClient, DnsServerConfig};
///
/// let config = DnsServerConfig {
///     enabled: true,
///     bind_addr: "127.0.0.1:5353".parse()?,
///     zones: vec!["aspen.local".to_string()],
///     upstreams: vec!["8.8.8.8:53".parse()?],
///     forwarding_enabled: true,
/// };
///
/// let client = Arc::new(AspenDnsClient::new());
/// let server = DnsProtocolServer::new(config, client)?;
/// server.run().await?;
/// ```
pub struct DnsProtocolServer {
    config: DnsServerConfig,
    #[allow(dead_code)]
    client: Arc<AspenDnsClient>,
    catalog: Catalog,
}

impl DnsProtocolServer {
    /// Create a new DNS protocol server.
    ///
    /// # Arguments
    ///
    /// * `config` - DNS server configuration
    /// * `client` - The DNS client with local cache
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Zone names are invalid
    pub fn new(config: DnsServerConfig, client: Arc<AspenDnsClient>) -> DnsResult<Self> {
        // Create catalog with authorities for each zone
        let mut catalog = Catalog::new();

        for zone in &config.zones {
            match AspenDnsAuthority::new(Arc::clone(&client), zone) {
                Ok(authority) => {
                    let origin = Authority::origin(&authority).clone();
                    // Wrap authority in Arc for the catalog
                    let authority_arc: Arc<dyn AuthorityObject> = Arc::new(authority);
                    catalog.upsert(origin, vec![authority_arc]);
                    info!(zone = %zone, "Added DNS authority for zone");
                }
                Err(e) => {
                    warn!(zone = %zone, error = %e, "Failed to create authority for zone");
                    return Err(DnsError::InvalidDomainWithName {
                        domain: zone.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        // TODO: Add forwarding resolver support for unknown queries
        if config.forwarding_enabled && !config.upstreams.is_empty() {
            info!(
                upstreams = ?config.upstreams,
                "Forwarding configured (not yet implemented)"
            );
        }

        Ok(Self {
            config,
            client,
            catalog,
        })
    }

    /// Start the DNS server.
    ///
    /// This binds to the configured address on both UDP and TCP and begins
    /// serving DNS queries. The method runs until the server is shut down.
    ///
    /// # Errors
    ///
    /// Returns an error if binding to the socket fails.
    pub async fn run(self) -> DnsResult<()> {
        let bind_addr = self.config.bind_addr;

        // Bind UDP socket
        let udp_socket = UdpSocket::bind(bind_addr).await.map_err(|e| {
            error!(bind_addr = %bind_addr, error = %e, "Failed to bind UDP socket");
            DnsError::ServerStart {
                reason: format!("UDP bind failed: {}", e),
            }
        })?;

        // Bind TCP listener
        let tcp_listener = TcpListener::bind(bind_addr).await.map_err(|e| {
            error!(bind_addr = %bind_addr, error = %e, "Failed to bind TCP socket");
            DnsError::ServerStart {
                reason: format!("TCP bind failed: {}", e),
            }
        })?;

        info!(
            bind_addr = %bind_addr,
            zones = ?self.config.zones,
            forwarding = self.config.forwarding_enabled,
            "DNS protocol server starting"
        );

        // Create and run server
        let mut server = ServerFuture::new(self.catalog);

        // Register UDP
        server.register_socket(udp_socket);

        // Register TCP
        server.register_listener(tcp_listener, Duration::from_secs(30));

        // Run server
        match server.block_until_done().await {
            Ok(()) => {
                info!("DNS server stopped");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "DNS server error");
                Err(DnsError::ServerStart { reason: e.to_string() })
            }
        }
    }

    /// Get a reference to the DNS client.
    pub fn client(&self) -> &Arc<AspenDnsClient> {
        &self.client
    }

    /// Get the bind address.
    pub fn bind_addr(&self) -> SocketAddr {
        self.config.bind_addr
    }

    /// Get the list of served zones.
    pub fn zones(&self) -> &[String] {
        &self.config.zones
    }

    /// Check if forwarding is enabled.
    pub fn forwarding_enabled(&self) -> bool {
        self.config.forwarding_enabled
    }
}

/// Builder for DnsProtocolServer with fluent API.
pub struct DnsProtocolServerBuilder {
    config: DnsServerConfig,
    client: Option<Arc<AspenDnsClient>>,
}

impl DnsProtocolServerBuilder {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: DnsServerConfig::default(),
            client: None,
        }
    }

    /// Set the DNS client.
    pub fn client(mut self, client: Arc<AspenDnsClient>) -> Self {
        self.client = Some(client);
        self
    }

    /// Set the bind address.
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.config.bind_addr = addr;
        self
    }

    /// Add a zone to serve.
    pub fn zone(mut self, zone: impl Into<String>) -> Self {
        self.config.zones.push(zone.into());
        self
    }

    /// Set the zones to serve.
    pub fn zones(mut self, zones: Vec<String>) -> Self {
        self.config.zones = zones;
        self
    }

    /// Add an upstream DNS server.
    pub fn upstream(mut self, addr: SocketAddr) -> Self {
        self.config.upstreams.push(addr);
        self
    }

    /// Set the upstream DNS servers.
    pub fn upstreams(mut self, upstreams: Vec<SocketAddr>) -> Self {
        self.config.upstreams = upstreams;
        self
    }

    /// Enable or disable forwarding.
    pub fn forwarding(mut self, enabled: bool) -> Self {
        self.config.forwarding_enabled = enabled;
        self
    }

    /// Build the DNS server.
    pub fn build(self) -> DnsResult<DnsProtocolServer> {
        let client = self.client.ok_or_else(|| DnsError::ServerStart {
            reason: "DNS client is required".to_string(),
        })?;

        DnsProtocolServer::new(self.config, client)
    }
}

impl Default for DnsProtocolServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = DnsProtocolServerBuilder::new();
        assert!(builder.client.is_none());
        assert_eq!(builder.config.bind_addr, "127.0.0.1:5353".parse().unwrap());
    }

    #[test]
    fn test_builder_config() {
        let builder = DnsProtocolServerBuilder::new()
            .bind_addr("0.0.0.0:5353".parse().unwrap())
            .zones(vec!["test.local".to_string(), "dev.local".to_string()])
            .upstreams(vec!["8.8.8.8:53".parse().unwrap()])
            .forwarding(true);

        assert_eq!(builder.config.bind_addr, "0.0.0.0:5353".parse().unwrap());
        assert_eq!(builder.config.zones, vec!["test.local", "dev.local"]);
        assert_eq!(builder.config.upstreams.len(), 1);
        assert!(builder.config.forwarding_enabled);
    }

    #[test]
    fn test_builder_requires_client() {
        let result = DnsProtocolServerBuilder::new().build();
        assert!(result.is_err());
    }
}
