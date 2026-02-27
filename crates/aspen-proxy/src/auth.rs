//! Aspen authentication handler for the HTTP proxy.
//!
//! Integrates Aspen's cluster cookie authentication with iroh-proxy-utils'
//! [`AuthHandler`] trait. Peers are identified by their iroh [`EndpointId`]
//! (Ed25519 public key), and authorization is checked against the cluster's
//! trusted peers or shared cookie.
//!
//! # Authentication Modes
//!
//! 1. **Cookie header**: If `X-Aspen-Cookie` is present and matches → authorized. If present but
//!    wrong → rejected. This provides explicit cluster membership proof.
//!
//! 2. **Trusted peers allowlist**: If no cookie header, and a trusted-peers set is configured, the
//!    `remote_id` is checked against it. Only cluster members (derived from Raft membership) are
//!    allowed through.
//!
//! 3. **Transport-only auth (fallback)**: If no cookie and no allowlist configured, any
//!    iroh-authenticated peer is accepted. This is the default for backwards compatibility but
//!    should not be used in production.

use std::collections::HashSet;
use std::sync::Arc;

use iroh::EndpointId;
use iroh_proxy_utils::HttpProxyRequest;
use iroh_proxy_utils::upstream::AuthError;
use iroh_proxy_utils::upstream::AuthHandler;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::warn;

/// Shared, dynamically updatable set of trusted peer endpoint IDs.
///
/// Wrapped in `Arc<RwLock<_>>` so cluster membership changes can propagate
/// to the auth handler without restart. Writers acquire the write lock
/// briefly when membership changes; readers (every proxy request) take
/// only a read lock.
pub type TrustedPeers = Arc<RwLock<HashSet<EndpointId>>>;

/// Authentication handler that validates proxy requests against an Aspen cluster cookie
/// and/or a trusted-peers allowlist.
///
/// # Construction
///
/// - [`AspenAuthHandler::new`]: Cookie-only mode (backwards compatible, no allowlist).
/// - [`AspenAuthHandler::with_trusted_peers`]: Cookie + allowlist mode (production).
///
/// # Thread Safety
///
/// The trusted-peers set is behind `Arc<RwLock<_>>` and can be updated
/// concurrently (e.g., from a cluster membership watcher task).
#[derive(Debug, Clone)]
pub struct AspenAuthHandler {
    cluster_cookie: String,
    /// Optional allowlist of trusted peer endpoint IDs.
    /// When `Some`, peers without a cookie must be in this set.
    /// When `None`, any iroh-authenticated peer is accepted (legacy mode).
    trusted_peers: Option<TrustedPeers>,
}

impl AspenAuthHandler {
    /// Create a new auth handler with cookie-only authentication.
    ///
    /// Without a trusted-peers allowlist, any iroh-authenticated peer that
    /// doesn't send a wrong cookie is accepted. Use [`Self::with_trusted_peers`]
    /// for production deployments.
    pub fn new(cluster_cookie: String) -> Self {
        Self {
            cluster_cookie,
            trusted_peers: None,
        }
    }

    /// Create a new auth handler with cookie + trusted-peers allowlist.
    ///
    /// Peers that don't provide a cookie header are checked against the
    /// allowlist. If the peer's `EndpointId` is not in the set, the
    /// request is rejected with `Forbidden`.
    ///
    /// The `TrustedPeers` set can be updated dynamically (e.g., from a
    /// cluster membership watcher) via its `Arc<RwLock<_>>` handle.
    pub fn with_trusted_peers(cluster_cookie: String, trusted_peers: TrustedPeers) -> Self {
        Self {
            cluster_cookie,
            trusted_peers: Some(trusted_peers),
        }
    }
}

/// HTTP header for Aspen cluster cookie authentication.
pub const ASPEN_COOKIE_HEADER: &str = "X-Aspen-Cookie";

impl AuthHandler for AspenAuthHandler {
    async fn authorize<'a>(&'a self, remote_id: EndpointId, req: &'a HttpProxyRequest) -> Result<(), AuthError> {
        // Check for cookie in custom header (optional additional validation).
        //
        // iroh-proxy-utils' DownstreamProxy sends bare CONNECT requests without
        // custom headers, so most connections will not carry the cookie. We accept
        // iroh-authenticated connections (QUIC TLS verifies EndpointId) and only
        // reject when a cookie IS provided but doesn't match (wrong cluster).
        if let Some(cookie) = req.headers.get(ASPEN_COOKIE_HEADER) {
            let cookie_str = cookie.to_str().map_err(|_| {
                warn!(remote=%remote_id.fmt_short(), "invalid cookie header encoding");
                AuthError::InvalidCredentials
            })?;

            if cookie_str == self.cluster_cookie {
                debug!(remote=%remote_id.fmt_short(), "authorized via cluster cookie");
                return Ok(());
            }
            warn!(remote=%remote_id.fmt_short(), "cookie mismatch");
            return Err(AuthError::Forbidden);
        }

        // No cookie provided — check trusted-peers allowlist if configured.
        if let Some(ref trusted_peers) = self.trusted_peers {
            let peers = trusted_peers.read().await;
            if peers.contains(&remote_id) {
                debug!(
                    remote=%remote_id.fmt_short(),
                    "authorized via trusted-peers allowlist"
                );
                return Ok(());
            }
            warn!(
                remote=%remote_id.fmt_short(),
                trusted_count=peers.len(),
                "peer not in trusted-peers allowlist"
            );
            return Err(AuthError::Forbidden);
        }

        // No cookie, no allowlist — accept based on iroh transport-level auth.
        //
        // iroh QUIC connections are TLS-authenticated: each peer has a verified
        // EndpointId (Ed25519 public key). To connect at all, the client must know
        // the node's EndpointId from the cluster ticket. The proxy ALPN is only
        // registered when --enable-proxy is set.
        //
        // This fallback is for backwards compatibility. Production deployments
        // should use `with_trusted_peers()` to restrict access to cluster members.
        debug!(
            remote=%remote_id.fmt_short(),
            "authorized via iroh transport authentication (no allowlist configured)"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use http::HeaderMap;
    use iroh_proxy_utils::Authority;

    use super::*;

    fn make_request(cookie: Option<&str>) -> HttpProxyRequest {
        use iroh_proxy_utils::HttpProxyRequestKind;

        let mut headers = HeaderMap::new();
        if let Some(c) = cookie {
            headers.insert(ASPEN_COOKIE_HEADER, c.parse().unwrap());
        }

        HttpProxyRequest {
            headers,
            kind: HttpProxyRequestKind::Tunnel {
                target: Authority::new("localhost".to_string(), 8080),
            },
        }
    }

    fn test_endpoint_id() -> EndpointId {
        let secret = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
        secret.public()
    }

    fn make_trusted_peers(ids: &[EndpointId]) -> TrustedPeers {
        Arc::new(RwLock::new(ids.iter().cloned().collect()))
    }

    // =========================================================================
    // Cookie-only mode (no allowlist)
    // =========================================================================

    #[tokio::test]
    async fn test_valid_cookie() {
        let handler = AspenAuthHandler::new("test-cookie".to_string());
        let remote = test_endpoint_id();
        let req = make_request(Some("test-cookie"));
        assert!(handler.authorize(remote, &req).await.is_ok());
    }

    #[tokio::test]
    async fn test_wrong_cookie() {
        let handler = AspenAuthHandler::new("test-cookie".to_string());
        let remote = test_endpoint_id();
        let req = make_request(Some("wrong-cookie"));
        assert!(handler.authorize(remote, &req).await.is_err());
    }

    #[tokio::test]
    async fn test_no_cookie_no_allowlist_accepted() {
        // Legacy mode: no cookie, no allowlist → accepted via transport auth.
        let handler = AspenAuthHandler::new("test-cookie".to_string());
        let remote = test_endpoint_id();
        let req = make_request(None);
        assert!(handler.authorize(remote, &req).await.is_ok());
    }

    // =========================================================================
    // Trusted-peers allowlist mode
    // =========================================================================

    #[tokio::test]
    async fn test_trusted_peer_accepted() {
        let remote = test_endpoint_id();
        let trusted = make_trusted_peers(&[remote]);
        let handler = AspenAuthHandler::with_trusted_peers("cookie".into(), trusted);
        let req = make_request(None);
        assert!(handler.authorize(remote, &req).await.is_ok());
    }

    #[tokio::test]
    async fn test_untrusted_peer_rejected() {
        let remote = test_endpoint_id();
        let other = test_endpoint_id();
        // Allowlist contains `other` but not `remote`
        let trusted = make_trusted_peers(&[other]);
        let handler = AspenAuthHandler::with_trusted_peers("cookie".into(), trusted);
        let req = make_request(None);
        assert!(handler.authorize(remote, &req).await.is_err());
    }

    #[tokio::test]
    async fn test_empty_allowlist_rejects_all() {
        let remote = test_endpoint_id();
        let trusted = make_trusted_peers(&[]);
        let handler = AspenAuthHandler::with_trusted_peers("cookie".into(), trusted);
        let req = make_request(None);
        assert!(handler.authorize(remote, &req).await.is_err());
    }

    #[tokio::test]
    async fn test_cookie_overrides_allowlist() {
        // Valid cookie should work even if peer is NOT in the allowlist.
        let remote = test_endpoint_id();
        let trusted = make_trusted_peers(&[]); // empty allowlist
        let handler = AspenAuthHandler::with_trusted_peers("cookie".into(), trusted);
        let req = make_request(Some("cookie"));
        assert!(handler.authorize(remote, &req).await.is_ok());
    }

    #[tokio::test]
    async fn test_wrong_cookie_rejected_despite_allowlist() {
        // Wrong cookie should fail even if peer IS in the allowlist.
        let remote = test_endpoint_id();
        let trusted = make_trusted_peers(&[remote]);
        let handler = AspenAuthHandler::with_trusted_peers("cookie".into(), trusted);
        let req = make_request(Some("wrong"));
        assert!(handler.authorize(remote, &req).await.is_err());
    }

    #[tokio::test]
    async fn test_dynamic_allowlist_update() {
        let remote = test_endpoint_id();
        let trusted = make_trusted_peers(&[]); // start empty
        let handler = AspenAuthHandler::with_trusted_peers("cookie".into(), trusted.clone());
        let req = make_request(None);

        // Should be rejected — not in allowlist
        assert!(handler.authorize(remote, &req).await.is_err());

        // Dynamically add the peer
        trusted.write().await.insert(remote);

        // Now should be accepted
        assert!(handler.authorize(remote, &req).await.is_ok());
    }
}
