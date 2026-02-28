//! Cross-cluster request proxy service.
//!
//! When a request requires an app that isn't loaded locally, the proxy
//! service forwards it to a discovered cluster that has the capability.
//!
//! # Architecture
//!
//! ```text
//! Client → Local HandlerRegistry → no handler → ProxyService
//!                                                    ↓
//!                                          FederationDiscovery
//!                                                    ↓
//!                                          Remote Cluster (via Iroh)
//!                                                    ↓
//!                                          Response ← back to client
//! ```
//!
//! # Tiger Style
//!
//! - Bounded hops: `proxy_hops` incremented on each forward, rejected at MAX_PROXY_HOPS
//! - Bounded targets: tries at most MAX_PROXY_TARGETS clusters
//! - Bounded timeout: PROXY_TIMEOUT_SECS per proxy attempt
//! - Loop detection: hop count prevents A→B→A cycles

#[cfg(all(feature = "forge", feature = "global-discovery"))]
use aspen_client_api::AuthenticatedRequest;
#[cfg(all(feature = "forge", feature = "global-discovery"))]
use aspen_client_api::CLIENT_ALPN;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
#[cfg(all(feature = "forge", feature = "global-discovery"))]
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::ProxyConfig;
use iroh::Endpoint;
#[cfg(all(feature = "forge", feature = "global-discovery"))]
use tracing::info;
use tracing::warn;

/// Cross-cluster request proxy service.
///
/// Forwards requests to discovered clusters when the required app
/// is not available locally.
#[allow(dead_code)]
pub struct ProxyService {
    /// Iroh endpoint for P2P connections.
    endpoint: Endpoint,
    /// Proxy configuration (hop limits, timeout, max targets).
    config: ProxyConfig,
}

impl ProxyService {
    /// Create a new proxy service.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Iroh endpoint for establishing connections
    /// * `config` - Proxy configuration (defaults from ProxyConfig::default())
    pub fn new(endpoint: Endpoint, config: ProxyConfig) -> Self {
        Self { endpoint, config }
    }

    /// Attempt to proxy a request to a remote cluster.
    ///
    /// Returns `Ok(Some(response))` if proxying succeeded,
    /// `Ok(None)` if proxying is disabled or no suitable clusters found,
    /// `Err(e)` if proxying failed with an error.
    ///
    /// # Arguments
    ///
    /// * `request` - The client RPC request to forward
    /// * `required_app` - The app ID required to handle this request
    /// * `proxy_hops` - Current hop count for this request
    /// * `ctx` - Client protocol context for accessing federation discovery
    ///
    /// # Tiger Style
    ///
    /// - Checks hop limit first (bounded hops)
    /// - Tries at most MAX_PROXY_TARGETS clusters (bounded attempts)
    /// - Uses PROXY_TIMEOUT_SECS per attempt (bounded time)
    /// - Returns first success, None if all fail
    pub async fn proxy_request(
        &self,
        request: ClientRpcRequest,
        required_app: &str,
        proxy_hops: u8,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<Option<ClientRpcResponse>> {
        // Check hop limit
        if proxy_hops >= self.config.max_hops {
            warn!(app = required_app, proxy_hops, max_hops = self.config.max_hops, "proxy hop limit exceeded");
            return Ok(None);
        }

        // Feature-gated: requires both forge and global-discovery
        #[cfg(all(feature = "forge", feature = "global-discovery"))]
        {
            // Get federation discovery service
            let discovery = match ctx.federation_discovery.as_ref() {
                Some(d) => d,
                None => {
                    warn!(app = required_app, "federation discovery not available for proxying");
                    return Ok(None);
                }
            };

            // Find clusters with the required app
            let clusters = discovery.find_clusters_with_app(required_app);
            if clusters.is_empty() {
                info!(app = required_app, "no clusters found with required app");
                return Ok(None);
            }

            // Rank clusters by trust, freshness, and capability score
            let ranked = if let Some(trust_manager) = ctx.federation_trust_manager.as_ref() {
                let selector = aspen_cluster::federation::DefaultClusterSelector::new(
                    aspen_cluster::federation::SelectionStrategy::Scored,
                );
                let ranked = selector.rank_clusters(&clusters, trust_manager);
                ranked.into_iter().map(|r| r.cluster).collect::<Vec<_>>()
            } else {
                clusters
            };

            // Try up to max_targets clusters (now in ranked order)
            let max_targets = self.config.max_targets.min(ranked.len());
            for cluster in ranked.into_iter().take(max_targets) {
                info!(
                    app = required_app,
                    cluster_key = %cluster.cluster_key,
                    cluster_name = %cluster.name,
                    proxy_hops,
                    "attempting to proxy request to cluster (ranked)"
                );

                match self.send_to_cluster(&cluster, request.clone(), proxy_hops).await {
                    Ok(response) => {
                        info!(
                            app = required_app,
                            cluster_key = %cluster.cluster_key,
                            proxy_hops,
                            "proxy request succeeded"
                        );
                        return Ok(Some(response));
                    }
                    Err(e) => {
                        warn!(
                            app = required_app,
                            cluster_key = %cluster.cluster_key,
                            error = %e,
                            "proxy attempt to cluster failed"
                        );
                        // Continue to next cluster
                    }
                }
            }

            info!(app = required_app, tried_clusters = max_targets, "all proxy attempts failed");
            Ok(None)
        }

        #[cfg(not(all(feature = "forge", feature = "global-discovery")))]
        {
            // Without federation discovery, proxying is unavailable
            let _ = (request, required_app, proxy_hops, ctx);
            warn!("proxy request attempted but forge/global-discovery features not enabled");
            Ok(None)
        }
    }

    /// Send a request to a specific cluster.
    ///
    /// Replicates the AspenClient::send_to_addr protocol:
    /// - Connect via Iroh with CLIENT_ALPN
    /// - Send AuthenticatedRequest with incremented proxy_hops
    /// - Read response with timeout
    ///
    /// # Feature-gated
    ///
    /// Only available when both `forge` and `global-discovery` features are enabled.
    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    async fn send_to_cluster(
        &self,
        cluster: &aspen_cluster::federation::DiscoveredCluster,
        request: ClientRpcRequest,
        proxy_hops: u8,
    ) -> anyhow::Result<ClientRpcResponse> {
        use tokio::time::timeout;

        // Use the first known node key from the discovered cluster as the
        // connection target. The discovery service populates node_keys from
        // the cluster's gossip announcements.
        let node_id =
            cluster.node_keys.first().ok_or_else(|| anyhow::anyhow!("discovered cluster has no node keys"))?;

        let target_addr = iroh::EndpointAddr::new(*node_id);

        // Connect to the remote cluster with timeout
        let connection = timeout(self.config.timeout, async {
            self.endpoint
                .connect(target_addr.clone(), CLIENT_ALPN)
                .await
                .map_err(|e| anyhow::anyhow!("failed to connect to cluster: {}", e))
        })
        .await
        .map_err(|_| anyhow::anyhow!("connection timeout"))??;

        // Open bidirectional stream
        let (mut send, mut recv) = connection.open_bi().await?;

        // Create authenticated request with incremented proxy_hops
        let authenticated_request = AuthenticatedRequest::with_proxy_hops(request, None, proxy_hops + 1);

        // Serialize and send request
        let request_bytes = postcard::to_stdvec(&authenticated_request)?;
        send.write_all(&request_bytes).await?;
        send.finish()?;

        // Read response with timeout
        let response_bytes = timeout(self.config.timeout, async {
            recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE)
                .await
                .map_err(|e| anyhow::anyhow!("failed to read response: {}", e))
        })
        .await
        .map_err(|_| anyhow::anyhow!("response timeout"))??;

        // Deserialize response
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes)?;

        // Close connection gracefully
        connection.close(iroh::endpoint::VarInt::from_u32(0), b"done");

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn proxy_service_respects_hop_limit() {
        // Create mock endpoint with deterministic key
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = 42;
        let secret_key = iroh::SecretKey::from_bytes(&key_bytes);
        let endpoint = Endpoint::builder().secret_key(secret_key).bind().await.unwrap();

        let config = ProxyConfig {
            enabled: true,
            max_hops: 3,
            timeout: std::time::Duration::from_secs(5),
            max_targets: 3,
        };

        let service = ProxyService::new(endpoint.clone(), config);

        // Create minimal context
        let mock_endpoint_provider = Arc::new(aspen_rpc_core::test_support::MockEndpointProvider::with_seed(42).await);
        let ctx = aspen_rpc_core::test_support::TestContextBuilder::new()
            .with_endpoint_manager(mock_endpoint_provider as Arc<dyn aspen_core::EndpointProvider>)
            .build();

        // Request with hops at limit should return None
        let result = service
            .proxy_request(ClientRpcRequest::Ping, "test-app", 3, &ctx)
            .await
            .expect("proxy_request should not error");

        assert!(result.is_none(), "should return None when hop limit exceeded");

        // Cleanup
        endpoint.close().await;
    }

    #[tokio::test]
    async fn proxy_service_returns_none_without_discovery() {
        // Create mock endpoint with deterministic key
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = 43;
        let secret_key = iroh::SecretKey::from_bytes(&key_bytes);
        let endpoint = Endpoint::builder().secret_key(secret_key).bind().await.unwrap();

        let config = ProxyConfig::default();
        let service = ProxyService::new(endpoint.clone(), config);

        // Create context without federation discovery
        let mock_endpoint_provider = Arc::new(aspen_rpc_core::test_support::MockEndpointProvider::with_seed(43).await);
        let ctx = aspen_rpc_core::test_support::TestContextBuilder::new()
            .with_endpoint_manager(mock_endpoint_provider as Arc<dyn aspen_core::EndpointProvider>)
            .build();

        let result = service
            .proxy_request(ClientRpcRequest::Ping, "test-app", 0, &ctx)
            .await
            .expect("proxy_request should not error");

        assert!(result.is_none(), "should return None without federation discovery");

        endpoint.close().await;
    }
}
