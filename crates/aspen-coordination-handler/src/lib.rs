//! Coordination primitives RPC handler for Aspen.
//!
//! This crate provides the coordination handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles distributed coordination primitives:
//! - Locks: Distributed mutual exclusion
//! - Counters: Atomic counters (unsigned and signed)
//! - Sequences: Monotonic ID generation
//! - Rate limiters: Token bucket rate limiting
//! - Barriers: Multi-party synchronization
//! - Semaphores: Counting semaphores
//! - RWLocks: Reader-writer locks
//! - Queues: Distributed message queues

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::CoordinationHandler;
