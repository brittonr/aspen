//! Pijul VCS RPC handler for Aspen.
//!
//! This crate provides the Pijul handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles Pijul operations:
//! - Repository: Init, List, Info
//! - Channel: List, Create, Delete, Fork, Info
//! - Change: Apply, Unrecord, Log, Show, Blame

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::PijulHandler;
