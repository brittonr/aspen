//! Native Jujutsu object envelope and blob encoding.
//!
//! This module is pure: it validates JJ object envelopes and converts them to
//! deterministic BLAKE3-addressed blob bytes. Persistence and consensus indexes
//! live in the Forge shell.

use std::collections::BTreeSet;
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
/// Maximum JJ objects validated in one publish graph.
pub const MAX_JJ_OBJECT_GRAPH_OBJECTS: usize = 10_000;
/// KV prefix for repo-scoped JJ object reachability indexes.
pub const JJ_REACHABILITY_KEY_PREFIX: &str = "forge:jj:reach:";
/// KV prefix for repo-scoped JJ bookmarks.
pub const JJ_BOOKMARK_KEY_PREFIX: &str = "forge:jj:bookmark:";
/// KV prefix for repo-scoped JJ change-id heads.
pub const JJ_CHANGE_HEAD_KEY_PREFIX: &str = "forge:jj:change:";
/// KV prefix for repo-scoped JJ staged session records.
pub const JJ_STAGED_SESSION_KEY_PREFIX: &str = "forge:jj:stage:";

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

/// JJ bookmark state stored under a repo-scoped namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjBookmarkRecord {
    /// Repo identifier this bookmark belongs to.
    pub repo_id: String,
    /// Bookmark name.
    pub name: String,
    /// Current JJ object head, or None for a deletion tombstone.
    pub head_object_id: Option<String>,
}

/// JJ change-id head stored under a repo-scoped namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjChangeHeadRecord {
    /// Repo identifier this change ID belongs to.
    pub repo_id: String,
    /// JJ change ID.
    pub change_id: String,
    /// Current JJ object head.
    pub head_object_id: String,
}

/// Mutable JJ head kind used for conflict reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JjHeadKind {
    /// JJ bookmark head.
    Bookmark,
    /// JJ change-id head.
    ChangeId,
}

/// Conflict detected before final JJ publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjPublishConflict {
    /// Conflicting head kind.
    pub kind: JjHeadKind,
    /// Bookmark name or change ID.
    pub name: String,
    /// Caller-supplied expected head.
    pub expected: Option<String>,
    /// Current authoritative head.
    pub actual: Option<String>,
}

/// Bookmark update staged for final publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjBookmarkUpdate {
    /// Bookmark name.
    pub name: String,
    /// Expected current head.
    pub expected_head: Option<String>,
    /// New head, or None to delete the bookmark.
    pub new_head: Option<String>,
}

/// Change-id head update staged for final publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjChangeHeadUpdate {
    /// JJ change ID.
    pub change_id: String,
    /// Expected current head.
    pub expected_head: Option<String>,
    /// New head.
    pub new_head: String,
}

/// Staged JJ session record used for quota and cleanup decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjStagedSessionRecord {
    /// Repo identifier.
    pub repo_id: String,
    /// Session identifier.
    pub session_id: String,
    /// Total staged bytes reserved by this session.
    pub staged_size_bytes: u64,
    /// Absolute expiry time in milliseconds.
    pub expires_at_ms: u64,
}

/// Staged-data quota for a JJ protocol session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjStagedQuota {
    /// Maximum staged bytes permitted for a session.
    pub max_staged_size_bytes: u64,
}

/// Staged JJ publish payload applied atomically at repo-visible heads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjStagedPublish {
    /// Optional staged session ID to clean up after publish/rejection.
    pub session_id: Option<String>,
    /// Repo identifier.
    pub repo_id: String,
    /// Object envelopes to store before head publication.
    pub objects: Vec<JjObjectEnvelope>,
    /// Already-reachable object IDs accepted as graph parents.
    pub known_object_ids: Vec<String>,
    /// Bookmark moves/deletes.
    pub bookmark_updates: Vec<JjBookmarkUpdate>,
    /// Change-id head updates.
    pub change_head_updates: Vec<JjChangeHeadUpdate>,
}

/// Result of a successful staged JJ publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjPublishReceipt {
    /// Stored object references.
    pub stored_objects: Vec<JjStoredObjectRef>,
    /// Published bookmark count.
    pub bookmark_updates: usize,
    /// Published change-head count.
    pub change_head_updates: usize,
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

/// JJ object graph validation failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JjObjectGraphError {
    /// Publish graph is too large.
    #[error("JJ object graph has {actual} objects, max {max}")]
    TooManyObjects { actual: usize, max: usize },
    /// Object envelope itself is invalid.
    #[error("JJ object '{object_id}' is invalid: {source}")]
    InvalidObject {
        object_id: String,
        source: JjObjectEncodingError,
    },
    /// Duplicate object ID in the same publish graph.
    #[error("duplicate JJ object ID '{object_id}'")]
    DuplicateObject { object_id: String },
    /// Parent reference is not in the graph or known existing set.
    #[error("JJ object '{object_id}' references missing parent '{parent_id}'")]
    MissingParent { object_id: String, parent_id: String },
    /// Change ID is empty.
    #[error("JJ object '{object_id}' has an empty change ID")]
    EmptyChangeId { object_id: String },
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
    /// Publish graph validation failed.
    #[error("JJ publish graph validation failed: {source}")]
    Graph { source: JjObjectGraphError },
    /// Final publish detected a stale head.
    #[error("JJ publish conflict on {conflict:?}")]
    Conflict { conflict: JjPublishConflict },
    /// Staged data exceeds quota.
    #[error("JJ staged data uses {used_bytes} bytes, max {max_bytes}")]
    QuotaExceeded { used_bytes: u64, max_bytes: u64 },
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

    /// Create or move a JJ bookmark through the KV namespace.
    pub async fn put_bookmark(
        &self,
        repo_id: &str,
        name: &str,
        head_object_id: &str,
    ) -> Result<JjBookmarkRecord, JjObjectStoreError> {
        let record = JjBookmarkRecord {
            repo_id: repo_id.to_string(),
            name: name.to_string(),
            head_object_id: Some(head_object_id.to_string()),
        };
        self.write_json(jj_bookmark_key(repo_id, name), &record).await?;
        Ok(record)
    }

    /// Delete a JJ bookmark from the KV namespace.
    pub async fn delete_bookmark(&self, repo_id: &str, name: &str) -> Result<bool, JjObjectStoreError> {
        self.delete_key(jj_bookmark_key(repo_id, name)).await
    }

    /// Read a JJ bookmark from the KV namespace.
    pub async fn get_bookmark(
        &self,
        repo_id: &str,
        name: &str,
    ) -> Result<Option<JjBookmarkRecord>, JjObjectStoreError> {
        self.read_json(jj_bookmark_key(repo_id, name)).await
    }

    /// Upsert a JJ change-id head through the KV namespace.
    pub async fn put_change_head(
        &self,
        repo_id: &str,
        change_id: &str,
        head_object_id: &str,
    ) -> Result<JjChangeHeadRecord, JjObjectStoreError> {
        let record = JjChangeHeadRecord {
            repo_id: repo_id.to_string(),
            change_id: change_id.to_string(),
            head_object_id: head_object_id.to_string(),
        };
        self.write_json(jj_change_head_key(repo_id, change_id), &record).await?;
        Ok(record)
    }

    /// Read a JJ change-id head from the KV namespace.
    pub async fn get_change_head(
        &self,
        repo_id: &str,
        change_id: &str,
    ) -> Result<Option<JjChangeHeadRecord>, JjObjectStoreError> {
        self.read_json(jj_change_head_key(repo_id, change_id)).await
    }

    /// Check the expected bookmark head before final publish.
    pub async fn check_bookmark_conflict(
        &self,
        repo_id: &str,
        name: &str,
        expected: Option<&str>,
    ) -> Result<Option<JjPublishConflict>, JjObjectStoreError> {
        let actual = self.get_bookmark(repo_id, name).await?.and_then(|record| record.head_object_id);
        Ok(conflict_if_head_mismatch(JjHeadKind::Bookmark, name, expected, actual.as_deref()))
    }

    /// Check the expected change-id head before final publish.
    pub async fn check_change_head_conflict(
        &self,
        repo_id: &str,
        change_id: &str,
        expected: Option<&str>,
    ) -> Result<Option<JjPublishConflict>, JjObjectStoreError> {
        let actual = self.get_change_head(repo_id, change_id).await?.map(|record| record.head_object_id);
        Ok(conflict_if_head_mismatch(JjHeadKind::ChangeId, change_id, expected, actual.as_deref()))
    }

    /// Publish a fully staged JJ payload after validation and conflict checks.
    pub async fn publish_staged(&self, staged: &JjStagedPublish) -> Result<JjPublishReceipt, JjObjectStoreError> {
        if let Err(source) = validate_jj_object_graph(&staged.objects, &staged.known_object_ids) {
            self.cleanup_staged_publish_session(staged).await?;
            return Err(JjObjectStoreError::Graph { source });
        }
        if let Err(error) = self.check_staged_conflicts(staged).await {
            self.cleanup_staged_publish_session(staged).await?;
            return Err(error);
        }

        let mut stored_objects = Vec::with_capacity(staged.objects.len());
        for object in &staged.objects {
            stored_objects.push(self.store_object(object).await?);
        }
        for update in &staged.bookmark_updates {
            self.apply_bookmark_update(&staged.repo_id, update).await?;
        }
        for update in &staged.change_head_updates {
            self.put_change_head(&staged.repo_id, &update.change_id, &update.new_head).await?;
        }
        self.cleanup_staged_publish_session(staged).await?;

        Ok(JjPublishReceipt {
            stored_objects,
            bookmark_updates: staged.bookmark_updates.len(),
            change_head_updates: staged.change_head_updates.len(),
        })
    }

    /// Record a staged session if it is within quota.
    pub async fn put_staged_session(
        &self,
        record: &JjStagedSessionRecord,
        quota: JjStagedQuota,
    ) -> Result<(), JjObjectStoreError> {
        enforce_staged_quota(record.staged_size_bytes, quota)?;
        self.write_json(jj_staged_session_key(&record.repo_id, &record.session_id), record).await
    }

    /// Read a staged session record.
    pub async fn get_staged_session(
        &self,
        repo_id: &str,
        session_id: &str,
    ) -> Result<Option<JjStagedSessionRecord>, JjObjectStoreError> {
        self.read_json(jj_staged_session_key(repo_id, session_id)).await
    }

    /// Delete a staged session record and return whether it existed.
    pub async fn delete_staged_session(&self, repo_id: &str, session_id: &str) -> Result<bool, JjObjectStoreError> {
        self.delete_key(jj_staged_session_key(repo_id, session_id)).await
    }

    /// Delete an expired staged session record.
    pub async fn cleanup_expired_staged_session(
        &self,
        repo_id: &str,
        session_id: &str,
        now_ms: u64,
    ) -> Result<bool, JjObjectStoreError> {
        let Some(record) = self.get_staged_session(repo_id, session_id).await? else {
            return Ok(false);
        };
        if !is_staged_session_expired(&record, now_ms) {
            return Ok(false);
        }
        self.delete_staged_session(repo_id, session_id).await
    }

    async fn cleanup_staged_publish_session(&self, staged: &JjStagedPublish) -> Result<(), JjObjectStoreError> {
        if let Some(session_id) = &staged.session_id {
            self.delete_staged_session(&staged.repo_id, session_id).await?;
        }
        Ok(())
    }

    async fn check_staged_conflicts(&self, staged: &JjStagedPublish) -> Result<(), JjObjectStoreError> {
        for update in &staged.bookmark_updates {
            if let Some(conflict) =
                self.check_bookmark_conflict(&staged.repo_id, &update.name, update.expected_head.as_deref()).await?
            {
                return Err(JjObjectStoreError::Conflict { conflict });
            }
        }
        for update in &staged.change_head_updates {
            if let Some(conflict) = self
                .check_change_head_conflict(&staged.repo_id, &update.change_id, update.expected_head.as_deref())
                .await?
            {
                return Err(JjObjectStoreError::Conflict { conflict });
            }
        }
        Ok(())
    }

    async fn apply_bookmark_update(&self, repo_id: &str, update: &JjBookmarkUpdate) -> Result<(), JjObjectStoreError> {
        match update.new_head.as_deref() {
            Some(head) => {
                self.put_bookmark(repo_id, &update.name, head).await?;
            }
            None => {
                self.delete_bookmark(repo_id, &update.name).await?;
            }
        }
        Ok(())
    }

    async fn write_json<T: Serialize>(&self, key: String, value: &T) -> Result<(), JjObjectStoreError> {
        let json = serde_json::to_string(value).map_err(|source| JjObjectStoreError::Kv {
            message: source.to_string(),
        })?;
        self.kv
            .write(aspen_core::WriteRequest::set(key, json))
            .await
            .map_err(|source| JjObjectStoreError::Kv {
                message: source.to_string(),
            })?;
        Ok(())
    }

    async fn read_json<T: for<'de> Deserialize<'de>>(&self, key: String) -> Result<Option<T>, JjObjectStoreError> {
        let result = match self.kv.read(aspen_core::ReadRequest::new(key)).await {
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
        serde_json::from_str(&kv.value).map(Some).map_err(|source| JjObjectStoreError::Kv {
            message: source.to_string(),
        })
    }

    async fn delete_key(&self, key: String) -> Result<bool, JjObjectStoreError> {
        match self.kv.delete(aspen_core::DeleteRequest::new(key)).await {
            Ok(result) => Ok(result.is_deleted),
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(source) => Err(JjObjectStoreError::Kv {
                message: source.to_string(),
            }),
        }
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

/// Build the repo-scoped key for a JJ bookmark.
#[must_use]
pub fn jj_bookmark_key(repo_id: &str, bookmark_name: &str) -> String {
    format!("{JJ_BOOKMARK_KEY_PREFIX}{repo_id}:{bookmark_name}")
}

/// Build the repo-scoped key for a JJ change-id head.
#[must_use]
pub fn jj_change_head_key(repo_id: &str, change_id: &str) -> String {
    format!("{JJ_CHANGE_HEAD_KEY_PREFIX}{repo_id}:{change_id}")
}

/// Build the repo-scoped key for a staged JJ session.
#[must_use]
pub fn jj_staged_session_key(repo_id: &str, session_id: &str) -> String {
    format!("{JJ_STAGED_SESSION_KEY_PREFIX}{repo_id}:{session_id}")
}

/// Return true when a staged session is expired at `now_ms`.
#[must_use]
pub fn is_staged_session_expired(record: &JjStagedSessionRecord, now_ms: u64) -> bool {
    now_ms >= record.expires_at_ms
}

/// Enforce staged-data quota for a session.
pub fn enforce_staged_quota(staged_size_bytes: u64, quota: JjStagedQuota) -> Result<(), JjObjectStoreError> {
    if staged_size_bytes <= quota.max_staged_size_bytes {
        return Ok(());
    }
    Err(JjObjectStoreError::QuotaExceeded {
        used_bytes: staged_size_bytes,
        max_bytes: quota.max_staged_size_bytes,
    })
}

/// Validate one JJ publish graph against already-known object IDs.
pub fn validate_jj_object_graph(
    objects: &[JjObjectEnvelope],
    known_object_ids: &[String],
) -> Result<(), JjObjectGraphError> {
    if objects.len() > MAX_JJ_OBJECT_GRAPH_OBJECTS {
        return Err(JjObjectGraphError::TooManyObjects {
            actual: objects.len(),
            max: MAX_JJ_OBJECT_GRAPH_OBJECTS,
        });
    }

    let mut object_ids = BTreeSet::new();
    for object in objects {
        validate_jj_object(object).map_err(|source| JjObjectGraphError::InvalidObject {
            object_id: object.object_id.clone(),
            source,
        })?;
        if !object_ids.insert(object.object_id.as_str()) {
            return Err(JjObjectGraphError::DuplicateObject {
                object_id: object.object_id.clone(),
            });
        }
        if object.change_ids.iter().any(|change_id| change_id.is_empty()) {
            return Err(JjObjectGraphError::EmptyChangeId {
                object_id: object.object_id.clone(),
            });
        }
    }

    let known_ids: BTreeSet<&str> = known_object_ids.iter().map(String::as_str).collect();
    for object in objects {
        for parent in &object.parents {
            if object_ids.contains(parent.as_str()) || known_ids.contains(parent.as_str()) {
                continue;
            }
            return Err(JjObjectGraphError::MissingParent {
                object_id: object.object_id.clone(),
                parent_id: parent.clone(),
            });
        }
    }

    Ok(())
}

/// Return a conflict when caller expectation differs from authoritative head.
#[must_use]
pub fn conflict_if_head_mismatch(
    kind: JjHeadKind,
    name: &str,
    expected: Option<&str>,
    actual: Option<&str>,
) -> Option<JjPublishConflict> {
    if expected == actual {
        return None;
    }

    Some(JjPublishConflict {
        kind,
        name: name.to_string(),
        expected: expected.map(ToString::to_string),
        actual: actual.map(ToString::to_string),
    })
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
    const TEST_BOOKMARK: &str = "main";
    const TEST_PARENT_ID: &str = "parent-1";
    const TEST_SESSION_ID: &str = "session-1";
    const TEST_STAGED_BYTES: u64 = 512;
    const TEST_MAX_STAGED_BYTES: u64 = 1_024;
    const TEST_SESSION_TIMEOUT_MS: u64 = 30_000;
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

    fn staged_session() -> JjStagedSessionRecord {
        JjStagedSessionRecord {
            repo_id: TEST_REPO_ID.to_string(),
            session_id: TEST_SESSION_ID.to_string(),
            staged_size_bytes: TEST_STAGED_BYTES,
            expires_at_ms: TEST_SESSION_TIMEOUT_MS,
        }
    }

    fn staged_quota() -> JjStagedQuota {
        JjStagedQuota {
            max_staged_size_bytes: TEST_MAX_STAGED_BYTES,
        }
    }

    #[test]
    fn jj_reachability_key_is_repo_scoped() {
        let key = jj_reachability_key(TEST_REPO_ID, TEST_OBJECT_ID);

        assert_eq!(key, "forge:jj:reach:repo-1:object-1");
    }

    #[test]
    fn jj_bookmark_and_change_keys_are_repo_scoped() {
        let bookmark_key = jj_bookmark_key(TEST_REPO_ID, TEST_BOOKMARK);
        let change_key = jj_change_head_key(TEST_REPO_ID, TEST_CHANGE_ID);

        assert_eq!(bookmark_key, "forge:jj:bookmark:repo-1:main");
        assert_eq!(change_key, "forge:jj:change:repo-1:change-1");
    }

    #[test]
    fn jj_object_graph_accepts_known_parent() {
        let mut envelope = object();
        envelope.parents = vec![TEST_PARENT_ID.to_string()];
        let known = vec![TEST_PARENT_ID.to_string()];

        let result = validate_jj_object_graph(&[envelope], &known);

        assert!(result.is_ok());
    }

    #[test]
    fn jj_object_graph_rejects_missing_parent() {
        let envelope = object();

        let err = validate_jj_object_graph(&[envelope], &[]).expect_err("missing parent rejected");

        assert_eq!(err, JjObjectGraphError::MissingParent {
            object_id: TEST_OBJECT_ID.to_string(),
            parent_id: TEST_PARENT_ID.to_string(),
        });
    }

    #[test]
    fn jj_object_graph_rejects_duplicate_object_id() {
        let envelope = object();

        let err = validate_jj_object_graph(&[envelope.clone(), envelope], &[]).expect_err("duplicate object rejected");

        assert_eq!(err, JjObjectGraphError::DuplicateObject {
            object_id: TEST_OBJECT_ID.to_string(),
        });
    }

    #[test]
    fn jj_object_graph_rejects_empty_change_id() {
        let mut envelope = object();
        envelope.change_ids = vec![String::new()];
        envelope.parents.clear();

        let err = validate_jj_object_graph(&[envelope], &[]).expect_err("empty change ID rejected");

        assert_eq!(err, JjObjectGraphError::EmptyChangeId {
            object_id: TEST_OBJECT_ID.to_string(),
        });
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

    #[tokio::test]
    async fn jj_object_store_moves_and_deletes_bookmarks() {
        let store = object_store();

        let first = store.put_bookmark(TEST_REPO_ID, TEST_BOOKMARK, TEST_OBJECT_ID).await.expect("bookmark creates");
        let moved = store.put_bookmark(TEST_REPO_ID, TEST_BOOKMARK, TEST_PARENT_ID).await.expect("bookmark moves");
        let found = store
            .get_bookmark(TEST_REPO_ID, TEST_BOOKMARK)
            .await
            .expect("bookmark lookup succeeds")
            .expect("bookmark exists");
        let was_deleted = store.delete_bookmark(TEST_REPO_ID, TEST_BOOKMARK).await.expect("bookmark deletes");
        let missing = store.get_bookmark(TEST_REPO_ID, TEST_BOOKMARK).await.expect("bookmark lookup succeeds");

        assert_eq!(first.head_object_id.as_deref(), Some(TEST_OBJECT_ID));
        assert_eq!(moved.head_object_id.as_deref(), Some(TEST_PARENT_ID));
        assert_eq!(found.head_object_id.as_deref(), Some(TEST_PARENT_ID));
        assert!(was_deleted);
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn jj_object_store_records_change_id_heads() {
        let store = object_store();

        let record = store
            .put_change_head(TEST_REPO_ID, TEST_CHANGE_ID, TEST_OBJECT_ID)
            .await
            .expect("change head writes");
        let found = store
            .get_change_head(TEST_REPO_ID, TEST_CHANGE_ID)
            .await
            .expect("change head lookup succeeds")
            .expect("change head exists");

        assert_eq!(record.head_object_id, TEST_OBJECT_ID);
        assert_eq!(found.change_id, TEST_CHANGE_ID);
        assert_eq!(found.head_object_id, TEST_OBJECT_ID);
    }

    #[tokio::test]
    async fn jj_object_store_records_and_expires_staged_sessions() {
        let store = object_store();
        let session = staged_session();

        store.put_staged_session(&session, staged_quota()).await.expect("staged session stores");
        let before_expiry = store
            .cleanup_expired_staged_session(TEST_REPO_ID, TEST_SESSION_ID, TEST_SESSION_TIMEOUT_MS - 1)
            .await
            .expect("cleanup checks expiry");
        let after_expiry = store
            .cleanup_expired_staged_session(TEST_REPO_ID, TEST_SESSION_ID, TEST_SESSION_TIMEOUT_MS)
            .await
            .expect("expired session cleans up");
        let missing = store
            .get_staged_session(TEST_REPO_ID, TEST_SESSION_ID)
            .await
            .expect("staged session lookup succeeds");

        assert!(!before_expiry);
        assert!(after_expiry);
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn jj_object_store_rejects_staged_session_over_quota() {
        let store = object_store();
        let mut session = staged_session();
        session.staged_size_bytes = TEST_MAX_STAGED_BYTES + 1;

        let err = store
            .put_staged_session(&session, staged_quota())
            .await
            .expect_err("oversized staged session rejected");

        assert!(matches!(err, JjObjectStoreError::QuotaExceeded { .. }));
    }

    #[tokio::test]
    async fn jj_object_store_publish_staged_cleans_session_after_success() {
        let store = object_store();
        store.put_staged_session(&staged_session(), staged_quota()).await.expect("staged session stores");
        let staged = JjStagedPublish {
            session_id: Some(TEST_SESSION_ID.to_string()),
            repo_id: TEST_REPO_ID.to_string(),
            objects: vec![object()],
            known_object_ids: vec![TEST_PARENT_ID.to_string()],
            bookmark_updates: Vec::new(),
            change_head_updates: Vec::new(),
        };

        store.publish_staged(&staged).await.expect("staged publish succeeds");
        let missing = store
            .get_staged_session(TEST_REPO_ID, TEST_SESSION_ID)
            .await
            .expect("staged session lookup succeeds");

        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn jj_object_store_publish_staged_cleans_session_after_rejection() {
        let store = object_store();
        store.put_staged_session(&staged_session(), staged_quota()).await.expect("staged session stores");
        let staged = JjStagedPublish {
            session_id: Some(TEST_SESSION_ID.to_string()),
            repo_id: TEST_REPO_ID.to_string(),
            objects: vec![object()],
            known_object_ids: Vec::new(),
            bookmark_updates: Vec::new(),
            change_head_updates: Vec::new(),
        };

        let err = store.publish_staged(&staged).await.expect_err("bad graph rejected");
        let missing = store
            .get_staged_session(TEST_REPO_ID, TEST_SESSION_ID)
            .await
            .expect("staged session lookup succeeds");

        assert!(matches!(err, JjObjectStoreError::Graph { .. }));
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn jj_object_store_publish_staged_updates_heads_after_validation() {
        let store = object_store();
        let staged = JjStagedPublish {
            session_id: None,
            repo_id: TEST_REPO_ID.to_string(),
            objects: vec![object()],
            known_object_ids: vec![TEST_PARENT_ID.to_string()],
            bookmark_updates: vec![JjBookmarkUpdate {
                name: TEST_BOOKMARK.to_string(),
                expected_head: None,
                new_head: Some(TEST_OBJECT_ID.to_string()),
            }],
            change_head_updates: vec![JjChangeHeadUpdate {
                change_id: TEST_CHANGE_ID.to_string(),
                expected_head: None,
                new_head: TEST_OBJECT_ID.to_string(),
            }],
        };

        let receipt = store.publish_staged(&staged).await.expect("staged publish succeeds");
        let bookmark = store
            .get_bookmark(TEST_REPO_ID, TEST_BOOKMARK)
            .await
            .expect("bookmark lookup succeeds")
            .expect("bookmark exists");
        let change = store
            .get_change_head(TEST_REPO_ID, TEST_CHANGE_ID)
            .await
            .expect("change lookup succeeds")
            .expect("change exists");

        assert_eq!(receipt.stored_objects.len(), 1);
        assert_eq!(receipt.bookmark_updates, 1);
        assert_eq!(receipt.change_head_updates, 1);
        assert_eq!(bookmark.head_object_id.as_deref(), Some(TEST_OBJECT_ID));
        assert_eq!(change.head_object_id, TEST_OBJECT_ID);
    }

    #[tokio::test]
    async fn jj_object_store_publish_staged_rejects_conflict_without_changing_heads() {
        let store = object_store();
        store.put_bookmark(TEST_REPO_ID, TEST_BOOKMARK, TEST_PARENT_ID).await.expect("seed bookmark writes");
        let staged = JjStagedPublish {
            session_id: None,
            repo_id: TEST_REPO_ID.to_string(),
            objects: vec![object()],
            known_object_ids: vec![TEST_PARENT_ID.to_string()],
            bookmark_updates: vec![JjBookmarkUpdate {
                name: TEST_BOOKMARK.to_string(),
                expected_head: Some(TEST_OBJECT_ID.to_string()),
                new_head: Some(TEST_OBJECT_ID.to_string()),
            }],
            change_head_updates: Vec::new(),
        };

        let err = store.publish_staged(&staged).await.expect_err("conflict rejected");
        let bookmark = store
            .get_bookmark(TEST_REPO_ID, TEST_BOOKMARK)
            .await
            .expect("bookmark lookup succeeds")
            .expect("bookmark remains");

        assert!(matches!(err, JjObjectStoreError::Conflict { .. }));
        assert_eq!(bookmark.head_object_id.as_deref(), Some(TEST_PARENT_ID));
    }

    #[tokio::test]
    async fn jj_object_store_detects_stale_bookmark_conflict() {
        let store = object_store();
        store.put_bookmark(TEST_REPO_ID, TEST_BOOKMARK, TEST_OBJECT_ID).await.expect("bookmark writes");

        let ok = store
            .check_bookmark_conflict(TEST_REPO_ID, TEST_BOOKMARK, Some(TEST_OBJECT_ID))
            .await
            .expect("conflict check succeeds");
        let conflict = store
            .check_bookmark_conflict(TEST_REPO_ID, TEST_BOOKMARK, Some(TEST_PARENT_ID))
            .await
            .expect("conflict check succeeds")
            .expect("stale expected head conflicts");

        assert!(ok.is_none());
        assert_eq!(conflict.kind, JjHeadKind::Bookmark);
        assert_eq!(conflict.expected.as_deref(), Some(TEST_PARENT_ID));
        assert_eq!(conflict.actual.as_deref(), Some(TEST_OBJECT_ID));
    }

    #[tokio::test]
    async fn jj_object_store_detects_stale_change_head_conflict() {
        let store = object_store();
        store
            .put_change_head(TEST_REPO_ID, TEST_CHANGE_ID, TEST_OBJECT_ID)
            .await
            .expect("change head writes");

        let conflict = store
            .check_change_head_conflict(TEST_REPO_ID, TEST_CHANGE_ID, None)
            .await
            .expect("conflict check succeeds")
            .expect("missing expected head conflicts");

        assert_eq!(conflict.kind, JjHeadKind::ChangeId);
        assert!(conflict.expected.is_none());
        assert_eq!(conflict.actual.as_deref(), Some(TEST_OBJECT_ID));
    }

    #[test]
    fn jj_conflict_helper_accepts_matching_absent_heads() {
        let conflict = conflict_if_head_mismatch(JjHeadKind::Bookmark, TEST_BOOKMARK, None, None);

        assert!(conflict.is_none());
    }

    #[tokio::test]
    async fn jj_object_store_returns_none_for_missing_bookmark_and_change() {
        let store = object_store();

        let bookmark = store.get_bookmark(TEST_REPO_ID, TEST_BOOKMARK).await.expect("bookmark lookup succeeds");
        let change = store.get_change_head(TEST_REPO_ID, TEST_CHANGE_ID).await.expect("change head lookup succeeds");

        assert!(bookmark.is_none());
        assert!(change.is_none());
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
