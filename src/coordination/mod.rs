//! Distributed coordination primitives built on CAS operations.
//!
//! This module provides high-level coordination primitives for distributed systems:
//!
//! - [`DistributedLock`] - Mutual exclusion with fencing tokens
//! - [`AtomicCounter`] - Race-free increment/decrement
//! - [`SequenceGenerator`] - Monotonically increasing unique IDs
//! - [`DistributedRateLimiter`] - Token bucket rate limiting
//!
//! All primitives are built on top of the [`KeyValueStore`] trait's CAS operations,
//! providing linearizable semantics through Raft consensus.
//!
//! ## Example
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

mod counter;
mod error;
mod lock;
mod rate_limiter;
mod sequence;
mod types;

pub use counter::{AtomicCounter, BufferedCounter, CounterConfig, SignedAtomicCounter};
pub use error::{CoordinationError, FenceError, RateLimitError};
pub use lock::{DistributedLock, LockConfig, LockGuard};
pub use rate_limiter::{DistributedRateLimiter, RateLimiterConfig};
pub use sequence::{SequenceConfig, SequenceGenerator};
pub use types::{FencingToken, LockEntry};
