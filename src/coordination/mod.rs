//! Distributed coordination primitives built on CAS operations.
//!
//! This module provides high-level coordination primitives for distributed systems:
//!
//! - [`DistributedLock`] - Mutual exclusion with fencing tokens
//! - [`LeaderElection`] - Leader election with automatic lease renewal
//! - [`AtomicCounter`] - Race-free increment/decrement
//! - [`SequenceGenerator`] - Monotonically increasing unique IDs
//! - [`DistributedRateLimiter`] - Token bucket rate limiting
//!
//! All primitives are built on top of the [`KeyValueStore`] trait's CAS operations,
//! providing linearizable semantics through Raft consensus.
//!
//! ## Leader Election Example
//!
//! ```ignore
//! use aspen::coordination::{LeaderElection, ElectionConfig};
//!
//! let election = LeaderElection::new(
//!     store,
//!     "my-service-leader",
//!     "node-1",
//!     ElectionConfig::default(),
//! );
//!
//! let handle = election.start().await?;
//!
//! // Check leadership state
//! if handle.is_leader() {
//!     let token = handle.fencing_token().unwrap();
//!     // Perform leader-only operations with fencing token
//! }
//!
//! // Graceful stepdown
//! handle.stepdown();
//! ```
//!
//! ## Lock Example
//!
//! ```ignore
//! use aspen::coordination::{DistributedLock, LockConfig};
//!
//! let lock = DistributedLock::new(store, "my_lock", "client_1", LockConfig::default());
//! let guard = lock.acquire().await?;
//!
//! // Protected critical section
//! // Fencing token can be passed to external services
//! let token = guard.fencing_token();
//!
//! // Lock released on drop
//! ```

mod barrier;
mod counter;
mod election;
mod error;
mod lock;
mod rate_limiter;
mod semaphore;
mod sequence;
mod types;

pub use barrier::{BarrierManager, BarrierPhase, BarrierState};
pub use counter::{AtomicCounter, BufferedCounter, CounterConfig, SignedAtomicCounter};
pub use election::{ElectionConfig, ElectionHandle, LeaderElection, LeadershipState};
pub use error::{CoordinationError, FenceError, RateLimitError};
pub use lock::{DistributedLock, LockConfig, LockGuard};
pub use rate_limiter::{DistributedRateLimiter, RateLimiterConfig};
pub use semaphore::{SemaphoreHolder, SemaphoreManager, SemaphoreState};
pub use sequence::{SequenceConfig, SequenceGenerator};
pub use types::{BucketState, FencingToken, LockEntry, now_unix_ms};
