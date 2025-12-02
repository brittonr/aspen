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

pub use kv::{KvClient, KvServiceBuilder};
pub use raft::RaftControlClient;
