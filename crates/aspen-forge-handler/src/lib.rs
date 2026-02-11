//! Forge (decentralized Git) RPC handler for Aspen distributed cluster.
//!
//! This crate implements the RPC handler for Forge operations, providing:
//! - Repository management (create, get, list)
//! - Git object operations (blob, tree, commit)
//! - Ref management (branches, tags)
//! - Issue and patch tracking (CRDT-based)
//! - Federation (cross-cluster sync)
//! - Git Bridge (git-remote-aspen interop)
//!
//! # Architecture
//!
//! The handler implements the `RequestHandler` trait from `aspen-rpc-core`,
//! allowing it to be registered with the `HandlerRegistry` in `aspen-rpc-handlers`.
//!
//! ```text
//! aspen-rpc-handlers
//!        │
//!        ▼
//!   HandlerRegistry
//!        │
//!        ▼
//!   ForgeHandler (this crate)
//!        │
//!        ├──► Repository Operations
//!        ├──► Git Object Operations
//!        ├──► Ref Operations
//!        ├──► Issue Operations
//!        ├──► Patch Operations
//!        ├──► Federation Operations
//!        └──► Git Bridge Operations
//! ```

mod handler;

// Re-export types that handlers need from aspen-rpc-core
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::ForgeHandler;
