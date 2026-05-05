//! Capability token structure and encoding.
//!
//! Tokens are self-contained and can be verified offline using only
//! the issuer's public key and current time.

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;

use iroh_base::PublicKey;
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
#[derive(Clone, Serialize, Deserialize)]
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
    /// Arbitrary key-value metadata (UCAN "facts").
    ///
    /// Facts carry signed metadata that does not grant authorization by itself.
    /// Higher-level routing/policy layers may require specific facts in addition
    /// to normal capability checks (for example federation proxy eligibility).
    /// Explicit default helper preserves backward-compatible deserialization.
    #[serde(default = "default_token_facts")]
    pub facts: Vec<(String, Vec<u8>)>,
    /// Ed25519 signature over all above fields.
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
}

fn default_token_facts() -> Vec<(String, Vec<u8>)> {
    Vec::new()
}

impl fmt::Debug for CapabilityToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapabilityToken")
            .field("version", &self.version)
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("capabilities", &self.capabilities)
            .field("issued_at", &self.issued_at)
            .field("expires_at", &self.expires_at)
            .field("has_nonce", &self.nonce.is_some())
            .field("has_proof", &self.proof.is_some())
            .field("delegation_depth", &self.delegation_depth)
            .field("facts_count", &self.facts.len())
            .field("signature", &"<redacted: 64 bytes>")
            .finish()
    }
}

fn max_token_size_usize() -> usize {
    match usize::try_from(MAX_TOKEN_SIZE) {
        Ok(max_size_bytes) => max_size_bytes,
        Err(_) => usize::MAX,
    }
}

fn token_exceeds_max_size(size_bytes: usize) -> bool {
    size_bytes > max_token_size_usize()
}

impl CapabilityToken {
    /// Encode token to bytes for transmission.
    ///
    /// Uses postcard for compact binary serialization.
    pub fn encode(&self) -> Result<Vec<u8>, AuthError> {
        let bytes = postcard::to_allocvec(self).map_err(|e| AuthError::EncodingError(e.to_string()))?;
        if token_exceeds_max_size(bytes.len()) {
            return Err(AuthError::TokenTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: u64::from(MAX_TOKEN_SIZE),
            });
        }
        Ok(bytes)
    }

    /// Decode token from bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, AuthError> {
        if token_exceeds_max_size(bytes.len()) {
            return Err(AuthError::TokenTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: u64::from(MAX_TOKEN_SIZE),
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
    use iroh_base::PublicKey;
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
    use alloc::format;
    use alloc::vec::Vec;

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

#[cfg(test)]
mod i7_serialization_tests {
    use super::*;
    use crate::Capability;

    fn golden_token() -> CapabilityToken {
        let issuer = iroh_base::SecretKey::from([7u8; 32]).public();
        let token = CapabilityToken {
            version: 1,
            issuer,
            audience: Audience::Bearer,
            capabilities: vec![
                Capability::Read {
                    prefix: "cfg/".to_string(),
                },
                Capability::Write {
                    prefix: "jobs/".to_string(),
                },
                Capability::Delegate,
            ],
            issued_at: 1_700_000_000,
            expires_at: 1_700_003_600,
            nonce: Some([0x11; 16]),
            proof: Some([0x22; 32]),
            delegation_depth: 2,
            facts: vec![("scope".to_string(), b"i7-golden".to_vec())],
            signature: [0xA5; 64],
        };
        assert!(token.expires_at > token.issued_at);
        assert!(!token.capabilities.is_empty());
        token
    }

    #[test]
    fn capability_token_golden_roundtrips_through_binary_and_base64() {
        let token = golden_token();
        let encoded = token.encode().expect("golden token encodes");
        assert!(encoded.len() > 100, "golden token should cover the full portable token shape");

        let decoded = CapabilityToken::decode(&encoded).expect("golden bytes decode");
        assert_eq!(decoded.version, token.version);
        assert_eq!(decoded.audience, token.audience);
        assert_eq!(decoded.capabilities, token.capabilities);
        assert_eq!(decoded.issued_at, token.issued_at);
        assert_eq!(decoded.expires_at, token.expires_at);
        assert_eq!(decoded.nonce, token.nonce);
        assert_eq!(decoded.proof, token.proof);
        assert_eq!(decoded.delegation_depth, token.delegation_depth);
        assert_eq!(decoded.facts, token.facts);
        assert_eq!(decoded.signature, token.signature);

        let encoded_text = token.to_base64().expect("golden token base64 encodes");
        let decoded_text = CapabilityToken::from_base64(&encoded_text).expect("golden token base64 decodes");
        assert_eq!(decoded_text.capabilities, token.capabilities);
        assert_eq!(decoded_text.facts, token.facts);
    }

    #[test]
    fn capability_token_debug_redacts_reconstructable_material() {
        let token = golden_token();
        let debug = format!("{token:?}");

        assert!(debug.contains("CapabilityToken"));
        assert!(debug.contains("has_nonce"));
        assert!(debug.contains("has_proof"));
        assert!(debug.contains("facts_count"));
        assert!(debug.contains("<redacted: 64 bytes>"));
        assert!(!debug.contains("signature: [165"));
        assert!(!debug.contains("nonce: Some"));
        assert!(!debug.contains("proof: Some"));
        assert!(!debug.contains("i7-golden"));
    }

    #[test]
    fn malformed_token_inputs_are_rejected() {
        assert!(matches!(CapabilityToken::from_base64("not valid base64!!!"), Err(AuthError::DecodingError(_))));
        assert!(matches!(CapabilityToken::decode(&[0xff, 0x00, 0x01]), Err(AuthError::DecodingError(_))));
        let oversized_bytes = vec![0u8; usize::try_from(MAX_TOKEN_SIZE).unwrap_or(4096).saturating_add(1)];
        assert!(matches!(CapabilityToken::decode(&oversized_bytes), Err(AuthError::TokenTooLarge { .. })));
    }
}
