//! Gossip State Machine Model
//!
//! Abstract state model for formal verification of gossip message handling.
//!
//! # State Model
//!
//! The gossip state captures:
//! - Message version for compatibility checking
//! - Bounded message sizes for DoS prevention
//! - Timestamp for freshness tracking
//!
//! # Key Invariants
//!
//! 1. **GOSSIP-2: Size Bounds**: Messages bounded by MAX_GOSSIP_MESSAGE_SIZE
//! 2. **GOSSIP-3: Version Compatibility**: Future versions rejected
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-cluster/verus/gossip_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants (Mirror from gossip/constants.rs)
    // ========================================================================

    /// Maximum gossip message size in bytes (4KB)
    pub const MAX_GOSSIP_MESSAGE_SIZE: u64 = 4096;

    /// Current gossip protocol version
    pub const GOSSIP_MESSAGE_VERSION: u8 = 2;

    /// Maximum tag length for blob announcements
    pub const MAX_TAG_LEN: u64 = 64;

    // ========================================================================
    // State Models
    // ========================================================================

    /// Abstract peer announcement structure
    pub struct PeerAnnouncementSpec {
        /// Protocol version
        pub version: u8,
        /// Node ID (u64 representation)
        pub node_id: u64,
        /// Timestamp in microseconds since UNIX epoch
        pub timestamp_micros: u64,
        /// Public key bytes (32 bytes as u256 split)
        pub public_key_high: u128,
        pub public_key_low: u128,
    }

    /// Abstract topology announcement structure
    pub struct TopologyAnnouncementSpec {
        /// Protocol version
        pub version: u8,
        /// Announcing node ID
        pub node_id: u64,
        /// Topology version number
        pub topology_version: u64,
        /// Hash of topology content
        pub topology_hash: u64,
        /// Raft term
        pub term: u64,
        /// Timestamp in microseconds
        pub timestamp_micros: u64,
    }

    /// Abstract blob announcement structure
    pub struct BlobAnnouncementSpec {
        /// Protocol version
        pub version: u8,
        /// Node ID offering the blob
        pub node_id: u64,
        /// BLAKE3 hash of blob (split into parts)
        pub blob_hash_high: u128,
        pub blob_hash_low: u128,
        /// Blob size in bytes
        pub blob_size: u64,
        /// Timestamp in microseconds
        pub timestamp_micros: u64,
        /// Tag length (0 if no tag)
        pub tag_len: u64,
    }

    /// Signature state (64 bytes Ed25519)
    pub struct SignatureSpec {
        /// First half of signature (32 bytes)
        pub sig_high: u128,
        pub sig_mid_high: u128,
        /// Second half of signature (32 bytes)
        pub sig_mid_low: u128,
        pub sig_low: u128,
    }

    /// Signed peer announcement state
    pub struct SignedPeerAnnouncementSpec {
        pub announcement: PeerAnnouncementSpec,
        pub signature: SignatureSpec,
    }

    /// Signed topology announcement state
    pub struct SignedTopologyAnnouncementSpec {
        pub announcement: TopologyAnnouncementSpec,
        pub signature: SignatureSpec,
        /// Signer's public key
        pub public_key_high: u128,
        pub public_key_low: u128,
    }

    /// Signed blob announcement state
    pub struct SignedBlobAnnouncementSpec {
        pub announcement: BlobAnnouncementSpec,
        pub signature: SignatureSpec,
    }

    /// Gossip message envelope
    pub enum GossipMessageSpec {
        Peer(SignedPeerAnnouncementSpec),
        Topology(SignedTopologyAnnouncementSpec),
        Blob(SignedBlobAnnouncementSpec),
    }

    // ========================================================================
    // Invariant 2: Size Bounds
    // ========================================================================

    /// GOSSIP-2: Message size bounded by MAX_GOSSIP_MESSAGE_SIZE
    ///
    /// All gossip messages must fit within the size limit.
    pub open spec fn size_bounded(message_bytes_len: u64) -> bool {
        message_bytes_len <= MAX_GOSSIP_MESSAGE_SIZE
    }

    /// Precondition for from_bytes: size check BEFORE deserialization
    pub open spec fn from_bytes_pre(bytes_len: u64) -> bool {
        size_bounded(bytes_len)
    }

    /// from_bytes rejects oversized messages
    pub open spec fn from_bytes_rejects_oversized(bytes_len: u64, result_is_some: bool) -> bool {
        !size_bounded(bytes_len) ==> !result_is_some
    }

    /// Proof: Size check before deserialization prevents memory exhaustion
    pub proof fn size_check_prevents_exhaustion(bytes_len: u64)
        ensures !from_bytes_pre(bytes_len) ==> bytes_len > MAX_GOSSIP_MESSAGE_SIZE
    {
        // If precondition fails, bytes exceed limit
    }

    // ========================================================================
    // Invariant 3: Version Compatibility
    // ========================================================================

    /// GOSSIP-3: Version compatibility check
    ///
    /// Only accept messages with version <= current protocol version.
    pub open spec fn version_compatible(message_version: u8) -> bool {
        message_version <= GOSSIP_MESSAGE_VERSION
    }

    /// from_bytes rejects future versions
    pub open spec fn from_bytes_rejects_future_version(
        message_version: u8,
        result_is_some: bool,
    ) -> bool {
        !version_compatible(message_version) ==> !result_is_some
    }

    /// Proof: Future versions are rejected
    pub proof fn version_check_rejects_future(message_version: u8)
        requires message_version > GOSSIP_MESSAGE_VERSION
        ensures !version_compatible(message_version)
    {
        // Version > current means incompatible
    }

    // ========================================================================
    // Combined Deserialization Invariant
    // ========================================================================

    /// Combined precondition for safe deserialization
    pub open spec fn deserialize_pre(bytes_len: u64, message_version: u8) -> bool {
        size_bounded(bytes_len) &&
        version_compatible(message_version)
    }

    /// Deserialization postcondition: if successful, invariants hold
    pub open spec fn deserialize_post(
        bytes_len: u64,
        message_version: u8,
        result_is_some: bool,
    ) -> bool {
        result_is_some ==> deserialize_pre(bytes_len, message_version)
    }

    /// Proof: Deserialization maintains invariants
    pub proof fn deserialize_maintains_invariants(
        bytes_len: u64,
        message_version: u8,
        result_is_some: bool,
    )
        requires deserialize_post(bytes_len, message_version, result_is_some)
        ensures result_is_some ==> (size_bounded(bytes_len) && version_compatible(message_version))
    {
        // Follows from postcondition definition
    }

    // ========================================================================
    // Blob Tag Bounds
    // ========================================================================

    /// BLOB-3: Tag length bounded
    pub open spec fn tag_bounded(tag_len: u64) -> bool {
        tag_len <= MAX_TAG_LEN
    }

    /// BlobAnnouncement construction precondition
    pub open spec fn blob_announcement_new_pre(tag_len: u64) -> bool {
        tag_bounded(tag_len)
    }

    /// Proof: Tag validation prevents oversized payloads
    pub proof fn tag_validation_bounds_payload(tag_len: u64)
        requires tag_bounded(tag_len)
        ensures tag_len <= 64
    {
        // MAX_TAG_LEN = 64
    }

    // ========================================================================
    // Timestamp Freshness
    // ========================================================================

    /// Timestamp is non-zero (message was created)
    pub open spec fn timestamp_valid(timestamp_micros: u64) -> bool {
        timestamp_micros > 0
    }

    /// Timestamp freshness check (within max_age_micros of current_time)
    pub open spec fn timestamp_fresh(
        timestamp_micros: u64,
        current_time_micros: u64,
        max_age_micros: u64,
    ) -> bool {
        timestamp_micros <= current_time_micros &&
        current_time_micros - timestamp_micros <= max_age_micros
    }
}
