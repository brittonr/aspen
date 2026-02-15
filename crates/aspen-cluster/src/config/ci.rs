//! CI/CD pipeline configuration.
//!
//! Configures the CI/CD system that executes pipelines defined in
//! `.aspen/ci.ncl` files.

use aspen_core::MAX_CI_JOB_MEMORY_BYTES;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// CI/CD pipeline configuration.
///
/// Configures the CI/CD system that executes pipelines defined in
/// `.aspen/ci.ncl` files when triggered by ref updates or manual invocation.
///
/// The CI system uses the aspen-jobs infrastructure for distributed
/// pipeline execution with Nickel-based type-safe configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CiConfig {
    /// Enable CI/CD orchestration on this node.
    ///
    /// When enabled, the node can:
    /// - Accept pipeline trigger requests
    /// - Execute pipeline jobs via the job system
    /// - Track pipeline run status
    ///
    /// Default: false
    #[serde(default, rename = "enabled")]
    pub is_enabled: bool,

    /// Enable automatic CI triggering on ref updates.
    ///
    /// When enabled, the trigger service watches for forge gossip
    /// events and automatically triggers CI for repositories that
    /// have `.aspen/ci.ncl` configurations.
    ///
    /// Default: false (manual triggering only)
    #[serde(default)]
    pub auto_trigger: bool,

    /// Maximum concurrent pipeline runs per repository.
    ///
    /// Limits how many pipelines can run simultaneously for a single
    /// repository. Older runs are queued or rejected based on strategy.
    ///
    /// Tiger Style: Max 10 concurrent runs per repo.
    ///
    /// Default: 3
    #[serde(default = "default_ci_max_concurrent_runs")]
    pub max_concurrent_runs: usize,

    /// Default pipeline timeout in seconds.
    ///
    /// Maximum duration for a complete pipeline run. Individual job
    /// timeouts can override this for specific stages.
    ///
    /// Tiger Style: Max 86400 seconds (24 hours).
    ///
    /// Default: 3600 (1 hour)
    #[serde(default = "default_ci_pipeline_timeout_secs")]
    pub pipeline_timeout_secs: u64,

    /// Repository IDs to automatically watch for CI triggers.
    ///
    /// When auto_trigger is enabled, the TriggerService will watch these
    /// repositories for ref updates and automatically start CI pipelines.
    ///
    /// Each entry is a hex-encoded 32-byte repository ID.
    ///
    /// Tiger Style: Max 100 watched repositories.
    ///
    /// Environment variable: `ASPEN_CI_WATCHED_REPOS` (comma-separated hex IDs)
    ///
    /// Default: empty (no repos watched initially)
    #[serde(default)]
    pub watched_repos: Vec<String>,

    /// Enable distributed execution for CI jobs.
    ///
    /// When enabled, CI jobs are distributed across all cluster nodes
    /// instead of running only on the node that triggered the pipeline.
    /// This prevents resource exhaustion on the leader node during
    /// intensive test runs.
    ///
    /// Requires `worker.enable_distributed = true` to be effective.
    ///
    /// Default: true (recommended for production clusters)
    #[serde(default = "default_ci_distributed_execution")]
    pub distributed_execution: bool,

    /// Avoid scheduling CI jobs on the Raft leader node.
    ///
    /// When enabled, CI jobs will prefer follower nodes to prevent
    /// resource contention with Raft consensus operations. This helps
    /// maintain cluster stability during intensive CI workloads.
    ///
    /// Only effective when `distributed_execution` is also true.
    ///
    /// Default: true (recommended for stability)
    #[serde(default = "default_ci_avoid_leader")]
    pub avoid_leader: bool,

    /// Enable cgroup-based resource isolation for CI jobs.
    ///
    /// When enabled, CI job processes are placed in cgroups with memory
    /// and CPU limits to prevent resource exhaustion. Requires cgroups v2
    /// to be available on the system.
    ///
    /// Default: true (falls back to no limits if cgroups unavailable)
    #[serde(default = "default_ci_resource_isolation")]
    pub resource_isolation: bool,

    /// Maximum memory per CI job in bytes.
    ///
    /// Hard limit on memory usage for individual CI jobs. Jobs exceeding
    /// this limit will be OOM killed by the cgroup.
    ///
    /// Default: 4 GB (4294967296 bytes)
    #[serde(default = "default_ci_max_memory_bytes")]
    pub max_job_memory_bytes: u64,
}

pub(crate) fn default_ci_max_concurrent_runs() -> usize {
    3
}

pub(crate) fn default_ci_pipeline_timeout_secs() -> u64 {
    3600
}

pub(crate) fn default_ci_distributed_execution() -> bool {
    true
}

pub(crate) fn default_ci_avoid_leader() -> bool {
    true
}

pub(crate) fn default_ci_resource_isolation() -> bool {
    true
}

pub(crate) fn default_ci_max_memory_bytes() -> u64 {
    MAX_CI_JOB_MEMORY_BYTES
}
