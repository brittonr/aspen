//! Types for iroh-blobs integration.
//!
//! Defines the core data structures for blob storage and references.

use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;
use serde::Deserialize;
use serde::Serialize;

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
    #[serde(rename = "size")]
    pub size_bytes: u64,
    /// Format of the blob (Raw or HashSeq for collections).
    pub format: BlobFormat,
}

impl BlobRef {
    /// Create a new blob reference.
    pub fn new(hash: Hash, size_bytes: u64, format: BlobFormat) -> Self {
        Self {
            hash,
            size_bytes,
            format,
        }
    }

    /// Serialize to a KV-storable string with prefix.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails. This should never happen
    /// for a valid `BlobRef` since all fields are guaranteed to be serializable.
    pub fn to_kv_value(&self) -> Result<String, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(format!("{}{}", BLOB_REF_PREFIX, json))
    }

    /// Parse from a KV value string.
    /// Returns None if the value is not a blob reference.
    pub fn from_kv_value(value: &str) -> Option<Self> {
        let json = value.strip_prefix(BLOB_REF_PREFIX)?;
        serde_json::from_str(json)
            .inspect_err(|e| tracing::debug!("failed to parse blob reference: {e}"))
            .ok()
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
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Whether the blob is complete (Tiger Style: boolean prefix `is_`).
    #[serde(alias = "complete")]
    pub is_complete: bool,
    /// Tags protecting this blob from GC.
    pub tags: Vec<String>,
}

/// Entry in a blob listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListEntry {
    /// The blob hash.
    pub hash: Hash,
    /// Size in bytes.
    #[serde(rename = "size")]
    pub size_bytes: u64,
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

        let kv_value = blob_ref.to_kv_value().expect("serialization should succeed");
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

    #[test]
    fn test_blob_ref_prefix_detection() {
        assert!(is_blob_ref("__blob:{\"hash\":\"...\"}"));
        assert!(!is_blob_ref(""));
        assert!(!is_blob_ref("__blob")); // no colon
    }

    #[test]
    fn test_blob_ref_from_kv_value_malformed_json() {
        // Has prefix but invalid JSON
        assert!(BlobRef::from_kv_value("__blob:not-json").is_none());
    }

    #[test]
    fn test_blob_ref_new_fields() {
        let hash = Hash::new([0u8; 32]);
        let br = BlobRef::new(hash, 0, BlobFormat::Raw);
        assert_eq!(br.size_bytes, 0);
        assert_eq!(br.format, BlobFormat::Raw);
    }

    #[test]
    fn test_blob_ref_hashseq_format() {
        let hash = Hash::new([1u8; 32]);
        let br = BlobRef::new(hash, 999, BlobFormat::HashSeq);
        let kv = br.to_kv_value().expect("serialize");
        let back = BlobRef::from_kv_value(&kv).expect("deserialize");
        assert_eq!(back.format, BlobFormat::HashSeq);
        assert_eq!(back.size_bytes, 999);
    }

    #[test]
    fn test_blob_ref_serde_json_roundtrip() {
        let hash = Hash::new([7u8; 32]);
        let br = BlobRef::new(hash, 42, BlobFormat::Raw);
        let json = serde_json::to_string(&br).expect("json serialize");
        let back: BlobRef = serde_json::from_str(&json).expect("json deserialize");
        assert_eq!(back, br);
    }

    #[test]
    fn test_blob_status_serde_roundtrip() {
        let status = BlobStatus {
            hash: Hash::new([5u8; 32]),
            size_bytes: Some(1024),
            is_complete: true,
            tags: vec!["keep".into(), "important".into()],
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let back: BlobStatus = serde_json::from_str(&json).expect("deserialize");
        assert!(back.is_complete);
        assert_eq!(back.size_bytes, Some(1024));
        assert_eq!(back.tags.len(), 2);
    }

    #[test]
    fn test_blob_status_complete_alias() {
        // Test that the "complete" alias works for deserialization
        let json = r#"{"hash":"0505050505050505050505050505050505050505050505050505050505050505","size":100,"complete":false,"tags":[]}"#;
        let status: BlobStatus = serde_json::from_str(json).expect("deserialize with alias");
        assert!(!status.is_complete);
    }

    #[test]
    fn test_blob_list_entry_serde() {
        let entry = BlobListEntry {
            hash: Hash::new([9u8; 32]),
            size_bytes: 2048,
            format: BlobFormat::Raw,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: BlobListEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.size_bytes, 2048);
    }

    #[test]
    fn test_blob_list_result_empty() {
        let result = BlobListResult {
            blobs: vec![],
            continuation_token: None,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: BlobListResult = serde_json::from_str(&json).expect("deserialize");
        assert!(back.blobs.is_empty());
        assert!(back.continuation_token.is_none());
    }

    #[test]
    fn test_blob_list_result_with_pagination() {
        let result = BlobListResult {
            blobs: vec![BlobListEntry {
                hash: Hash::new([0u8; 32]),
                size_bytes: 100,
                format: BlobFormat::Raw,
            }],
            continuation_token: Some("next-page-token".into()),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let back: BlobListResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.blobs.len(), 1);
        assert_eq!(back.continuation_token.as_deref(), Some("next-page-token"));
    }
}
