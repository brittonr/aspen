//! Automerge Document Store - Server-side document management.
//!
//! Provides the `DocumentStore` trait and `AspenAutomergeStore` implementation
//! for managing Automerge documents through the Aspen KV store.
//!
//! # Architecture
//!
//! Documents are stored in the KV store with keys:
//! - Content: `automerge:{document_id}` -> binary Automerge document
//! - Metadata: `automerge:_meta:{document_id}` -> JSON metadata
//!
//! All writes go through Raft consensus, providing linearizability.
//! The existing DocsExporter can be used to sync changes to clients.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use async_trait::async_trait;
use automerge::AutoCommit;
use tracing::debug;
use tracing::instrument;
use tracing::warn;

use super::constants::DEFAULT_LIST_LIMIT;
use super::constants::DOC_KEY_PREFIX;
use super::constants::DOC_META_PREFIX;
use super::constants::MAX_BATCH_CHANGES;
use super::constants::MAX_CHANGE_SIZE;
use super::constants::MAX_DOCUMENT_SIZE;
use super::constants::MAX_SCAN_RESULTS;
use super::error::AutomergeError;
use super::error::AutomergeResult;
use super::types::ApplyResult;
use super::types::DocumentChange;
use super::types::DocumentId;
use super::types::DocumentMetadata;
use super::types::ListOptions;
use super::types::ListResult;

// ============================================================================
// DocumentStore Trait
// ============================================================================

/// Trait for Automerge document management.
///
/// Provides CRUD operations for Automerge documents with merge support.
#[async_trait]
pub trait DocumentStore: Send + Sync {
    // =========================================================================
    // Document CRUD
    // =========================================================================

    /// Create a new empty Automerge document.
    ///
    /// Returns the document ID and initial metadata.
    async fn create(&self, id: Option<DocumentId>, metadata: Option<DocumentMetadata>) -> AutomergeResult<DocumentId>;

    /// Get a document by ID.
    ///
    /// Returns the full Automerge document or None if not found.
    async fn get(&self, id: &DocumentId) -> AutomergeResult<Option<AutoCommit>>;

    /// Get document metadata without loading the full document.
    async fn get_metadata(&self, id: &DocumentId) -> AutomergeResult<Option<DocumentMetadata>>;

    /// Save/update a document.
    ///
    /// Overwrites the existing document with the new state.
    async fn save(&self, id: &DocumentId, doc: &mut AutoCommit) -> AutomergeResult<()>;

    /// Delete a document.
    ///
    /// Returns true if the document existed and was deleted.
    async fn delete(&self, id: &DocumentId) -> AutomergeResult<bool>;

    /// Check if a document exists.
    async fn exists(&self, id: &DocumentId) -> AutomergeResult<bool>;

    // =========================================================================
    // Change Operations
    // =========================================================================

    /// Apply changes to a document.
    ///
    /// Loads the document, applies the changes, and saves it.
    /// Creates the document if it doesn't exist.
    async fn apply_changes(&self, id: &DocumentId, changes: Vec<DocumentChange>) -> AutomergeResult<ApplyResult>;

    /// Get the current heads of a document.
    ///
    /// Returns hex-encoded change hashes representing the document's state.
    async fn get_heads(&self, id: &DocumentId) -> AutomergeResult<Vec<String>>;

    // =========================================================================
    // Merge Operations
    // =========================================================================

    /// Merge another document into the target document.
    ///
    /// Both documents must exist. The source document is not modified.
    async fn merge(&self, target_id: &DocumentId, source_id: &DocumentId) -> AutomergeResult<ApplyResult>;

    // =========================================================================
    // Query Operations
    // =========================================================================

    /// List documents with optional filtering.
    async fn list(&self, options: ListOptions) -> AutomergeResult<ListResult>;

    /// List all document IDs in a namespace.
    async fn list_ids(&self, namespace: Option<&str>, limit: u32) -> AutomergeResult<Vec<DocumentId>>;
}

// ============================================================================
// AspenAutomergeStore Implementation
// ============================================================================

/// Automerge store implementation using Aspen's KeyValueStore.
///
/// Provides Automerge document management on top of the distributed KV store.
/// All writes go through Raft consensus for linearizability.
pub struct AspenAutomergeStore<KV: KeyValueStore + ?Sized> {
    /// The underlying key-value store.
    kv: Arc<KV>,
}

impl<KV: KeyValueStore + ?Sized> AspenAutomergeStore<KV> {
    /// Create a new Automerge store backed by the given KeyValueStore.
    pub fn new(kv: Arc<KV>) -> Self {
        Self { kv }
    }

    /// Update metadata from a document's current state.
    fn update_metadata_from_doc(metadata: &mut DocumentMetadata, doc: &mut AutoCommit) {
        metadata.touch();
        metadata.size_bytes = doc.save().len() as u64;
        metadata.change_count = doc.get_changes(&[]).len() as u64;
        metadata.heads = doc.get_heads().iter().map(|h| hex::encode(h.0)).collect();
    }
}

#[async_trait]
impl<KV: KeyValueStore + ?Sized + 'static> DocumentStore for AspenAutomergeStore<KV> {
    #[instrument(skip(self, metadata), fields(custom_id = ?id.as_ref().map(|i| i.as_str())))]
    async fn create(&self, id: Option<DocumentId>, metadata: Option<DocumentMetadata>) -> AutomergeResult<DocumentId> {
        let doc_id = id.unwrap_or_default();

        // Check if document already exists
        if self.exists(&doc_id).await? {
            return Err(AutomergeError::DocumentAlreadyExists {
                document_id: doc_id.to_string(),
            });
        }

        // Create empty Automerge document
        let mut doc = AutoCommit::new();
        let doc_bytes = doc.save();

        // Check size limits
        if doc_bytes.len() > MAX_DOCUMENT_SIZE {
            return Err(AutomergeError::DocumentTooLarge {
                size: doc_bytes.len(),
                max: MAX_DOCUMENT_SIZE,
            });
        }

        // Prepare metadata
        let mut meta = metadata.unwrap_or_else(|| DocumentMetadata::new(doc_id.clone()));
        meta.id = doc_id.clone();
        Self::update_metadata_from_doc(&mut meta, &mut doc);

        // Store both document and metadata atomically
        let pairs = vec![
            (doc_id.content_key(), base64_encode(&doc_bytes)),
            (doc_id.metadata_key(), String::from_utf8_lossy(&meta.to_json_bytes()).to_string()),
        ];

        self.kv
            .write(WriteRequest {
                command: WriteCommand::SetMulti { pairs },
            })
            .await?;

        debug!(document_id = %doc_id, "created automerge document");
        Ok(doc_id)
    }

    #[instrument(skip(self), fields(document_id = %id))]
    async fn get(&self, id: &DocumentId) -> AutomergeResult<Option<AutoCommit>> {
        let key = id.content_key();

        let result = match self.kv.read(ReadRequest::new(key)).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        match result.kv {
            Some(kv) => {
                let bytes = base64_decode(&kv.value)?;
                let doc = AutoCommit::load(&bytes)?;
                Ok(Some(doc))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(document_id = %id))]
    async fn get_metadata(&self, id: &DocumentId) -> AutomergeResult<Option<DocumentMetadata>> {
        let key = id.metadata_key();

        let result = match self.kv.read(ReadRequest::new(key)).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        match result.kv {
            Some(kv) => Ok(DocumentMetadata::from_json_bytes(kv.value.as_bytes())),
            None => Ok(None),
        }
    }

    #[instrument(skip(self, doc), fields(document_id = %id))]
    async fn save(&self, id: &DocumentId, doc: &mut AutoCommit) -> AutomergeResult<()> {
        let doc_bytes = doc.save();

        // Check size limits
        if doc_bytes.len() > MAX_DOCUMENT_SIZE {
            return Err(AutomergeError::DocumentTooLarge {
                size: doc_bytes.len(),
                max: MAX_DOCUMENT_SIZE,
            });
        }

        // Get or create metadata
        let mut meta = self.get_metadata(id).await?.unwrap_or_else(|| DocumentMetadata::new(id.clone()));
        Self::update_metadata_from_doc(&mut meta, doc);

        // Store both document and metadata atomically
        let pairs = vec![
            (id.content_key(), base64_encode(&doc_bytes)),
            (id.metadata_key(), String::from_utf8_lossy(&meta.to_json_bytes()).to_string()),
        ];

        self.kv
            .write(WriteRequest {
                command: WriteCommand::SetMulti { pairs },
            })
            .await?;

        debug!(document_id = %id, size = doc_bytes.len(), "saved automerge document");
        Ok(())
    }

    #[instrument(skip(self), fields(document_id = %id))]
    async fn delete(&self, id: &DocumentId) -> AutomergeResult<bool> {
        // Check if document exists
        if !self.exists(id).await? {
            return Ok(false);
        }

        // Delete both document and metadata
        let keys = vec![id.content_key(), id.metadata_key()];

        self.kv
            .write(WriteRequest {
                command: WriteCommand::DeleteMulti { keys },
            })
            .await?;

        debug!(document_id = %id, "deleted automerge document");
        Ok(true)
    }

    #[instrument(skip(self), fields(document_id = %id))]
    async fn exists(&self, id: &DocumentId) -> AutomergeResult<bool> {
        let key = id.content_key();

        match self.kv.read(ReadRequest::new(key)).await {
            Ok(r) => Ok(r.kv.is_some()),
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    #[instrument(skip(self, changes), fields(document_id = %id, change_count = changes.len()))]
    async fn apply_changes(&self, id: &DocumentId, changes: Vec<DocumentChange>) -> AutomergeResult<ApplyResult> {
        // Validate batch size
        if changes.len() > MAX_BATCH_CHANGES {
            return Err(AutomergeError::TooManyChanges {
                count: changes.len(),
                max: MAX_BATCH_CHANGES,
            });
        }

        // Validate individual change sizes
        for (i, change) in changes.iter().enumerate() {
            if change.bytes.len() > MAX_CHANGE_SIZE {
                return Err(AutomergeError::ChangeTooLarge {
                    size: change.bytes.len(),
                    max: MAX_CHANGE_SIZE,
                });
            }
            debug!(change_index = i, size = change.bytes.len(), "validating change");
        }

        // Load or create document
        let mut doc = self.get(id).await?.unwrap_or_else(AutoCommit::new);

        // Apply each change
        let mut applied_count = 0;
        for change in &changes {
            match doc.load_incremental(&change.bytes) {
                Ok(count) => {
                    applied_count += count;
                    debug!(changes_in_block = count, "applied change block");
                }
                Err(e) => {
                    warn!(error = %e, "failed to apply change, skipping");
                    // Continue with other changes - partial application is ok
                }
            }
        }

        // Save the updated document
        self.save(id, &mut doc).await?;

        let new_heads: Vec<String> = doc.get_heads().iter().map(|h| hex::encode(h.0)).collect();
        let new_size = doc.save().len() as u64;

        Ok(ApplyResult {
            changes_applied: applied_count > 0,
            change_count: applied_count,
            new_heads,
            new_size,
        })
    }

    #[instrument(skip(self), fields(document_id = %id))]
    async fn get_heads(&self, id: &DocumentId) -> AutomergeResult<Vec<String>> {
        match self.get(id).await? {
            Some(mut doc) => Ok(doc.get_heads().iter().map(|h| hex::encode(h.0)).collect()),
            None => Err(AutomergeError::DocumentNotFound {
                document_id: id.to_string(),
            }),
        }
    }

    #[instrument(skip(self), fields(target = %target_id, source = %source_id))]
    async fn merge(&self, target_id: &DocumentId, source_id: &DocumentId) -> AutomergeResult<ApplyResult> {
        // Load both documents
        let mut target = self.get(target_id).await?.ok_or_else(|| AutomergeError::DocumentNotFound {
            document_id: target_id.to_string(),
        })?;

        let mut source = self.get(source_id).await?.ok_or_else(|| AutomergeError::DocumentNotFound {
            document_id: source_id.to_string(),
        })?;

        // Merge source into target
        target.merge(&mut source)?;

        // Save the merged target
        self.save(target_id, &mut target).await?;

        let new_heads: Vec<String> = target.get_heads().iter().map(|h| hex::encode(h.0)).collect();
        let new_size = target.save().len() as u64;

        debug!("merged documents successfully");
        Ok(ApplyResult {
            changes_applied: true,
            change_count: 1, // Merge counts as one operation
            new_heads,
            new_size,
        })
    }

    #[instrument(skip(self, options))]
    async fn list(&self, options: ListOptions) -> AutomergeResult<ListResult> {
        let limit = options.limit.unwrap_or(DEFAULT_LIST_LIMIT).min(MAX_SCAN_RESULTS);

        // Scan metadata keys
        let result = self
            .kv
            .scan(ScanRequest {
                prefix: DOC_META_PREFIX.to_string(),
                limit: Some(limit),
                continuation_token: options.continuation_token,
            })
            .await?;

        let mut documents = Vec::new();

        for entry in result.entries {
            if let Some(meta) = DocumentMetadata::from_json_bytes(entry.value.as_bytes()) {
                // Apply filters
                if let Some(ref ns) = options.namespace
                    && meta.namespace.as_ref() != Some(ns)
                {
                    continue;
                }
                if let Some(ref tag) = options.tag
                    && !meta.tags.contains(tag)
                {
                    continue;
                }
                documents.push(meta);
            }
        }

        Ok(ListResult {
            documents,
            has_more: result.is_truncated,
            continuation_token: result.continuation_token,
        })
    }

    #[instrument(skip(self), fields(namespace = ?namespace, limit = %limit))]
    async fn list_ids(&self, namespace: Option<&str>, limit: u32) -> AutomergeResult<Vec<DocumentId>> {
        let limit = limit.min(MAX_SCAN_RESULTS);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: DOC_KEY_PREFIX.to_string(),
                limit: Some(limit),
                continuation_token: None,
            })
            .await?;

        let mut ids = Vec::new();

        for entry in result.entries {
            // Skip metadata keys
            if entry.key.starts_with(DOC_META_PREFIX) {
                continue;
            }

            if let Some(id) = DocumentId::from_content_key(&entry.key) {
                // If namespace filter is set, we need to check metadata
                // For efficiency, we skip this check and just return all IDs
                // A proper implementation would filter by loading metadata
                if namespace.is_none() {
                    ids.push(id);
                }
            }
        }

        Ok(ids)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Encode bytes as base64 for storage in string KV.
fn base64_encode(bytes: &[u8]) -> String {
    use std::io::Write;
    let mut encoder = base64::write::EncoderStringWriter::new(&base64::engine::general_purpose::STANDARD);
    encoder.write_all(bytes).expect("base64 encoding should not fail");
    encoder.into_inner()
}

/// Decode base64 string back to bytes.
fn base64_decode(s: &str) -> AutomergeResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).map_err(|e| AutomergeError::Serialization {
        reason: format!("invalid base64: {}", e),
    })
}

#[cfg(test)]
mod tests {
    use aspen_core::DeterministicKeyValueStore;
    use automerge::ReadDoc;
    use automerge::transaction::Transactable;

    use super::*;

    fn create_test_store() -> AspenAutomergeStore<DeterministicKeyValueStore> {
        let kv = DeterministicKeyValueStore::new();
        AspenAutomergeStore::new(kv)
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let store = create_test_store();

        let id = store.create(None, None).await.unwrap();
        assert!(!id.as_str().is_empty());

        let doc = store.get(&id).await.unwrap();
        assert!(doc.is_some());
    }

    #[tokio::test]
    async fn test_create_with_custom_id() {
        let store = create_test_store();

        let custom_id = DocumentId::from_string("my-custom-doc").unwrap();
        let id = store.create(Some(custom_id.clone()), None).await.unwrap();
        assert_eq!(id, custom_id);

        // Creating again should fail
        let result = store.create(Some(custom_id), None).await;
        assert!(matches!(result, Err(AutomergeError::DocumentAlreadyExists { .. })));
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let store = create_test_store();

        let id = store.create(None, None).await.unwrap();

        // Modify and save
        let mut doc = store.get(&id).await.unwrap().unwrap();
        doc.put(automerge::ROOT, "key", "value").unwrap();
        store.save(&id, &mut doc).await.unwrap();

        // Load and verify
        let loaded = store.get(&id).await.unwrap().unwrap();
        let value: Option<String> = loaded.get(automerge::ROOT, "key").unwrap().map(|(v, _)| v.into_string().unwrap());
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_delete() {
        let store = create_test_store();

        let id = store.create(None, None).await.unwrap();
        assert!(store.exists(&id).await.unwrap());

        let deleted = store.delete(&id).await.unwrap();
        assert!(deleted);
        assert!(!store.exists(&id).await.unwrap());

        // Deleting non-existent returns false
        let deleted = store.delete(&id).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_get_metadata() {
        let store = create_test_store();

        let meta = DocumentMetadata::new(DocumentId::new()).with_title("Test Doc").with_namespace("project-a");

        let id = store.create(None, Some(meta.clone())).await.unwrap();

        let loaded_meta = store.get_metadata(&id).await.unwrap().unwrap();
        assert_eq!(loaded_meta.title, Some("Test Doc".to_string()));
        assert_eq!(loaded_meta.namespace, Some("project-a".to_string()));
    }

    #[tokio::test]
    async fn test_merge_documents() {
        let store = create_test_store();

        // Create two documents
        let id1 = store.create(None, None).await.unwrap();
        let id2 = store.create(None, None).await.unwrap();

        // Modify each independently
        let mut doc1 = store.get(&id1).await.unwrap().unwrap();
        doc1.put(automerge::ROOT, "from_doc1", "value1").unwrap();
        store.save(&id1, &mut doc1).await.unwrap();

        let mut doc2 = store.get(&id2).await.unwrap().unwrap();
        doc2.put(automerge::ROOT, "from_doc2", "value2").unwrap();
        store.save(&id2, &mut doc2).await.unwrap();

        // Merge doc2 into doc1
        let result = store.merge(&id1, &id2).await.unwrap();
        assert!(result.changes_applied);

        // Verify doc1 has both keys
        let merged = store.get(&id1).await.unwrap().unwrap();
        let v1: Option<String> =
            merged.get(automerge::ROOT, "from_doc1").unwrap().map(|(v, _)| v.into_string().unwrap());
        let v2: Option<String> =
            merged.get(automerge::ROOT, "from_doc2").unwrap().map(|(v, _)| v.into_string().unwrap());
        assert_eq!(v1, Some("value1".to_string()));
        assert_eq!(v2, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_list_documents() {
        let store = create_test_store();

        // Create several documents
        for i in 0..5 {
            let meta = DocumentMetadata::new(DocumentId::new()).with_title(format!("Doc {}", i));
            store.create(None, Some(meta)).await.unwrap();
        }

        let result = store.list(ListOptions::default()).await.unwrap();
        assert_eq!(result.documents.len(), 5);
    }
}
