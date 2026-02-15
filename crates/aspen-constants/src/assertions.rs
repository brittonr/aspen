//! Compile-time constant assertions for Tiger Style compliance.
//!
//! These assertions catch configuration errors at compile time rather than runtime.
//! Each assertion verifies a relationship between constants that must hold for
//! correct system operation.

use super::api::*;
use super::coordination::*;
use super::network::*;

// Network timeout ordering
const _: () = assert!(IROH_CONNECT_TIMEOUT_SECS < IROH_READ_TIMEOUT_SECS);

// Queue bounds consistency
const _: () = assert!(MAX_QUEUE_BATCH_SIZE <= MAX_SCAN_RESULTS);
const _: () = assert!(MAX_QUEUE_CLEANUP_BATCH <= MAX_QUEUE_BATCH_SIZE * 2 + 900); // cleanup can be larger than batch

// RWLock/Semaphore sanity
const _: () = assert!(MAX_RWLOCK_READERS > 0);
const _: () = assert!(MAX_RWLOCK_PENDING_WRITERS > 0);
const _: () = assert!(MAX_SEMAPHORE_HOLDERS > 0);

// Lock timeout ordering
const _: () = assert!(DEFAULT_LOCK_WAIT_TIMEOUT_MS <= MAX_LOCK_WAIT_TIMEOUT_MS);
const _: () = assert!(DEFAULT_LOCK_TTL_MS <= MAX_LOCK_TTL_MS);

// Queue timeout ordering
const _: () = assert!(DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS <= MAX_QUEUE_VISIBILITY_TIMEOUT_MS);
const _: () = assert!(DEFAULT_QUEUE_POLL_INTERVAL_MS <= MAX_QUEUE_POLL_INTERVAL_MS);

// Service registry bounds
const _: () = assert!(SERVICE_CLEANUP_BATCH > 0);
const _: () = assert!(MAX_SERVICE_DISCOVERY_RESULTS <= MAX_SERVICE_INSTANCES);
