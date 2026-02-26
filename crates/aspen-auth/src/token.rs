//! Capability token structure and encoding.
//!
//! Tokens are self-contained and can be verified offline using only
//! the issuer's public key and current time.

use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;

use crate::capability::Capability;
use crate::constants::MAX_TOKEN_SIZE;
use crate::error::AuthError;

/// Who can use this token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Audience {
    /// Only this specific public key can use the token.
    Key(#[serde(with = "public_key_serde")] PublicKey),
    /// Anyone holding the token can use it (bearer token).
    Bearer,
}

/// A capability token granting specific permissions.
///
/// # Structure
///
/// - `version`: Protocol version for future compatibility
/// - `issuer`: Ed25519 public key that signed this token
/// - `audience`: Who can use this token
/// - `capabilities`: What operations are allowed
/// - `issued_at/expires_at`: Validity window (Unix seconds)
/// - `nonce`: Optional unique identifier for revocation
/// - `proof`: Hash of parent token (for delegation chains)
/// - `delegation_depth`: How many levels deep in the delegation chain (0 = root)
/// - `signature`: Ed25519 signature over all above fields
///
/// # Tiger Style
///
/// - Fixed size nonces (16 bytes)
/// - Fixed size proof hashes (32 bytes)
/// - Fixed size signatures (64 bytes)
/// - Bounded delegation depth (max 8 levels)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Version for future compatibility.
    pub version: u8,
    /// Who issued (signed) this token.
    #[serde(with = "public_key_serde")]
    pub issuer: PublicKey,
    /// Who can use this token.
    pub audience: Audience,
    /// What operations are allowed.
    pub capabilities: Vec<Capability>,
    /// Unix timestamp when token was issued.
    pub issued_at: u64,
    /// Unix timestamp when token expires.
    pub expires_at: u64,
    /// Optional nonce for uniqueness (enables revocation).
    pub nonce: Option<[u8; 16]>,
    /// Hash of parent token (for delegation chain verification).
    pub proof: Option<[u8; 32]>,
    /// Delegation depth in the chain (0 = root token, 1+ = delegated).
    /// Used to enforce MAX_DELEGATION_DEPTH limit.
    #[serde(default)]
    pub delegation_depth: u8,
    /// Ed25519 signature over all above fields.
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
}

impl CapabilityToken {
    /// Encode token to bytes for transmission.
    ///
    /// Uses postcard for compact binary serialization.
    pub fn encode(&self) -> Result<Vec<u8>, AuthError> {
        let bytes = postcard::to_allocvec(self).map_err(|e| AuthError::EncodingError(e.to_string()))?;
        if bytes.len() > MAX_TOKEN_SIZE as usize {
            return Err(AuthError::TokenTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_TOKEN_SIZE as u64,
            });
        }
        Ok(bytes)
    }

    /// Decode token from bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, AuthError> {
        if bytes.len() > MAX_TOKEN_SIZE as usize {
            return Err(AuthError::TokenTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_TOKEN_SIZE as u64,
            });
        }
        Ok(postcard::from_bytes(bytes)?)
    }

    /// Encode to base64 for text transmission.
    pub fn to_base64(&self) -> Result<String, AuthError> {
        use base64::Engine;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.encode()?))
    }

    /// Decode from base64.
    pub fn from_base64(s: &str) -> Result<Self, AuthError> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s)?;
        Self::decode(&bytes)
    }

    /// Compute hash of this token (for revocation and proof chains).
    ///
    /// Uses BLAKE3 for fast, secure hashing.
    pub fn hash(&self) -> [u8; 32] {
        // Use the encoded form for consistent hashing
        match self.encode() {
            Ok(bytes) => *blake3::hash(&bytes).as_bytes(),
            Err(_) => [0u8; 32], // Should never happen for a valid token
        }
    }
}

/// Serde helper for PublicKey.
mod public_key_serde {
    use iroh::PublicKey;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;

    pub fn serialize<S: Serializer>(key: &PublicKey, s: S) -> Result<S::Ok, S::Error> {
        key.as_bytes().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PublicKey, D::Error> {
        let bytes: [u8; 32] = Deserialize::deserialize(d)?;
        PublicKey::try_from(&bytes[..]).map_err(serde::de::Error::custom)
    }
}

/// Serde helper for Ed25519 signatures (64 bytes).
mod signature_serde {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;

    pub fn serialize<S: Serializer>(sig: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(sig)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let bytes: Vec<u8> = Deserialize::deserialize(d)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom(format!("expected 64 bytes, got {}", bytes.len())));
        }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&bytes);
        Ok(sig)
    }
}
