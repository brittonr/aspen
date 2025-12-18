//! Types for iroh-blobs integration.
//!
//! Defines the core data structures for blob storage and references.

use iroh_blobs::{BlobFormat, Hash};
use serde::{Deserialize, Serialize};

use super::constants::BLOB_REF_PREFIX;

/// Reference to a blob stored in the content-addressed store.
///
/// This is stored in the KV store when a value exceeds `BLOB_THRESHOLD`.
/// The actual data is stored in iroh-blobs and retrieved via the hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRef {
    /// BLAKE3 hash of the blob content.
    pub hash: Hash,
    /// Size of the blob in bytes.
    pub size: u64,
    /// Format of the blob (Raw or HashSeq for collections).
    pub format: BlobFormat,
}

impl BlobRef {
    /// Create a new blob reference.
    pub fn new(hash: Hash, size: u64, format: BlobFormat) -> Self {
        Self { hash, size, format }
    }

    /// Serialize to a KV-storable string with prefix.
    pub fn to_kv_value(&self) -> String {
        let json = serde_json::to_string(self).expect("BlobRef serialization should not fail");
        format!("{}{}", BLOB_REF_PREFIX, json)
    }

    /// Parse from a KV value string.
    /// Returns None if the value is not a blob reference.
    pub fn from_kv_value(value: &str) -> Option<Self> {
        let json = value.strip_prefix(BLOB_REF_PREFIX)?;
        serde_json::from_str(json).ok()
    }
}

/// Check if a KV value is a blob reference.
pub fn is_blob_ref(value: &str) -> bool {
    value.starts_with(BLOB_REF_PREFIX)
}

/// Result of adding a blob to the store.
#[derive(Debug, Clone)]
pub struct AddBlobResult {
    /// Reference to the stored blob.
    pub blob_ref: BlobRef,
    /// Whether the blob was newly added (false if already existed).
    pub was_new: bool,
}

/// Status of a blob in the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobStatus {
    /// The blob hash.
    pub hash: Hash,
    /// Size in bytes (if known).
    pub size: Option<u64>,
    /// Whether the blob is complete.
    pub complete: bool,
    /// Tags protecting this blob from GC.
    pub tags: Vec<String>,
}

/// Entry in a blob listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListEntry {
    /// The blob hash.
    pub hash: Hash,
    /// Size in bytes.
    pub size: u64,
    /// Format of the blob.
    pub format: BlobFormat,
}

/// Result of listing blobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListResult {
    /// List of blobs.
    pub blobs: Vec<BlobListEntry>,
    /// Continuation token for pagination (None if no more results).
    pub continuation_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_ref_roundtrip() {
        let hash = Hash::new([42u8; 32]);
        let blob_ref = BlobRef::new(hash, 12345, BlobFormat::Raw);

        let kv_value = blob_ref.to_kv_value();
        assert!(is_blob_ref(&kv_value));

        let parsed = BlobRef::from_kv_value(&kv_value).expect("should parse");
        assert_eq!(parsed, blob_ref);
    }

    #[test]
    fn test_not_blob_ref() {
        assert!(!is_blob_ref("hello world"));
        assert!(!is_blob_ref("__blo:not quite"));
        assert!(BlobRef::from_kv_value("regular value").is_none());
    }
}
