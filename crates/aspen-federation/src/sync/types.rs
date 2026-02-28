//! Federation protocol request/response types and data structures.
//!
//! Wire format uses postcard serialization with length-prefixed framing.

use std::collections::HashMap;

use aspen_core::Signature;
use aspen_core::hlc::SerializableTimestamp;
use serde::Deserialize;
use serde::Serialize;

use crate::identity::SignedClusterIdentity;
use crate::policy::ResourcePolicy;
use crate::types::FederatedId;

// ============================================================================
// Request Types
// ============================================================================

/// Federation protocol request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationRequest {
    /// Exchange cluster identities and capabilities.
    Handshake {
        /// Our signed cluster identity.
        identity: SignedClusterIdentity,
        /// Protocol version we support.
        protocol_version: u8,
        /// Capabilities we support.
        capabilities: Vec<String>,
    },

    /// List federated resources available on this cluster.
    ListResources {
        /// Resource type filter (e.g., "forge:repo").
        resource_type: Option<String>,
        /// Pagination cursor.
        cursor: Option<String>,
        /// Maximum results.
        limit: u32,
    },

    /// Get current state of a federated resource.
    GetResourceState {
        /// The federated resource ID.
        fed_id: FederatedId,
    },

    /// Request missing objects for a resource.
    SyncObjects {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// Object types we want (e.g., "refs", "blobs", "cobs").
        want_types: Vec<String>,
        /// Hashes we already have (to avoid re-sending).
        have_hashes: Vec<[u8; 32]>,
        /// Maximum objects to return.
        limit: u32,
    },

    /// Verify a ref update signature (legacy alias for VerifyUpdate).
    VerifyRefUpdate {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// Ref name.
        ref_name: String,
        /// New hash.
        new_hash: [u8; 32],
        /// Signature.
        signature: Signature,
        /// Signer public key.
        signer: [u8; 32],
    },

    /// Verify an update signature (generic, any resource type).
    ///
    /// This is the app-agnostic version of `VerifyRefUpdate`. Any application
    /// can use this to verify that an update was signed by an authorized delegate.
    VerifyUpdate {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// Update type (e.g., "ref", "cob", "blob_collection", "crdt_doc").
        update_type: String,
        /// Key identifying what was updated (e.g., ref name, blob hash, doc ID).
        key: String,
        /// New value hash (BLAKE3).
        new_value: [u8; 32],
        /// Signature over the update.
        signature: Signature,
        /// Signer public key.
        signer: [u8; 32],
    },
}

// ============================================================================
// Response Types
// ============================================================================

/// Federation protocol response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationResponse {
    /// Handshake response with peer's identity.
    Handshake {
        /// Peer's signed cluster identity.
        identity: SignedClusterIdentity,
        /// Protocol version peer supports.
        protocol_version: u8,
        /// Capabilities peer supports.
        capabilities: Vec<String>,
        /// Whether we're trusted by this peer.
        trusted: bool,
    },

    /// List of federated resources.
    ResourceList {
        /// Available resources.
        resources: Vec<ResourceInfo>,
        /// Next cursor for pagination.
        next_cursor: Option<String>,
        /// Total count (if known).
        total: Option<u32>,
    },

    /// Current state of a resource.
    ResourceState {
        /// Whether the resource was found.
        was_found: bool,
        /// Ref heads (ref_name -> hash).
        heads: HashMap<String, [u8; 32]>,
        /// Resource metadata.
        metadata: Option<ResourceMetadata>,
    },

    /// Synced objects.
    Objects {
        /// The objects.
        objects: Vec<SyncObject>,
        /// Whether there are more objects available.
        has_more: bool,
    },

    /// Ref verification result.
    VerifyResult {
        /// Whether the signature is valid.
        is_valid: bool,
        /// Error message if invalid.
        error: Option<String>,
    },

    /// Error response.
    Error {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
    },
}

/// Information about a federated resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Federated resource ID.
    pub fed_id: FederatedId,
    /// Resource type (e.g., "forge:repo").
    pub resource_type: String,
    /// Human-readable name.
    pub name: String,
    /// Federation mode.
    pub mode: String,
    /// HLC timestamp of last update.
    pub updated_at_hlc: SerializableTimestamp,
}

/// Resource metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    /// Resource type.
    pub resource_type: String,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Delegate public keys (for Forge repos).
    pub delegates: Vec<[u8; 32]>,
    /// Signature threshold (number of replicas required).
    #[serde(rename = "threshold")]
    pub threshold_replicas: u32,
    /// HLC timestamp when created.
    pub created_at_hlc: SerializableTimestamp,
    /// HLC timestamp when last updated.
    pub updated_at_hlc: SerializableTimestamp,
    /// Generic federation policy for this resource.
    ///
    /// When set, provides quorum requirements, selection strategy, and
    /// fork detection mode. This is the platform-level policy; apps configure
    /// it via [`ResourcePolicy`] and the federation layer enforces it.
    #[serde(default)]
    pub policy: Option<ResourcePolicy>,
    /// Opaque application-specific metadata (postcard-encoded bytes).
    ///
    /// Applications store their own data here. For example, Forge stores
    /// delegate signing keys. The federation layer does not interpret this.
    #[serde(default)]
    pub app_metadata: Vec<u8>,
}

/// A synced object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncObject {
    /// Object type (e.g., "blob", "tree", "commit", "ref", "cob").
    pub object_type: String,
    /// Object hash (BLAKE3).
    pub hash: [u8; 32],
    /// Object data.
    pub data: Vec<u8>,
    /// Optional signature (for signed objects).
    pub signature: Option<Signature>,
    /// Optional signer (for signed objects).
    pub signer: Option<[u8; 32]>,
}
