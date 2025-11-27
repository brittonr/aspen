//! Hiqlite Service - Distributed SQLite with Raft Consensus
//!
//! This module provides a distributed state management layer using hiqlite,
//! a Raft-replicated SQLite database.

mod service;
mod schema;
mod cluster_formation;

// Re-export the main service
pub use service::HiqliteService;
pub use cluster_formation::{ClusterFormationCoordinator, ClusterFormationConfig};
