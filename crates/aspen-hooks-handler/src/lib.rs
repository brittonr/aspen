//! Hooks RPC handler for Aspen.
//!
//! This crate provides the hooks handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles hook system operations:
//! - HookList: List configured handlers
//! - HookGetMetrics: Get execution metrics
//! - HookTrigger: Manually trigger events

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::HooksHandler;
