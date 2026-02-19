//! Automerge CRDT document layer for Aspen.
//!
//! This module provides Automerge document management built on top of Aspen's
//! distributed key-value store. It enables:
//!
//! - **Document CRUD**: Create, read, update, delete Automerge documents
//! - **Collaborative Editing**: Automatic merge of concurrent changes
//! - **Version History**: Full change history preserved by Automerge
//! - **Conflict Resolution**: Automerge's deterministic conflict resolution
//!
//! # Architecture
//!
//! ```text
//! Application Layer (collaborative editing, rich text, etc.)
//!         |
//!         v
//! AspenAutomergeStore (this crate)
//!    - Document CRUD operations
//!    - Change application
//!    - Merge operations
//!         |
//!         v
//! KeyValueStore (Raft consensus)
//!    - Key: automerge:{document_id} -> base64(document_bytes)
//!    - Key: automerge:_meta:{document_id} -> JSON metadata
//!         |
//!         v
//! Optional: DocsExporter -> iroh-docs P2P sync
//! ```
//!
//! # Key Format
//!
//! Documents are stored in the KV store with the following key convention:
//! - Content: `automerge:{document_id}` - Base64-encoded Automerge document
//! - Metadata: `automerge:_meta:{document_id}` - JSON metadata
//!
//! # Usage
//!
//! ```ignore
//! use aspen_automerge::{AspenAutomergeStore, DocumentStore, DocumentId};
//! use automerge::AutoCommit;
//!
//! // Create store wrapping your KeyValueStore
//! let store = AspenAutomergeStore::new(kv_store);
//!
//! // Create a new document
//! let id = store.create(None, None).await?;
//!
//! // Load, modify, and save
//! let mut doc = store.get(&id).await?.unwrap();
//! doc.put(automerge::ROOT, "title", "My Document")?;
//! doc.put(automerge::ROOT, "content", "Hello, world!")?;
//! store.save(&id, &doc).await?;
//!
//! // Merge changes from another document
//! store.merge(&id, &other_id).await?;
//!
//! // List all documents
//! let result = store.list(ListOptions::default()).await?;
//! for meta in result.documents {
//!     println!("{}: {}", meta.id, meta.title.unwrap_or_default());
//! }
//! ```
//!
//! # Collaborative Editing Pattern
//!
//! For real-time collaboration:
//!
//! 1. Each client creates local changes on their `AutoCommit`
//! 2. Serialize changes with `doc.save_incremental()` or `doc.save()`
//! 3. Send changes to server via `apply_changes()`
//! 4. Server merges and broadcasts to other clients
//! 5. Clients apply received changes locally
//!
//! # Automerge Concepts
//!
//! - **Document**: A JSON-like data structure that tracks all changes
//! - **Change**: An atomic modification to the document
//! - **Heads**: Hash(es) of the latest change(s) - multiple heads indicate concurrent edits
//! - **Actor**: Unique identifier for each editing session
//! - **Merge**: Combining changes from different actors into a single document
//!
//! # Tiger Style Limits
//!
//! - `MAX_DOCUMENT_SIZE` = 16 MB
//! - `MAX_CHANGE_SIZE` = 1 MB
//! - `MAX_BATCH_CHANGES` = 1,000
//! - `MAX_SCAN_RESULTS` = 1,000
//! - `MAX_DOCUMENTS_PER_NAMESPACE` = 100,000

pub mod capability;
pub mod constants;
pub mod error;
pub mod store;
pub mod sync_protocol;
pub mod types;

// Re-export main types for convenience
pub use capability::CapabilityError;
pub use capability::Permission;
pub use capability::SignedCapability;
pub use capability::SyncCapability;
pub use constants::AUTOMERGE_TICKET_PREFIX;
pub use constants::DEFAULT_LIST_LIMIT;
pub use constants::DOC_KEY_PREFIX;
pub use constants::DOC_META_PREFIX;
pub use constants::MAX_BATCH_CHANGES;
pub use constants::MAX_CHANGE_SIZE;
pub use constants::MAX_DOCUMENT_SIZE;
pub use constants::MAX_DOCUMENTS_PER_NAMESPACE;
pub use constants::MAX_SCAN_RESULTS;
pub use error::AutomergeError;
pub use error::AutomergeResult;
pub use store::AspenAutomergeStore;
pub use store::DocumentStore;
pub use sync_protocol::AUTOMERGE_SYNC_ALPN;
pub use sync_protocol::AutomergeSyncHandler;
pub use sync_protocol::SyncError;
pub use sync_protocol::SyncProtocolMessage;
// Sync protocol exports
pub use sync_protocol::sync_with_peer;
pub use sync_protocol::sync_with_peer_cap;
pub use types::ApplyResult;
pub use types::DocumentChange;
pub use types::DocumentEvent;
pub use types::DocumentId;
pub use types::DocumentMetadata;
pub use types::ListOptions;
pub use types::ListResult;
pub use types::SyncState;
pub use types::SyncStatus;
