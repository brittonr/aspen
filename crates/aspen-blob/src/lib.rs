//! Iroh-blobs integration for content-addressed blob storage.
//!
//! This module provides content-addressed storage (CAS) for large data that
//! doesn't fit well in the KV store. Key features:
//!
//! - **Content-addressed**: Blobs are identified by their BLAKE3 hash
//! - **Automatic deduplication**: Same content stored once regardless of how many times it's added
//! - **Large value offloading**: KV values >1MB automatically stored as blobs
//! - **Garbage collection**: Unreferenced blobs are automatically cleaned up
//! - **P2P distribution**: Blobs can be downloaded from any node that has them
//!
//! ## Architecture
//!
//! ```text
//! KeyValueStore (values > BLOB_THRESHOLD)
//!        |
//!        v
//! BlobAwareKeyValueStore (wrapper)
//!        |
//!        +--> Small values: Direct to RaftNode
//!        |
//!        +--> Large values: Store in BlobStore, save BlobRef in RaftNode
//!                |
//!                v
//!          IrohBlobStore
//!                |
//!                v
//!          iroh-blobs FsStore
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use aspen_blob::{BlobStore, IrohBlobStore};
//!
//! // Create blob store
//! let store = IrohBlobStore::new(data_dir, endpoint).await?;
//!
//! // Add a blob
//! let result = store.add_bytes(b"large data here").await?;
//! println!("Stored blob: {}", result.blob_ref.hash);
//!
//! // Retrieve a blob
//! let data = store.get_bytes(&result.blob_ref.hash).await?;
//!
//! // Create a ticket for sharing
//! let ticket = store.ticket(&result.blob_ref.hash).await?;
//!
//! // Download from another node
//! let blob_ref = store.download(&ticket).await?;
//! ```

pub mod background_downloader;
pub mod constants;
pub mod events;
pub mod kv_integration;
pub mod replication;
pub mod store;
pub mod traits;
pub mod types;

pub use background_downloader::BACKGROUND_DOWNLOAD_TIMEOUT;
// Background downloader types
pub use background_downloader::BackgroundBlobDownloader;
pub use background_downloader::BackgroundDownloadError;
pub use background_downloader::DownloadRequest;
pub use background_downloader::DownloadStats;
pub use background_downloader::MAX_BACKGROUND_DOWNLOAD_SIZE;
pub use background_downloader::MAX_CONCURRENT_BACKGROUND_DOWNLOADS;
pub use constants::BLOB_GC_GRACE_PERIOD;
pub use constants::BLOB_GC_INTERVAL;
pub use constants::BLOB_REF_PREFIX;
pub use constants::BLOB_THRESHOLD;
pub use constants::BLOB_WAIT_POLL_INTERVAL;
pub use constants::DEFAULT_BLOB_WAIT_TIMEOUT;
pub use constants::KV_TAG_PREFIX;
pub use constants::MAX_BLOB_SIZE;
pub use constants::MAX_BLOB_WAIT_TIMEOUT;
pub use constants::USER_TAG_PREFIX;
pub use events::BLOB_EVENT_BUFFER_SIZE;
// Event types
pub use events::BlobEvent;
pub use events::BlobEventBroadcaster;
pub use events::BlobEventType;
pub use events::BlobSource;
pub use events::INLINE_BLOB_THRESHOLD;
pub use events::UnprotectReason;
pub use events::create_blob_event_channel;
pub use kv_integration::BlobAwareKeyValueStore;
// Replication types
pub use replication::BlobReplicationManager;
pub use replication::IrohBlobTransfer;
pub use replication::KvReplicaMetadataStore;
pub use replication::MAX_CONCURRENT_REPLICATIONS;
pub use replication::MAX_REPAIR_BATCH_SIZE;
pub use replication::MAX_REPLICATION_FACTOR;
pub use replication::MIN_REPLICATION_FACTOR;
pub use replication::NodeInfo;
pub use replication::PlacementStrategy;
pub use replication::REPLICA_KEY_PREFIX;
pub use replication::ReplicaBlobTransfer;
pub use replication::ReplicaMetadataStore;
pub use replication::ReplicaSet;
pub use replication::ReplicationConfig;
pub use replication::ReplicationPolicy;
pub use replication::ReplicationRequest;
pub use replication::ReplicationResult;
pub use replication::ReplicationStatus;
pub use replication::WeightedPlacement;
pub use replication::spawn_topology_watcher;
pub use store::AsyncReadSeek;
pub use store::BlobStoreError;
pub use store::InMemoryBlobStore;
pub use store::IrohBlobStore;
// Re-export all trait types from traits module
pub use traits::BlobQuery;
pub use traits::BlobRead;
pub use traits::BlobStore;
pub use traits::BlobTransfer;
pub use traits::BlobWrite;

/// Prelude module for convenient trait imports.
///
/// Import everything from this module to bring all blob storage traits into scope:
/// ```ignore
/// use aspen_blob::prelude::*;
/// ```
pub mod prelude {
    pub use super::traits::BlobQuery;
    pub use super::traits::BlobRead;
    pub use super::traits::BlobStore;
    pub use super::traits::BlobTransfer;
    pub use super::traits::BlobWrite;
}
pub use types::AddBlobResult;
pub use types::BlobListEntry;
pub use types::BlobListResult;
pub use types::BlobRef;
pub use types::BlobStatus;
pub use types::is_blob_ref;
