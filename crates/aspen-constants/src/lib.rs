//! Centralized constants for Aspen distributed system.
//!
//! This crate contains all configuration constants used throughout the Aspen
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
//! - [`directory`]: Directory service constants
//!
//! # Usage
//!
//! Access constants via their submodule:
//! ```
//! use aspen_constants::api::MAX_KEY_SIZE;
//! use aspen_constants::network::CONNECT_TIMEOUT_SECS;
//! ```
//!
//! Or use the prelude for common constants:
//! ```
//! use aspen_constants::prelude::*;
//! ```

pub mod api;
mod assertions;
pub mod ci;
pub mod coordination;
pub mod directory;
pub mod network;
pub mod raft;

/// Prelude module for commonly used constants.
pub mod prelude {
    // Key-value size limits
    pub use crate::api::DEFAULT_SCAN_LIMIT;
    // SQL constants (feature-gated)
    #[cfg(feature = "sql")]
    pub use crate::api::DEFAULT_SQL_RESULT_ROWS;
    #[cfg(feature = "sql")]
    pub use crate::api::DEFAULT_SQL_TIMEOUT_MS;
    pub use crate::api::MAX_KEY_SIZE;
    pub use crate::api::MAX_SCAN_RESULTS;
    pub use crate::api::MAX_SETMULTI_KEYS;
    #[cfg(feature = "sql")]
    pub use crate::api::MAX_SQL_PARAMS;
    #[cfg(feature = "sql")]
    pub use crate::api::MAX_SQL_QUERY_SIZE;
    #[cfg(feature = "sql")]
    pub use crate::api::MAX_SQL_RESULT_ROWS;
    #[cfg(feature = "sql")]
    pub use crate::api::MAX_SQL_TIMEOUT_MS;
    pub use crate::api::MAX_VALUE_SIZE;
    // CAS retry constants
    pub use crate::coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
    pub use crate::coordination::CAS_RETRY_MAX_BACKOFF_MS;
    pub use crate::coordination::MAX_CAS_RETRIES;
    // RWLock and Semaphore constants
    pub use crate::coordination::MAX_RWLOCK_PENDING_WRITERS;
    pub use crate::coordination::MAX_RWLOCK_READERS;
    pub use crate::coordination::MAX_SEMAPHORE_HOLDERS;
}

// Re-export commonly used constants at crate root for backwards compatibility
pub use api::DEFAULT_SCAN_LIMIT;
#[cfg(feature = "sql")]
pub use api::DEFAULT_SQL_RESULT_ROWS;
#[cfg(feature = "sql")]
pub use api::DEFAULT_SQL_TIMEOUT_MS;
pub use api::MAX_KEY_SIZE;
pub use api::MAX_SCAN_RESULTS;
pub use api::MAX_SETMULTI_KEYS;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_PARAMS;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_QUERY_SIZE;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_RESULT_ROWS;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_TIMEOUT_MS;
pub use api::MAX_VALUE_SIZE;
pub use coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
pub use coordination::CAS_RETRY_MAX_BACKOFF_MS;
pub use coordination::MAX_CAS_RETRIES;
pub use coordination::MAX_RWLOCK_PENDING_WRITERS;
pub use coordination::MAX_RWLOCK_READERS;
pub use coordination::MAX_SEMAPHORE_HOLDERS;
