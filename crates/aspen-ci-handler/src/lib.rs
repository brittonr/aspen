//! CI/CD pipeline RPC handler for Aspen distributed cluster.
//!
//! This crate implements the RPC handler for CI/CD operations, providing:
//! - Pipeline triggering and execution
//! - Pipeline status queries
//! - Pipeline run listing and filtering
//! - Repository watching for automatic CI triggers
//! - Artifact listing and retrieval
//! - Job log fetching and subscription
//! - Job output retrieval
//!
//! # Architecture
//!
//! The handler implements the `RequestHandler` trait from `aspen-rpc-core`,
//! allowing it to be registered with the `HandlerRegistry` in `aspen-rpc-handlers`.
//!
//! ```text
//! aspen-rpc-handlers
//!        |
//!        v
//!   HandlerRegistry
//!        |
//!        v
//!   CiHandler (this crate)
//!        |
//!        +---> Pipeline Operations (trigger, status, list, cancel)
//!        +---> Watch Operations (watch/unwatch repo)
//!        +---> Artifact Operations (list, get)
//!        +---> Log Operations (get logs, subscribe, get output)
//! ```

mod handler;

// Re-export types that handlers need from aspen-rpc-core
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::CiHandler;
