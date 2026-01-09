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
pub mod hooks;
pub mod issue;
pub mod job;
pub mod kv;
pub mod lease;
pub mod lock;
pub mod patch;
pub mod peer;
#[cfg(feature = "pijul")]
pub mod pijul;
pub mod queue;
pub mod ratelimit;
pub mod rwlock;
#[cfg(feature = "secrets")]
pub mod secrets;
pub mod semaphore;
pub mod sequence;
pub mod service;
#[cfg(feature = "sql")]
pub mod sql;
pub mod tag;
pub mod verify;
