//! Raft RPC authentication using HMAC-SHA256.
//!
//! Provides challenge-response authentication for Raft connections using
//! the cluster cookie as the HMAC key. Prevents unauthorized nodes from
//! participating in consensus.
//!
//! # Protocol Flow
//!
//! 1. Server sends AuthChallenge with random nonce and timestamp
//! 2. Client computes HMAC-SHA256(cookie_key, nonce || timestamp || client_id)
//! 3. Client sends AuthResponse with HMAC and its own nonce
//! 4. Server verifies HMAC and sends AuthResult
//!
//! # Replay Attack Prevention
//!
//! - Nonce: Random 32-byte value prevents replay
//! - Timestamp: 60-second window prevents old challenges
//! - Client ID: Binds authentication to specific endpoint
//!
//! # Tiger Style
//!
//! - Fixed nonce size (32 bytes = 256 bits)
//! - Fixed HMAC output (32 bytes SHA-256)
//! - Constant-time comparison prevents timing attacks
//! - Explicit timeout on challenge validity

use std::time::Duration;

use aspen_core::utils;
use hmac::Hmac;
use hmac::Mac;
use rand::RngCore;
use serde::Deserialize;
use serde::Serialize;
use sha2::Sha256;

use crate::pure::constant_time_compare;
use crate::pure::derive_hmac_key;
use crate::pure::is_challenge_valid;

/// Type alias for HMAC-SHA256.
type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// Constants
// ============================================================================

/// Size of authentication nonces (32 bytes = 256 bits).
pub const AUTH_NONCE_SIZE: usize = 32;

/// HMAC output size (SHA-256 = 32 bytes).
pub const AUTH_HMAC_SIZE: usize = 32;

/// Maximum age of a valid challenge (60 seconds).
pub const AUTH_CHALLENGE_MAX_AGE_SECS: u64 = 60;

/// Timeout for authentication handshake (5 seconds).
pub const AUTH_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Current authentication protocol version.
pub const AUTH_PROTOCOL_VERSION: u8 = 1;

/// Maximum size of serialized auth messages (256 bytes).
///
/// Tiger Style: Fixed upper bound on auth message size.
pub const MAX_AUTH_MESSAGE_SIZE: usize = 256;

// ============================================================================
// Protocol Types
// ============================================================================

/// Authentication challenge sent by the server.
///
/// The server generates a random nonce and current timestamp, then sends this
/// to the connecting client. The client must respond with a valid HMAC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    /// Random nonce (32 bytes) for replay prevention.
    pub nonce: [u8; AUTH_NONCE_SIZE],
    /// Challenge creation timestamp (milliseconds since UNIX epoch).
    pub timestamp_ms: u64,
    /// Protocol version for forward compatibility.
    pub protocol_version: u8,
}

/// Authentication response sent by the client.
///
/// Contains the HMAC computed over the challenge data using the cluster cookie
/// as the key, plus the client's endpoint ID for identity binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// HMAC-SHA256 of (server_nonce || timestamp || client_endpoint_id).
    pub hmac: [u8; AUTH_HMAC_SIZE],
    /// Client's own nonce for mutual authentication.
    pub client_nonce: [u8; AUTH_NONCE_SIZE],
    /// Client's Iroh endpoint ID (32 bytes) for identity binding.
    pub client_endpoint_id: [u8; 32],
}

/// Authentication result sent by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthResult {
    /// Authentication succeeded, proceed with protocol.
    Ok,
    /// Challenge expired (timestamp too old).
    ChallengeExpired,
    /// HMAC verification failed (wrong cookie or tampered data).
    InvalidHmac,
    /// Protocol version not supported.
    UnsupportedVersion,
    /// Generic failure (internal error).
    Failed,
}

impl AuthResult {
    /// Returns true if authentication succeeded.
    pub fn is_ok(&self) -> bool {
        matches!(self, AuthResult::Ok)
    }
}

// ============================================================================
// Authentication Context
// ============================================================================

/// Authentication context containing the derived HMAC key.
///
/// Created once from the cluster cookie and reused for all authentication
/// operations. The key is derived using Blake3 to ensure proper entropy
/// even if the cookie has low entropy.
#[derive(Clone)]
pub struct AuthContext {
    /// HMAC key derived from cluster cookie via Blake3.
    key: [u8; 32],
}

impl AuthContext {
    /// Create authentication context from cluster cookie.
    ///
    /// Uses Blake3 to derive a proper 256-bit key from the cookie,
    /// ensuring the HMAC key has full entropy regardless of cookie
    /// quality.
    ///
    /// # Arguments
    ///
    /// * `cookie` - The cluster cookie string
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ctx = AuthContext::new("my-secure-cluster-cookie");
    /// ```
    pub fn new(cookie: &str) -> Self {
        // Use pure function for key derivation
        Self {
            key: derive_hmac_key(cookie),
        }
    }

    /// Generate a new authentication challenge.
    ///
    /// Creates a challenge with:
    /// - Random 32-byte nonce
    /// - Current timestamp
    /// - Protocol version
    pub fn generate_challenge(&self) -> AuthChallenge {
        let mut nonce = [0u8; AUTH_NONCE_SIZE];
        rand::rng().fill_bytes(&mut nonce);

        AuthChallenge {
            nonce,
            timestamp_ms: current_time_ms(),
            protocol_version: AUTH_PROTOCOL_VERSION,
        }
    }

    /// Compute authentication response for a challenge.
    ///
    /// # Arguments
    ///
    /// * `challenge` - The server's challenge
    /// * `client_endpoint_id` - This client's Iroh endpoint ID (32 bytes)
    ///
    /// # Returns
    ///
    /// AuthResponse containing the HMAC and client information.
    pub fn compute_response(&self, challenge: &AuthChallenge, client_endpoint_id: &[u8; 32]) -> AuthResponse {
        let mut client_nonce = [0u8; AUTH_NONCE_SIZE];
        rand::rng().fill_bytes(&mut client_nonce);

        let hmac = self.compute_hmac(&challenge.nonce, challenge.timestamp_ms, client_endpoint_id);

        AuthResponse {
            hmac,
            client_nonce,
            client_endpoint_id: *client_endpoint_id,
        }
    }

    /// Verify a client's authentication response.
    ///
    /// Checks:
    /// 1. Challenge timestamp is not expired
    /// 2. Protocol version is supported
    /// 3. HMAC matches expected value
    ///
    /// # Arguments
    ///
    /// * `challenge` - The original challenge sent to client
    /// * `response` - The client's response
    ///
    /// # Returns
    ///
    /// AuthResult indicating success or reason for failure.
    pub fn verify_response(&self, challenge: &AuthChallenge, response: &AuthResponse) -> AuthResult {
        // Check timestamp freshness using pure function
        let now = current_time_ms();
        if !is_challenge_valid(challenge.timestamp_ms, now, AUTH_CHALLENGE_MAX_AGE_SECS) {
            return AuthResult::ChallengeExpired;
        }

        // Verify protocol version
        if challenge.protocol_version != AUTH_PROTOCOL_VERSION {
            return AuthResult::UnsupportedVersion;
        }

        // Compute expected HMAC
        let expected = self.compute_hmac(&challenge.nonce, challenge.timestamp_ms, &response.client_endpoint_id);

        // Constant-time comparison using pure function
        if constant_time_compare(&expected, &response.hmac) {
            AuthResult::Ok
        } else {
            AuthResult::InvalidHmac
        }
    }

    /// Compute HMAC over authentication data.
    ///
    /// HMAC input: nonce || timestamp || client_endpoint_id
    ///
    /// # Panics
    ///
    /// This function uses `.expect()` on HMAC key creation. Per RFC 2104,
    /// HMAC-SHA256 accepts keys of any length (shorter keys are zero-padded,
    /// longer keys are hashed first). Since `self.key` is always a fixed
    /// `[u8; 32]` array derived from Blake3, this will never fail. The
    /// `.expect()` is retained for defense-in-depth against future refactoring
    /// that might change the key type.
    fn compute_hmac(
        &self,
        nonce: &[u8; AUTH_NONCE_SIZE],
        timestamp_ms: u64,
        client_id: &[u8; 32],
    ) -> [u8; AUTH_HMAC_SIZE] {
        // SAFETY: HMAC-SHA256 accepts any key length per RFC 2104. Our key is
        // a fixed [u8; 32] from AuthContext, which guarantees valid input.
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key size per RFC 2104");

        // Feed data in deterministic order
        mac.update(nonce);
        mac.update(&timestamp_ms.to_le_bytes());
        mac.update(client_id);

        let result = mac.finalize();
        let mut output = [0u8; AUTH_HMAC_SIZE];
        output.copy_from_slice(&result.into_bytes());
        output
    }
}

impl std::fmt::Debug for AuthContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't expose the key in debug output
        f.debug_struct("AuthContext").field("key", &"[REDACTED]").finish()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current time in milliseconds since UNIX epoch.
///
/// Delegates to `aspen_core::utils::current_time_ms()` for Tiger Style compliance.
#[inline]
fn current_time_ms() -> u64 {
    utils::current_time_ms()
}

// constant_time_compare is now imported from crate::pure

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_creation() {
        let ctx1 = AuthContext::new("test-cookie");
        let ctx2 = AuthContext::new("test-cookie");
        let ctx3 = AuthContext::new("different-cookie");

        // Same cookie produces same key
        assert_eq!(ctx1.key, ctx2.key);

        // Different cookie produces different key
        assert_ne!(ctx1.key, ctx3.key);
    }

    #[test]
    fn test_challenge_generation() {
        let ctx = AuthContext::new("test-cookie");

        let challenge1 = ctx.generate_challenge();
        let challenge2 = ctx.generate_challenge();

        // Each challenge has unique nonce
        assert_ne!(challenge1.nonce, challenge2.nonce);

        // Protocol version is set
        assert_eq!(challenge1.protocol_version, AUTH_PROTOCOL_VERSION);

        // Timestamp is recent
        let now = current_time_ms();
        assert!(challenge1.timestamp_ms <= now);
        assert!(now - challenge1.timestamp_ms < 1000); // Within 1 second
    }

    #[test]
    fn test_successful_authentication() {
        let ctx = AuthContext::new("test-cookie");
        let client_id = [42u8; 32];

        let challenge = ctx.generate_challenge();
        let response = ctx.compute_response(&challenge, &client_id);
        let result = ctx.verify_response(&challenge, &response);

        assert_eq!(result, AuthResult::Ok);
    }

    #[test]
    fn test_wrong_cookie_fails() {
        let server_ctx = AuthContext::new("server-cookie");
        let client_ctx = AuthContext::new("wrong-cookie");
        let client_id = [42u8; 32];

        let challenge = server_ctx.generate_challenge();
        let response = client_ctx.compute_response(&challenge, &client_id);
        let result = server_ctx.verify_response(&challenge, &response);

        assert_eq!(result, AuthResult::InvalidHmac);
    }

    #[test]
    fn test_tampered_response_fails() {
        let ctx = AuthContext::new("test-cookie");
        let client_id = [42u8; 32];

        let challenge = ctx.generate_challenge();
        let mut response = ctx.compute_response(&challenge, &client_id);

        // Tamper with the HMAC
        response.hmac[0] ^= 0xFF;

        let result = ctx.verify_response(&challenge, &response);
        assert_eq!(result, AuthResult::InvalidHmac);
    }

    #[test]
    fn test_wrong_client_id_fails() {
        let ctx = AuthContext::new("test-cookie");
        let real_client_id = [42u8; 32];
        let fake_client_id = [99u8; 32];

        let challenge = ctx.generate_challenge();

        // Compute response with real ID
        let mut response = ctx.compute_response(&challenge, &real_client_id);

        // But claim to be fake ID
        response.client_endpoint_id = fake_client_id;

        let result = ctx.verify_response(&challenge, &response);
        assert_eq!(result, AuthResult::InvalidHmac);
    }

    #[test]
    fn test_constant_time_compare() {
        let a = [1u8; AUTH_HMAC_SIZE];
        let b = [1u8; AUTH_HMAC_SIZE];
        let c = [2u8; AUTH_HMAC_SIZE];

        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));

        // Different at single byte
        let mut d = a;
        d[15] = 99;
        assert!(!constant_time_compare(&a, &d));
    }

    #[test]
    fn test_auth_result_is_ok() {
        assert!(AuthResult::Ok.is_ok());
        assert!(!AuthResult::ChallengeExpired.is_ok());
        assert!(!AuthResult::InvalidHmac.is_ok());
        assert!(!AuthResult::UnsupportedVersion.is_ok());
        assert!(!AuthResult::Failed.is_ok());
    }
}
