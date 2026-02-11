//! SQL RPC handler for Aspen.
//!
//! This crate provides the SQL handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles SQL query operations via ExecuteSql request.

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::SqlHandler;
