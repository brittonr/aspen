//! Key-Value RPC handler for Aspen.
//!
//! This crate provides the KV handler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles key-value operations:
//! - ReadKey: Read a single key
//! - WriteKey: Write a single key-value pair
//! - DeleteKey: Delete a key
//! - ScanKeys: Scan keys with a prefix
//! - BatchRead: Read multiple keys atomically
//! - BatchWrite: Write multiple key-value pairs atomically
//! - ConditionalBatchWrite: Conditional batch writes with preconditions
//! - CompareAndSwapKey: Atomic compare-and-swap operation
//! - CompareAndDeleteKey: Atomic compare-and-delete operation

mod error_sanitization;
mod handler;
mod verified;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::KvHandler;
