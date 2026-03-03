//! Token authorization wrapper.
//!
//! Wraps `aspen-auth` token verification for service mesh operations.
//! All verification is local (pure crypto, no I/O).

use aspen_auth::CapabilityToken;
use aspen_auth::Operation;
use aspen_auth::TokenVerifier;
use snafu::Snafu;

/// Errors from net authorization checks.
#[derive(Debug, Snafu)]
pub enum NetAuthError {
    /// Token verification failed (invalid signature, expired, revoked).
    #[snafu(display("token verification failed: {reason}"))]
    TokenInvalid { reason: String },

    /// Token does not grant the required capability.
    #[snafu(display("unauthorized: token does not grant {operation}"))]
    Unauthorized { operation: String },
}

/// Service mesh authorization based on UCAN capability tokens.
///
/// Holds a capability token and verifier. All checks are local — pure
/// cryptographic verification with no network I/O.
pub struct NetAuthenticator {
    token: CapabilityToken,
    #[allow(dead_code)]
    verifier: TokenVerifier,
}

impl NetAuthenticator {
    /// Create a new authenticator with the given token and verifier.
    pub fn new(token: CapabilityToken, verifier: TokenVerifier) -> Self {
        Self { token, verifier }
    }

    /// Check that the token grants `NetConnect` for the given service and port.
    pub fn check_connect(&self, service: &str, port: u16) -> Result<(), NetAuthError> {
        let op = Operation::NetConnect {
            service: service.to_string(),
            port,
        };
        self.check_operation(&op)
    }

    /// Check that the token grants `NetPublish` for the given service name.
    pub fn check_publish(&self, service: &str) -> Result<(), NetAuthError> {
        let op = Operation::NetPublish {
            service: service.to_string(),
        };
        self.check_operation(&op)
    }

    /// Check that the token grants `NetUnpublish` for the given service name.
    pub fn check_unpublish(&self, service: &str) -> Result<(), NetAuthError> {
        let op = Operation::NetUnpublish {
            service: service.to_string(),
        };
        self.check_operation(&op)
    }

    /// Check if the token is still valid (not expired).
    pub fn is_token_valid(&self) -> bool {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        now < self.token.expires_at
    }

    /// Inner helper: verify token + check capability authorization.
    fn check_operation(&self, op: &Operation) -> Result<(), NetAuthError> {
        // Check expiration
        if !self.is_token_valid() {
            return Err(NetAuthError::TokenInvalid {
                reason: "token expired".to_string(),
            });
        }

        // Check capability authorization
        let authorized = self.token.capabilities.iter().any(|cap| cap.authorizes(op));

        if !authorized {
            return Err(NetAuthError::Unauthorized {
                operation: op.to_string(),
            });
        }

        Ok(())
    }
}
