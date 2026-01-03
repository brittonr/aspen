//! Client API protocol definitions for Aspen.
//!
//! This crate provides the protocol definitions for the Aspen Client RPC API,
//! which allows clients to communicate with aspen-node instances over Iroh P2P
//! connections using the `aspen-client` ALPN identifier.
//!
//! # Architecture
//!
//! The Client RPC uses a distinct ALPN to distinguish it from Raft RPC, allowing
//! clients to connect directly to nodes without needing HTTP. All communication
//! uses authenticated requests with optional capability tokens for authorization.
//!
//! # Core Types
//!
//! - [`AuthenticatedRequest`] - Wrapper for requests with optional auth tokens
//! - [`ClientRpcRequest`] - Enum of all supported client operations
//! - [`ClientRpcResponse`] - Enum of all possible response types
//!
//! # Protocol Constants
//!
//! - [`CLIENT_ALPN`] - ALPN identifier for client connections
//! - [`MAX_CLIENT_MESSAGE_SIZE`] - Maximum message size (1 MB)
//! - [`MAX_CLUSTER_NODES`] - Maximum nodes in cluster state response
//!
//! # Example
//!
//! ```rust
//! use aspen_client_api::{ClientRpcRequest, AuthenticatedRequest};
//!
//! let request = ClientRpcRequest::GetHealth;
//! let auth_request = AuthenticatedRequest::unauthenticated(request);
//! ```

pub mod messages;

// Re-export all public types for convenience
pub use messages::*;
