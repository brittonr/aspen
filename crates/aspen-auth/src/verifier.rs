//! Token verification and authorization.
//!
//! Verifies token signatures and checks if capabilities authorize operations.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::RwLock;

use iroh::PublicKey;

use crate::builder::bytes_to_sign;
use crate::capability::Operation;
use crate::constants::MAX_DELEGATION_DEPTH;
use crate::constants::TOKEN_CLOCK_SKEW_SECS;
use crate::error::AuthError;
use crate::token::Audience;
use crate::token::CapabilityToken;
use crate::utils::current_time_secs;

/// Verifies capability tokens and checks authorization.
///
/// Maintains a revocation list, optional parent token cache for delegation chain
/// verification, and can optionally restrict to trusted root issuers.
///
/// # Delegation Chain Verification
///
/// When `trusted_roots` is configured, delegated tokens require their parent tokens
/// to be either:
/// 1. Registered via `register_parent_token()`, or
/// 2. Provided via `verify_with_chain()` method
///
/// This ensures the entire delegation chain leads back to a trusted root.
pub struct TokenVerifier {
    /// Set of revoked token hashes.
    revoked: RwLock<HashSet<[u8; 32]>>,
    /// Cache of parent tokens by their hash (for chain verification).
    /// Populated via `register_parent_token()`.
    parent_cache: RwLock<HashMap<[u8; 32], CapabilityToken>>,
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
            parent_cache: RwLock::new(HashMap::new()),
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

    /// Register a parent token in the cache for delegation chain verification.
    ///
    /// When verifying delegated tokens with trusted roots configured, the verifier
    /// needs access to parent tokens to walk the chain back to a trusted root.
    /// This method caches tokens by their hash for later lookup.
    ///
    /// # Arguments
    ///
    /// * `token` - The token to register (will be cached by its hash)
    ///
    /// Returns `Err` if internal lock is poisoned.
    pub fn register_parent_token(&self, token: CapabilityToken) -> Result<(), AuthError> {
        let hash = token.hash();
        let mut cache = self.parent_cache.write().map_err(|_| AuthError::InternalError {
            reason: "parent cache lock poisoned".to_string(),
        })?;
        cache.insert(hash, token);
        Ok(())
    }

    /// Clear all cached parent tokens.
    ///
    /// Returns `Err` if internal lock is poisoned.
    pub fn clear_parent_cache(&self) -> Result<(), AuthError> {
        self.parent_cache
            .write()
            .map_err(|_| AuthError::InternalError {
                reason: "parent cache lock poisoned".to_string(),
            })?
            .clear();
        Ok(())
    }

    /// Verify token signature and validity.
    ///
    /// Checks:
    /// 1. Signature is valid
    /// 2. Token is not expired
    /// 3. Token was not issued in the future
    /// 4. Audience matches presenter (if Key audience)
    /// 5. Token is not revoked
    /// 6. Trusted roots (if configured): root tokens must be from trusted issuers, delegated tokens
    ///    must have their chain verified via parent cache
    ///
    /// # Arguments
    ///
    /// * `token` - The token to verify
    /// * `presenter` - Optional public key of who is presenting the token
    ///
    /// # Trusted Root Behavior
    ///
    /// When trusted roots are configured:
    /// - Root tokens (no proof): issuer must be in trusted_roots
    /// - Delegated tokens: parent must be in cache, and chain must lead to trusted root
    pub fn verify(&self, token: &CapabilityToken, presenter: Option<&PublicKey>) -> Result<(), AuthError> {
        self.verify_internal(token, presenter, 0)
    }

    /// Internal recursive verification with depth tracking.
    fn verify_internal(
        &self,
        token: &CapabilityToken,
        presenter: Option<&PublicKey>,
        chain_depth: u8,
    ) -> Result<(), AuthError> {
        // Prevent infinite recursion with depth limit
        if chain_depth > MAX_DELEGATION_DEPTH {
            return Err(AuthError::DelegationTooDeep {
                depth: chain_depth,
                max: MAX_DELEGATION_DEPTH,
            });
        }

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

        // 4. Check audience (only for the leaf token being presented, not parents in chain)
        if chain_depth == 0 {
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
        }

        // 5. Check revocation
        let hash = token.hash();
        let revoked_guard = self.revoked.read().map_err(|_| AuthError::InternalError {
            reason: "revocation lock poisoned".to_string(),
        })?;
        if revoked_guard.contains(&hash) {
            return Err(AuthError::TokenRevoked);
        }
        drop(revoked_guard); // Release lock before potential recursion

        // 6. Verify trusted roots and delegation chain
        if !self.trusted_roots.is_empty() {
            if let Some(parent_hash) = token.proof {
                // Delegated token: verify parent chain
                let cache = self.parent_cache.read().map_err(|_| AuthError::InternalError {
                    reason: "parent cache lock poisoned".to_string(),
                })?;

                if let Some(parent) = cache.get(&parent_hash) {
                    // Clone parent to release lock before recursive call
                    let parent = parent.clone();
                    drop(cache);

                    // Recursively verify parent (with no presenter, audience doesn't apply)
                    self.verify_internal(&parent, None, chain_depth + 1)?;
                } else {
                    // Parent not in cache - cannot verify chain
                    return Err(AuthError::ParentTokenRequired);
                }
            } else {
                // Root token: issuer must be in trusted_roots
                if !self.trusted_roots.contains(&token.issuer) {
                    return Err(AuthError::UntrustedRoot);
                }
            }
        }

        Ok(())
    }

    /// Verify token with an explicit chain of parent tokens.
    ///
    /// Use this method when you have the full delegation chain available
    /// and want to verify without registering tokens in the cache.
    ///
    /// # Arguments
    ///
    /// * `token` - The token to verify
    /// * `chain` - Slice of parent tokens, ordered from immediate parent to root
    /// * `presenter` - Optional public key of who is presenting the token
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // For a chain: root -> service -> client
    /// verifier.verify_with_chain(&client_token, &[service_token, root_token], presenter)?;
    /// ```
    pub fn verify_with_chain(
        &self,
        token: &CapabilityToken,
        chain: &[CapabilityToken],
        presenter: Option<&PublicKey>,
    ) -> Result<(), AuthError> {
        // Build a temporary lookup map for the chain
        let chain_map: HashMap<[u8; 32], &CapabilityToken> = chain.iter().map(|t| (t.hash(), t)).collect();

        self.verify_with_chain_internal(token, &chain_map, presenter, 0)
    }

    /// Internal recursive verification with explicit chain.
    fn verify_with_chain_internal(
        &self,
        token: &CapabilityToken,
        chain_map: &HashMap<[u8; 32], &CapabilityToken>,
        presenter: Option<&PublicKey>,
        chain_depth: u8,
    ) -> Result<(), AuthError> {
        // Prevent infinite recursion with depth limit
        if chain_depth > MAX_DELEGATION_DEPTH {
            return Err(AuthError::DelegationTooDeep {
                depth: chain_depth,
                max: MAX_DELEGATION_DEPTH,
            });
        }

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

        // 4. Check audience (only for the leaf token being presented, not parents in chain)
        if chain_depth == 0 {
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
        }

        // 5. Check revocation
        let hash = token.hash();
        let revoked_guard = self.revoked.read().map_err(|_| AuthError::InternalError {
            reason: "revocation lock poisoned".to_string(),
        })?;
        if revoked_guard.contains(&hash) {
            return Err(AuthError::TokenRevoked);
        }
        drop(revoked_guard); // Release lock before potential recursion

        // 6. Verify trusted roots and delegation chain
        if !self.trusted_roots.is_empty() {
            if let Some(parent_hash) = token.proof {
                // Delegated token: verify parent chain
                if let Some(parent) = chain_map.get(&parent_hash) {
                    // Recursively verify parent (with no presenter, audience doesn't apply)
                    self.verify_with_chain_internal(parent, chain_map, None, chain_depth + 1)?;
                } else {
                    // Also check parent cache as fallback
                    let cache = self.parent_cache.read().map_err(|_| AuthError::InternalError {
                        reason: "parent cache lock poisoned".to_string(),
                    })?;

                    if let Some(parent) = cache.get(&parent_hash) {
                        let parent = parent.clone();
                        drop(cache);
                        self.verify_with_chain_internal(&parent, chain_map, None, chain_depth + 1)?;
                    } else {
                        // Parent not found in chain or cache
                        return Err(AuthError::ParentTokenRequired);
                    }
                }
            } else {
                // Root token: issuer must be in trusted_roots
                if !self.trusted_roots.contains(&token.issuer) {
                    return Err(AuthError::UntrustedRoot);
                }
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
