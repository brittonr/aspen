//! Aspen library entry point.
//!
//! The previous iterations of the crate had a much richer surface area that
//! bound `openraft`, Iroh, and `ractor`. Those modules were wiped so we could
//! rebuild the stack deliberately. For now we expose lightweight scaffolding
//! that mirrors the control-plane and transport traits used throughout the
//! integration tests and examples. Each module intentionally keeps the API
//! narrow so we can iterate quickly while wiring the real implementations back
//! in.

pub mod api;
pub mod cluster;
pub mod kv;
pub mod raft;
pub mod simulation;

/// Testing infrastructure for deterministic multi-node Raft tests.
///
/// Provides `AspenRouter` for managing in-memory Raft clusters with simulated
/// networking. Used by integration tests in tests/ directory.
pub mod testing;

/// System utility functions for resource management and health checks.
///
/// Provides Tiger Style resource management including disk space checking
/// with fixed thresholds and fail-fast semantics.
pub mod utils;

pub use kv::{KvClient, KvServiceBuilder};
pub use raft::RaftControlClient;
