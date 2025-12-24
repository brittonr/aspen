//! Token builder for creating capability tokens.
//!
//! Provides a fluent API for constructing tokens with proper signing.

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use iroh::PublicKey;
use iroh::SecretKey;
use rand::RngCore;

use crate::auth::capability::Capability;
use crate::auth::error::AuthError;
use crate::auth::token::Audience;
use crate::auth::token::CapabilityToken;
use crate::raft::constants::MAX_CAPABILITIES_PER_TOKEN;
use crate::raft::constants::MAX_DELEGATION_DEPTH;

/// Builder for creating capability tokens.
///
/// # Example
///
/// ```rust,ignore
/// let token = TokenBuilder::new(secret_key)
///     .for_key(client_public_key)
///     .with_capability(Capability::Full { prefix: "myapp:".into() })
///     .with_lifetime(Duration::from_secs(3600))
///     .build()?;
/// ```
pub struct TokenBuilder {
    issuer_key: SecretKey,
    audience: Audience,
    capabilities: Vec<Capability>,
    lifetime: Duration,
    nonce: Option<[u8; 16]>,
    parent: Option<CapabilityToken>,
}

impl TokenBuilder {
    /// Create a new token builder.
    ///
    /// # Arguments
    ///
    /// * `issuer_key` - The secret key that will sign the token
    pub fn new(issuer_key: SecretKey) -> Self {
        Self {
            issuer_key,
            audience: Audience::Bearer,
            capabilities: Vec::new(),
            lifetime: Duration::from_secs(3600), // 1 hour default
            nonce: None,
            parent: None,
        }
    }

    /// Set the audience (who can use this token).
    pub fn for_audience(mut self, audience: Audience) -> Self {
        self.audience = audience;
        self
    }

    /// Set audience to a specific public key.
    pub fn for_key(mut self, key: PublicKey) -> Self {
        self.audience = Audience::Key(key);
        self
    }

    /// Add a capability.
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Add multiple capabilities.
    pub fn with_capabilities(mut self, caps: impl IntoIterator<Item = Capability>) -> Self {
        self.capabilities.extend(caps);
        self
    }

    /// Set token lifetime.
    pub fn with_lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime = lifetime;
        self
    }

    /// Set a specific nonce for uniqueness (enables revocation).
    pub fn with_nonce(mut self, nonce: [u8; 16]) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Generate a random nonce.
    pub fn with_random_nonce(mut self) -> Self {
        let mut nonce = [0u8; 16];
        rand::rng().fill_bytes(&mut nonce);
        self.nonce = Some(nonce);
        self
    }

    /// Delegate from a parent token (attenuated delegation).
    ///
    /// The new token can only have capabilities that are subsets
    /// of the parent token's capabilities.
    pub fn delegated_from(mut self, parent: CapabilityToken) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Build and sign the token.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Too many capabilities
    /// - Delegation chain too deep
    /// - Capability escalation attempted
    /// - Parent doesn't allow delegation
    pub fn build(self) -> Result<CapabilityToken, AuthError> {
        // Validate capability count
        if self.capabilities.len() > MAX_CAPABILITIES_PER_TOKEN as usize {
            return Err(AuthError::TooManyCapabilities {
                count: self.capabilities.len(),
                max: MAX_CAPABILITIES_PER_TOKEN,
            });
        }

        // Validate delegation
        if let Some(ref parent) = self.parent {
            let depth = self.delegation_depth(parent);
            if depth > MAX_DELEGATION_DEPTH {
                return Err(AuthError::DelegationTooDeep {
                    depth,
                    max: MAX_DELEGATION_DEPTH,
                });
            }

            // Validate attenuation: child capabilities must be subset of parent
            for cap in &self.capabilities {
                if !parent.capabilities.iter().any(|p| p.contains(cap)) {
                    return Err(AuthError::CapabilityEscalation {
                        requested: format!("{:?}", cap),
                    });
                }
            }

            // Parent must have Delegate capability
            if !parent.capabilities.contains(&Capability::Delegate) {
                return Err(AuthError::DelegationNotAllowed);
            }
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("system time before UNIX epoch").as_secs();

        // Create token without signature first
        let mut token = CapabilityToken {
            version: 1,
            issuer: self.issuer_key.public(),
            audience: self.audience,
            capabilities: self.capabilities,
            issued_at: now,
            expires_at: now + self.lifetime.as_secs(),
            nonce: self.nonce,
            proof: self.parent.as_ref().map(|p| p.hash()),
            signature: [0u8; 64], // Placeholder
        };

        // Sign the token
        let sign_bytes = bytes_to_sign(&token);
        let signature = self.issuer_key.sign(&sign_bytes);
        token.signature = signature.to_bytes();

        Ok(token)
    }

    /// Calculate delegation depth from parent token.
    fn delegation_depth(&self, token: &CapabilityToken) -> u8 {
        // Count proof chain depth
        // For MVP: trust the parent's depth + 1
        // Full implementation would verify the entire chain
        if token.proof.is_some() {
            2 // Parent is already delegated, so we're at depth 2+
        } else {
            1 // Parent is root, so we're at depth 1
        }
    }
}

/// Generate a root capability token with full cluster access.
///
/// This creates a token with:
/// - `Full { prefix: "" }` - read/write/delete all keys
/// - `ClusterAdmin` - cluster management operations
/// - `Delegate` - ability to create child tokens
///
/// Use this during cluster bootstrap to create the initial admin token.
///
/// # Arguments
///
/// * `secret_key` - The Ed25519 key that will sign the token (becomes trusted root)
/// * `lifetime` - How long the token should be valid
///
/// # Example
///
/// ```rust,ignore
/// use aspen::auth::generate_root_token;
/// use std::time::Duration;
///
/// let root_token = generate_root_token(&secret_key, Duration::from_secs(86400 * 365))?;
/// println!("Root token: {}", root_token.to_base64()?);
/// ```
pub fn generate_root_token(secret_key: &SecretKey, lifetime: Duration) -> Result<CapabilityToken, AuthError> {
    TokenBuilder::new(secret_key.clone())
        .with_capability(Capability::Full { prefix: String::new() })
        .with_capability(Capability::ClusterAdmin)
        .with_capability(Capability::Delegate)
        .with_lifetime(lifetime)
        .with_random_nonce()
        .build()
}

/// Compute bytes to sign for a token.
///
/// Signs everything except the signature field itself.
pub(crate) fn bytes_to_sign(token: &CapabilityToken) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(256);

    bytes.push(token.version);
    bytes.extend_from_slice(token.issuer.as_bytes());

    // Serialize audience
    if let Ok(audience_bytes) = postcard::to_allocvec(&token.audience) {
        bytes.extend_from_slice(&audience_bytes);
    }

    // Serialize capabilities
    if let Ok(cap_bytes) = postcard::to_allocvec(&token.capabilities) {
        bytes.extend_from_slice(&cap_bytes);
    }

    bytes.extend_from_slice(&token.issued_at.to_le_bytes());
    bytes.extend_from_slice(&token.expires_at.to_le_bytes());

    if let Some(nonce) = token.nonce {
        bytes.extend_from_slice(&nonce);
    }
    if let Some(proof) = token.proof {
        bytes.extend_from_slice(&proof);
    }

    bytes
}
