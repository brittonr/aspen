//! Content-addressed execution cache for Aspen CI.
//!
//! Caches process execution results by hashing their inputs (command, args,
//! environment, and file contents read via FUSE). On cache hit, outputs are
//! replayed from iroh-blobs without re-executing the process.
//!
//! # Architecture
//!
//! - **Cache key**: `BLAKE3(command || args || env_hash || sorted_input_hashes)`
//! - **Cache index**: Raft KV with `_exec_cache:` key prefix
//! - **Output storage**: iroh-blobs (content-addressed, deduplicated)
//! - **Input tracking**: FUSE-level read tracking in `aspen-fuse`
//!
//! # Feature flags
//!
//! This crate is unconditional. The `exec-cache` feature flag on the root
//! workspace controls whether it's wired into CI executors.

pub mod constants;
pub mod error;
pub mod eviction;
pub mod executor;
pub mod index;
pub mod read_tracker;
pub mod types;

pub mod verified;

pub use error::ExecCacheError;
pub use error::Result;
pub use executor::BlobStore;
pub use executor::CacheStats;
pub use executor::CachedExecResult;
pub use executor::CachedExecutor;
pub use index::CacheKvStore;
pub use index::ExecCacheIndex;
pub use read_tracker::ReadTracker;
pub use types::CacheEntry;
pub use types::CacheKey;
pub use types::OutputMapping;
