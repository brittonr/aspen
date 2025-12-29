//! Command modules for aspen-cli.
//!
//! Each module handles a category of operations.

pub mod barrier;
pub mod blob;
pub mod branch;
pub mod cluster;
pub mod counter;
#[cfg(feature = "dns")]
pub mod dns;
pub mod docs;
pub mod federation;
pub mod git;
pub mod issue;
pub mod kv;
pub mod lease;
pub mod lock;
pub mod patch;
pub mod peer;
pub mod queue;
pub mod ratelimit;
pub mod rwlock;
pub mod semaphore;
pub mod sequence;
pub mod service;
#[cfg(feature = "sql")]
pub mod sql;
pub mod tag;
pub mod verify;
