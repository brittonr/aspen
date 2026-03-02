//! Embedded iroh relay server for cluster-internal NAT traversal.
//!
//! When the `relay-server` feature is enabled, each Raft node runs an iroh relay
//! server. This allows cluster members to relay encrypted traffic through each
//! other when direct P2P connections aren't possible (e.g., behind NATs).
//!
//! All Raft nodes run relay servers for maximum redundancy — there is no single
//! point of failure for relay connectivity.
//!
//! # Security Model
//!
//! - Relay traffic is end-to-end encrypted by iroh (QUIC encryption)
//! - Access control restricts relaying to known cluster members only
//! - TLS on the relay HTTP endpoint is planned as a future enhancement
//!
//! # Integration Points
//!
//! The relay server integrates with the Raft membership system:
//! - `build_relay_map_from_membership()`: Constructs a `RelayMap` from member metadata
//! - `extract_endpoint_ids()`: Extracts endpoint IDs for access control
//! - Membership changes trigger relay map + access control updates via the node's Raft metrics
//!   subscription (wired in the node startup sequence)
//!
//! Note: iroh endpoints do not support changing the relay map after creation.
//! The initial relay map is set at endpoint creation time. Dynamic updates would
//! require endpoint recreation, which is a future enhancement. For now, the relay
//! map is built from initial membership and new nodes joining discover existing
//! relays from the cluster ticket.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_raft_types::member::RaftMemberInfo;
use iroh::EndpointId;
use n0_future::FutureExt as _;
use tokio::sync::RwLock;
use tracing::info;
use tracing::warn;

use crate::config::RelayServerConfig;

/// Shared set of endpoint IDs allowed to use the relay.
///
/// Updated when Raft membership changes. The relay server's access control
/// callback reads from this set to allow/deny connections.
///
/// Tiger Style: Uses `Arc<RwLock<HashSet<EndpointId>>>` for concurrent reads
/// with infrequent writes (membership changes are rare).
pub type AllowedEndpoints = Arc<RwLock<HashSet<EndpointId>>>;

/// Creates a new empty allowed-endpoints set.
pub fn new_allowed_endpoints() -> AllowedEndpoints {
    Arc::new(RwLock::new(HashSet::new()))
}

// ============================================================================
// Relay Map Construction
// ============================================================================

/// Build an iroh `RelayMap` from the relay URLs in Raft membership metadata.
///
/// Iterates over all nodes in the membership, extracts their `relay_url` if present,
/// and constructs a `RelayMap` suitable for configuring an iroh endpoint.
///
/// Returns `None` if no nodes have relay URLs configured.
pub fn build_relay_map_from_membership<I>(members: I) -> Option<iroh::RelayMap>
where I: IntoIterator<Item = RaftMemberInfo> {
    let relay_urls: Vec<iroh::RelayUrl> = members
        .into_iter()
        .filter_map(|member| {
            member.relay_url.as_ref().and_then(|url_str| match url_str.parse::<iroh::RelayUrl>() {
                Ok(url) => {
                    tracing::debug!(relay_url = %url_str, "discovered cluster relay");
                    Some(url)
                }
                Err(e) => {
                    tracing::warn!(
                        relay_url = %url_str,
                        error = %e,
                        "failed to parse cluster relay URL, skipping"
                    );
                    None
                }
            })
        })
        .collect();

    if relay_urls.is_empty() {
        None
    } else {
        let count = relay_urls.len();
        let map: iroh::RelayMap = relay_urls.into_iter().collect();
        info!(relay_count = count, "built cluster relay map from membership");
        Some(map)
    }
}

/// Extract all endpoint IDs from Raft membership metadata.
///
/// Used to update the relay server's allowed-endpoints set when membership changes.
pub fn extract_endpoint_ids<I>(members: I) -> HashSet<EndpointId>
where I: IntoIterator<Item = RaftMemberInfo> {
    members.into_iter().map(|member| member.iroh_addr.id).collect()
}

/// Pre-configure an `IrohEndpointConfig` to use the cluster's own relay server.
///
/// Call this after spawning the relay server but before creating the iroh endpoint.
/// This sets `relay_mode: Custom` with the relay server's URL, unless the user has
/// explicitly configured a different relay mode.
///
/// # Bootstrap Behavior
/// - First node: uses its own relay URL as the sole entry
/// - Subsequent nodes: additional relay URLs from membership can be added to `relay_urls`
///
/// # Override Behavior
/// - If `relay_mode` is explicitly `Custom` or `Disabled`, this is a no-op
/// - Only auto-configures when `relay_mode` is `Default` (the default value)
pub fn configure_endpoint_for_cluster_relay(config: &mut crate::IrohEndpointConfig, relay_url: &str) {
    // Only auto-configure if relay_mode hasn't been explicitly set
    match config.relay_mode {
        crate::config::RelayMode::Default => {
            // Auto-configure: switch from n0 public relays to cluster's own relay
            config.relay_mode = crate::config::RelayMode::Custom;
            config.relay_urls.push(relay_url.to_string());
            tracing::info!(
                relay_url = %relay_url,
                "auto-configured endpoint to use cluster relay server"
            );
        }
        crate::config::RelayMode::Custom => {
            // User explicitly configured custom relays — add ours to the list
            config.relay_urls.push(relay_url.to_string());
            tracing::info!(
                relay_url = %relay_url,
                total_relays = config.relay_urls.len(),
                "added cluster relay to existing custom relay configuration"
            );
        }
        crate::config::RelayMode::Disabled => {
            // User explicitly disabled relays — respect the override
            tracing::info!(
                "relay mode explicitly disabled — cluster relay server running but not used by this endpoint"
            );
        }
    }
}

// ============================================================================
// Relay Server
// ============================================================================

/// Embedded iroh relay server wrapper.
///
/// Manages the lifecycle of an `iroh_relay::server::Server` running alongside
/// the Aspen node's iroh endpoint. Provides cluster-aware access control
/// and integrates with the node's startup/shutdown sequence.
pub struct RelayServer {
    /// The underlying iroh relay server.
    server: iroh_relay::server::Server,
    /// The HTTP address the relay is listening on.
    http_addr: SocketAddr,
    /// The shared set of allowed endpoint IDs.
    allowed_endpoints: AllowedEndpoints,
}

impl RelayServer {
    /// Spawn a new relay server with the given configuration.
    ///
    /// The server starts immediately and listens on the configured bind address.
    /// Access control is enforced using the provided `allowed_endpoints` set.
    ///
    /// # Arguments
    /// * `config` - Relay server configuration (bind address, port, limits)
    /// * `allowed_endpoints` - Shared set of authorized endpoint IDs
    ///
    /// # Tiger Style
    /// - Fails fast if the server cannot bind to the configured address
    /// - Bounded key cache capacity
    pub async fn spawn(config: &RelayServerConfig, allowed_endpoints: AllowedEndpoints) -> Result<Self> {
        let bind_addr: SocketAddr = format!("{}:{}", config.bind_addr, config.bind_port)
            .parse()
            .context("failed to parse relay server bind address")?;

        // Build rate limits if configured
        let limits = Self::build_limits(config);

        // Build access control from the shared allowed-endpoints set
        let access = Self::build_access_config(allowed_endpoints.clone());

        let server_config = iroh_relay::server::ServerConfig::<(), ()> {
            relay: Some(iroh_relay::server::RelayConfig {
                http_bind_addr: bind_addr,
                tls: None, // HTTP only for initial implementation
                limits,
                key_cache_capacity: Some(config.key_cache_capacity as usize),
                access,
            }),
            quic: None, // QUIC relay (QAD) disabled for now
            metrics_addr: None,
        };

        let server = iroh_relay::server::Server::spawn(server_config)
            .await
            .context("failed to spawn iroh relay server")?;

        let http_addr = server.http_addr().context("relay server has no HTTP address")?;

        info!(
            addr = %http_addr,
            key_cache = config.key_cache_capacity,
            "iroh relay server started"
        );

        Ok(Self {
            server,
            http_addr,
            allowed_endpoints,
        })
    }

    /// Build rate limits from configuration.
    fn build_limits(config: &RelayServerConfig) -> iroh_relay::server::Limits {
        let client_rx = config.client_rx_bytes_per_second.and_then(|bps| {
            NonZeroU32::new(bps).map(|rate| iroh_relay::server::ClientRateLimit {
                bytes_per_second: rate,
                max_burst_bytes: None, // Use default burst
            })
        });

        iroh_relay::server::Limits {
            accept_conn_limit: None,
            accept_conn_burst: None,
            client_rx,
        }
    }

    /// Build access control configuration from the allowed-endpoints set.
    ///
    /// Returns `AccessConfig::Restricted` that checks whether the connecting
    /// endpoint ID is in the allowed set. If the set is empty (during initial
    /// bootstrap), all connections are allowed.
    fn build_access_config(allowed_endpoints: AllowedEndpoints) -> iroh_relay::server::AccessConfig {
        iroh_relay::server::AccessConfig::Restricted(Box::new(move |endpoint_id| {
            let allowed = allowed_endpoints.clone();
            async move {
                let set = allowed.read().await;
                // During bootstrap, if no endpoints are registered yet, allow all
                if set.is_empty() {
                    return iroh_relay::server::Access::Allow;
                }
                if set.contains(&endpoint_id) {
                    iroh_relay::server::Access::Allow
                } else {
                    warn!(
                        endpoint = %endpoint_id,
                        "relay access denied: endpoint not in cluster membership"
                    );
                    iroh_relay::server::Access::Deny
                }
            }
            .boxed()
        }))
    }

    /// Get the HTTP address the relay server is listening on.
    pub fn http_addr(&self) -> SocketAddr {
        self.http_addr
    }

    /// Get the relay URL suitable for configuring an iroh endpoint's `RelayMap`.
    ///
    /// Returns an `http://` URL since TLS is not enabled in the initial implementation.
    pub fn relay_url(&self) -> String {
        format!("http://{}", self.http_addr)
    }

    /// Get a reference to the allowed-endpoints set.
    ///
    /// Use this to update the set when Raft membership changes.
    pub fn allowed_endpoints(&self) -> &AllowedEndpoints {
        &self.allowed_endpoints
    }

    /// Update the set of allowed endpoint IDs.
    ///
    /// Called when Raft membership changes to add/remove authorized endpoints.
    pub async fn update_allowed_endpoints(&self, endpoints: HashSet<EndpointId>) {
        let mut set = self.allowed_endpoints.write().await;
        let old_count = set.len();
        *set = endpoints;
        let new_count = set.len();
        info!(old_count, new_count, "updated relay server allowed endpoints");
    }

    /// Gracefully shutdown the relay server.
    ///
    /// Tiger Style: Bounded shutdown with graceful connection drain.
    pub async fn shutdown(self) -> Result<()> {
        info!(addr = %self.http_addr, "shutting down iroh relay server");
        self.server.shutdown().await.map_err(|e| anyhow::anyhow!("relay server shutdown error: {}", e))?;
        info!("iroh relay server shutdown complete");
        Ok(())
    }
}

impl std::fmt::Debug for RelayServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayServer").field("http_addr", &self.http_addr).finish()
    }
}

#[cfg(test)]
mod tests {
    use iroh::EndpointAddr;
    use iroh::SecretKey;

    use super::*;

    fn make_member(seed: u8, relay_url: Option<&str>) -> RaftMemberInfo {
        let key = SecretKey::from([seed; 32]);
        let addr = EndpointAddr::new(key.public());
        RaftMemberInfo {
            iroh_addr: addr,
            relay_url: relay_url.map(String::from),
        }
    }

    #[test]
    fn test_build_relay_map_empty_members() {
        let result = build_relay_map_from_membership(Vec::<RaftMemberInfo>::new());
        assert!(result.is_none());
    }

    #[test]
    fn test_build_relay_map_no_relay_urls() {
        let members = vec![make_member(1, None), make_member(2, None)];
        let result = build_relay_map_from_membership(members);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_relay_map_with_relay_urls() {
        let members = vec![
            make_member(1, Some("http://10.0.0.1:3340")),
            make_member(2, Some("http://10.0.0.2:3340")),
            make_member(3, None),
        ];
        let result = build_relay_map_from_membership(members);
        assert!(result.is_some());
        let map = result.unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_build_relay_map_invalid_url_skipped() {
        let members = vec![
            make_member(1, Some("http://10.0.0.1:3340")),
            make_member(2, Some("not-a-valid-url")),
        ];
        let result = build_relay_map_from_membership(members);
        assert!(result.is_some());
        let map = result.unwrap();
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_extract_endpoint_ids() {
        let m1 = make_member(1, None);
        let m2 = make_member(2, None);
        let id1 = m1.iroh_addr.id;
        let id2 = m2.iroh_addr.id;
        let ids = extract_endpoint_ids(vec![m1, m2]);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }
}
