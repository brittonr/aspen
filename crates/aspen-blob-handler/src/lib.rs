//! Blob storage RPC handler for Aspen.
//!
//! This crate provides the blob handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles blob storage operations:
//! - AddBlob, GetBlob, HasBlob: Basic blob operations
//! - GetBlobTicket, ListBlobs: Blob discovery
//! - ProtectBlob, UnprotectBlob, DeleteBlob: Blob lifecycle
//! - DownloadBlob, DownloadBlobByHash, DownloadBlobByProvider: Remote blob retrieval
//! - GetBlobStatus: Blob status inspection
//! - BlobReplicatePull, GetBlobReplicationStatus, TriggerBlobReplication: Replication
//! - RunBlobRepairCycle: Cluster-wide blob repair

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::BlobHandler;
