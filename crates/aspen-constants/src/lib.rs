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
//! use aspen_constants::network::IROH_CONNECT_TIMEOUT_SECS;
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
pub mod plugin;
pub mod proxy;
pub mod raft;
pub mod wasm;

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
pub use api::DEFAULT_TRACE_QUERY_LIMIT;
pub use api::MAX_CUSTOM_INDEXES;
pub use api::MAX_INDEX_NAME_SIZE;
pub use api::MAX_KEY_SIZE;
pub use api::MAX_SCAN_RESULTS;
pub use api::MAX_SETMULTI_KEYS;
pub use api::MAX_SPAN_ATTRIBUTES;
pub use api::MAX_SPAN_EVENTS;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_PARAMS;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_QUERY_SIZE;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_RESULT_ROWS;
#[cfg(feature = "sql")]
pub use api::MAX_SQL_TIMEOUT_MS;
pub use api::MAX_TRACE_BATCH_SIZE;
pub use api::MAX_TRACE_QUERY_RESULTS;
pub use api::MAX_VALUE_SIZE;
pub use coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
pub use coordination::CAS_RETRY_MAX_BACKOFF_MS;
pub use coordination::MAX_CAS_RETRIES;
pub use coordination::MAX_RWLOCK_PENDING_WRITERS;
pub use coordination::MAX_RWLOCK_READERS;
pub use coordination::MAX_SEMAPHORE_HOLDERS;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Golden values — catch accidental constant changes.
    // If a value changes intentionally, update the test AND audit all callers.
    // =========================================================================

    #[test]
    fn golden_api_constants() {
        assert_eq!(api::MAX_KEY_SIZE, 1024, "MAX_KEY_SIZE changed");
        assert_eq!(api::MAX_VALUE_SIZE, 1024 * 1024, "MAX_VALUE_SIZE changed");
        assert_eq!(api::MAX_SETMULTI_KEYS, 100, "MAX_SETMULTI_KEYS changed");
        assert_eq!(api::MAX_SCAN_RESULTS, 10_000, "MAX_SCAN_RESULTS changed");
        assert_eq!(api::DEFAULT_SCAN_LIMIT, 1_000, "DEFAULT_SCAN_LIMIT changed");
        assert_eq!(api::MAX_BINARY_SIZE, 50 * 1024 * 1024, "MAX_BINARY_SIZE changed");
        assert_eq!(api::MAX_BUILD_TIME_MS, 300_000, "MAX_BUILD_TIME_MS changed");
        assert_eq!(api::MAX_CONCURRENT_VMS, 100, "MAX_CONCURRENT_VMS changed");
        assert_eq!(api::VM_STARTUP_TIMEOUT_MS, 10, "VM_STARTUP_TIMEOUT_MS changed");
        assert_eq!(api::DEFAULT_VM_TIMEOUT_MS, 5_000, "DEFAULT_VM_TIMEOUT_MS changed");
        assert_eq!(api::MAX_VM_TIMEOUT_MS, 60_000, "MAX_VM_TIMEOUT_MS changed");
        assert_eq!(network::CLIENT_ALPN, b"aspen-client", "CLIENT_ALPN changed");
    }

    #[test]
    fn golden_network_constants() {
        assert_eq!(network::MAX_RPC_MESSAGE_SIZE, 10 * 1024 * 1024);
        assert_eq!(network::IROH_CONNECT_TIMEOUT_SECS, 5);
        assert_eq!(network::IROH_STREAM_OPEN_TIMEOUT_SECS, 2);
        assert_eq!(network::IROH_READ_TIMEOUT_SECS, 10);
        assert_eq!(network::MAX_SNAPSHOT_SIZE, 100 * 1024 * 1024);
        assert_eq!(network::MAX_STREAMS_PER_CONNECTION, 100);
        assert_eq!(network::MAX_CONCURRENT_CONNECTIONS, 500);
        assert_eq!(network::MAX_PEERS, 1000);
        assert_eq!(network::MAX_UNREACHABLE_NODES, 1000);
        assert_eq!(network::MAX_PEER_COUNT, 1000);
        assert_eq!(network::FAILURE_DETECTOR_CHANNEL_CAPACITY, 100);
    }

    #[test]
    fn golden_gossip_constants() {
        assert_eq!(network::GOSSIP_MAX_TRACKED_PEERS, 256);
        assert_eq!(network::GOSSIP_PER_PEER_RATE_PER_MINUTE, 12);
        assert_eq!(network::GOSSIP_PER_PEER_BURST, 3);
        assert_eq!(network::GOSSIP_GLOBAL_RATE_PER_MINUTE, 10_000);
        assert_eq!(network::GOSSIP_GLOBAL_BURST, 100);
        assert_eq!(network::GOSSIP_MAX_STREAM_RETRIES, 5);
        assert_eq!(network::GOSSIP_STREAM_BACKOFF_SECS, [1, 2, 4, 8, 16]);
        assert_eq!(network::GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS, 10);
        assert_eq!(network::GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS, 60);
        assert_eq!(network::GOSSIP_ANNOUNCE_FAILURE_THRESHOLD, 3);
    }

    #[test]
    fn golden_timeout_constants() {
        assert_eq!(network::READ_INDEX_TIMEOUT_SECS, 5);
        assert_eq!(network::MEMBERSHIP_OPERATION_TIMEOUT_SECS, 30);
        assert_eq!(network::GOSSIP_SUBSCRIBE_TIMEOUT_SECS, 10);
        assert_eq!(network::SNAPSHOT_INSTALL_TIMEOUT_MS, 5000);
    }

    #[test]
    fn golden_coordination_constants() {
        assert_eq!(coordination::MAX_CAS_RETRIES, 100);
        assert_eq!(coordination::CAS_RETRY_INITIAL_BACKOFF_MS, 1);
        assert_eq!(coordination::CAS_RETRY_MAX_BACKOFF_MS, 128);
        assert_eq!(coordination::MAX_QUEUE_ITEM_SIZE, 1024 * 1024);
        assert_eq!(coordination::MAX_QUEUE_BATCH_SIZE, 100);
        assert_eq!(coordination::MAX_QUEUE_VISIBILITY_TIMEOUT_MS, 3_600_000);
        assert_eq!(coordination::DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS, 30_000);
        assert_eq!(coordination::MAX_RWLOCK_READERS, 128);
        assert_eq!(coordination::MAX_RWLOCK_PENDING_WRITERS, 64);
        assert_eq!(coordination::MAX_SEMAPHORE_HOLDERS, 256);
        assert_eq!(coordination::MAX_LOCK_WAIT_TIMEOUT_MS, 300_000);
        assert_eq!(coordination::DEFAULT_LOCK_WAIT_TIMEOUT_MS, 30_000);
        assert_eq!(coordination::MAX_LOCK_TTL_MS, 3_600_000);
        assert_eq!(coordination::DEFAULT_LOCK_TTL_MS, 60_000);
    }

    #[test]
    fn golden_raft_constants() {
        assert_eq!(raft::MAX_BATCH_SIZE, 1000);
    }

    #[test]
    fn golden_file_size_constants() {
        assert_eq!(network::MAX_CONFIG_FILE_SIZE, 10 * 1024 * 1024);
        assert_eq!(network::MAX_JOB_SPEC_SIZE, 1024 * 1024);
        assert_eq!(network::MAX_SOPS_FILE_SIZE, 10 * 1024 * 1024);
        assert_eq!(network::MAX_SQL_FILE_SIZE, 1024 * 1024);
        assert_eq!(network::MAX_KEY_FILE_SIZE, 64 * 1024);
        assert_eq!(network::MAX_GIT_OBJECT_TREE_DEPTH, 1000);
        assert_eq!(network::MAX_GIT_OBJECTS_PER_PUSH, 100_000);
        assert_eq!(network::MAX_GIT_OBJECT_SIZE, 100 * 1024 * 1024);
    }

    // =========================================================================
    // Prelude re-exports match direct module access
    // =========================================================================

    #[test]
    fn prelude_reexports_match_modules() {
        assert_eq!(prelude::MAX_KEY_SIZE, api::MAX_KEY_SIZE);
        assert_eq!(prelude::MAX_VALUE_SIZE, api::MAX_VALUE_SIZE);
        assert_eq!(prelude::MAX_SCAN_RESULTS, api::MAX_SCAN_RESULTS);
        assert_eq!(prelude::DEFAULT_SCAN_LIMIT, api::DEFAULT_SCAN_LIMIT);
        assert_eq!(prelude::MAX_CAS_RETRIES, coordination::MAX_CAS_RETRIES);
        assert_eq!(prelude::CAS_RETRY_INITIAL_BACKOFF_MS, coordination::CAS_RETRY_INITIAL_BACKOFF_MS);
        assert_eq!(prelude::CAS_RETRY_MAX_BACKOFF_MS, coordination::CAS_RETRY_MAX_BACKOFF_MS);
        assert_eq!(prelude::MAX_RWLOCK_READERS, coordination::MAX_RWLOCK_READERS);
        assert_eq!(prelude::MAX_RWLOCK_PENDING_WRITERS, coordination::MAX_RWLOCK_PENDING_WRITERS);
        assert_eq!(prelude::MAX_SEMAPHORE_HOLDERS, coordination::MAX_SEMAPHORE_HOLDERS);
    }

    // =========================================================================
    // Crate root re-exports match modules
    // =========================================================================

    #[test]
    fn root_reexports_match_modules() {
        assert_eq!(MAX_KEY_SIZE, api::MAX_KEY_SIZE);
        assert_eq!(MAX_VALUE_SIZE, api::MAX_VALUE_SIZE);
        assert_eq!(MAX_SCAN_RESULTS, api::MAX_SCAN_RESULTS);
        assert_eq!(DEFAULT_SCAN_LIMIT, api::DEFAULT_SCAN_LIMIT);
        assert_eq!(MAX_SETMULTI_KEYS, api::MAX_SETMULTI_KEYS);
        assert_eq!(MAX_CAS_RETRIES, coordination::MAX_CAS_RETRIES);
        assert_eq!(CAS_RETRY_INITIAL_BACKOFF_MS, coordination::CAS_RETRY_INITIAL_BACKOFF_MS);
        assert_eq!(CAS_RETRY_MAX_BACKOFF_MS, coordination::CAS_RETRY_MAX_BACKOFF_MS);
    }

    // =========================================================================
    // Relationship invariants (runtime check, complements compile-time assertions)
    // =========================================================================

    #[test]
    fn timeout_ordering() {
        assert!(network::IROH_CONNECT_TIMEOUT_SECS < network::IROH_READ_TIMEOUT_SECS);
        assert!(network::IROH_STREAM_OPEN_TIMEOUT_SECS < network::IROH_READ_TIMEOUT_SECS);
        assert!(network::MEMBERSHIP_OPERATION_TIMEOUT_SECS > network::READ_INDEX_TIMEOUT_SECS);
    }

    #[test]
    fn size_ordering() {
        assert!(api::MAX_KEY_SIZE < api::MAX_VALUE_SIZE);
        assert!(api::DEFAULT_SCAN_LIMIT <= api::MAX_SCAN_RESULTS);
        assert!(network::MAX_RPC_MESSAGE_SIZE >= api::MAX_VALUE_SIZE, "RPC must fit largest value");
        assert!(network::MAX_SNAPSHOT_SIZE >= network::MAX_RPC_MESSAGE_SIZE as u64);
    }

    #[test]
    fn coordination_ordering() {
        assert!(coordination::CAS_RETRY_INITIAL_BACKOFF_MS <= coordination::CAS_RETRY_MAX_BACKOFF_MS);
        assert!(coordination::DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS <= coordination::MAX_QUEUE_VISIBILITY_TIMEOUT_MS);
        assert!(coordination::DEFAULT_QUEUE_POLL_INTERVAL_MS <= coordination::MAX_QUEUE_POLL_INTERVAL_MS);
        assert!(coordination::DEFAULT_LOCK_WAIT_TIMEOUT_MS <= coordination::MAX_LOCK_WAIT_TIMEOUT_MS);
        assert!(coordination::DEFAULT_LOCK_TTL_MS <= coordination::MAX_LOCK_TTL_MS);
        assert!(coordination::DEFAULT_SERVICE_TTL_MS <= coordination::MAX_SERVICE_TTL_MS);
    }

    #[test]
    fn gossip_rate_ordering() {
        assert!(
            network::GOSSIP_GLOBAL_RATE_PER_MINUTE > network::GOSSIP_PER_PEER_RATE_PER_MINUTE,
            "global rate must exceed per-peer rate"
        );
        assert!(
            network::GOSSIP_GLOBAL_BURST >= network::GOSSIP_PER_PEER_BURST,
            "global burst must be >= per-peer burst"
        );
    }
}
