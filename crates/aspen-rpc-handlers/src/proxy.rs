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
use std::future::Future;

#[cfg(all(feature = "forge", feature = "global-discovery"))]
use anyhow::Context;
#[cfg(all(feature = "forge", feature = "global-discovery"))]
use aspen_auth::Audience;
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
use tokio::time::Instant;
#[cfg(all(feature = "forge", feature = "global-discovery"))]
use tokio::time::timeout;
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

#[cfg(all(feature = "forge", feature = "global-discovery"))]
fn remaining_timeout(deadline: Instant, timeout_context: &'static str) -> anyhow::Result<std::time::Duration> {
    match deadline.checked_duration_since(Instant::now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        _ => Err(anyhow::anyhow!(timeout_context)),
    }
}

#[cfg(all(feature = "forge", feature = "global-discovery"))]
async fn run_stage_with_deadline<F, T, E>(
    deadline: Instant,
    future: F,
    timeout_context: &'static str,
    error_context: &'static str,
) -> anyhow::Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    timeout(remaining_timeout(deadline, timeout_context)?, future)
        .await
        .map_err(|_| anyhow::anyhow!(timeout_context))?
        .map_err(anyhow::Error::new)
        .map_err(|error| error.context(error_context))
}

#[cfg(all(feature = "forge", feature = "global-discovery"))]
const FEDERATION_PROXY_FACT_KEY: &str = "aspen:federation-proxy";

#[cfg(all(feature = "forge", feature = "global-discovery"))]
const FEDERATION_PROXY_FACT_VALUE: &[u8] = b"v1";

#[cfg(all(feature = "forge", feature = "global-discovery"))]
const MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS: u64 = 15 * 60;

#[cfg(all(feature = "forge", feature = "global-discovery"))]
fn token_lifetime_secs(token: &aspen_auth::CapabilityToken) -> u64 {
    token.expires_at.saturating_sub(token.issued_at)
}

#[cfg(all(feature = "forge", feature = "global-discovery"))]
fn token_has_federation_proxy_fact(token: &aspen_auth::CapabilityToken) -> bool {
    token
        .facts
        .iter()
        .any(|(key, value)| key == FEDERATION_PROXY_FACT_KEY && value.as_slice() == FEDERATION_PROXY_FACT_VALUE)
}

#[cfg(all(feature = "forge", feature = "global-discovery"))]
fn token_allows_proxying(token: Option<&aspen_auth::CapabilityToken>) -> bool {
    let Some(token) = token else {
        return true;
    };

    match &token.audience {
        Audience::Key(_) => false,
        Audience::Bearer => {
            token.proof.is_some()
                && token.delegation_depth > 0
                && token_lifetime_secs(token) <= MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS
                && token_has_federation_proxy_fact(token)
        }
    }
}

#[cfg(all(feature = "forge", feature = "global-discovery"))]
fn proxied_authenticated_request(
    request: ClientRpcRequest,
    token: Option<aspen_auth::CapabilityToken>,
    proxy_hops: u8,
) -> AuthenticatedRequest {
    AuthenticatedRequest::with_proxy_hops(request, token, proxy_hops + 1)
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
        token: Option<aspen_auth::CapabilityToken>,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<Option<ClientRpcResponse>> {
        // Check hop limit
        if proxy_hops >= self.config.max_hops {
            warn!(app = required_app, proxy_hops, max_hops = self.config.max_hops, "proxy hop limit exceeded");
            return Ok(None);
        }

        // Key-bound client tokens are bound to the original Iroh presenter. A proxy
        // would present its own node key to the remote cluster, so forwarding such a
        // token cannot satisfy the remote audience check. Generic bearer tokens are
        // also rejected: authenticated cross-cluster proxying must use a short-lived,
        // delegated bearer token with an explicit signed federation-proxy fact. Fail
        // closed locally instead of turning broad credentials into implicit
        // cross-cluster credentials.
        #[cfg(all(feature = "forge", feature = "global-discovery"))]
        if !token_allows_proxying(token.as_ref()) {
            warn!(
                app = required_app,
                proxy_hops, "capability token cannot be proxied without explicit federation proxy delegation"
            );
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

                match self.send_to_cluster(&cluster, request.clone(), token.clone(), proxy_hops).await {
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
            let _ = (request, required_app, proxy_hops, token, ctx);
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
        token: Option<aspen_auth::CapabilityToken>,
        proxy_hops: u8,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Use the first known node key from the discovered cluster as the
        // connection target. The discovery service populates node_keys from
        // the cluster's gossip announcements.
        let node_id =
            cluster.node_keys.first().ok_or_else(|| anyhow::anyhow!("discovered cluster has no node keys"))?;

        let target_addr = iroh::EndpointAddr::new(*node_id);

        let connection = timeout(self.config.timeout, async {
            self.endpoint
                .connect(target_addr.clone(), CLIENT_ALPN)
                .await
                .map_err(|e| anyhow::anyhow!("failed to connect to cluster: {}", e))
        })
        .await
        .map_err(|_| anyhow::anyhow!("connection timeout"))??;

        let authenticated_request = proxied_authenticated_request(request, token, proxy_hops);
        let request_bytes = postcard::to_stdvec(&authenticated_request)?;

        let deadline = Instant::now() + self.config.timeout;
        let response_bytes = match async {
            let (mut send, mut recv) =
                run_stage_with_deadline(deadline, connection.open_bi(), "stream open timeout", "failed to open stream")
                    .await?;
            run_stage_with_deadline(
                deadline,
                send.write_all(&request_bytes),
                "request write timeout",
                "failed to send request",
            )
            .await?;
            send.finish().map_err(anyhow::Error::new).context("failed to finish send stream")?;
            run_stage_with_deadline(
                deadline,
                recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE),
                "response timeout",
                "failed to read response",
            )
            .await
        }
        .await
        {
            Ok(response_bytes) => response_bytes,
            Err(error) => {
                connection.close(iroh::endpoint::VarInt::from_u32(1), b"error");
                return Err(error);
            }
        };

        let response: ClientRpcResponse = match postcard::from_bytes(&response_bytes) {
            Ok(response) => response,
            Err(error) => {
                connection.close(iroh::endpoint::VarInt::from_u32(1), b"error");
                return Err(anyhow::Error::new(error).context("failed to deserialize response"));
            }
        };

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
        let endpoint = Endpoint::builder(iroh::endpoint::presets::N0).secret_key(secret_key).bind().await.unwrap();

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
            .proxy_request(ClientRpcRequest::Ping, "test-app", 3, None, &ctx)
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
        let endpoint = Endpoint::builder(iroh::endpoint::presets::N0).secret_key(secret_key).bind().await.unwrap();

        let config = ProxyConfig::default();
        let service = ProxyService::new(endpoint.clone(), config);

        // Create context without federation discovery
        let mock_endpoint_provider = Arc::new(aspen_rpc_core::test_support::MockEndpointProvider::with_seed(43).await);
        let ctx = aspen_rpc_core::test_support::TestContextBuilder::new()
            .with_endpoint_manager(mock_endpoint_provider as Arc<dyn aspen_core::EndpointProvider>)
            .build();

        let result = service
            .proxy_request(ClientRpcRequest::Ping, "test-app", 0, None, &ctx)
            .await
            .expect("proxy_request should not error");

        assert!(result.is_none(), "should return None without federation discovery");

        endpoint.close().await;
    }

    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    fn test_token(audience: aspen_auth::Audience) -> aspen_auth::CapabilityToken {
        let issuer = iroh::SecretKey::from_bytes(&[7u8; 32]).public();
        aspen_auth::CapabilityToken {
            version: 1,
            issuer,
            audience,
            capabilities: vec![aspen_auth::Capability::Read {
                prefix: "repo/".to_string(),
            }],
            issued_at: 1,
            expires_at: 1 + MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS,
            nonce: Some([3u8; 16]),
            proof: Some([5u8; 32]),
            delegation_depth: 1,
            facts: vec![(FEDERATION_PROXY_FACT_KEY.to_string(), FEDERATION_PROXY_FACT_VALUE.to_vec())],
            signature: [4u8; 64],
        }
    }

    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    #[test]
    fn token_allows_proxying_only_for_public_or_explicit_delegated_bearer_presenters() {
        let bearer = test_token(aspen_auth::Audience::Bearer);
        let audience_key = iroh::SecretKey::from_bytes(&[8u8; 32]).public();
        let key_bound = test_token(aspen_auth::Audience::Key(audience_key));
        let mut generic_bearer = bearer.clone();
        generic_bearer.delegation_depth = 0;
        generic_bearer.proof = None;
        let mut malformed_delegated_bearer = bearer.clone();
        malformed_delegated_bearer.proof = None;
        let mut long_lived_bearer = bearer.clone();
        long_lived_bearer.expires_at =
            long_lived_bearer.issued_at.saturating_add(MAX_FEDERATION_PROXY_TOKEN_LIFETIME_SECS + 1);
        let mut unmarked_bearer = bearer.clone();
        unmarked_bearer.facts = Vec::new();

        assert!(token_allows_proxying(None));
        assert!(token_allows_proxying(Some(&bearer)));
        assert!(!token_allows_proxying(Some(&key_bound)));
        assert!(!token_allows_proxying(Some(&generic_bearer)));
        assert!(!token_allows_proxying(Some(&malformed_delegated_bearer)));
        assert!(!token_allows_proxying(Some(&long_lived_bearer)));
        assert!(!token_allows_proxying(Some(&unmarked_bearer)));
    }

    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    #[test]
    fn proxied_authenticated_request_preserves_client_token() {
        let token = test_token(aspen_auth::Audience::Bearer);

        let proxied = proxied_authenticated_request(ClientRpcRequest::Ping, Some(token.clone()), 2);

        assert_eq!(proxied.proxy_hops, 3);
        assert!(proxied.token.is_some(), "proxying must not strip the verified client capability token");
        assert_eq!(proxied.token.expect("token should be forwarded").nonce, token.nonce);
    }
}
