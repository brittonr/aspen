//! Types and error definitions for the worker service.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_jobs::AffinityJobManager;
use aspen_jobs::DistributedJobRouter;
use aspen_jobs::DistributedWorkerPool;
use aspen_jobs::JobManager;
use aspen_jobs::Worker;
use aspen_jobs::WorkerPool;
use iroh::PublicKey as NodeId;
use snafu::Snafu;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;

use crate::config::WorkerConfig;

/// Errors that can occur in the worker service.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum WorkerServiceError {
    /// Failed to initialize worker pool.
    #[snafu(display("failed to initialize worker pool: {}", source))]
    InitializePool { source: aspen_jobs::JobError },

    /// Failed to register worker handler.
    #[snafu(display("failed to register worker handler '{}': {}", job_type, source))]
    RegisterHandler {
        job_type: String,
        source: aspen_jobs::JobError,
    },

    /// Failed to start workers.
    #[snafu(display("failed to start {} workers: {}", count, source))]
    StartWorkers { count: u32, source: aspen_jobs::JobError },

    /// Failed to update worker metadata.
    #[snafu(display("failed to update worker metadata: {}", source))]
    UpdateMetadata { source: aspen_jobs::JobError },

    /// Worker configuration is invalid.
    #[snafu(display("invalid worker configuration: {}", reason))]
    InvalidConfig { reason: String },

    /// Failed to shutdown workers.
    #[snafu(display("failed to shutdown workers: {}", source))]
    Shutdown { source: aspen_jobs::JobError },
}

pub type Result<T> = std::result::Result<T, WorkerServiceError>;

/// Service that manages worker pools on an Aspen node.
///
/// The WorkerService integrates with the node's job manager to provide
/// distributed job execution capabilities. It manages worker lifecycle,
/// registers handlers, and tracks worker health.
pub struct WorkerService {
    /// Node identifier.
    pub(super) node_id: u64,

    /// Iroh node ID for P2P affinity.
    pub(super) iroh_node_id: NodeId,

    /// Worker configuration from node config.
    pub(super) config: WorkerConfig,

    /// Job manager for the cluster.
    pub(super) job_manager: Arc<JobManager<dyn KeyValueStore>>,

    /// Affinity manager for P2P-aware job routing.
    pub(super) affinity_manager: Arc<AffinityJobManager<dyn KeyValueStore>>,

    /// Worker pool instance.
    pub(super) pool: Arc<WorkerPool<dyn KeyValueStore>>,

    /// Distributed worker pool (optional, enabled via config).
    pub(super) distributed_pool: Option<Arc<DistributedWorkerPool<dyn KeyValueStore>>>,

    /// Distributed job router.
    pub(super) distributed_router: Option<DistributedJobRouter<dyn KeyValueStore>>,

    /// Registered worker handlers.
    pub(super) handlers: Arc<RwLock<Vec<Arc<dyn Worker>>>>,

    /// Handle to the worker monitoring task.
    pub(super) monitor_handle: Option<JoinHandle<()>>,

    /// Task tracker for spawned tasks (monitoring, etc.).
    pub(super) task_tracker: TaskTracker,

    /// Shutdown signal.
    pub(super) shutdown: Arc<tokio::sync::Notify>,
}
