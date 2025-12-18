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
//! use aspen::blob::{BlobStore, IrohBlobStore};
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

pub mod constants;
pub mod kv_integration;
pub mod store;
pub mod types;

pub use constants::{
    BLOB_GC_GRACE_PERIOD, BLOB_GC_INTERVAL, BLOB_REF_PREFIX, BLOB_THRESHOLD, KV_TAG_PREFIX,
    MAX_BLOB_SIZE, USER_TAG_PREFIX,
};

pub use store::{BlobStore, BlobStoreError, IrohBlobStore};

pub use types::{AddBlobResult, BlobListEntry, BlobListResult, BlobRef, BlobStatus, is_blob_ref};

pub use kv_integration::BlobAwareKeyValueStore;
