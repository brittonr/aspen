//! Worker pool configuration for distributed job execution.
//!
//! Configures how a node participates in the distributed job queue system.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Worker pool configuration for distributed job execution.
///
/// Configures how this node participates in the distributed job queue system.
/// Workers can be specialized by job type, tagged with capabilities, and
/// configured with resource limits.
///
/// # Tiger Style
///
/// - Fixed limits: Max workers, concurrent jobs bounded
/// - Explicit types: Tags and job types as Vec<String>
/// - Sensible defaults: CPU cores for worker count
///
/// # Example
///
/// ```toml
/// [worker]
/// enabled = true
/// worker_count = 4
/// max_concurrent_jobs = 10
/// job_types = ["process_data", "ml_inference"]
/// tags = ["gpu", "high_memory"]
/// prefer_local = true
/// data_locality_weight = 0.8
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkerConfig {
    /// Enable workers on this node.
    ///
    /// When enabled, the node starts a worker pool to process jobs
    /// from the distributed queue.
    ///
    /// Default: false
    #[serde(default, rename = "enabled")]
    pub is_enabled: bool,

    /// Number of workers to start.
    ///
    /// Each worker can process one job at a time. More workers allow
    /// parallel job execution but consume more resources.
    ///
    /// Tiger Style: Max 64 workers per node.
    ///
    /// Default: Number of CPU cores (capped at 8)
    #[serde(default = "default_worker_count")]
    pub worker_count: usize,

    /// Maximum concurrent jobs per worker.
    ///
    /// Limits how many jobs a single worker can execute in parallel.
    /// Usually set to 1 for CPU-bound work, higher for I/O-bound.
    ///
    /// Tiger Style: Max 100 concurrent jobs per worker.
    ///
    /// Default: 1
    #[serde(default = "default_max_concurrent_jobs")]
    pub max_concurrent_jobs: usize,

    /// Job types this node can handle.
    ///
    /// Empty means the node can handle any job type.
    /// When specified, only jobs matching these types are accepted.
    ///
    /// Tiger Style: Max 32 job types per node.
    ///
    /// Default: [] (handle all types)
    #[serde(default)]
    pub job_types: Vec<String>,

    /// Node capability tags.
    ///
    /// Tags describe node capabilities (e.g., "gpu", "ssd", "high_memory").
    /// Jobs can request specific tags for placement affinity.
    ///
    /// Tiger Style: Max 16 tags per node.
    ///
    /// Default: []
    #[serde(default)]
    pub tags: Vec<String>,

    /// Prefer executing jobs with local data.
    ///
    /// When true, jobs are preferentially routed to nodes that have
    /// the required data (iroh-blobs) locally available.
    ///
    /// Default: true
    #[serde(default = "default_prefer_local")]
    pub prefer_local: bool,

    /// Weight for data locality in job placement (0.0 to 1.0).
    ///
    /// Higher values prioritize data locality over other factors
    /// like load balancing or network proximity.
    ///
    /// Default: 0.7
    #[serde(default = "default_data_locality_weight")]
    pub data_locality_weight: f32,

    /// Poll interval for checking job queue (milliseconds).
    ///
    /// How often workers check for new jobs when idle.
    /// Lower values reduce latency but increase load.
    ///
    /// Tiger Style: Min 100ms, Max 60000ms.
    ///
    /// Default: 1000 (1 second)
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,

    /// Visibility timeout for dequeued jobs (seconds).
    ///
    /// How long a job remains invisible to other workers after being
    /// dequeued. Should be longer than typical job execution time.
    ///
    /// Tiger Style: Max 3600 seconds (1 hour).
    ///
    /// Default: 300 (5 minutes)
    #[serde(default = "default_visibility_timeout_secs")]
    pub visibility_timeout_secs: u64,

    /// Heartbeat interval for worker health checks (milliseconds).
    ///
    /// How often workers report their status to the cluster.
    ///
    /// Default: 5000 (5 seconds)
    #[serde(default = "default_worker_heartbeat_ms")]
    pub heartbeat_interval_ms: u64,

    /// Shutdown timeout for graceful worker termination (milliseconds).
    ///
    /// Maximum time to wait for workers to finish current jobs
    /// during shutdown.
    ///
    /// Default: 30000 (30 seconds)
    #[serde(default = "default_shutdown_timeout_ms")]
    pub shutdown_timeout_ms: u64,

    /// Enable distributed worker coordination.
    ///
    /// When enabled, workers coordinate across nodes for load balancing,
    /// work stealing, and failover.
    ///
    /// Default: false
    #[serde(default)]
    pub enable_distributed: bool,

    /// Enable work stealing from overloaded nodes.
    ///
    /// When enabled, idle workers can steal jobs from overloaded nodes
    /// to improve cluster-wide load balancing.
    ///
    /// Default: None (uses distributed coordinator default)
    #[serde(default)]
    pub enable_work_stealing: Option<bool>,

    /// Load balancing strategy for distributed coordination.
    ///
    /// Options: "round_robin", "least_loaded", "affinity", "consistent_hash"
    ///
    /// Default: None (uses distributed coordinator default)
    #[serde(default)]
    pub load_balancing_strategy: Option<String>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            worker_count: default_worker_count(),
            max_concurrent_jobs: default_max_concurrent_jobs(),
            job_types: vec![],
            tags: vec![],
            prefer_local: default_prefer_local(),
            data_locality_weight: default_data_locality_weight(),
            poll_interval_ms: default_poll_interval_ms(),
            visibility_timeout_secs: default_visibility_timeout_secs(),
            heartbeat_interval_ms: default_worker_heartbeat_ms(),
            shutdown_timeout_ms: default_shutdown_timeout_ms(),
            enable_distributed: false,
            enable_work_stealing: None,
            load_balancing_strategy: None,
        }
    }
}

pub(crate) fn default_worker_count() -> usize {
    std::cmp::min(num_cpus::get(), 8)
}

pub(crate) fn default_max_concurrent_jobs() -> usize {
    1
}

pub(crate) fn default_prefer_local() -> bool {
    true
}

pub(crate) fn default_data_locality_weight() -> f32 {
    0.7
}

pub(crate) fn default_poll_interval_ms() -> u64 {
    1000
}

pub(crate) fn default_visibility_timeout_secs() -> u64 {
    300
}

pub(crate) fn default_worker_heartbeat_ms() -> u64 {
    5000
}

pub(crate) fn default_shutdown_timeout_ms() -> u64 {
    30000
}
