//! Job manager for submitting and managing jobs.

mod config;
mod dependencies;
mod dlq;
mod lifecycle;
mod query;
mod scheduling;
mod storage;

use std::collections::HashMap;
use std::sync::Arc;

use aspen_coordination::QueueConfig;
use aspen_coordination::QueueManager;
use aspen_coordination::ServiceRegistry;
use aspen_traits::KeyValueStore;
pub use config::JobManagerConfig;
use tokio::sync::RwLock;
use tracing::info;

use crate::dependency_tracker::DependencyGraph;
use crate::error::JobError;
use crate::error::Result;
use crate::types::Priority;

/// Callback type for job completion notifications.
pub type JobCompletionCallback = Arc<
    dyn Fn(crate::job::JobId, crate::job::JobResult) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// Job storage key prefix.
const JOB_PREFIX: &str = "__jobs:";
/// Job schedule prefix.
const JOB_SCHEDULE_PREFIX: &str = "__jobs:schedule:";

/// Maximum retries when we hit NotLeader errors during Raft operations.
/// Leadership gaps during elections are typically brief, so we retry aggressively.
const MAX_NOT_LEADER_RETRIES: u32 = 100;

/// Check if an error is a NotLeader error, returning the leader hint if so.
fn is_not_leader_error(err: &aspen_kv_types::KeyValueStoreError) -> Option<Option<u64>> {
    use aspen_kv_types::KeyValueStoreError;
    match err {
        KeyValueStoreError::NotLeader { leader, .. } => Some(*leader),
        KeyValueStoreError::Failed { reason } if reason.contains("forward") => Some(None),
        _ => None,
    }
}

/// Manager for job submission and lifecycle.
pub struct JobManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    queue_managers: HashMap<Priority, QueueManager<S>>,
    config: JobManagerConfig,
    #[allow(dead_code)] // Reserved for future service discovery integration
    service_registry: ServiceRegistry<S>,
    dependency_graph: Arc<DependencyGraph>,
    initialized: Arc<RwLock<bool>>,
    /// Optional callback for job completion notifications (used by workflow manager).
    completion_callback: RwLock<Option<JobCompletionCallback>>,
}

impl<S: KeyValueStore + ?Sized + 'static> JobManager<S> {
    /// Create a new job manager.
    pub fn new(store: Arc<S>) -> Self {
        Self::with_config(store, JobManagerConfig::default())
    }

    /// Create a job manager with custom configuration.
    pub fn with_config(store: Arc<S>, config: JobManagerConfig) -> Self {
        let mut queue_managers = HashMap::new();

        // Create queue manager for each priority level
        for priority in Priority::all_ordered() {
            let queue_manager = QueueManager::new(store.clone());
            queue_managers.insert(priority, queue_manager);
        }

        let service_registry = ServiceRegistry::new(store.clone());
        let dependency_graph = Arc::new(DependencyGraph::new());
        let initialized = Arc::new(RwLock::new(false));

        Self {
            store,
            queue_managers,
            config,
            service_registry,
            dependency_graph,
            initialized,
            completion_callback: RwLock::new(None),
        }
    }

    /// Ensure the job system is initialized (lazy initialization).
    async fn ensure_initialized(&self) -> Result<()> {
        // Fast path: already initialized
        let initialized = self.initialized.read().await;
        if *initialized {
            return Ok(());
        }
        drop(initialized);

        // Slow path: need to initialize
        let mut initialized = self.initialized.write().await;
        if !*initialized {
            self.initialize_internal().await?;
            *initialized = true;
        }
        Ok(())
    }

    /// Initialize the job system (create queues).
    pub async fn initialize(&self) -> Result<()> {
        self.ensure_initialized().await
    }

    /// Internal initialization logic.
    async fn initialize_internal(&self) -> Result<()> {
        // Create a queue for each priority level
        for priority in Priority::all_ordered() {
            let queue_name = format!("{}:{}", JOB_PREFIX, priority.queue_name());
            let queue_config = QueueConfig {
                default_visibility_timeout_ms: Some(self.config.default_visibility_timeout.as_millis() as u64),
                default_ttl_ms: None,
                max_delivery_attempts: Some(3),
            };

            if let Some(queue_manager) = self.queue_managers.get(&priority) {
                queue_manager
                    .create(&queue_name, queue_config)
                    .await
                    .map_err(|e| JobError::QueueError { source: e })?;

                info!(queue_name, ?priority, "initialized job queue");
            }
        }

        Ok(())
    }
}
