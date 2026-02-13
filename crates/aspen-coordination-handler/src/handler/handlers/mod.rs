//! Handler sub-modules for coordination primitive operations.
//!
//! Each module contains handlers for a specific category of operations:
//! - `lock`: Distributed locks (acquire, release, renew)
//! - `counter`: Atomic counters (get, increment, decrement, set)
//! - `primitives`: Sequence, RateLimiter, Barrier, Semaphore
//! - `rwlock`: Reader-writer locks
//! - `queue`: Message queues and dead-letter queue operations

pub mod counter;
pub mod lock;
pub mod primitives;
pub mod queue;
pub mod rwlock;
