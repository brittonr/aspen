//! Read operation types for querying key-value state.

use serde::Deserialize;
use serde::Serialize;

use crate::KeyValueWithRevision;

/// Consistency level for read operations.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReadConsistency {
    #[default]
    Linearizable,
    Lease,
    Stale,
}

/// Request to read a single key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadRequest {
    pub key: String,
    #[serde(default)]
    pub consistency: ReadConsistency,
}

impl ReadRequest {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::default(),
        }
    }

    pub fn with_lease(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::Lease,
        }
    }

    pub fn stale(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::Stale,
        }
    }
}

/// Response from a read operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadResult {
    pub kv: Option<KeyValueWithRevision>,
}

/// Request to delete a key from the distributed store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteRequest {
    pub key: String,
}

impl DeleteRequest {
    /// Create a delete request for the specified key.
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

/// Result of a delete operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteResult {
    pub key: String,
    /// Whether the key was deleted (Tiger Style: boolean prefix `is_`).
    #[serde(alias = "deleted")]
    pub is_deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_request_new_default_consistency() {
        let req = ReadRequest::new("key");
        assert_eq!(req.key, "key");
        assert_eq!(req.consistency, ReadConsistency::Linearizable);
    }

    #[test]
    fn read_request_with_lease() {
        let req = ReadRequest::with_lease("key");
        assert_eq!(req.key, "key");
        assert_eq!(req.consistency, ReadConsistency::Lease);
    }

    #[test]
    fn read_request_stale() {
        let req = ReadRequest::stale("key");
        assert_eq!(req.key, "key");
        assert_eq!(req.consistency, ReadConsistency::Stale);
    }

    #[test]
    fn delete_request_new() {
        let req = DeleteRequest::new("key");
        assert_eq!(req.key, "key");
    }

    #[test]
    fn read_consistency_default() {
        assert_eq!(ReadConsistency::default(), ReadConsistency::Linearizable);
    }

    #[test]
    fn read_consistency_clone_and_eq() {
        assert_eq!(ReadConsistency::Linearizable, ReadConsistency::Linearizable.clone());
        assert_eq!(ReadConsistency::Lease, ReadConsistency::Lease.clone());
        assert_eq!(ReadConsistency::Stale, ReadConsistency::Stale.clone());
        assert_ne!(ReadConsistency::Linearizable, ReadConsistency::Stale);
    }

    #[test]
    fn read_request_serialization_roundtrip() {
        let req = ReadRequest::with_lease("key");
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ReadRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }
}
