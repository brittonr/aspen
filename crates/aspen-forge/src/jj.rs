//! Native Jujutsu object envelope and blob encoding.
//!
//! This module is pure: it validates JJ object envelopes and converts them to
//! deterministic BLAKE3-addressed blob bytes. Persistence and consensus indexes
//! live in the Forge shell.

use std::sync::Arc;

use aspen_blob::BlobStore;
use aspen_core::KeyValueStore;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Magic prefix for Aspen JJ blob envelopes.
pub const JJ_OBJECT_MAGIC: &[u8] = b"ASPEN-JJ\0";
/// Current JJ object envelope encoding version.
pub const JJ_OBJECT_ENCODING_VERSION: u16 = 1;
/// Maximum parent object identifiers carried by one JJ object envelope.
pub const MAX_JJ_OBJECT_PARENTS: usize = 64;
/// Maximum change IDs carried by one JJ object envelope.
pub const MAX_JJ_OBJECT_CHANGE_IDS: usize = 32;
/// Maximum payload bytes in a single JJ object envelope.
pub const MAX_JJ_OBJECT_PAYLOAD_BYTES: usize = 128 * 1024 * 1024;
/// KV prefix for repo-scoped JJ object reachability indexes.
pub const JJ_REACHABILITY_KEY_PREFIX: &str = "forge:jj:reach:";

/// Native JJ object kind stored by Forge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JjObjectKind {
    /// JJ commit object.
    Commit,
    /// JJ tree object.
    Tree,
    /// File contents or file metadata object.
    File,
    /// Conflict-preserving JJ object.
    Conflict,
    /// Repo-scoped JJ metadata object.
    Metadata,
}

/// Versioned JJ object envelope stored in Forge blobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjObjectEnvelope {
    /// Envelope encoding version.
    pub version: u16,
    /// Native JJ object kind.
    pub kind: JjObjectKind,
    /// Repo identifier this object belongs to.
    pub repo_id: String,
    /// JJ object identifier as provided by the JJ client/plugin.
    pub object_id: String,
    /// Repo-scoped JJ change IDs associated with this object.
    pub change_ids: Vec<String>,
    /// Parent JJ object identifiers.
    pub parents: Vec<String>,
    /// Native JJ payload bytes.
    pub payload: Vec<u8>,
}

/// Encoded JJ object ready for content-addressed blob storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedJjObject {
    /// Full encoded bytes, including magic prefix.
    pub bytes: Vec<u8>,
    /// BLAKE3 digest of [`bytes`](Self::bytes).
    pub digest: [u8; blake3::OUT_LEN],
}

/// Stored JJ object reference recorded in the reachability index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjStoredObjectRef {
    /// Repo identifier this object belongs to.
    pub repo_id: String,
    /// JJ object identifier.
    pub object_id: String,
    /// BLAKE3 blob hash in text form.
    pub blob_hash: String,
    /// Encoded blob size in bytes.
    pub encoded_size_bytes: u64,
    /// Whether the blob was newly added to the blob store.
    pub was_new: bool,
}

/// JJ object envelope validation or encoding failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JjObjectEncodingError {
    /// Envelope version is not supported.
    #[error("unsupported JJ object encoding version {version}")]
    UnsupportedVersion { version: u16 },
    /// Required string field is empty.
    #[error("JJ object field '{field}' must not be empty")]
    EmptyField { field: &'static str },
    /// Parent count exceeds the hard bound.
    #[error("JJ object has {actual} parents, max {max}")]
    TooManyParents { actual: usize, max: usize },
    /// Change-id count exceeds the hard bound.
    #[error("JJ object has {actual} change IDs, max {max}")]
    TooManyChangeIds { actual: usize, max: usize },
    /// Payload exceeds the hard bound.
    #[error("JJ object payload is {actual} bytes, max {max}")]
    PayloadTooLarge { actual: usize, max: usize },
    /// Encoded blob is missing the expected magic prefix.
    #[error("JJ object blob has invalid magic prefix")]
    InvalidMagic,
    /// Postcard serialization failed.
    #[error("JJ object serialization failed: {message}")]
    Serialize { message: String },
    /// Postcard deserialization failed.
    #[error("JJ object deserialization failed: {message}")]
    Deserialize { message: String },
}

/// JJ object persistence or reachability failure.
#[derive(Debug, Error)]
pub enum JjObjectStoreError {
    /// Envelope validation or blob encoding failed.
    #[error("JJ object encoding failed: {source}")]
    Encoding { source: JjObjectEncodingError },
    /// Blob storage failed.
    #[error("JJ object blob storage failed: {message}")]
    Blob { message: String },
    /// KV index operation failed.
    #[error("JJ object reachability index failed: {message}")]
    Kv { message: String },
    /// Stored blob size does not fit in u64.
    #[error("JJ object encoded size overflow")]
    SizeOverflow,
}

impl From<JjObjectEncodingError> for JjObjectStoreError {
    fn from(source: JjObjectEncodingError) -> Self {
        Self::Encoding { source }
    }
}

/// Repo-scoped JJ object persistence over Aspen blobs and KV reachability.
pub struct JjObjectStore<K: ?Sized, B: BlobStore> {
    kv: Arc<K>,
    blobs: Arc<B>,
}

impl<K: KeyValueStore + ?Sized, B: BlobStore> JjObjectStore<K, B> {
    /// Create a JJ object store from existing Forge storage handles.
    #[must_use]
    pub fn new(kv: Arc<K>, blobs: Arc<B>) -> Self {
        Self { kv, blobs }
    }

    /// Store a native JJ object blob and record repo-scoped reachability.
    pub async fn store_object(&self, envelope: &JjObjectEnvelope) -> Result<JjStoredObjectRef, JjObjectStoreError> {
        let encoded = encode_jj_object(envelope)?;
        let add_result = self.blobs.add_bytes(&encoded.bytes).await.map_err(|source| JjObjectStoreError::Blob {
            message: source.to_string(),
        })?;
        let encoded_size_bytes = u64::try_from(encoded.bytes.len()).map_err(|_| JjObjectStoreError::SizeOverflow)?;
        let stored_ref = JjStoredObjectRef {
            repo_id: envelope.repo_id.clone(),
            object_id: envelope.object_id.clone(),
            blob_hash: add_result.blob_ref.hash.to_string(),
            encoded_size_bytes,
            was_new: add_result.was_new,
        };
        let index_value = serde_json::to_string(&stored_ref).map_err(|source| JjObjectStoreError::Kv {
            message: source.to_string(),
        })?;
        self.kv
            .write(aspen_core::WriteRequest::set(
                jj_reachability_key(&stored_ref.repo_id, &stored_ref.object_id),
                index_value,
            ))
            .await
            .map_err(|source| JjObjectStoreError::Kv {
                message: source.to_string(),
            })?;

        Ok(stored_ref)
    }

    /// Look up the stored blob reference for a repo-scoped JJ object.
    pub async fn lookup_object(
        &self,
        repo_id: &str,
        object_id: &str,
    ) -> Result<Option<JjStoredObjectRef>, JjObjectStoreError> {
        let result = match self.kv.read(aspen_core::ReadRequest::new(jj_reachability_key(repo_id, object_id))).await {
            Ok(result) => result,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(source) => {
                return Err(JjObjectStoreError::Kv {
                    message: source.to_string(),
                });
            }
        };
        let Some(kv) = result.kv else {
            return Ok(None);
        };
        let stored_ref = serde_json::from_str(&kv.value).map_err(|source| JjObjectStoreError::Kv {
            message: source.to_string(),
        })?;
        Ok(Some(stored_ref))
    }
}

impl JjObjectEnvelope {
    /// Construct an envelope using the current encoding version.
    #[must_use]
    pub fn current(
        kind: JjObjectKind,
        repo_id: impl Into<String>,
        object_id: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: JJ_OBJECT_ENCODING_VERSION,
            kind,
            repo_id: repo_id.into(),
            object_id: object_id.into(),
            change_ids: Vec::new(),
            parents: Vec::new(),
            payload,
        }
    }
}

/// Build the repo-scoped reachability key for a JJ object.
#[must_use]
pub fn jj_reachability_key(repo_id: &str, object_id: &str) -> String {
    format!("{JJ_REACHABILITY_KEY_PREFIX}{repo_id}:{object_id}")
}

/// Validate a JJ object envelope without encoding it.
pub fn validate_jj_object(envelope: &JjObjectEnvelope) -> Result<(), JjObjectEncodingError> {
    if envelope.version != JJ_OBJECT_ENCODING_VERSION {
        return Err(JjObjectEncodingError::UnsupportedVersion {
            version: envelope.version,
        });
    }
    if envelope.repo_id.is_empty() {
        return Err(JjObjectEncodingError::EmptyField { field: "repo_id" });
    }
    if envelope.object_id.is_empty() {
        return Err(JjObjectEncodingError::EmptyField { field: "object_id" });
    }
    if envelope.parents.len() > MAX_JJ_OBJECT_PARENTS {
        return Err(JjObjectEncodingError::TooManyParents {
            actual: envelope.parents.len(),
            max: MAX_JJ_OBJECT_PARENTS,
        });
    }
    if envelope.change_ids.len() > MAX_JJ_OBJECT_CHANGE_IDS {
        return Err(JjObjectEncodingError::TooManyChangeIds {
            actual: envelope.change_ids.len(),
            max: MAX_JJ_OBJECT_CHANGE_IDS,
        });
    }
    if envelope.payload.len() > MAX_JJ_OBJECT_PAYLOAD_BYTES {
        return Err(JjObjectEncodingError::PayloadTooLarge {
            actual: envelope.payload.len(),
            max: MAX_JJ_OBJECT_PAYLOAD_BYTES,
        });
    }

    Ok(())
}

/// Encode a JJ object envelope with a magic prefix and BLAKE3 digest.
pub fn encode_jj_object(envelope: &JjObjectEnvelope) -> Result<EncodedJjObject, JjObjectEncodingError> {
    validate_jj_object(envelope)?;

    let mut bytes = Vec::with_capacity(JJ_OBJECT_MAGIC.len().saturating_add(envelope.payload.len()));
    bytes.extend_from_slice(JJ_OBJECT_MAGIC);
    let mut encoded = postcard::to_allocvec(envelope).map_err(|source| JjObjectEncodingError::Serialize {
        message: source.to_string(),
    })?;
    bytes.append(&mut encoded);
    let digest = *blake3::hash(&bytes).as_bytes();

    Ok(EncodedJjObject { bytes, digest })
}

/// Decode and validate a JJ object envelope from blob bytes.
pub fn decode_jj_object(bytes: &[u8]) -> Result<JjObjectEnvelope, JjObjectEncodingError> {
    if !bytes.starts_with(JJ_OBJECT_MAGIC) {
        return Err(JjObjectEncodingError::InvalidMagic);
    }
    let payload = &bytes[JJ_OBJECT_MAGIC.len()..];
    let envelope: JjObjectEnvelope =
        postcard::from_bytes(payload).map_err(|source| JjObjectEncodingError::Deserialize {
            message: source.to_string(),
        })?;
    validate_jj_object(&envelope)?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_REPO_ID: &str = "repo-1";
    const TEST_OBJECT_ID: &str = "object-1";
    const TEST_CHANGE_ID: &str = "change-1";
    const TEST_PARENT_ID: &str = "parent-1";
    const TEST_PAYLOAD: &[u8] = b"native-jj-payload";
    const UNSUPPORTED_VERSION: u16 = JJ_OBJECT_ENCODING_VERSION + 1;

    fn object() -> JjObjectEnvelope {
        let mut envelope =
            JjObjectEnvelope::current(JjObjectKind::Commit, TEST_REPO_ID, TEST_OBJECT_ID, TEST_PAYLOAD.to_vec());
        envelope.change_ids.push(TEST_CHANGE_ID.to_string());
        envelope.parents.push(TEST_PARENT_ID.to_string());
        envelope
    }

    fn object_store() -> JjObjectStore<dyn aspen_core::KeyValueStore, aspen_blob::InMemoryBlobStore> {
        let kv: Arc<dyn aspen_core::KeyValueStore> = Arc::new(aspen_testing_core::DeterministicKeyValueStore::new());
        let blobs = Arc::new(aspen_blob::InMemoryBlobStore::new());
        JjObjectStore::new(kv, blobs)
    }

    #[test]
    fn jj_reachability_key_is_repo_scoped() {
        let key = jj_reachability_key(TEST_REPO_ID, TEST_OBJECT_ID);

        assert_eq!(key, "forge:jj:reach:repo-1:object-1");
    }

    #[test]
    fn jj_object_encode_decode_roundtrip_preserves_native_payload() {
        let envelope = object();

        let encoded = encode_jj_object(&envelope).expect("object encodes");
        let decoded = decode_jj_object(&encoded.bytes).expect("object decodes");

        assert_eq!(decoded, envelope);
        assert_eq!(encoded.digest, *blake3::hash(&encoded.bytes).as_bytes());
    }

    #[test]
    fn jj_object_decode_rejects_invalid_magic() {
        let err = decode_jj_object(TEST_PAYLOAD).expect_err("invalid magic rejected");

        assert_eq!(err, JjObjectEncodingError::InvalidMagic);
    }

    #[test]
    fn jj_object_validate_rejects_empty_object_id() {
        let mut envelope = object();
        envelope.object_id.clear();

        let err = validate_jj_object(&envelope).expect_err("empty object ID rejected");

        assert_eq!(err, JjObjectEncodingError::EmptyField { field: "object_id" });
    }

    #[tokio::test]
    async fn jj_object_store_persists_blob_and_reachability_index() {
        let store = object_store();
        let envelope = object();

        let stored = store.store_object(&envelope).await.expect("object stores");
        let found = store
            .lookup_object(TEST_REPO_ID, TEST_OBJECT_ID)
            .await
            .expect("reachability lookup succeeds")
            .expect("object is indexed");

        assert_eq!(found.repo_id, TEST_REPO_ID);
        assert_eq!(found.object_id, TEST_OBJECT_ID);
        assert_eq!(found.blob_hash, stored.blob_hash);
        assert!(found.encoded_size_bytes > 0);
    }

    #[tokio::test]
    async fn jj_object_store_returns_none_for_missing_reachability() {
        let store = object_store();

        let found = store.lookup_object(TEST_REPO_ID, "missing-object").await.expect("lookup succeeds");

        assert!(found.is_none());
    }

    #[test]
    fn jj_object_validate_rejects_unsupported_version() {
        let mut envelope = object();
        envelope.version = UNSUPPORTED_VERSION;

        let err = validate_jj_object(&envelope).expect_err("unsupported version rejected");

        assert_eq!(err, JjObjectEncodingError::UnsupportedVersion {
            version: UNSUPPORTED_VERSION
        });
    }
}
