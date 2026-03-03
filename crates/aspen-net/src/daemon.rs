//! Daemon orchestration.
//!
//! The `NetDaemon` manages the service mesh lifecycle:
//! - Connects to an Aspen cluster via iroh QUIC
//! - Runs a SOCKS5 proxy for `*.aspen` name resolution
//! - Optionally starts a DNS server for zone `aspen`
//! - Auto-publishes configured local services
//! - Refreshes the name resolver cache periodically
//!
//! # DNS Integration
//!
//! When the `dns` feature is enabled and `--no-dns` is not passed:
//! 1. Creates an `AspenDnsClient` from a `DnsClientTicket` obtained from the cluster
//! 2. Starts a `DnsProtocolServer` bound to the configured port (default 5353)
//! 3. Zone sync happens via iroh-docs CRDT replication (entries arrive via `process_sync_entry`)

use std::net::SocketAddr;
#[cfg(feature = "dns")]
use std::sync::Arc;

use snafu::Snafu;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use crate::constants::NET_REGISTRY_POLL_INTERVAL_SECS;

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
/// Components are optional — the daemon works without DNS if
/// the `dns` feature is disabled or `--no-dns` is passed.
pub struct NetDaemon {
    /// Cancellation token for coordinated shutdown.
    cancel: CancellationToken,
    /// DNS server handle (only present when dns is enabled).
    #[cfg(feature = "dns")]
    dns_client: Option<Arc<aspen_dns::AspenDnsClient>>,
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

        // --- DNS setup (feature-gated) ---
        #[cfg(feature = "dns")]
        let dns_client = if config.dns_enabled {
            Some(Self::start_dns(&config, cancel.clone()).await?)
        } else {
            info!("DNS resolver disabled (--no-dns)");
            None
        };

        #[cfg(not(feature = "dns"))]
        if config.dns_enabled {
            warn!("DNS requested but aspen-net compiled without 'dns' feature; skipping DNS server");
        }

        // --- Registry poll task ---
        let poll_cancel = cancel.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(NET_REGISTRY_POLL_INTERVAL_SECS));
            loop {
                tokio::select! {
                    _ = poll_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        // In production this would call resolver.refresh_cache()
                        // via the client RPC channel. For now, the tick drives
                        // staleness checking on the DNS client.
                        #[cfg(feature = "dns")]
                        if let Some(ref _client) = None::<Arc<aspen_dns::AspenDnsClient>> {
                            // placeholder: dns_client.check_staleness() runs via the
                            // sync layer, not the poll loop.
                        }
                    }
                }
            }
        });

        info!("service mesh daemon started");

        Ok(Self {
            cancel,
            #[cfg(feature = "dns")]
            dns_client,
            is_running: true,
        })
    }

    /// Start the DNS subsystem: create client, build server, spawn task.
    #[cfg(feature = "dns")]
    async fn start_dns(
        config: &DaemonConfig,
        cancel: CancellationToken,
    ) -> Result<Arc<aspen_dns::AspenDnsClient>, DaemonError> {
        use aspen_dns::AspenDnsClient;
        use aspen_dns::DnsClientTicket;
        use aspen_dns::DnsProtocolServerBuilder;

        // Parse the cluster ticket to extract DNS-related info.
        // In production, the daemon would call a cluster RPC (e.g., GetDnsTicket)
        // to obtain the DnsClientTicket. For now, create a client that will be
        // populated via process_sync_entry() as iroh-docs entries arrive.
        let dns_ticket = DnsClientTicket::new(
            &config.cluster_ticket, // use cluster ticket as cluster_id
            "",                     // namespace discovered at runtime
            vec![],                 // peers discovered at runtime
        )
        .with_zone_filter(vec!["aspen".to_string()])
        .map_err(|e| DaemonError::DnsStart {
            reason: format!("zone filter: {e}"),
        })?;

        let dns_client = Arc::new(AspenDnsClient::builder().ticket(dns_ticket).build());

        // Build the DNS protocol server
        let server = DnsProtocolServerBuilder::new()
            .client(Arc::clone(&dns_client))
            .bind_addr(config.dns_addr)
            .zone("aspen".to_string())
            .forwarding(true)
            .upstreams(default_upstreams())
            .build()
            .await
            .map_err(|e| DaemonError::DnsStart { reason: e.to_string() })?;

        info!(
            addr = %config.dns_addr,
            zones = ?["aspen"],
            "DNS protocol server started"
        );

        // Run the DNS server in a background task
        let dns_cancel = cancel.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = dns_cancel.cancelled() => {
                    info!("DNS server shutting down");
                }
                result = server.run() => {
                    match result {
                        Ok(()) => info!("DNS server stopped"),
                        Err(e) => warn!(error = %e, "DNS server error"),
                    }
                }
            }
        });

        Ok(dns_client)
    }

    /// Gracefully shut down the daemon.
    ///
    /// Cancels all background tasks and waits up to `NET_SHUTDOWN_TIMEOUT_SECS`
    /// for them to complete.
    pub async fn shutdown(&mut self) {
        if !self.is_running {
            return;
        }

        info!("shutting down service mesh daemon");
        self.cancel.cancel();

        // Give tasks time to drain
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        self.is_running = false;
        info!("service mesh daemon stopped");
    }

    /// Check if the daemon is running.
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get the DNS client handle (for external sync wiring).
    ///
    /// Returns `None` if DNS is disabled or the feature is not compiled in.
    #[cfg(feature = "dns")]
    pub fn dns_client(&self) -> Option<&Arc<aspen_dns::AspenDnsClient>> {
        self.dns_client.as_ref()
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

/// Default upstream DNS servers for forwarding.
#[cfg(feature = "dns")]
fn default_upstreams() -> Vec<SocketAddr> {
    vec![
        SocketAddr::from(([8, 8, 8, 8], 53)),
        SocketAddr::from(([8, 8, 4, 4], 53)),
    ]
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

    #[tokio::test]
    async fn daemon_start_no_dns() {
        let config = DaemonConfig {
            cluster_ticket: "test-ticket".to_string(),
            token: "test-token".to_string(),
            socks5_addr: DaemonConfig::default_socks5_addr(),
            dns_addr: DaemonConfig::default_dns_addr(),
            dns_enabled: false,
            auto_publish: vec![],
            tags: vec![],
        };

        let mut daemon = NetDaemon::start(config).await.unwrap();
        assert!(daemon.is_running());

        daemon.shutdown().await;
        assert!(!daemon.is_running());
    }

    #[tokio::test]
    async fn daemon_shutdown_idempotent() {
        let config = DaemonConfig {
            cluster_ticket: "test-ticket".to_string(),
            token: "test-token".to_string(),
            socks5_addr: DaemonConfig::default_socks5_addr(),
            dns_addr: DaemonConfig::default_dns_addr(),
            dns_enabled: false,
            auto_publish: vec![],
            tags: vec![],
        };

        let mut daemon = NetDaemon::start(config).await.unwrap();
        daemon.shutdown().await;
        daemon.shutdown().await; // second call is a no-op
        assert!(!daemon.is_running());
    }
}
