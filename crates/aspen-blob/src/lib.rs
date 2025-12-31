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

pub mod constants;
pub mod kv_integration;
pub mod store;
pub mod types;

pub use constants::BLOB_GC_GRACE_PERIOD;
pub use constants::BLOB_GC_INTERVAL;
pub use constants::BLOB_REF_PREFIX;
pub use constants::BLOB_THRESHOLD;
pub use constants::KV_TAG_PREFIX;
pub use constants::MAX_BLOB_SIZE;
pub use constants::USER_TAG_PREFIX;
pub use kv_integration::BlobAwareKeyValueStore;
pub use store::BlobStore;
pub use store::BlobStoreError;
pub use store::InMemoryBlobStore;
pub use store::IrohBlobStore;
pub use types::AddBlobResult;
pub use types::BlobListEntry;
pub use types::BlobListResult;
pub use types::BlobRef;
pub use types::BlobStatus;
pub use types::is_blob_ref;
