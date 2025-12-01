//! Aspen's Raft-backed key-value subsystem.
//!
//! The module is built around OpenRaft's storage traits and provides a concrete
//! state machine (`KvStateMachine`) and log store (`KvLogStore`) that persist
//! data in `redb`. Networking and node orchestration live in dedicated files so
//! the implementation remains easy to reason about in isolation.

pub mod api;
pub mod client;
pub mod error;
pub mod network;
pub mod node;
pub mod service;
pub mod store;
pub mod types;
