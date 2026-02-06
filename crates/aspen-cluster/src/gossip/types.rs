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

use super::constants::GOSSIP_MESSAGE_VERSION;
use super::constants::MAX_GOSSIP_MESSAGE_SIZE;

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
    /// Returns None for:
    /// - Messages exceeding MAX_GOSSIP_MESSAGE_SIZE (DoS prevention)
    /// - Unknown future versions (forward compatibility)
    /// - Malformed postcard data
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Tiger Style: Check size BEFORE deserialization to prevent memory exhaustion
        if bytes.len() > MAX_GOSSIP_MESSAGE_SIZE {
            return None;
        }

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
    ///
    /// Returns None for:
    /// - Messages exceeding MAX_GOSSIP_MESSAGE_SIZE (DoS prevention)
    /// - Unknown future versions (forward compatibility)
    /// - Malformed postcard data
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Tiger Style: Check size BEFORE deserialization to prevent memory exhaustion
        if bytes.len() > MAX_GOSSIP_MESSAGE_SIZE {
            return None;
        }

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
    /// Returns None for:
    /// - Messages exceeding MAX_GOSSIP_MESSAGE_SIZE (DoS prevention)
    /// - Malformed postcard data
    ///
    /// Falls back to parsing as legacy SignedPeerAnnouncement for backwards compatibility.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Tiger Style: Check size BEFORE deserialization to prevent memory exhaustion
        if bytes.len() > MAX_GOSSIP_MESSAGE_SIZE {
            return None;
        }

        // Try new envelope format first
        if let Ok(msg) = postcard::from_bytes::<Self>(bytes) {
            return Some(msg);
        }

        // Fall back to legacy SignedPeerAnnouncement format for backwards compat
        // Note: SignedPeerAnnouncement::from_bytes also checks size, but we've already checked above
        if let Some(signed) = SignedPeerAnnouncement::from_bytes(bytes) {
            return Some(GossipMessage::PeerAnnouncement(signed));
        }

        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use iroh_blobs::BlobFormat;

    use super::*;

    /// Create a deterministic secret key from a seed for reproducible tests.
    fn secret_key_from_seed(seed: u64) -> SecretKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
        SecretKey::from_bytes(&key_bytes)
    }

    /// Create a mock EndpointAddr from a secret key.
    fn endpoint_addr_from_secret_key(secret_key: &SecretKey) -> EndpointAddr {
        EndpointAddr::new(secret_key.public())
    }

    // =========================================================================
    // PeerAnnouncement Tests
    // =========================================================================

    #[test]
    fn test_peer_announcement_new_sets_version() {
        let secret_key = secret_key_from_seed(1);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(1u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();

        assert_eq!(announcement.version, GOSSIP_MESSAGE_VERSION);
        assert_eq!(announcement.node_id, node_id);
        assert!(announcement.timestamp_micros > 0);
    }

    #[test]
    fn test_peer_announcement_to_bytes_roundtrip() {
        let secret_key = secret_key_from_seed(2);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(42u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr.clone()).unwrap();
        let bytes = announcement.to_bytes().unwrap();

        // Deserialize and verify
        let decoded: PeerAnnouncement = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.version, announcement.version);
        assert_eq!(decoded.node_id, announcement.node_id);
        assert_eq!(decoded.endpoint_addr.id, announcement.endpoint_addr.id);
        assert_eq!(decoded.timestamp_micros, announcement.timestamp_micros);
    }

    #[test]
    fn test_peer_announcement_timestamp_is_current() {
        let secret_key = secret_key_from_seed(3);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let before = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;

        let announcement = PeerAnnouncement::new(NodeId::from(1u64), endpoint_addr).unwrap();

        let after = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;

        assert!(announcement.timestamp_micros >= before);
        assert!(announcement.timestamp_micros <= after);
    }

    // =========================================================================
    // SignedPeerAnnouncement Tests
    // =========================================================================

    #[test]
    fn test_signed_peer_announcement_sign_and_verify() {
        let secret_key = secret_key_from_seed(10);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(100u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        // Verification should succeed
        let verified = signed.verify();
        assert!(verified.is_some());
        let inner = verified.unwrap();
        assert_eq!(inner.node_id, node_id);
    }

    #[test]
    fn test_signed_peer_announcement_reject_wrong_key() {
        let secret_key1 = secret_key_from_seed(20);
        let secret_key2 = secret_key_from_seed(21);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key1);
        let node_id = NodeId::from(200u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        // Sign with a DIFFERENT key than the one in endpoint_addr
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key2).unwrap();

        // Verification should fail because public key in endpoint_addr doesn't match signer
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_peer_announcement_reject_tampered_node_id() {
        let secret_key = secret_key_from_seed(30);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(300u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        let mut signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        // Tamper with the node_id
        signed.announcement.node_id = NodeId::from(999u64);

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_peer_announcement_reject_tampered_timestamp() {
        let secret_key = secret_key_from_seed(31);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(310u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        let mut signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        // Tamper with the timestamp
        signed.announcement.timestamp_micros = 0;

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_peer_announcement_to_bytes_roundtrip() {
        let secret_key = secret_key_from_seed(40);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(400u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        let bytes = signed.to_bytes().unwrap();
        let decoded = SignedPeerAnnouncement::from_bytes(&bytes).unwrap();

        // Verify the decoded signed announcement
        assert!(decoded.verify().is_some());
        assert_eq!(decoded.announcement.node_id, node_id);
    }

    #[test]
    fn test_signed_peer_announcement_from_bytes_rejects_oversized() {
        // Create a message larger than MAX_GOSSIP_MESSAGE_SIZE
        let oversized = vec![0u8; MAX_GOSSIP_MESSAGE_SIZE + 1];

        let result = SignedPeerAnnouncement::from_bytes(&oversized);
        assert!(result.is_none());
    }

    #[test]
    fn test_signed_peer_announcement_from_bytes_rejects_future_version() {
        let secret_key = secret_key_from_seed(50);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(500u64);

        let mut announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        // Set a future version
        announcement.version = GOSSIP_MESSAGE_VERSION + 1;
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        let bytes = signed.to_bytes().unwrap();
        let result = SignedPeerAnnouncement::from_bytes(&bytes);

        // Should reject future versions
        assert!(result.is_none());
    }

    #[test]
    fn test_signed_peer_announcement_from_bytes_rejects_malformed() {
        let malformed = vec![0xFF, 0xFE, 0xFD, 0xFC]; // Random garbage bytes

        let result = SignedPeerAnnouncement::from_bytes(&malformed);
        assert!(result.is_none());
    }

    // =========================================================================
    // SignedTopologyAnnouncement Tests
    // =========================================================================

    #[test]
    fn test_signed_topology_announcement_sign_and_verify() {
        let secret_key = secret_key_from_seed(60);
        let announcement = TopologyAnnouncement {
            version: TopologyAnnouncement::PROTOCOL_VERSION,
            node_id: 600,
            topology_version: 1,
            topology_hash: 12345678,
            term: 1,
            timestamp_micros: 0,
        };

        let signed = SignedTopologyAnnouncement::sign(announcement.clone(), &secret_key).unwrap();

        let verified = signed.verify();
        assert!(verified.is_some());
        let inner = verified.unwrap();
        assert_eq!(inner.node_id, 600);
        assert_eq!(inner.topology_version, 1);
    }

    #[test]
    fn test_signed_topology_announcement_reject_tampered() {
        let secret_key = secret_key_from_seed(61);
        let announcement = TopologyAnnouncement {
            version: TopologyAnnouncement::PROTOCOL_VERSION,
            node_id: 610,
            topology_version: 2,
            topology_hash: 87654321,
            term: 1,
            timestamp_micros: 0,
        };

        let mut signed = SignedTopologyAnnouncement::sign(announcement, &secret_key).unwrap();

        // Tamper with topology version
        signed.announcement.topology_version = 999;

        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_topology_announcement_roundtrip() {
        let secret_key = secret_key_from_seed(62);
        let announcement = TopologyAnnouncement {
            version: TopologyAnnouncement::PROTOCOL_VERSION,
            node_id: 620,
            topology_version: 5,
            topology_hash: 11111111,
            term: 2,
            timestamp_micros: 1000000,
        };

        let signed = SignedTopologyAnnouncement::sign(announcement, &secret_key).unwrap();
        let bytes = signed.to_bytes().unwrap();
        let decoded = SignedTopologyAnnouncement::from_bytes(&bytes).unwrap();

        assert!(decoded.verify().is_some());
        assert_eq!(decoded.announcement.topology_version, 5);
    }

    #[test]
    fn test_signed_topology_announcement_from_bytes_rejects_oversized() {
        let oversized = vec![0u8; MAX_GOSSIP_MESSAGE_SIZE + 1];
        assert!(SignedTopologyAnnouncement::from_bytes(&oversized).is_none());
    }

    #[test]
    fn test_signed_topology_announcement_from_bytes_rejects_future_version() {
        let secret_key = secret_key_from_seed(63);
        let mut announcement = TopologyAnnouncement {
            version: TopologyAnnouncement::PROTOCOL_VERSION,
            node_id: 630,
            topology_version: 1,
            topology_hash: 0,
            term: 1,
            timestamp_micros: 0,
        };
        // Set a future version
        announcement.version = TopologyAnnouncement::PROTOCOL_VERSION + 1;

        let signed = SignedTopologyAnnouncement::sign(announcement, &secret_key).unwrap();
        let bytes = signed.to_bytes().unwrap();

        assert!(SignedTopologyAnnouncement::from_bytes(&bytes).is_none());
    }

    // =========================================================================
    // BlobAnnouncement Tests
    // =========================================================================

    #[test]
    fn test_blob_announcement_new_valid() {
        let secret_key = secret_key_from_seed(70);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(700u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0xAB; 32]);

        let announcement = BlobAnnouncement::new(
            node_id,
            endpoint_addr,
            blob_hash,
            1024,
            BlobFormat::Raw,
            Some("test-tag".to_string()),
        )
        .unwrap();

        assert_eq!(announcement.version, GOSSIP_MESSAGE_VERSION);
        assert_eq!(announcement.node_id, node_id);
        assert_eq!(announcement.blob_hash, blob_hash);
        assert_eq!(announcement.blob_size, 1024);
        assert_eq!(announcement.blob_format, BlobFormat::Raw);
        assert_eq!(announcement.tag, Some("test-tag".to_string()));
        assert!(announcement.timestamp_micros > 0);
    }

    #[test]
    fn test_blob_announcement_new_no_tag() {
        let secret_key = secret_key_from_seed(71);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(710u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0xCD; 32]);

        let announcement =
            BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, 2048, BlobFormat::HashSeq, None).unwrap();

        assert!(announcement.tag.is_none());
    }

    #[test]
    fn test_blob_announcement_tag_length_at_limit() {
        let secret_key = secret_key_from_seed(72);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(720u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0xEF; 32]);

        // Exactly 64 bytes - should succeed
        let tag = "a".repeat(BlobAnnouncement::MAX_TAG_LEN);
        let result = BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, 4096, BlobFormat::Raw, Some(tag.clone()));

        assert!(result.is_ok());
        assert_eq!(result.unwrap().tag, Some(tag));
    }

    #[test]
    fn test_blob_announcement_tag_length_exceeds_limit() {
        let secret_key = secret_key_from_seed(73);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(730u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x11; 32]);

        // 65 bytes - should fail
        let tag = "a".repeat(BlobAnnouncement::MAX_TAG_LEN + 1);
        let result = BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, 4096, BlobFormat::Raw, Some(tag));

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("tag too long"));
        assert!(err_msg.contains("65"));
        assert!(err_msg.contains("64"));
    }

    #[test]
    fn test_blob_announcement_to_bytes_roundtrip() {
        let secret_key = secret_key_from_seed(74);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(740u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x22; 32]);

        let announcement = BlobAnnouncement::new(
            node_id,
            endpoint_addr.clone(),
            blob_hash,
            8192,
            BlobFormat::HashSeq,
            Some("roundtrip".to_string()),
        )
        .unwrap();

        let bytes = announcement.to_bytes().unwrap();
        let decoded: BlobAnnouncement = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.node_id, node_id);
        assert_eq!(decoded.blob_hash, blob_hash);
        assert_eq!(decoded.blob_size, 8192);
        assert_eq!(decoded.blob_format, BlobFormat::HashSeq);
        assert_eq!(decoded.tag, Some("roundtrip".to_string()));
    }

    // =========================================================================
    // SignedBlobAnnouncement Tests
    // =========================================================================

    #[test]
    fn test_signed_blob_announcement_sign_and_verify() {
        let secret_key = secret_key_from_seed(80);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(800u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x33; 32]);

        let announcement =
            BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, 16384, BlobFormat::Raw, None).unwrap();

        let signed = SignedBlobAnnouncement::sign(announcement, &secret_key).unwrap();

        let verified = signed.verify();
        assert!(verified.is_some());
        let inner = verified.unwrap();
        assert_eq!(inner.node_id, node_id);
        assert_eq!(inner.blob_hash, blob_hash);
    }

    #[test]
    fn test_signed_blob_announcement_reject_wrong_key() {
        let secret_key1 = secret_key_from_seed(81);
        let secret_key2 = secret_key_from_seed(82);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key1);
        let node_id = NodeId::from(810u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x44; 32]);

        let announcement =
            BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, 1024, BlobFormat::Raw, None).unwrap();

        // Sign with different key than endpoint_addr
        let signed = SignedBlobAnnouncement::sign(announcement, &secret_key2).unwrap();

        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_blob_announcement_reject_tampered_hash() {
        let secret_key = secret_key_from_seed(83);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(830u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x55; 32]);

        let announcement =
            BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, 2048, BlobFormat::Raw, None).unwrap();

        let mut signed = SignedBlobAnnouncement::sign(announcement, &secret_key).unwrap();

        // Tamper with the blob hash
        signed.announcement.blob_hash = iroh_blobs::Hash::from_bytes([0xFF; 32]);

        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_blob_announcement_reject_tampered_size() {
        let secret_key = secret_key_from_seed(84);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(840u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x66; 32]);

        let announcement =
            BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, 4096, BlobFormat::Raw, None).unwrap();

        let mut signed = SignedBlobAnnouncement::sign(announcement, &secret_key).unwrap();

        // Tamper with the size
        signed.announcement.blob_size = 999999;

        assert!(signed.verify().is_none());
    }

    // =========================================================================
    // GossipMessage Tests
    // =========================================================================

    #[test]
    fn test_gossip_message_peer_announcement_roundtrip() {
        let secret_key = secret_key_from_seed(90);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(900u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();
        let message = GossipMessage::PeerAnnouncement(signed);

        let bytes = message.to_bytes().unwrap();
        let decoded = GossipMessage::from_bytes(&bytes).unwrap();

        match decoded {
            GossipMessage::PeerAnnouncement(signed) => {
                assert!(signed.verify().is_some());
                assert_eq!(signed.announcement.node_id, node_id);
            }
            _ => panic!("expected PeerAnnouncement"),
        }
    }

    #[test]
    fn test_gossip_message_topology_announcement_roundtrip() {
        let secret_key = secret_key_from_seed(91);
        let announcement = TopologyAnnouncement {
            version: TopologyAnnouncement::PROTOCOL_VERSION,
            node_id: 910,
            topology_version: 10,
            topology_hash: 123456,
            term: 3,
            timestamp_micros: 2000000,
        };

        let signed = SignedTopologyAnnouncement::sign(announcement, &secret_key).unwrap();
        let message = GossipMessage::TopologyAnnouncement(signed);

        let bytes = message.to_bytes().unwrap();
        let decoded = GossipMessage::from_bytes(&bytes).unwrap();

        match decoded {
            GossipMessage::TopologyAnnouncement(signed) => {
                assert!(signed.verify().is_some());
                assert_eq!(signed.announcement.topology_version, 10);
            }
            _ => panic!("expected TopologyAnnouncement"),
        }
    }

    #[test]
    fn test_gossip_message_blob_announcement_roundtrip() {
        let secret_key = secret_key_from_seed(92);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(920u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x77; 32]);

        let announcement = BlobAnnouncement::new(
            node_id,
            endpoint_addr,
            blob_hash,
            65536,
            BlobFormat::HashSeq,
            Some("gossip-test".to_string()),
        )
        .unwrap();

        let signed = SignedBlobAnnouncement::sign(announcement, &secret_key).unwrap();
        let message = GossipMessage::BlobAnnouncement(signed);

        let bytes = message.to_bytes().unwrap();
        let decoded = GossipMessage::from_bytes(&bytes).unwrap();

        match decoded {
            GossipMessage::BlobAnnouncement(signed) => {
                assert!(signed.verify().is_some());
                assert_eq!(signed.announcement.blob_size, 65536);
                assert_eq!(signed.announcement.tag, Some("gossip-test".to_string()));
            }
            _ => panic!("expected BlobAnnouncement"),
        }
    }

    #[test]
    fn test_gossip_message_from_bytes_rejects_oversized() {
        let oversized = vec![0u8; MAX_GOSSIP_MESSAGE_SIZE + 1];
        assert!(GossipMessage::from_bytes(&oversized).is_none());
    }

    #[test]
    fn test_gossip_message_from_bytes_rejects_malformed() {
        let malformed = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB];
        assert!(GossipMessage::from_bytes(&malformed).is_none());
    }

    #[test]
    fn test_gossip_message_legacy_fallback_peer_announcement() {
        // Test that GossipMessage::from_bytes can parse a legacy SignedPeerAnnouncement
        // (i.e., a SignedPeerAnnouncement serialized directly without the GossipMessage envelope)
        let secret_key = secret_key_from_seed(93);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(930u64);

        let announcement = PeerAnnouncement::new(node_id, endpoint_addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        // Serialize without the GossipMessage envelope (legacy format)
        let legacy_bytes = signed.to_bytes().unwrap();

        // GossipMessage::from_bytes should fall back to parsing as SignedPeerAnnouncement
        let decoded = GossipMessage::from_bytes(&legacy_bytes).unwrap();

        match decoded {
            GossipMessage::PeerAnnouncement(signed) => {
                assert!(signed.verify().is_some());
                assert_eq!(signed.announcement.node_id, node_id);
            }
            _ => panic!("expected PeerAnnouncement from legacy fallback"),
        }
    }

    #[test]
    fn test_gossip_message_size_within_limit() {
        // Verify that a typical GossipMessage stays within MAX_GOSSIP_MESSAGE_SIZE
        let secret_key = secret_key_from_seed(94);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(940u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0x88; 32]);

        // Create a blob announcement with maximum tag length
        let max_tag = "x".repeat(BlobAnnouncement::MAX_TAG_LEN);
        let announcement =
            BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, u64::MAX, BlobFormat::HashSeq, Some(max_tag))
                .unwrap();

        let signed = SignedBlobAnnouncement::sign(announcement, &secret_key).unwrap();
        let message = GossipMessage::BlobAnnouncement(signed);

        let bytes = message.to_bytes().unwrap();

        // Should be well within the limit
        assert!(
            bytes.len() <= MAX_GOSSIP_MESSAGE_SIZE,
            "message size {} exceeds limit {}",
            bytes.len(),
            MAX_GOSSIP_MESSAGE_SIZE
        );

        // Also verify it can be parsed back
        assert!(GossipMessage::from_bytes(&bytes).is_some());
    }
}
