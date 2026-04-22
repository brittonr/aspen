//! Centralized constants for Aspen distributed system.
//!
//! This module re-exports constants from `aspen-constants` for backward
//! compatibility. New code should prefer importing from `aspen_constants`
//! directly for lighter dependencies.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.
//!
//! # Modules
//!
//! - [`api`]: Public API bounds (KV sizes, SQL limits, scan results)
//! - [`coordination`]: Distributed primitives (queues, registries, rate limits)
//! - [`network`]: Network timeout values expressed as primitive units
//! - [`raft`]: Consensus internals (membership, backoff, integrity)
//! - [`ci`]: CI/CD VM limits, log streaming, job execution
//! - [`directory`]: Directory service constants
//!
//! # Usage
//!
//! Access constants via their submodule:
//! ```
//! use aspen_core::constants::api::MAX_KEY_SIZE;
//! use aspen_core::constants::network::IROH_CONNECT_TIMEOUT_SECS;
//! ```
// Re-export all modules from aspen-constants.
pub use aspen_constants::api;
pub use aspen_constants::ci;
pub use aspen_constants::coordination;
pub use aspen_constants::directory;
pub use aspen_constants::network;
pub use aspen_constants::raft;

// Re-export commonly used constants at module root for backwards compatibility.
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
pub use coordination::MAX_LOCKSET_KEYS;
pub use coordination::MAX_RWLOCK_PENDING_WRITERS;
pub use coordination::MAX_RWLOCK_READERS;
pub use coordination::MAX_SEMAPHORE_HOLDERS;
