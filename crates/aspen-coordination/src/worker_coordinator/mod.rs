//! Distributed worker coordinator for cross-node worker management.
//!
//! This module provides cluster-wide coordination for distributed workers,
//! enabling load balancing, work stealing, and worker group management.
//!
//! ## Features
//!
//! - Global worker registry and discovery
//! - Load-based job routing with pluggable strategies
//! - Work stealing for dynamic load rebalancing
//! - Worker group coordination for multi-worker tasks
//! - Automatic failover and job redistribution
//! - Health monitoring and capacity management
//!
//! ## Tiger Style
//!
//! - Fixed limits on workers and groups (MAX_WORKERS = 1024, MAX_GROUPS = 64)
//! - Bounded work stealing batches (MAX_STEAL_BATCH = 10)
//! - Fail-fast on invalid configurations
//! - All operations through Raft consensus for consistency

pub(crate) mod constants;
mod failover;
mod group_coordination;
mod load_balancing;
mod registry;
pub(crate) mod types;
mod work_stealing;

use std::collections::HashMap;
use std::sync::Arc;

use aspen_traits::KeyValueStore;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
pub use types::GroupState;
pub use types::LoadBalancingStrategy;
pub use types::StealHint;
pub use types::WorkerCoordinatorConfig;
pub use types::WorkerFilter;
pub use types::WorkerGroup;
pub use types::WorkerInfo;
pub use types::WorkerStats;

use crate::registry::ServiceRegistry;

/// Distributed worker coordinator for cluster-wide worker management.
#[derive(Clone)]
pub struct DistributedWorkerCoordinator<S: KeyValueStore + ?Sized> {
    /// Key-value store for coordination state.
    store: Arc<S>,
    /// Service registry for worker discovery.
    registry: ServiceRegistry<S>,
    /// Configuration.
    config: WorkerCoordinatorConfig,
    /// Local cache of worker info.
    workers: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    /// Worker groups.
    groups: Arc<RwLock<HashMap<String, WorkerGroup>>>,
    /// Round-robin counter for simple distribution.
    round_robin_counter: Arc<RwLock<usize>>,
    /// Round-robin counter for work stealing source selection.
    steal_source_counter: Arc<RwLock<usize>>,
    /// Background task handles.
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// Shutdown signal.
    shutdown: Arc<tokio::sync::Notify>,
}

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerCoordinator<S> {
    /// Create a new distributed worker coordinator.
    pub fn new(store: Arc<S>) -> Self {
        Self::with_config(store, WorkerCoordinatorConfig::default())
    }

    /// Create a coordinator with custom configuration.
    pub fn with_config(store: Arc<S>, config: WorkerCoordinatorConfig) -> Self {
        let registry = ServiceRegistry::new(store.clone());

        Self {
            store,
            registry,
            config,
            workers: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counter: Arc::new(RwLock::new(0)),
            steal_source_counter: Arc::new(RwLock::new(0)),
            tasks: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }
}

#[cfg(test)]
mod tests;
