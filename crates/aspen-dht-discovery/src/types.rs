//! Data types for DHT content discovery.

use anyhow::Context;
use anyhow::Result;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;
use serde::Deserialize;
use serde::Serialize;

/// Information about a content provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Public key of the provider node.
    pub node_id: PublicKey,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Format of the blob (Raw or HashSeq).
    pub blob_format: BlobFormat,
    /// Timestamp when this provider was discovered (microseconds since epoch).
    pub discovered_at: u64,
    /// Whether this provider has been verified (we successfully connected).
    pub is_verified: bool,
}

/// Announce record stored in the DHT.
///
/// This is the payload signed by the announcing node and stored
/// in the DHT using BEP-0044 mutable items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DhtAnnounce {
    /// Protocol version for forward compatibility.
    pub(crate) version: u8,
    /// BLAKE3 hash of the content.
    pub(crate) blob_hash: Hash,
    /// Size of the blob.
    pub(crate) blob_size: u64,
    /// Format of the blob.
    pub(crate) blob_format: BlobFormat,
    /// Timestamp (microseconds since epoch).
    pub(crate) timestamp_micros: u64,
}

impl DhtAnnounce {
    pub(crate) const VERSION: u8 = 1;

    pub(crate) fn new(blob_hash: Hash, blob_size: u64, blob_format: BlobFormat) -> Result<Self> {
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: Self::VERSION,
            blob_hash,
            blob_size,
            blob_format,
            timestamp_micros,
        })
    }

    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize DHT announce")
    }

    #[allow(dead_code)]
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes)
            .inspect_err(|e| tracing::debug!("failed to deserialize DHT announce: {e}"))
            .ok()
    }
}

/// Node address information stored in DHT via BEP-44 mutable items.
///
/// This structure contains the full information needed to connect to an iroh node.
/// It's stored as a BEP-44 mutable item with:
/// - Key: The node's ed25519 public key (same as iroh PublicKey)
/// - Salt: The 20-byte infohash of the content
/// - Value: Serialized DhtNodeAddr
///
/// This solves the 20-byte vs 32-byte peer ID problem by using the DHT's
/// mutable item storage instead of the limited peer announcement protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtNodeAddr {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// The node's iroh public key (32 bytes).
    pub public_key: [u8; 32],
    /// Optional relay URL for NAT traversal.
    pub relay_url: Option<String>,
    /// Direct socket addresses (IPv4 and IPv6).
    pub direct_addrs: Vec<String>,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Format of the blob.
    pub blob_format: BlobFormat,
    /// Timestamp (microseconds since epoch).
    pub timestamp_micros: u64,
}

impl DhtNodeAddr {
    const VERSION: u8 = 1;

    /// Create a new DhtNodeAddr from the current endpoint state.
    pub fn new(
        public_key: PublicKey,
        relay_url: Option<&url::Url>,
        direct_addrs: impl IntoIterator<Item = std::net::SocketAddr>,
        blob_size: u64,
        blob_format: BlobFormat,
    ) -> Result<Self> {
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: Self::VERSION,
            public_key: *public_key.as_bytes(),
            relay_url: relay_url.map(|u| u.to_string()),
            direct_addrs: direct_addrs.into_iter().map(|a| a.to_string()).collect(),
            blob_size,
            blob_format,
            timestamp_micros,
        })
    }

    /// Serialize to bytes for DHT storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize DhtNodeAddr")
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes)
            .inspect_err(|e| tracing::debug!("failed to deserialize DhtNodeAddr: {e}"))
            .ok()
    }

    /// Get the iroh PublicKey.
    pub fn iroh_public_key(&self) -> Option<PublicKey> {
        PublicKey::from_bytes(&self.public_key)
            .inspect_err(|e| tracing::debug!("failed to parse iroh public key: {e}"))
            .ok()
    }
}

/// Signed announce record for DHT storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SignedDhtAnnounce {
    /// The announce payload.
    pub(crate) announce: DhtAnnounce,
    /// Public key of the announcing node.
    pub(crate) public_key: PublicKey,
    /// Ed25519 signature over the serialized announce.
    pub(crate) signature: Signature,
}

impl SignedDhtAnnounce {
    pub(crate) fn sign(announce: DhtAnnounce, secret_key: &SecretKey) -> Result<Self> {
        let announce_bytes = announce.to_bytes()?;
        let signature = secret_key.sign(&announce_bytes);
        let public_key = secret_key.public();

        Ok(Self {
            announce,
            public_key,
            signature,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn verify(&self) -> Option<&DhtAnnounce> {
        let announce_bytes = self
            .announce
            .to_bytes()
            .inspect_err(|e| tracing::debug!("failed to serialize announce for verification: {e}"))
            .ok()?;
        self.public_key
            .verify(&announce_bytes, &self.signature)
            .inspect_err(|e| tracing::debug!("DHT announce signature verification failed: {e}"))
            .ok()?;
        Some(&self.announce)
    }

    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize signed DHT announce")
    }

    #[allow(dead_code)]
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }
}
