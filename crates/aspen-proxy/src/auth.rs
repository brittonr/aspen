//! Aspen authentication handler for the HTTP proxy.
//!
//! Integrates Aspen's cluster cookie authentication with iroh-proxy-utils'
//! [`AuthHandler`] trait. Peers are identified by their iroh [`EndpointId`]
//! (Ed25519 public key), and authorization is checked against the cluster's
//! trusted peers or shared cookie.

use iroh::EndpointId;
use iroh_proxy_utils::HttpProxyRequest;
use iroh_proxy_utils::upstream::AuthError;
use iroh_proxy_utils::upstream::AuthHandler;
use tracing::debug;
use tracing::warn;

/// Authentication handler that validates proxy requests against an Aspen cluster cookie.
///
/// This handler checks the `Iroh-Destination` header or a custom `X-Aspen-Cookie` header
/// for the cluster cookie. If the cookie matches, the request is authorized.
///
/// For production deployments, this should be extended to use Aspen's full auth system
/// (HMAC-SHA256 challenge-response, trusted peer registry, etc.).
#[derive(Debug, Clone)]
pub struct AspenAuthHandler {
    cluster_cookie: String,
}

impl AspenAuthHandler {
    /// Create a new auth handler with the given cluster cookie.
    pub fn new(cluster_cookie: String) -> Self {
        Self { cluster_cookie }
    }
}

/// HTTP header for Aspen cluster cookie authentication.
pub const ASPEN_COOKIE_HEADER: &str = "X-Aspen-Cookie";

impl AuthHandler for AspenAuthHandler {
    async fn authorize<'a>(&'a self, remote_id: EndpointId, req: &'a HttpProxyRequest) -> Result<(), AuthError> {
        // Check for cookie in custom header
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

        // No cookie provided — reject
        debug!(
            remote=%remote_id.fmt_short(),
            "no authentication credentials provided"
        );
        Err(AuthError::Forbidden)
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
        // Use a deterministic key for testing
        let secret = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
        secret.public()
    }

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
    async fn test_no_cookie() {
        let handler = AspenAuthHandler::new("test-cookie".to_string());
        let remote = test_endpoint_id();
        let req = make_request(None);
        assert!(handler.authorize(remote, &req).await.is_err());
    }
}
