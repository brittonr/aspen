//! Daemon orchestration.
//!
//! The `NetDaemon` manages the service mesh lifecycle:
//! - Connects to an Aspen cluster via iroh QUIC
//! - Runs a SOCKS5 proxy for `*.aspen` name resolution
//! - Optionally starts a DNS server for zone `aspen`
//! - Auto-publishes configured local services
//! - Refreshes the name resolver cache periodically

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use aspen_client::AspenClient;
use aspen_client::AuthToken;
use snafu::Snafu;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use crate::auth::NetAuthenticator;
use crate::client_kv::ClientKvAdapter;
use crate::constants::NET_REGISTRY_POLL_INTERVAL_SECS;
use crate::registry::ServiceRegistry;
use crate::resolver::NameResolver;
use crate::socks5::Socks5Server;
use crate::tunnel::TunnelAcceptor;
use crate::types::ServiceEntry;

#[inline]
#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "net daemon auto-publish metadata stores wall-clock timestamps"
)]
fn current_time_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

/// Configuration for the service mesh daemon.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Cluster ticket for connecting to the Aspen cluster.
    pub cluster_ticket: String,
    /// Capability token (base64-encoded UCAN).
    pub token: String,
    /// SOCKS5 proxy bind address.
    pub socks5_addr: SocketAddr,
    /// DNS server bind address (ignored if `dns_enabled` is false).
    pub dns_addr: SocketAddr,
    /// Whether to start the DNS resolver.
    pub dns_enabled: bool,
    /// Services to auto-publish on startup (format: "name:port:proto").
    pub auto_publish: Vec<String>,
    /// Tags to apply to auto-published services.
    pub tags: Vec<String>,
}

impl DaemonConfig {
    /// Default SOCKS5 bind address.
    pub fn default_socks5_addr() -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], 1080))
    }

    /// Default DNS bind address.
    pub fn default_dns_addr() -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], 5353))
    }
}

/// Errors from daemon operations.
#[derive(Debug, Snafu)]
pub enum DaemonError {
    /// Failed to connect to the cluster.
    #[snafu(display("cluster connection failed: {reason}"))]
    ClusterConnect { reason: String },

    /// Failed to start DNS server.
    #[snafu(display("dns server failed: {reason}"))]
    DnsStart { reason: String },

    /// Failed to start SOCKS5 server.
    #[snafu(display("socks5 server failed: {reason}"))]
    Socks5Start { reason: String },

    /// Invalid auto-publish spec.
    #[snafu(display("invalid publish spec '{spec}': {reason}"))]
    InvalidPublishSpec { spec: String, reason: String },
}

/// Service mesh daemon handle.
///
/// Holds all running components and provides graceful shutdown.
#[derive(Debug)]
pub struct NetDaemon {
    /// Cancellation token for coordinated shutdown.
    cancel: CancellationToken,
    /// Whether the daemon is running.
    is_running: bool,
}

impl NetDaemon {
    /// Start the service mesh daemon with the given configuration.
    ///
    /// Connects to the Aspen cluster, starts SOCKS5 proxy, optionally
    /// starts DNS server, and auto-publishes configured services.
    pub async fn start(config: DaemonConfig) -> Result<Self, DaemonError> {
        let cancel = CancellationToken::new();

        info!(
            socks5 = %config.socks5_addr,
            dns = config.dns_enabled,
            dns_addr = %config.dns_addr,
            auto_publish = config.auto_publish.len(),
            "starting service mesh daemon"
        );

        // --- Connect to the Aspen cluster ---
        let token = if config.token.is_empty() {
            None
        } else {
            Some(AuthToken::from_base64(&config.token).map_err(|e| DaemonError::ClusterConnect {
                reason: format!("invalid token: {e}"),
            })?)
        };

        let client = AspenClient::connect(&config.cluster_ticket, Duration::from_secs(10), token)
            .await
            .map_err(|e| DaemonError::ClusterConnect { reason: e.to_string() })?;

        let endpoint = Arc::new(client.endpoint().clone());
        let client = Arc::new(client);

        info!(endpoint_id = %endpoint.id(), "connected to cluster");

        // --- Build the KV adapter, registry, and resolver ---
        let kv_adapter = Arc::new(ClientKvAdapter::new(Arc::clone(&client)));
        let registry = Arc::new(ServiceRegistry::new(kv_adapter as Arc<ClientKvAdapter>));
        let resolver = Arc::new(NameResolver::new(registry.clone()));

        // --- Auto-publish configured services ---
        for spec in &config.auto_publish {
            let (name, port, proto) = parse_publish_spec(spec)?;
            let entry = ServiceEntry {
                name: name.clone(),
                endpoint_id: endpoint.id().to_string(),
                port,
                proto,
                tags: config.tags.clone(),
                hostname: None,
                published_at_ms: current_time_ms(),
            };
            match registry.publish(entry).await {
                Ok(()) => info!(name, port, "auto-published service"),
                Err(e) => warn!(name, port, error = %e, "failed to auto-publish service"),
            }
        }

        // --- Build authenticator ---
        // For now, use a permissive authenticator (allows all .aspen connections).
        // Real auth would validate the daemon's token against the cluster.
        let auth = Arc::new(NetAuthenticator::permissive());

        // --- Start SOCKS5 proxy ---
        let listener = TcpListener::bind(config.socks5_addr).await.map_err(|e| DaemonError::Socks5Start {
            reason: format!("bind {}: {e}", config.socks5_addr),
        })?;

        let socks5_addr = listener.local_addr().map_err(|e| DaemonError::Socks5Start {
            reason: format!("local_addr: {e}"),
        })?;

        let socks5 = Socks5Server::new(resolver.clone(), auth, Arc::clone(&endpoint), cancel.clone());

        let socks5_cancel = cancel.clone();
        tokio::spawn(async move {
            if let Err(e) = socks5.run(listener).await
                && !socks5_cancel.is_cancelled()
            {
                warn!(error = %e, "SOCKS5 server error");
            }
        });

        info!(addr = %socks5_addr, "SOCKS5 proxy listening");

        // --- Register TunnelAcceptor via iroh Router ---
        // The daemon can also receive tunnel connections if other proxies
        // need to route through it. Register on the endpoint.
        let tunnel_acceptor = TunnelAcceptor::new(cancel.clone());
        let _router = iroh::protocol::Router::builder((*endpoint).clone())
            .accept(aspen_transport::constants::NET_TUNNEL_ALPN, tunnel_acceptor)
            .spawn();

        // --- Registry poll task ---
        let poll_cancel = cancel.clone();
        let poll_resolver = resolver.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(NET_REGISTRY_POLL_INTERVAL_SECS));
            loop {
                tokio::select! {
                    _ = poll_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        if let Err(e) = poll_resolver.refresh_cache().await {
                            warn!(error = %e, "registry cache refresh failed");
                        }
                    }
                }
            }
        });

        // --- DNS setup (feature-gated) ---
        #[cfg(not(feature = "dns"))]
        if config.dns_enabled {
            warn!("DNS requested but aspen-net compiled without 'dns' feature; skipping DNS server");
        }

        info!("service mesh daemon started");

        Ok(Self {
            cancel,
            is_running: true,
        })
    }

    /// Gracefully shut down the daemon.
    pub async fn shutdown(&mut self) {
        if !self.is_running {
            return;
        }

        info!("shutting down service mesh daemon");
        self.cancel.cancel();

        // Give tasks time to drain
        tokio::time::sleep(Duration::from_millis(100)).await;

        self.is_running = false;
        info!("service mesh daemon stopped");
    }

    /// Check if the daemon is running.
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get the cancellation token for external coordination.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }
}

/// Parse an auto-publish spec: "name:port:proto" → (name, port, proto).
pub fn parse_publish_spec(spec: &str) -> Result<(String, u16, String), DaemonError> {
    let parts: Vec<&str> = spec.split(':').collect();
    match parts.len() {
        2 => {
            let name = parts[0].to_string();
            let port: u16 = parts[1].parse().map_err(|_| DaemonError::InvalidPublishSpec {
                spec: spec.to_string(),
                reason: format!("'{}' is not a valid port", parts[1]),
            })?;
            Ok((name, port, "tcp".to_string()))
        }
        3 => {
            let name = parts[0].to_string();
            let port: u16 = parts[1].parse().map_err(|_| DaemonError::InvalidPublishSpec {
                spec: spec.to_string(),
                reason: format!("'{}' is not a valid port", parts[1]),
            })?;
            let proto = parts[2].to_string();
            Ok((name, port, proto))
        }
        _ => Err(DaemonError::InvalidPublishSpec {
            spec: spec.to_string(),
            reason: "expected name:port[:proto]".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_publish_spec_two_parts() {
        let (name, port, proto) = parse_publish_spec("mydb:5432").unwrap();
        assert_eq!(name, "mydb");
        assert_eq!(port, 5432);
        assert_eq!(proto, "tcp");
    }

    #[test]
    fn parse_publish_spec_three_parts() {
        let (name, port, proto) = parse_publish_spec("web:8080:http").unwrap();
        assert_eq!(name, "web");
        assert_eq!(port, 8080);
        assert_eq!(proto, "http");
    }

    #[test]
    fn parse_publish_spec_invalid() {
        assert!(parse_publish_spec("").is_err());
        assert!(parse_publish_spec("single").is_err());
        assert!(parse_publish_spec("a:b:c:d").is_err());
        assert!(parse_publish_spec("mydb:notaport").is_err());
    }

    #[test]
    fn daemon_config_defaults() {
        assert_eq!(DaemonConfig::default_socks5_addr(), SocketAddr::from(([127, 0, 0, 1], 1080)));
        assert_eq!(DaemonConfig::default_dns_addr(), SocketAddr::from(([127, 0, 0, 1], 5353)));
    }
}
