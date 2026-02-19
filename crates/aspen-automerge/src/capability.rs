//! Capability-based authorization for automerge sync.
//!
//! A [`SyncCapability`] is a signed token granting a specific peer (or bearer)
//! permission to sync specific documents. The token is signed by the cluster
//! node's ed25519 key and verified on every sync request.
//!
//! # Design
//!
//! - **Capability = authorization**: possession of a valid, signed token is sufficient proof of
//!   access. No server-side ACL lookup needed.
//! - **Bound to peer**: tokens optionally bind to a specific node ID, so a stolen token can't be
//!   used from a different endpoint.
//! - **Document scoping**: tokens grant access to a specific document ID or to all documents
//!   (wildcard `*`).
//! - **Expiry**: tokens can optionally expire.
//! - **Signed by cluster key**: the cluster's iroh SecretKey signs the token; verification uses the
//!   corresponding PublicKey.

use iroh_base::PublicKey;
use iroh_base::SecretKey;
use iroh_base::Signature;
use serde::Deserialize;
use serde::Serialize;

/// Permissions granted by a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    /// Can pull document state (receive changes).
    Read,
    /// Can push document state (send changes).
    Write,
    /// Can both push and pull.
    ReadWrite,
}

impl Permission {
    /// Whether this permission allows reading (pulling state).
    pub fn can_read(self) -> bool {
        matches!(self, Permission::Read | Permission::ReadWrite)
    }

    /// Whether this permission allows writing (pushing state).
    pub fn can_write(self) -> bool {
        matches!(self, Permission::Write | Permission::ReadWrite)
    }
}

/// The unsigned capability claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCapability {
    /// Document ID this capability grants access to. `"*"` means all documents.
    pub document_id: String,
    /// What the holder can do.
    pub permission: Permission,
    /// If set, the capability is only valid for this specific peer.
    /// If `None`, the token is a bearer token (anyone holding it can use it).
    pub node_id: Option<[u8; 32]>,
    /// Unix timestamp (seconds) after which this capability is no longer valid.
    /// `None` means no expiry.
    pub expires_at: Option<u64>,
}

/// A capability token: the serialized claim + its ed25519 signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCapability {
    /// The serialized (postcard) capability claim.
    pub capability_bytes: Vec<u8>,
    /// ed25519 signature over `capability_bytes`, produced by the issuer's secret key.
    pub signature: Signature,
}

impl SignedCapability {
    /// Sign a capability with the given secret key.
    pub fn sign(secret_key: &SecretKey, capability: &SyncCapability) -> Self {
        let capability_bytes = postcard::to_stdvec(capability).expect("SyncCapability serialization cannot fail");
        let signature = secret_key.sign(&capability_bytes);
        Self {
            capability_bytes,
            signature,
        }
    }

    /// Verify this token against the issuer's public key.
    ///
    /// On success, returns the deserialized capability claim.
    /// On failure, returns a description of what went wrong.
    pub fn verify(&self, issuer: &PublicKey) -> Result<SyncCapability, CapabilityError> {
        // Verify signature
        issuer
            .verify(&self.capability_bytes, &self.signature)
            .map_err(|_| CapabilityError::InvalidSignature)?;

        // Deserialize the claim
        let cap: SyncCapability =
            postcard::from_bytes(&self.capability_bytes).map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;

        // Check expiry
        if let Some(expires_at) = cap.expires_at {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            if now > expires_at {
                return Err(CapabilityError::Expired);
            }
        }

        Ok(cap)
    }

    /// Verify this token and check that it authorizes a specific request.
    pub fn authorize(
        &self,
        issuer: &PublicKey,
        document_id: &str,
        peer_node_id: &PublicKey,
        needs_write: bool,
    ) -> Result<(), CapabilityError> {
        let cap = self.verify(issuer)?;

        // Check document scope
        if cap.document_id != "*" && cap.document_id != document_id {
            return Err(CapabilityError::WrongDocument {
                granted: cap.document_id,
                requested: document_id.to_string(),
            });
        }

        // Check peer binding
        if let Some(bound_node) = cap.node_id
            && bound_node != peer_node_id.as_bytes()[..]
        {
            return Err(CapabilityError::WrongPeer);
        }

        // Check permission
        if needs_write && !cap.permission.can_write() {
            return Err(CapabilityError::InsufficientPermission);
        }
        if !needs_write && !cap.permission.can_read() {
            return Err(CapabilityError::InsufficientPermission);
        }

        Ok(())
    }

    /// Encode to bytes for embedding in tickets or protocol messages.
    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("SignedCapability serialization cannot fail")
    }

    /// Decode from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CapabilityError> {
        postcard::from_bytes(bytes).map_err(|e| CapabilityError::InvalidPayload(e.to_string()))
    }
}

/// Errors during capability verification.
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("invalid signature")]
    InvalidSignature,

    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("capability expired")]
    Expired,

    #[error("capability is for document '{granted}', not '{requested}'")]
    WrongDocument { granted: String, requested: String },

    #[error("capability is bound to a different peer")]
    WrongPeer,

    #[error("insufficient permission")]
    InsufficientPermission,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keys() -> (SecretKey, PublicKey) {
        let secret = SecretKey::generate(&mut rand::rngs::ThreadRng::default());
        let public = secret.public();
        (secret, public)
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let (secret, public) = test_keys();
        let cap = SyncCapability {
            document_id: "test-doc".into(),
            permission: Permission::ReadWrite,
            node_id: None,
            expires_at: None,
        };

        let signed = SignedCapability::sign(&secret, &cap);
        let verified = signed.verify(&public).unwrap();
        assert_eq!(verified.document_id, "test-doc");
        assert_eq!(verified.permission, Permission::ReadWrite);
    }

    #[test]
    fn wrong_key_rejected() {
        let (secret, _) = test_keys();
        let (_, wrong_public) = test_keys();
        let cap = SyncCapability {
            document_id: "doc".into(),
            permission: Permission::Read,
            node_id: None,
            expires_at: None,
        };

        let signed = SignedCapability::sign(&secret, &cap);
        assert!(signed.verify(&wrong_public).is_err());
    }

    #[test]
    fn expired_token_rejected() {
        let (secret, public) = test_keys();
        let cap = SyncCapability {
            document_id: "doc".into(),
            permission: Permission::Read,
            node_id: None,
            expires_at: Some(0), // expired in 1970
        };

        let signed = SignedCapability::sign(&secret, &cap);
        assert!(matches!(signed.verify(&public), Err(CapabilityError::Expired)));
    }

    #[test]
    fn wildcard_document_allows_any() {
        let (secret, public) = test_keys();
        let peer = SecretKey::generate(&mut rand::rngs::ThreadRng::default()).public();
        let cap = SyncCapability {
            document_id: "*".into(),
            permission: Permission::ReadWrite,
            node_id: None,
            expires_at: None,
        };

        let signed = SignedCapability::sign(&secret, &cap);
        assert!(signed.authorize(&public, "any-doc-id", &peer, true).is_ok());
    }

    #[test]
    fn wrong_document_rejected() {
        let (secret, public) = test_keys();
        let peer = SecretKey::generate(&mut rand::rngs::ThreadRng::default()).public();
        let cap = SyncCapability {
            document_id: "doc-a".into(),
            permission: Permission::ReadWrite,
            node_id: None,
            expires_at: None,
        };

        let signed = SignedCapability::sign(&secret, &cap);
        assert!(matches!(
            signed.authorize(&public, "doc-b", &peer, false),
            Err(CapabilityError::WrongDocument { .. })
        ));
    }

    #[test]
    fn bound_to_peer_rejects_others() {
        let (secret, public) = test_keys();
        let allowed = SecretKey::generate(&mut rand::rngs::ThreadRng::default()).public();
        let intruder = SecretKey::generate(&mut rand::rngs::ThreadRng::default()).public();
        let cap = SyncCapability {
            document_id: "*".into(),
            permission: Permission::ReadWrite,
            node_id: Some(*allowed.as_bytes()),
            expires_at: None,
        };

        let signed = SignedCapability::sign(&secret, &cap);
        assert!(signed.authorize(&public, "doc", &allowed, true).is_ok());
        assert!(matches!(signed.authorize(&public, "doc", &intruder, true), Err(CapabilityError::WrongPeer)));
    }

    #[test]
    fn read_only_rejects_write() {
        let (secret, public) = test_keys();
        let peer = SecretKey::generate(&mut rand::rngs::ThreadRng::default()).public();
        let cap = SyncCapability {
            document_id: "*".into(),
            permission: Permission::Read,
            node_id: None,
            expires_at: None,
        };

        let signed = SignedCapability::sign(&secret, &cap);
        assert!(signed.authorize(&public, "doc", &peer, false).is_ok());
        assert!(matches!(
            signed.authorize(&public, "doc", &peer, true),
            Err(CapabilityError::InsufficientPermission)
        ));
    }

    #[test]
    fn serialize_roundtrip() {
        let (secret, public) = test_keys();
        let cap = SyncCapability {
            document_id: "doc".into(),
            permission: Permission::Write,
            node_id: None,
            expires_at: Some(9999999999),
        };

        let signed = SignedCapability::sign(&secret, &cap);
        let bytes = signed.to_bytes();
        let restored = SignedCapability::from_bytes(&bytes).unwrap();
        let verified = restored.verify(&public).unwrap();
        assert_eq!(verified.document_id, "doc");
        assert_eq!(verified.permission, Permission::Write);
    }
}
