//! SNIX storage integration for Aspen distributed cache.
//!
//! This crate provides implementations of SNIX storage traits backed by
//! Aspen's distributed infrastructure:
//!
//! - [`IrohBlobService`]: Content-addressed blob storage via iroh-blobs
//! - [`RaftDirectoryService`]: Directory metadata stored in Raft KV
//! - [`RaftPathInfoService`]: Nix store path info stored in Raft KV
//!
//! # Architecture
//!
//! SNIX uses a content-addressed storage model with three layers:
//!
//! 1. **Blobs**: Raw content chunks, addressed by BLAKE3 hash
//! 2. **Directories**: Merkle tree nodes linking names to blobs/subdirs
//! 3. **PathInfo**: Nix store path metadata (NAR hash, size, references)
//!
//! This crate maps these to Aspen's storage:
//!
//! - Blobs → iroh-blobs (P2P content-addressed storage)
//! - Directories → Raft KV (linearizable metadata)
//! - PathInfo → Raft KV (linearizable metadata)
//!
//! # Tiger Style
//!
//! All operations follow Tiger Style principles:
//! - Explicit resource bounds (see [`constants`])
//! - No panics in production code
//! - Structured error handling via snafu

pub mod blob_service;
pub mod constants;
pub mod directory_service;
pub mod error;
pub mod migration;
pub mod pathinfo_service;
pub mod rpc_directory_service;
pub mod rpc_pathinfo_service;

pub use blob_service::IrohBlobService;
pub use constants::*;
pub use directory_service::RaftDirectoryService;
pub use error::Error;
pub use error::Result;
pub use migration::CacheEntryVersion;
pub use migration::MigrationAwareCacheIndex;
pub use migration::MigrationError;
pub use migration::MigrationProgress;
pub use migration::MigrationWorker;
pub use migration::VersionedCacheEntry;
pub use pathinfo_service::RaftPathInfoService;
pub use rpc_directory_service::RpcDirectoryService;
pub use rpc_pathinfo_service::RpcPathInfoService;

/// Re-export SNIX types for convenience.
pub mod snix {
    pub use nix_compat::store_path::StorePath;
    pub use snix_castore::B3Digest;
    pub use snix_castore::Directory;
    pub use snix_castore::Node;
    pub use snix_store::pathinfoservice::PathInfo;
}
