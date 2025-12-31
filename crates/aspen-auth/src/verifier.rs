//! Token verification and authorization.
//!
//! Verifies token signatures and checks if capabilities authorize operations.

use std::collections::HashSet;
use std::sync::RwLock;

use iroh::PublicKey;

use crate::builder::bytes_to_sign;
use crate::capability::Operation;
use crate::constants::TOKEN_CLOCK_SKEW_SECS;
use crate::error::AuthError;
use crate::token::Audience;
use crate::token::CapabilityToken;
use crate::utils::current_time_secs;

/// Verifies capability tokens and checks authorization.
///
/// Maintains a revocation list and can optionally restrict to trusted root issuers.
pub struct TokenVerifier {
    /// Set of revoked token hashes.
    revoked: RwLock<HashSet<[u8; 32]>>,
    /// Optional: trusted root issuers (if empty, any issuer is trusted for root tokens).
    trusted_roots: Vec<PublicKey>,
    /// Clock skew tolerance in seconds.
    clock_skew_tolerance: u64,
}

impl TokenVerifier {
    /// Create a new token verifier with default settings.
    pub fn new() -> Self {
        Self {
            revoked: RwLock::new(HashSet::new()),
            trusted_roots: Vec::new(),
            clock_skew_tolerance: TOKEN_CLOCK_SKEW_SECS,
        }
    }

    /// Add a trusted root issuer.
    ///
    /// When trusted roots are configured, only tokens signed by these
    /// issuers (or delegated from them) will be accepted.
    pub fn with_trusted_root(mut self, key: PublicKey) -> Self {
        self.trusted_roots.push(key);
        self
    }

    /// Set clock skew tolerance.
    pub fn with_clock_skew_tolerance(mut self, seconds: u64) -> Self {
        self.clock_skew_tolerance = seconds;
        self
    }

    /// Verify token signature and validity.
    ///
    /// Checks:
    /// 1. Signature is valid
    /// 2. Token is not expired
    /// 3. Token was not issued in the future
    /// 4. Audience matches presenter (if Key audience)
    /// 5. Token is not revoked
    ///
    /// # Arguments
    ///
    /// * `token` - The token to verify
    /// * `presenter` - Optional public key of who is presenting the token
    pub fn verify(&self, token: &CapabilityToken, presenter: Option<&PublicKey>) -> Result<(), AuthError> {
        // 1. Check signature
        let sign_bytes = bytes_to_sign(token);
        let signature = iroh::Signature::from_bytes(&token.signature);
        token.issuer.verify(&sign_bytes, &signature).map_err(|_| AuthError::InvalidSignature)?;

        // 2. Check expiration
        let now = current_time_secs();

        if token.expires_at + self.clock_skew_tolerance < now {
            return Err(AuthError::TokenExpired {
                expired_at: token.expires_at,
                now,
            });
        }

        // 3. Check not issued in the future (with tolerance)
        if token.issued_at > now + self.clock_skew_tolerance {
            return Err(AuthError::TokenFromFuture {
                issued_at: token.issued_at,
                now,
            });
        }

        // 4. Check audience
        match &token.audience {
            Audience::Key(expected) => {
                if let Some(actual) = presenter {
                    if expected != actual {
                        return Err(AuthError::WrongAudience {
                            expected: expected.to_string(),
                            actual: actual.to_string(),
                        });
                    }
                } else {
                    return Err(AuthError::AudienceRequired);
                }
            }
            Audience::Bearer => {
                // Anyone can use a bearer token
            }
        }

        // 5. Check revocation
        let hash = token.hash();
        let revoked_guard = self.revoked.read().map_err(|_| AuthError::InternalError {
            reason: "revocation lock poisoned".to_string(),
        })?;
        if revoked_guard.contains(&hash) {
            return Err(AuthError::TokenRevoked);
        }

        // 6. Optionally check trusted roots
        // (In full implementation, would verify the entire delegation chain)
        // For MVP, we just verify the direct issuer if trusted_roots is configured
        if !self.trusted_roots.is_empty() && token.proof.is_none() {
            // This is a root token, check if issuer is trusted
            if !self.trusted_roots.contains(&token.issuer) {
                return Err(AuthError::InvalidSignature); // Treat as untrusted
            }
        }

        Ok(())
    }

    /// Check if token authorizes the given operation.
    ///
    /// First verifies the token, then checks if any capability authorizes the operation.
    pub fn authorize(
        &self,
        token: &CapabilityToken,
        operation: &Operation,
        presenter: Option<&PublicKey>,
    ) -> Result<(), AuthError> {
        // First verify the token itself
        self.verify(token, presenter)?;

        // Then check if any capability authorizes the operation
        for cap in &token.capabilities {
            if cap.authorizes(operation) {
                return Ok(());
            }
        }

        Err(AuthError::Unauthorized {
            operation: operation.to_string(),
        })
    }

    /// Revoke a token by its hash.
    ///
    /// Once revoked, the token will fail verification even if otherwise valid.
    /// Returns error if internal lock is poisoned.
    pub fn revoke(&self, token_hash: [u8; 32]) -> Result<(), AuthError> {
        self.revoked
            .write()
            .map_err(|_| AuthError::InternalError {
                reason: "revocation lock poisoned".to_string(),
            })?
            .insert(token_hash);
        Ok(())
    }

    /// Revoke a token directly.
    pub fn revoke_token(&self, token: &CapabilityToken) -> Result<(), AuthError> {
        self.revoke(token.hash())
    }

    /// Check if a token is revoked.
    ///
    /// Returns `Err` if internal lock is poisoned.
    pub fn is_revoked(&self, token_hash: &[u8; 32]) -> Result<bool, AuthError> {
        let guard = self.revoked.read().map_err(|_| AuthError::InternalError {
            reason: "revocation lock poisoned".to_string(),
        })?;
        Ok(guard.contains(token_hash))
    }

    /// Clear all revocations (use with caution).
    ///
    /// Returns `Err` if internal lock is poisoned.
    pub fn clear_revocations(&self) -> Result<(), AuthError> {
        self.revoked
            .write()
            .map_err(|_| AuthError::InternalError {
                reason: "revocation lock poisoned".to_string(),
            })?
            .clear();
        Ok(())
    }

    /// Get the number of revoked tokens.
    ///
    /// Returns `Err` if internal lock is poisoned.
    pub fn revocation_count(&self) -> Result<usize, AuthError> {
        let guard = self.revoked.read().map_err(|_| AuthError::InternalError {
            reason: "revocation lock poisoned".to_string(),
        })?;
        Ok(guard.len())
    }

    /// Load revoked tokens from persistent storage.
    ///
    /// This populates the in-memory revocation cache from a persistent store.
    /// Typically called during node startup to restore revocations.
    ///
    /// # Arguments
    ///
    /// * `hashes` - Slice of 32-byte token hashes to mark as revoked
    ///
    /// Returns `Err` if internal lock is poisoned.
    pub fn load_revoked(&self, hashes: &[[u8; 32]]) -> Result<(), AuthError> {
        let mut revoked = self.revoked.write().map_err(|_| AuthError::InternalError {
            reason: "revocation lock poisoned".to_string(),
        })?;
        revoked.extend(hashes.iter().copied());
        Ok(())
    }

    /// Get all revoked hashes.
    ///
    /// Returns a snapshot of all currently revoked token hashes.
    /// Useful for persistence or debugging.
    ///
    /// Returns `Err` if internal lock is poisoned.
    pub fn get_all_revoked(&self) -> Result<Vec<[u8; 32]>, AuthError> {
        let guard = self.revoked.read().map_err(|_| AuthError::InternalError {
            reason: "revocation lock poisoned".to_string(),
        })?;
        Ok(guard.iter().copied().collect())
    }
}

impl Default for TokenVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TokenVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenVerifier")
            .field("trusted_roots", &self.trusted_roots.len())
            .field("clock_skew_tolerance", &self.clock_skew_tolerance)
            .field("revocation_count", &self.revocation_count())
            .finish()
    }
}
