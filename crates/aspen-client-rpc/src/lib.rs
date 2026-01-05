//! Client RPC protocol for Aspen communication over Iroh.
//!
//! This crate re-exports types from [`aspen_client_api`] for backwards compatibility
//! and adds RPC-specific constants. All protocol types are defined in `aspen-client-api`.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN (`aspen-client`) to distinguish it from Raft RPC.
//! This allows clients to connect directly to nodes without needing HTTP.
//!
//! # Tiger Style
//!
//! - Explicit request/response pairs
//! - Bounded message sizes
//! - Fail-fast on invalid requests
//!
//! # Migration Note
//!
//! This crate previously defined its own `ClientRpcRequest` and `ClientRpcResponse` types.
//! These are now re-exported from `aspen-client-api` to eliminate duplication.
//! All imports should continue to work unchanged.

// Re-export all public types from aspen-client-api for backwards compatibility
pub use aspen_client_api::*;
