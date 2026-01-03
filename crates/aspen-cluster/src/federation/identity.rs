//! Cluster identity for federation.
//!
//! Each federated cluster has a stable Ed25519 identity that persists across
//! node membership changes. This identity is used to:
//!
//! - Sign federation announcements (proving cluster ownership)
//! - Establish trust relationships between clusters
//! - Create globally unique resource identifiers (origin prefix)
//!
//! # Key Management
//!
//! The cluster keypair should be:
//! - Generated once when the cluster is first created
//! - Stored securely (config file or secrets manager)
//! - Shared among all nodes in the cluster (via config)
//! - Never rotated without careful migration planning
//!
//! Unlike node keys (which are per-node), the cluster key represents the
//! organizational identity and should be treated as a long-lived credential.

use iroh::PublicKey;
use iroh::SecretKey;
use serde::Deserialize;
use serde::Serialize;

use aspen_core::Signature;

/// Tiger Style: Maximum length for cluster name.
pub const MAX_CLUSTER_NAME_LEN: usize = 128;

/// Tiger Style: Maximum length for cluster description.
pub const MAX_CLUSTER_DESCRIPTION_LEN: usize = 1024;

/// Cluster identity for federation purposes.
///
/// A cluster's identity consists of an Ed25519 keypair and metadata.
/// The public key serves as the cluster's globally unique identifier
/// and is used as the "origin" in federated resource IDs.
///
/// # Example
///
/// ```ignore
/// use aspen::cluster::federation::ClusterIdentity;
///
/// // Generate a new identity
/// let identity = ClusterIdentity::generate("my-organization".to_string());
///
/// // Or load from existing key
/// let secret_key = SecretKey::from_bytes(&key_bytes);
/// let identity = ClusterIdentity::from_secret_key(
///     secret_key,
///     "my-organization".to_string(),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct ClusterIdentity {
    /// The cluster's secret key (used for signing).
    secret_key: SecretKey,

    /// The cluster's public key (globally unique identifier).
    public_key: PublicKey,

    /// Human-readable cluster name.
    ///
    /// Tiger Style: Bounded to MAX_CLUSTER_NAME_LEN characters.
    name: String,

    /// Optional description of the cluster.
    ///
    /// Tiger Style: Bounded to MAX_CLUSTER_DESCRIPTION_LEN characters.
    description: Option<String>,

    /// Timestamp when this identity was created (milliseconds since epoch).
    created_at_ms: u64,
}

impl ClusterIdentity {
    /// Generate a new cluster identity with a random keypair.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable cluster name (truncated if too long)
    ///
    /// # Returns
    ///
    /// A new `ClusterIdentity` with a freshly generated Ed25519 keypair.
    pub fn generate(name: String) -> Self {
        let secret_key = SecretKey::generate(&mut rand::rng());
        Self::from_secret_key(secret_key, name)
    }

    /// Create a cluster identity from an existing secret key.
    ///
    /// Use this when loading a previously generated cluster key from config.
    ///
    /// # Arguments
    ///
    /// * `secret_key` - The cluster's Ed25519 secret key
    /// * `name` - Human-readable cluster name (truncated if too long)
    pub fn from_secret_key(secret_key: SecretKey, name: String) -> Self {
        let public_key = secret_key.public();

        // Tiger Style: Truncate name if too long
        let name = if name.len() > MAX_CLUSTER_NAME_LEN {
            name[..MAX_CLUSTER_NAME_LEN].to_string()
        } else {
            name
        };

        let created_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            secret_key,
            public_key,
            name,
            description: None,
            created_at_ms,
        }
    }

    /// Set the cluster description.
    ///
    /// Tiger Style: Truncated if too long.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(if description.len() > MAX_CLUSTER_DESCRIPTION_LEN {
            description[..MAX_CLUSTER_DESCRIPTION_LEN].to_string()
        } else {
            description
        });
        self
    }

    /// Set the creation timestamp (for loading from storage).
    pub fn with_created_at(mut self, created_at_ms: u64) -> Self {
        self.created_at_ms = created_at_ms;
        self
    }

    /// Get the cluster's public key.
    ///
    /// This is the globally unique identifier for this cluster and is used
    /// as the "origin" prefix in federated resource IDs.
    pub fn public_key(&self) -> PublicKey {
        self.public_key
    }

    /// Get the cluster's secret key.
    ///
    /// Used for signing federation announcements. Handle with care.
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// Get the cluster name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the cluster description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the creation timestamp.
    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    /// Sign a message with the cluster's secret key.
    ///
    /// Returns a signature that can be verified with `public_key()`.
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.secret_key.sign(message);
        Signature(sig.to_bytes())
    }

    /// Create a signed identity document for sharing with other clusters.
    pub fn to_signed(&self) -> SignedClusterIdentity {
        let identity_doc = ClusterIdentityDocument {
            public_key: self.public_key,
            name: self.name.clone(),
            description: self.description.clone(),
            created_at_ms: self.created_at_ms,
        };

        let doc_bytes = postcard::to_allocvec(&identity_doc).expect("serialization should not fail");
        let signature = self.sign(&doc_bytes);

        SignedClusterIdentity {
            identity: identity_doc,
            signature,
        }
    }

    /// Get the secret key bytes for serialization.
    ///
    /// Use this when saving the cluster identity to config.
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.secret_key.to_bytes()
    }

    /// Create identity from secret key bytes (for loading from config).
    pub fn from_secret_key_bytes(bytes: [u8; 32], name: String) -> Self {
        let secret_key = SecretKey::from(bytes);
        Self::from_secret_key(secret_key, name)
    }

    /// Create identity from hex-encoded secret key string.
    ///
    /// The hex string should be 64 characters (32 bytes).
    ///
    /// # Errors
    ///
    /// Returns an error if the hex string is invalid or the wrong length.
    pub fn from_hex_key(hex: &str, name: String) -> Result<Self, &'static str> {
        if hex.len() != 64 {
            return Err("cluster key must be exactly 64 hex characters (32 bytes)");
        }

        let mut bytes = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let s = std::str::from_utf8(chunk).map_err(|_| "invalid UTF-8 in hex string")?;
            bytes[i] = u8::from_str_radix(s, 16).map_err(|_| "invalid hex character")?;
        }

        Ok(Self::from_secret_key_bytes(bytes, name))
    }

    /// Export the secret key as a hex string.
    ///
    /// Use this when saving the cluster key to config.
    pub fn secret_key_hex(&self) -> String {
        hex::encode(self.secret_key.to_bytes())
    }
}

/// Serializable cluster identity document (without secret key).
///
/// This is what gets shared with other clusters and signed to prove ownership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterIdentityDocument {
    /// The cluster's public key.
    pub public_key: PublicKey,

    /// Human-readable cluster name.
    pub name: String,

    /// Optional description.
    pub description: Option<String>,

    /// Creation timestamp.
    pub created_at_ms: u64,
}

/// A signed cluster identity for sharing with other clusters.
///
/// This proves that the identity document was created by the holder
/// of the corresponding secret key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedClusterIdentity {
    /// The identity document.
    pub identity: ClusterIdentityDocument,

    /// Signature over the serialized identity document.
    pub signature: Signature,
}

impl SignedClusterIdentity {
    /// Verify the signature on this identity document.
    ///
    /// Returns `true` if the signature is valid, `false` otherwise.
    pub fn verify(&self) -> bool {
        let doc_bytes = match postcard::to_allocvec(&self.identity) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let sig_bytes: [u8; 64] = match self.signature.0.try_into() {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        let sig = iroh::Signature::from_bytes(&sig_bytes);

        self.identity.public_key.verify(&doc_bytes, &sig).is_ok()
    }

    /// Get the public key from this identity.
    pub fn public_key(&self) -> PublicKey {
        self.identity.public_key
    }

    /// Get the cluster name.
    pub fn name(&self) -> &str {
        &self.identity.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_identity_generation() {
        let identity = ClusterIdentity::generate("test-cluster".to_string());

        assert_eq!(identity.name(), "test-cluster");
        assert!(identity.description().is_none());
        assert!(identity.created_at_ms() > 0);
    }

    #[test]
    fn test_cluster_identity_with_description() {
        let identity = ClusterIdentity::generate("test-cluster".to_string())
            .with_description("A test cluster for unit tests".to_string());

        assert_eq!(identity.description(), Some("A test cluster for unit tests"));
    }

    #[test]
    fn test_cluster_identity_signing() {
        let identity = ClusterIdentity::generate("test-cluster".to_string());
        let message = b"hello, federation!";

        let signature = identity.sign(message);

        // Verify the signature
        let sig_bytes: [u8; 64] = signature.0.try_into().expect("signature should be 64 bytes");
        let sig = iroh::Signature::from_bytes(&sig_bytes);
        assert!(identity.public_key().verify(message, &sig).is_ok());
    }

    #[test]
    fn test_signed_identity_verification() {
        let identity =
            ClusterIdentity::generate("test-cluster".to_string()).with_description("Test description".to_string());

        let signed = identity.to_signed();

        assert!(signed.verify());
        assert_eq!(signed.name(), "test-cluster");
        assert_eq!(signed.public_key(), identity.public_key());
    }

    #[test]
    fn test_signed_identity_tamper_detection() {
        let identity = ClusterIdentity::generate("test-cluster".to_string());
        let mut signed = identity.to_signed();

        // Tamper with the name
        signed.identity.name = "tampered-name".to_string();

        // Verification should fail
        assert!(!signed.verify());
    }

    #[test]
    fn test_name_truncation() {
        let long_name = "a".repeat(200);
        let identity = ClusterIdentity::generate(long_name);

        assert_eq!(identity.name().len(), MAX_CLUSTER_NAME_LEN);
    }

    #[test]
    fn test_description_truncation() {
        let long_desc = "b".repeat(2000);
        let identity = ClusterIdentity::generate("test".to_string()).with_description(long_desc);

        assert_eq!(identity.description().unwrap().len(), MAX_CLUSTER_DESCRIPTION_LEN);
    }

    #[test]
    fn test_secret_key_roundtrip() {
        let original = ClusterIdentity::generate("test-cluster".to_string());
        let key_bytes = original.secret_key_bytes();

        let restored = ClusterIdentity::from_secret_key_bytes(key_bytes, "test-cluster".to_string());

        assert_eq!(original.public_key(), restored.public_key());
    }
}
