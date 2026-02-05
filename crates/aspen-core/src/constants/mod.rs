//! Centralized constants for Aspen distributed system.
//!
//! This module contains all configuration constants used throughout the Aspen
//! implementation, organized by category for easy discovery and maintenance.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.
//!
//! # Modules
//!
//! - [`api`]: Public API bounds (KV sizes, SQL limits, scan results)
//! - [`coordination`]: Distributed primitives (queues, registries, rate limits)
//! - [`network`]: Network timeouts, gossip, RPC limits
//! - [`raft`]: Consensus internals (membership, backoff, integrity)
//! - [`ci`]: CI/CD VM limits, log streaming, job execution

pub mod api;
pub mod ci;
pub mod coordination;
pub mod directory;
pub mod network;
pub mod raft;

// Re-export all constants at module level for convenience
pub use api::*;
pub use ci::*;
pub use coordination::*;
pub use directory::*;
pub use network::*;
pub use raft::*;
