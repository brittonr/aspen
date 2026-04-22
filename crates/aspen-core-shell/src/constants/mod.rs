//! Centralized constants for Aspen distributed system shell consumers.
//!
//! This wrapper re-exports the alloc-safe `aspen-core::constants` surface and
//! adds `core::time::Duration` compatibility constants for runtime helpers.

pub use aspen_core::constants::api;
pub use aspen_core::constants::ci;
pub use aspen_core::constants::coordination;
pub use aspen_core::constants::directory;
pub use aspen_core::constants::raft;

pub mod network {
    //! Network constants for Aspen distributed system shell consumers.

    use core::time::Duration;

    pub use aspen_core::constants::network::*;

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

pub mod raft_compat {
    //! Raft constants with `Duration` versions for backward compatibility.

    use core::time::Duration;

    pub use aspen_core::constants::raft::*;

    /// Cooldown period for membership changes (300 seconds / 5 minutes).
    pub const MEMBERSHIP_COOLDOWN: Duration = Duration::from_secs(MEMBERSHIP_COOLDOWN_SECS);
}

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
pub use network::GOSSIP_SUBSCRIBE_TIMEOUT;
pub use network::IROH_CONNECT_TIMEOUT;
pub use network::IROH_READ_TIMEOUT;
pub use network::IROH_STREAM_OPEN_TIMEOUT;
pub use network::MEMBERSHIP_OPERATION_TIMEOUT;
pub use network::READ_INDEX_TIMEOUT;
pub use raft_compat::MEMBERSHIP_COOLDOWN;
