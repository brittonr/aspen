//! Centralized constants for Aspen distributed system.
//!
//! This module re-exports constants from `aspen-constants` for backward compatibility.
//! New code should prefer importing from `aspen_constants` directly for lighter dependencies.
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
//! use aspen_core::constants::api::MAX_KEY_SIZE;
//! use aspen_core::constants::network::IROH_CONNECT_TIMEOUT;
//! ```
//!
//! Or use the prelude for common constants:
//! ```
//! use aspen_core::prelude::*; // Includes MAX_KEY_SIZE, MAX_VALUE_SIZE, etc.
//! ```

// Re-export all modules from aspen-constants
pub use aspen_constants::api;
pub use aspen_constants::ci;
pub use aspen_constants::coordination;
pub use aspen_constants::directory;
pub use aspen_constants::raft;

// Create a network module that re-exports base constants and adds Duration versions
pub mod network {
    //! Network constants for Aspen distributed system.
    //!
    //! This module re-exports constants from `aspen_constants::network` and provides
    //! `std::time::Duration` versions for convenient use with async timeouts.

    use std::time::Duration;

    // Re-export all base constants from aspen-constants
    pub use aspen_constants::network::*;

    // Duration versions of timeout constants for backward compatibility
    // These are used with tokio::time::timeout() and other async functions

    /// Timeout for Iroh connection establishment (5 seconds).
    pub const IROH_CONNECT_TIMEOUT: Duration = Duration::from_secs(IROH_CONNECT_TIMEOUT_SECS);

    /// Timeout for bidirectional stream open (2 seconds).
    pub const IROH_STREAM_OPEN_TIMEOUT: Duration = Duration::from_secs(IROH_STREAM_OPEN_TIMEOUT_SECS);

    /// Timeout for RPC response read (10 seconds).
    pub const IROH_READ_TIMEOUT: Duration = Duration::from_secs(IROH_READ_TIMEOUT_SECS);

    /// Timeout for ReadIndex linearizability check (5 seconds).
    pub const READ_INDEX_TIMEOUT: Duration = Duration::from_secs(READ_INDEX_TIMEOUT_SECS);

    /// Timeout for cluster membership operations (30 seconds).
    pub const MEMBERSHIP_OPERATION_TIMEOUT: Duration = Duration::from_secs(MEMBERSHIP_OPERATION_TIMEOUT_SECS);

    /// Timeout for gossip subscription (10 seconds).
    pub const GOSSIP_SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(GOSSIP_SUBSCRIBE_TIMEOUT_SECS);
}

// Create a raft module that re-exports base constants and adds Duration versions
pub mod raft_compat {
    //! Raft constants with Duration versions for backward compatibility.

    use std::time::Duration;

    // Re-export all base constants from aspen-constants
    pub use aspen_constants::raft::*;

    /// Cooldown period for membership changes (300 seconds / 5 minutes).
    pub const MEMBERSHIP_COOLDOWN: Duration = Duration::from_secs(MEMBERSHIP_COOLDOWN_SECS);
}

// Re-export commonly used constants at module root for backwards compatibility
pub use api::DEFAULT_SCAN_LIMIT;
// SQL constants (feature-gated)
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
// CAS retry constants
pub use coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
pub use coordination::CAS_RETRY_MAX_BACKOFF_MS;
pub use coordination::MAX_CAS_RETRIES;
// RWLock and Semaphore constants
pub use coordination::MAX_RWLOCK_PENDING_WRITERS;
pub use coordination::MAX_RWLOCK_READERS;
pub use coordination::MAX_SEMAPHORE_HOLDERS;
// Re-export Duration constants at module root for backward compatibility
pub use network::GOSSIP_SUBSCRIBE_TIMEOUT;
pub use network::IROH_CONNECT_TIMEOUT;
pub use network::IROH_READ_TIMEOUT;
pub use network::IROH_STREAM_OPEN_TIMEOUT;
pub use network::MEMBERSHIP_OPERATION_TIMEOUT;
pub use network::READ_INDEX_TIMEOUT;
pub use raft_compat::MEMBERSHIP_COOLDOWN;
