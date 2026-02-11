//! Automerge CRDT RPC handler for Aspen.
//!
//! This crate provides the Automerge handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles Automerge CRDT document operations:
//! - Create, Get, Save, Delete: Document lifecycle
//! - ApplyChanges, Merge: Change operations
//! - List, GetMetadata, Exists: Query operations
//! - GenerateSyncMessage, ReceiveSyncMessage: Sync operations

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::AutomergeHandler;
