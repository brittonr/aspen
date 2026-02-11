//! DNS RPC handler for Aspen.
//!
//! This crate provides the DNS handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles all DNS* operations for DNS record management:
//! - Record operations: Set, Get, Delete, Scan
//! - Zone operations: Set, Get, List, Delete
//! - Resolution: Resolve queries

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::DnsHandler;
