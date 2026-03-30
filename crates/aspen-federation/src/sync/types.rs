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
        /// Optional credential for token-based auth (replaces trust-level auth).
        /// `#[serde(default)]` for backward compatibility with clusters that
        /// don't yet send credentials.
        #[serde(default)]
        credential: Option<aspen_auth::Credential>,
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

    /// Refresh a near-expiry federation token.
    RefreshToken {
        /// Current credential (near-expiry) to refresh.
        credential: aspen_auth::Credential,
    },

    /// Push objects and ref updates to a remote cluster.
    ///
    /// The origin cluster sends git objects and ref updates to the receiver.
    /// The receiver validates trust, imports objects, and updates mirror refs.
    PushObjects {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// Git objects to push.
        objects: Vec<SyncObject>,
        /// Ref updates to apply after importing objects.
        ref_updates: Vec<RefEntry>,
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

    /// Result of a push operation.
    PushResult {
        /// Whether the push was accepted.
        accepted: bool,
        /// Number of objects imported.
        imported: u32,
        /// Number of objects skipped (already present).
        skipped: u32,
        /// Number of refs updated.
        refs_updated: u32,
        /// Errors encountered during import (non-fatal).
        errors: Vec<String>,
    },

    /// Token refresh response.
    TokenRefreshed {
        /// Fresh token with same capabilities and new expiry.
        token: aspen_auth::CapabilityToken,
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

/// A ref entry transferred during federation sync.
///
/// Represents a single ref head (e.g., `heads/main` → commit hash).
/// Serialized with postcard inside `SyncObject.data` when `object_type == "ref"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefEntry {
    /// Ref name (e.g., "heads/main", "tags/v1.0").
    pub ref_name: String,
    /// Commit hash this ref points to (envelope BLAKE3 on the source).
    pub head_hash: [u8; 32],
    /// Git SHA1 of the commit (deterministic across clusters).
    ///
    /// Used by the receiver to match this ref to the correct locally
    /// imported commit. SHA1 is computed from the raw git content and
    /// is identical on source and destination.
    #[serde(default)]
    pub commit_sha1: Option<[u8; 20]>,
}

/// A synced object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncObject {
    /// Object type (e.g., "blob", "tree", "commit", "ref", "cob").
    pub object_type: String,
    /// Object hash (BLAKE3 of `data`).
    pub hash: [u8; 32],
    /// Object data.
    pub data: Vec<u8>,
    /// Optional signature (for signed objects).
    pub signature: Option<Signature>,
    /// Optional signer (for signed objects).
    pub signer: Option<[u8; 32]>,
    /// Optional envelope BLAKE3 hash (the storage-layer hash of the
    /// `SignedObject` wrapping this content). Present for git objects
    /// exported by the Forge resolver. Clients return these in
    /// `have_hashes` for dedup instead of computing SHA-1, avoiding
    /// byte-level mismatches from tree/commit re-serialization.
    #[serde(default)]
    pub envelope_hash: Option<[u8; 32]>,
    /// Original SHA-1 hash from the initial git push import. May differ
    /// from the SHA-1 computed from `data` for trees/commits. The receiver
    /// stores a mapping from this SHA-1 to its local BLAKE3 so that git
    /// clients can look up objects by their original SHA-1.
    #[serde(default)]
    pub origin_sha1: Option<[u8; 20]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_objects_request_roundtrip() {
        let fed_id = FederatedId::new(iroh::SecretKey::generate(&mut rand::rng()).public(), [0xaa; 32]);
        let obj = SyncObject {
            object_type: "blob".to_string(),
            hash: [0xbb; 32],
            data: b"hello world".to_vec(),
            signature: None,
            signer: None,
            envelope_hash: None,

            origin_sha1: None,
        };
        let ref_entry = RefEntry {
            ref_name: "heads/main".to_string(),
            head_hash: [0xcc; 32],
            commit_sha1: Some([0xdd; 20]),
        };
        let request = FederationRequest::PushObjects {
            fed_id,
            objects: vec![obj],
            ref_updates: vec![ref_entry],
        };
        let bytes = postcard::to_stdvec(&request).expect("serialize");
        let decoded: FederationRequest = postcard::from_bytes(&bytes).expect("deserialize");
        match decoded {
            FederationRequest::PushObjects {
                fed_id: fid,
                objects,
                ref_updates,
            } => {
                assert_eq!(fid, fed_id);
                assert_eq!(objects.len(), 1);
                assert_eq!(objects[0].object_type, "blob");
                assert_eq!(ref_updates.len(), 1);
                assert_eq!(ref_updates[0].ref_name, "heads/main");
            }
            _ => panic!("expected PushObjects"),
        }
    }

    #[test]
    fn push_result_response_roundtrip() {
        let response = FederationResponse::PushResult {
            accepted: true,
            imported: 5,
            skipped: 2,
            refs_updated: 1,
            errors: vec!["minor issue".to_string()],
        };
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let decoded: FederationResponse = postcard::from_bytes(&bytes).expect("deserialize");
        match decoded {
            FederationResponse::PushResult {
                accepted,
                imported,
                skipped,
                refs_updated,
                errors,
            } => {
                assert!(accepted);
                assert_eq!(imported, 5);
                assert_eq!(skipped, 2);
                assert_eq!(refs_updated, 1);
                assert_eq!(errors, vec!["minor issue"]);
            }
            _ => panic!("expected PushResult"),
        }
    }
}
