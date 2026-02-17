//! Command modules for aspen-cli.
//!
//! Each module handles a category of operations.

#[cfg(feature = "automerge")]
pub mod automerge;
pub mod barrier;
pub mod blob;
pub mod branch;
#[cfg(feature = "ci")]
pub mod cache;
#[cfg(feature = "ci")]
pub mod ci;
pub mod cluster;
pub mod counter;
#[cfg(feature = "dns")]
pub mod dns;
pub mod docs;
pub mod federation;
pub mod git;
pub mod hooks;
pub mod index;
pub mod issue;
pub mod job;
pub mod kv;
pub mod lease;
pub mod lock;
pub mod patch;
pub mod peer;
#[cfg(feature = "pijul")]
pub mod pijul;
#[cfg(feature = "plugins-rpc")]
pub mod plugin;
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
