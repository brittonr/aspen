//! SNIX RPC handler for Aspen.
//!
//! This crate provides the SNIX handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles SNIX-related RPC requests from ephemeral CI workers:
//! - DirectoryService get/put operations
//! - PathInfoService get/put operations
//!
//! Workers use these RPCs to upload build artifacts to the cluster's
//! SNIX binary cache without needing direct Raft access.

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::SnixHandler;
