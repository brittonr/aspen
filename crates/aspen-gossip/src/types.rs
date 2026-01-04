//! Types and message structures for gossip peer discovery.

use anyhow::Context;
use anyhow::Result;
use aspen_raft_types::NodeId;
use aspen_sharding::TopologyAnnouncement;
use iroh::EndpointAddr;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::GOSSIP_MESSAGE_VERSION;

/// Announcement message broadcast to the gossip topic.
///
/// Contains node's ID, EndpointAddr, and a timestamp for freshness tracking.
///
/// Tiger Style: Fixed-size payload, explicit timestamp in microseconds, versioned for forward
/// compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAnnouncement {
    /// Protocol version for forward compatibility checking.
    pub version: u8,
    /// Node ID of the announcing node.
    pub node_id: NodeId,
    /// The endpoint address of the announcing node.
    pub endpoint_addr: EndpointAddr,
    /// Timestamp when this announcement was created (microseconds since UNIX epoch).
    pub timestamp_micros: u64,
}

impl PeerAnnouncement {
    /// Create a new announcement with the current timestamp.
    pub fn new(node_id: NodeId, endpoint_addr: EndpointAddr) -> Result<Self> {
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: GOSSIP_MESSAGE_VERSION,
            node_id,
            endpoint_addr,
            timestamp_micros,
        })
    }

    /// Serialize to bytes using postcard.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize peer announcement")
    }
}

/// Signed peer announcement for cryptographic verification.
///
/// Wraps a `PeerAnnouncement` with an Ed25519 signature from the sender's
/// Iroh secret key. Recipients verify using the public key embedded in
/// the announcement's `endpoint_addr.id`.
///
/// Tiger Style: Fixed 64-byte signature, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPeerAnnouncement {
    /// The announcement payload (node_id, endpoint_addr, timestamp).
    pub announcement: PeerAnnouncement,
    /// Ed25519 signature over the serialized announcement (64 bytes).
    pub signature: Signature,
}

impl SignedPeerAnnouncement {
    /// Create a signed announcement.
    ///
    /// Signs the serialized `PeerAnnouncement` bytes with the provided secret key.
    pub fn sign(announcement: PeerAnnouncement, secret_key: &SecretKey) -> Result<Self> {
        let announcement_bytes = announcement.to_bytes()?;
        let signature = secret_key.sign(&announcement_bytes);

        Ok(Self {
            announcement,
            signature,
        })
    }

    /// Verify the signature and return the inner announcement if valid.
    ///
    /// Extracts the public key from `announcement.endpoint_addr.id` and verifies
    /// that the signature was created by the corresponding secret key.
    ///
    /// Returns `None` if:
    /// - Signature verification fails (tampered or wrong key)
    /// - Announcement deserialization fails
    pub fn verify(&self) -> Option<&PeerAnnouncement> {
        // Re-serialize announcement to get canonical bytes for verification
        let announcement_bytes = self.announcement.to_bytes().ok()?;

        // The endpoint_addr.id IS the PublicKey
        let public_key = self.announcement.endpoint_addr.id;

        // Verify signature
        match public_key.verify(&announcement_bytes, &self.signature) {
            Ok(()) => Some(&self.announcement),
            Err(_) => None,
        }
    }

    /// Serialize to bytes using postcard.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize signed peer announcement")
    }

    /// Deserialize from bytes using postcard.
    ///
    /// Returns None for unknown future versions to allow graceful handling.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let signed: Self = postcard::from_bytes(bytes).ok()?;

        // Reject unknown future versions
        if signed.announcement.version > GOSSIP_MESSAGE_VERSION {
            return None;
        }

        Some(signed)
    }
}

/// Signed topology announcement for cryptographic verification.
///
/// Wraps a `TopologyAnnouncement` with an Ed25519 signature from the sender's
/// Iroh secret key. Used to broadcast topology version updates so nodes can
/// detect when they have stale topology information.
///
/// Tiger Style: Fixed 64-byte signature, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTopologyAnnouncement {
    /// The topology announcement payload.
    pub announcement: TopologyAnnouncement,
    /// Ed25519 signature over the serialized announcement (64 bytes).
    pub signature: Signature,
    /// Public key of the signing node (for verification without endpoint_addr).
    pub public_key: PublicKey,
}

impl SignedTopologyAnnouncement {
    /// Create a signed topology announcement.
    pub fn sign(announcement: TopologyAnnouncement, secret_key: &SecretKey) -> Result<Self> {
        let announcement_bytes =
            postcard::to_stdvec(&announcement).context("failed to serialize topology announcement")?;
        let signature = secret_key.sign(&announcement_bytes);
        let public_key = secret_key.public();

        Ok(Self {
            announcement,
            signature,
            public_key,
        })
    }

    /// Verify the signature and return the inner announcement if valid.
    pub fn verify(&self) -> Option<&TopologyAnnouncement> {
        let announcement_bytes = postcard::to_stdvec(&self.announcement).ok()?;

        match self.public_key.verify(&announcement_bytes, &self.signature) {
            Ok(()) => Some(&self.announcement),
            Err(_) => None,
        }
    }

    /// Serialize to bytes using postcard.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize signed topology announcement")
    }

    /// Deserialize from bytes using postcard.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let signed: Self = postcard::from_bytes(bytes).ok()?;

        // Reject unknown future versions
        if signed.announcement.version > TopologyAnnouncement::PROTOCOL_VERSION {
            return None;
        }

        Some(signed)
    }
}

/// Blob announcement for P2P content seeding.
///
/// Announces that a node has a specific blob available for download.
/// Recipients can use this information to fetch the blob for redundancy
/// or when they need the content.
///
/// Tiger Style: Fixed-size payload, explicit timestamp, versioned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobAnnouncement {
    /// Protocol version for forward compatibility checking.
    pub version: u8,
    /// Node ID of the node offering this blob.
    pub node_id: NodeId,
    /// Endpoint address for downloading the blob.
    pub endpoint_addr: EndpointAddr,
    /// BLAKE3 hash of the blob content.
    pub blob_hash: iroh_blobs::Hash,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Format of the blob (Raw or HashSeq).
    pub blob_format: iroh_blobs::BlobFormat,
    /// Timestamp when this announcement was created (microseconds since UNIX epoch).
    pub timestamp_micros: u64,
    /// Optional tag for categorization (e.g., "kv-offload", "user-upload").
    /// Max 64 bytes to limit payload size.
    pub tag: Option<String>,
}

impl BlobAnnouncement {
    /// Maximum tag length in bytes.
    const MAX_TAG_LEN: usize = 64;

    /// Create a new blob announcement with the current timestamp.
    pub fn new(
        node_id: NodeId,
        endpoint_addr: EndpointAddr,
        blob_hash: iroh_blobs::Hash,
        blob_size: u64,
        blob_format: iroh_blobs::BlobFormat,
        tag: Option<String>,
    ) -> Result<Self> {
        // Validate tag length (Tiger Style: bounded strings)
        if let Some(ref t) = tag
            && t.len() > Self::MAX_TAG_LEN
        {
            anyhow::bail!("blob announcement tag too long: {} > {}", t.len(), Self::MAX_TAG_LEN);
        }

        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: GOSSIP_MESSAGE_VERSION,
            node_id,
            endpoint_addr,
            blob_hash,
            blob_size,
            blob_format,
            timestamp_micros,
            tag,
        })
    }

    /// Serialize to bytes using postcard.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize blob announcement")
    }
}

/// Signed blob announcement for cryptographic verification.
///
/// Wraps a `BlobAnnouncement` with an Ed25519 signature from the sender's
/// Iroh secret key. Recipients verify using the public key embedded in
/// the announcement's `endpoint_addr.id`.
///
/// Tiger Style: Fixed 64-byte signature, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedBlobAnnouncement {
    /// The blob announcement payload.
    pub announcement: BlobAnnouncement,
    /// Ed25519 signature over the serialized announcement (64 bytes).
    pub signature: Signature,
}

impl SignedBlobAnnouncement {
    /// Create a signed blob announcement.
    pub fn sign(announcement: BlobAnnouncement, secret_key: &SecretKey) -> Result<Self> {
        let announcement_bytes = announcement.to_bytes()?;
        let signature = secret_key.sign(&announcement_bytes);

        Ok(Self {
            announcement,
            signature,
        })
    }

    /// Verify the signature and return the inner announcement if valid.
    pub fn verify(&self) -> Option<&BlobAnnouncement> {
        let announcement_bytes = self.announcement.to_bytes().ok()?;
        let public_key = self.announcement.endpoint_addr.id;

        match public_key.verify(&announcement_bytes, &self.signature) {
            Ok(()) => Some(&self.announcement),
            Err(_) => None,
        }
    }
}

/// Envelope for gossip messages supporting multiple message types.
///
/// This allows peer announcements, topology announcements, and blob announcements
/// to share the same gossip topic while being distinguishable by type.
///
/// Tiger Style: Versioned enum for forward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum GossipMessage {
    /// Peer endpoint address announcement.
    PeerAnnouncement(SignedPeerAnnouncement),
    /// Topology version announcement (for cache invalidation).
    TopologyAnnouncement(SignedTopologyAnnouncement),
    /// Blob availability announcement for P2P content seeding.
    BlobAnnouncement(SignedBlobAnnouncement),
}

impl GossipMessage {
    /// Serialize to bytes using postcard.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize gossip message")
    }

    /// Deserialize from bytes using postcard.
    ///
    /// Falls back to parsing as legacy SignedPeerAnnouncement for backwards compatibility.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Try new envelope format first
        if let Ok(msg) = postcard::from_bytes::<Self>(bytes) {
            return Some(msg);
        }

        // Fall back to legacy SignedPeerAnnouncement format for backwards compat
        if let Some(signed) = SignedPeerAnnouncement::from_bytes(bytes) {
            return Some(GossipMessage::PeerAnnouncement(signed));
        }

        None
    }
}
