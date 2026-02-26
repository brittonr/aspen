//! Syncable object types and per-resource data storage for federation sync testing.

use std::collections::HashMap;
use std::collections::HashSet;

use aspen_core::hlc::SerializableTimestamp;
use iroh::PublicKey;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::warn;

use super::MAX_OBJECTS_PER_RESOURCE;
use super::MAX_SYNC_BATCH_SIZE;

// ============================================================================
// Syncable Objects - Simulated data for federation sync testing
// ============================================================================

/// A syncable object with content-addressed storage and HLC timestamp.
///
/// Uses Last-Write-Wins (LWW) conflict resolution based on HLC timestamps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncableObject {
    /// Content hash (Blake3) - serves as the object ID.
    pub hash: [u8; 32],
    /// Object data (limited to 1MB in real system, smaller for tests).
    pub data: Vec<u8>,
    /// HLC timestamp for LWW conflict resolution.
    pub timestamp: SerializableTimestamp,
    /// Origin cluster that created this object.
    pub origin_cluster: [u8; 32],
}

impl SyncableObject {
    /// Create a new syncable object.
    pub fn new(data: Vec<u8>, timestamp: SerializableTimestamp, origin_cluster: PublicKey) -> Self {
        let hash = *blake3::hash(&data).as_bytes();
        Self {
            hash,
            data,
            timestamp,
            origin_cluster: *origin_cluster.as_bytes(),
        }
    }

    /// Get the object's content hash as a hex string.
    pub fn hash_hex(&self) -> String {
        hex::encode(&self.hash[..8])
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Objects that were received.
    pub received: Vec<SyncableObject>,
    /// Objects that had conflicts (resolved by LWW).
    pub conflicts_resolved: usize,
    /// Whether the sync completed fully.
    pub is_complete: bool,
    /// Error message if sync failed.
    pub error: Option<String>,
}

impl SyncResult {
    /// Create a successful sync result.
    pub fn success(received: Vec<SyncableObject>, conflicts_resolved: usize) -> Self {
        Self {
            received,
            conflicts_resolved,
            is_complete: true,
            error: None,
        }
    }

    /// Create a failed sync result.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            received: vec![],
            conflicts_resolved: 0,
            is_complete: false,
            error: Some(error.into()),
        }
    }

    /// Check if the sync was successful.
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.is_complete
    }
}

/// Per-resource data store within a cluster.
#[derive(Debug, Default)]
pub struct ResourceDataStore {
    /// Objects stored for this resource (hash -> object).
    objects: RwLock<HashMap<[u8; 32], SyncableObject>>,
    /// Ref heads (ref_name -> object_hash).
    refs: RwLock<HashMap<String, [u8; 32]>>,
}

impl ResourceDataStore {
    /// Create a new empty data store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an object, using LWW for conflicts.
    ///
    /// Returns true if the object was stored (new or newer timestamp).
    pub fn store_object(&self, obj: SyncableObject) -> bool {
        let mut objects = self.objects.write();

        // Check for existing object with same hash
        if let Some(existing) = objects.get(&obj.hash) {
            // LWW: keep the one with later timestamp
            if obj.timestamp <= existing.timestamp {
                debug!(
                    hash = %hex::encode(&obj.hash[..8]),
                    "object already exists with same or later timestamp"
                );
                return false;
            }
        }

        // Check capacity
        if objects.len() >= MAX_OBJECTS_PER_RESOURCE && !objects.contains_key(&obj.hash) {
            warn!(max = MAX_OBJECTS_PER_RESOURCE, "resource data store at capacity, rejecting object");
            return false;
        }

        objects.insert(obj.hash, obj);
        true
    }

    /// Get an object by hash.
    pub fn get_object(&self, hash: &[u8; 32]) -> Option<SyncableObject> {
        self.objects.read().get(hash).cloned()
    }

    /// Get all objects.
    pub fn all_objects(&self) -> Vec<SyncableObject> {
        self.objects.read().values().cloned().collect()
    }

    /// Get object count.
    pub fn object_count(&self) -> usize {
        self.objects.read().len()
    }

    /// Set a ref to point to an object hash.
    pub fn set_ref(&self, name: impl Into<String>, hash: [u8; 32]) {
        self.refs.write().insert(name.into(), hash);
    }

    /// Get a ref's target hash.
    pub fn get_ref(&self, name: &str) -> Option<[u8; 32]> {
        self.refs.read().get(name).copied()
    }

    /// Get all refs.
    pub fn all_refs(&self) -> HashMap<String, [u8; 32]> {
        self.refs.read().clone()
    }

    /// Get objects that the other side is missing.
    ///
    /// `their_hashes` is the set of hashes the remote side already has.
    pub fn objects_missing_from(&self, their_hashes: &HashSet<[u8; 32]>) -> Vec<SyncableObject> {
        self.objects
            .read()
            .values()
            .filter(|obj| !their_hashes.contains(&obj.hash))
            .take(MAX_SYNC_BATCH_SIZE)
            .cloned()
            .collect()
    }
}
