//! Event bridges between cluster subsystems and the hook service.
//!
//! This crate contains the bridge modules that convert domain-specific events
//! (Raft log entries, blob events, docs events, system metrics, snapshots, TTL
//! expirations) into `HookEvent`s and dispatch them to registered handlers via
//! `HookService`.
//!
//! Each bridge runs as an independent background task and is feature-gated to
//! keep the dependency footprint minimal.

#![allow(dead_code, unused_imports)]

#[cfg(feature = "hooks")]
pub mod hooks_bridge;

#[cfg(feature = "hooks")]
pub mod ttl_events_bridge;

#[cfg(feature = "hooks")]
pub mod system_events_bridge;

#[cfg(feature = "hooks")]
pub mod snapshot_events_bridge;

#[cfg(all(feature = "blob", feature = "hooks"))]
pub mod blob_bridge;

#[cfg(all(feature = "docs", feature = "hooks"))]
pub mod docs_bridge;
