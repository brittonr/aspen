//! Log subscriber protocol handler.
//!
//! This module re-exports the log subscriber protocol handler from the raft module.
//! The implementation is consolidated in `crate::raft::log_subscriber` for maintainability.

pub use crate::raft::log_subscriber::LogSubscriberProtocolHandler;
