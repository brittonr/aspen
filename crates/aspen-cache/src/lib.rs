//! Distributed Nix binary cache for Aspen.
//!
//! This crate provides a distributed Nix binary cache built on top of Aspen's
//! iroh-blobs storage and Raft KV store. It enables CI builds to automatically
//! cache their outputs for reuse by other builds and developers.
//!
//! # Architecture
//!
//! The cache has two layers:
//!
//! 1. **Metadata Index** (Raft KV): Maps Nix store hashes to [`CacheEntry`] metadata. Entries
//!    contain the blob hash, NAR hash, references, and other narinfo fields.
//!
//! 2. **NAR Storage** (iroh-blobs): Stores the actual NAR archives as content-addressed blobs,
//!    distributed across cluster nodes via P2P.
//!
//! # Usage
//!
//! The cache is populated automatically by the CI Nix build worker when it builds
//! packages. Client applications can query the cache and download NARs via the
//! Aspen client protocol.
//!
//! ## Storing an Entry
//!
//! ```ignore
//! use aspen_cache::{CacheEntry, CacheIndex, KvCacheIndex};
//!
//! let index = KvCacheIndex::new(kv_store);
//!
//! let entry = CacheEntry::new(
//!     "/nix/store/abc123-hello".to_string(),
//!     "abc123".to_string(),
//!     "blake3_hash".to_string(),
//!     1024,
//!     "sha256:deadbeef".to_string(),
//!     timestamp,
//!     node_id,
//! );
//!
//! index.put(entry).await?;
//! ```
//!
//! ## Querying the Cache
//!
//! ```ignore
//! if let Some(entry) = index.get("abc123").await? {
//!     // Entry found, download NAR via blob_hash
//!     let nar_data = blob_store.get_bytes(&entry.blob_hash).await?;
//! }
//! ```
//!
//! # Tiger Style
//!
//! All operations have explicit bounds:
//! - Max references per entry: 1000
//! - Max deriver path length: 1024 bytes
//! - Store hash validation: 32-64 lowercase alphanumeric chars

pub mod error;
pub mod index;
pub mod narinfo;

pub use error::CacheError;
pub use error::Result;
pub use index::CacheIndex;
pub use index::KvCacheIndex;
pub use index::parse_store_path;
pub use narinfo::CACHE_KEY_PREFIX;
pub use narinfo::CACHE_STATS_KEY;
pub use narinfo::CacheEntry;
pub use narinfo::CacheStats;
pub use narinfo::MAX_DERIVER_LENGTH;
pub use narinfo::MAX_REFERENCES;
