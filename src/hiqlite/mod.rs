//! Hiqlite Service - Distributed SQLite with Raft Consensus
//!
//! This module provides a distributed state management layer using hiqlite,
//! a Raft-replicated SQLite database.

mod service;
mod schema;

// Re-export the main service
pub use service::HiqliteService;
