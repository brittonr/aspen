//! Guest agent library for Cloud Hypervisor CI VMs.
//!
//! This crate provides both a binary (`aspen-ci-agent`) that runs inside VMs
//! and a library with shared types for host-guest communication.
//!
//! ## Host-Guest Protocol
//!
//! Communication uses length-prefixed JSON frames over vsock:
//!
//! ```text
//! +----------------+------------------+
//! | Length (4 BE)  | JSON payload     |
//! +----------------+------------------+
//! ```
//!
//! The host sends [`HostMessage`] and receives [`AgentMessage`].
//!
//! [`HostMessage`]: protocol::HostMessage
//! [`AgentMessage`]: protocol::AgentMessage

// Allow unused code - this is API surface for the binary and host-side code
#![allow(dead_code)]

pub mod error;
pub mod executor;
pub mod protocol;
pub mod vsock_server;

pub use executor::Executor;
pub use protocol::AgentMessage;
pub use protocol::ExecutionRequest;
pub use protocol::ExecutionResult;
pub use protocol::HostMessage;
pub use protocol::LogMessage;
pub use protocol::MAX_MESSAGE_SIZE;
