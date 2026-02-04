//! Client RPC protocol for Aspen communication over Iroh.
//!
//! **DEPRECATED**: This crate is deprecated. Use [`aspen_client_api`] directly instead.
//!
//! This crate re-exports types from [`aspen_client_api`] for backwards compatibility.
//! All protocol types are defined in `aspen-client-api`.
//!
//! # Migration
//!
//! Replace:
//! ```ignore
//! use aspen_client_rpc::ClientRpcRequest;
//! ```
//!
//! With:
//! ```ignore
//! use aspen_client_api::ClientRpcRequest;
//! ```
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN (`aspen-client`) to distinguish it from Raft RPC.
//! This allows clients to connect directly to nodes without needing HTTP.

#![deprecated(
    since = "0.2.0",
    note = "Use aspen-client-api directly instead. This crate will be removed in a future version."
)]

// Re-export all public types from aspen-client-api for backwards compatibility
#[allow(deprecated)]
pub use aspen_client_api::*;
