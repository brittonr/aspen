//! Configuration types for CI pipelines.
//!
//! These types are deserialized from Nickel configuration files
//! after validation against the CI schema contracts.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::error::CiCoreError;

/// Job execution type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    /// Build using Nix flake.
    Nix,
    /// Execute shell command.
    #[default]
    Shell,
    /// Run in isolated VM.
    Vm,
    /// Deploy a build artifact to the cluster.
    Deploy,
}

/// Job queue route for shell CI jobs.
pub const CI_JOB_TYPE_SHELL: &str = "shell_command";
/// Job queue route for Nix build CI jobs.
pub const CI_JOB_TYPE_NIX: &str = "ci_nix_build";
/// Job queue route for VM CI jobs.
pub const CI_JOB_TYPE_VM: &str = "ci_vm";
/// Job queue route for deploy jobs intercepted by the CI runtime.
pub const CI_JOB_TYPE_DEPLOY: &str = "ci_deploy";

/// Return the queue route used when a CI job type is converted into an Aspen job.
#[must_use]
pub const fn job_type_route(job_type: JobType) -> &'static str {
    match job_type {
        JobType::Shell => CI_JOB_TYPE_SHELL,
        JobType::Nix => CI_JOB_TYPE_NIX,
        JobType::Vm => CI_JOB_TYPE_VM,
        JobType::Deploy => CI_JOB_TYPE_DEPLOY,
    }
}

/// Convert a CI priority into the canonical Aspen jobs priority.
#[must_use]
pub const fn to_jobs_priority(priority: Priority) -> aspen_jobs_core::Priority {
    match priority {
        Priority::High => aspen_jobs_core::Priority::High,
        Priority::Normal => aspen_jobs_core::Priority::Normal,
        Priority::Low => aspen_jobs_core::Priority::Low,
    }
}

/// Convert a CI retry count into the canonical Aspen jobs retry policy.
#[must_use]
pub fn retry_count_to_jobs_policy(retry_count: u32) -> aspen_jobs_core::RetryPolicy {
    if retry_count > 0 {
        aspen_jobs_core::RetryPolicy::exponential(retry_count)
    } else {
        aspen_jobs_core::RetryPolicy::none()
    }
}

/// Job isolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IsolationMode {
    /// Use Nix sandbox for isolation.
    #[default]
    NixSandbox,
    /// Run in full VM isolation.
    Vm,
    /// No isolation (use with caution).
    None,
}

/// Artifact storage backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStorage {
    /// Store in iroh-blobs (P2P distributed).
    #[default]
    Blobs,
    /// Store on local filesystem.
    Local,
    /// Don't store artifacts.
    None,
}

/// Pipeline priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// High priority (processed first).
    High,
    /// Normal priority.
    #[default]
    Normal,
    /// Low priority (processed last).
    Low,
}

/// Configuration for a single job within a stage.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobConfig {
    /// Job name (unique within stage).
    pub name: String,

    /// Job execution type.
    #[serde(rename = "type", default)]
    pub job_type: JobType,

    /// Shell command to execute (for shell jobs).
    #[serde(default = "default_optional_string")]
    pub command: Option<String>,

    /// Command arguments.
    #[serde(default = "default_string_vec")]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default = "default_string_map")]
    pub env: HashMap<String, String>,

    /// Working directory.
    #[serde(default = "default_optional_string")]
    pub working_dir: Option<String>,

    /// Nix flake URL (for nix jobs).
    #[serde(default = "default_optional_string")]
    pub flake_url: Option<String>,

    /// Nix flake attribute (for nix jobs).
    #[serde(default = "default_optional_string")]
    pub flake_attr: Option<String>,

    /// Binary blob hash (for VM jobs).
    #[serde(default = "default_optional_string")]
    pub binary_hash: Option<String>,

    /// Job timeout in seconds.
    #[serde(default = "default_job_timeout")]
    pub timeout_secs: u64,

    /// Isolation mode.
    #[serde(default)]
    pub isolation: IsolationMode,

    /// Cache key for build caching.
    #[serde(default = "default_optional_string")]
    pub cache_key: Option<String>,

    /// Artifact glob patterns to collect.
    #[serde(default = "default_string_vec")]
    pub artifacts: Vec<String>,

    /// Job dependencies (names of other jobs).
    #[serde(default = "default_string_vec")]
    pub depends_on: Vec<String>,

    /// Number of retry attempts.
    #[serde(default)]
    pub retry_count: u32,

    /// Allow pipeline to continue if this job fails.
    #[serde(default)]
    pub allow_failure: bool,

    /// Tags for worker affinity.
    #[serde(default = "default_string_vec")]
    pub tags: Vec<String>,

    /// Whether to upload build results to blob store (for nix jobs).
    /// Defaults to true.
    #[serde(default = "default_true")]
    pub should_upload_result: bool,

    /// Whether to publish build outputs to the Nix binary cache (for nix jobs).
    /// When true, built store paths are uploaded to the SNIX cache so they're
    /// available as substituters for future builds.
    /// Defaults to true.
    #[serde(default = "default_true")]
    pub publish_to_cache: bool,

    /// Name of a build job in a preceding stage whose artifact this deploy
    /// job should deploy. Required for `Deploy` jobs.
    #[serde(default = "default_optional_string")]
    pub artifact_from: Option<String>,

    /// Deployment strategy (e.g., "rolling"). Defaults to "rolling".
    #[serde(default = "default_optional_string")]
    pub strategy: Option<String>,

    /// Health check timeout in seconds for deployment. Uses the cluster
    /// default (`DEPLOY_HEALTH_TIMEOUT_SECS`) when not set.
    #[serde(default = "default_optional_u64")]
    pub health_check_timeout_secs: Option<u64>,

    /// Maximum number of nodes to upgrade concurrently during rolling deploy.
    #[serde(default = "default_optional_u32")]
    pub max_concurrent: Option<u32>,

    /// Binary to validate inside a Nix store path during deployment.
    /// Defaults to `bin/aspen-node`. Set to `bin/cowsay` for non-aspen deploys.
    #[serde(default = "default_optional_string")]
    pub expected_binary: Option<String>,

    /// Whether to track deployment lifecycle state in Raft KV.
    /// When `true`, the deploy executor writes metadata, per-node status,
    /// and rollback points under `_deploy:state:{deploy_id}:`.
    /// When `false`, only CI job logs are persisted (stateless push deploy).
    /// Defaults to `true`.
    #[serde(default = "default_optional_bool")]
    pub stateful: Option<bool>,

    /// When `true`, only validate the artifact exists and the expected binary
    /// is present. Skip profile switch and process restart. Use this for CI
    /// pipeline tests that verify the deploy stage resolves artifacts without
    /// modifying the running cluster.
    /// Defaults to `false`.
    #[serde(default = "default_optional_bool")]
    pub validate_only: Option<bool>,

    /// Force cold-boot for VM jobs, bypassing snapshot restore.
    ///
    /// When `true`, the VM pool will cold-boot a fresh VM even if a golden
    /// snapshot is available. Useful for debugging snapshot issues or when
    /// the job requires a completely pristine VM.
    /// Only applies to `JobType::Vm` jobs.
    /// Defaults to `false`.
    #[serde(default)]
    pub force_cold_boot: bool,

    /// Enable content-addressed execution caching for this job.
    ///
    /// When `true`, process executions within this job are cached by
    /// BLAKE3-hashing their inputs (command, args, env, and files read
    /// via the FUSE mount). On cache hit, outputs are replayed from
    /// iroh-blobs without re-executing the process.
    /// Only effective when the workspace is a FUSE mount with the
    /// `exec-cache` feature enabled.
    /// Defaults to `false`.
    #[serde(default)]
    pub cached_execution: bool,

    /// Number of parallel speculative VMs to fork from the golden snapshot.
    ///
    /// When > 1, the pool forks N VMs from the same snapshot, each running
    /// the same job with independent workspace state (KV branch). The first
    /// VM to succeed commits its results; others are killed.
    /// Only applies to `JobType::Vm` jobs with snapshots enabled.
    /// Defaults to `None` (single VM, no speculation).
    #[serde(default = "default_optional_u32")]
    pub speculative_count: Option<u32>,
}

fn default_job_timeout() -> u64 {
    3600 // 1 hour
}

fn default_string_vec() -> Vec<String> {
    Vec::new()
}

fn default_string_map() -> HashMap<String, String> {
    HashMap::new()
}

fn default_optional_string() -> Option<String> {
    None
}

fn default_optional_u64() -> Option<u64> {
    None
}

fn default_optional_u32() -> Option<u32> {
    None
}

fn default_optional_bool() -> Option<bool> {
    None
}

impl JobConfig {
    /// Validate the job configuration.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.name.is_empty() {
            return Err(CiCoreError::InvalidConfig {
                reason: "Job name cannot be empty".to_string(),
            });
        }

        match self.job_type {
            JobType::Shell => {
                if self.command.is_none() {
                    return Err(CiCoreError::InvalidConfig {
                        reason: format!("Shell job '{}' requires a command", self.name),
                    });
                }
            }
            JobType::Nix => {
                if self.flake_url.is_none() && self.flake_attr.is_none() {
                    return Err(CiCoreError::InvalidConfig {
                        reason: format!("Nix job '{}' requires flake_url or flake_attr", self.name),
                    });
                }
            }
            JobType::Vm => {
                if self.binary_hash.is_none() && self.flake_attr.is_none() {
                    return Err(CiCoreError::InvalidConfig {
                        reason: format!("VM job '{}' requires binary_hash or flake_attr", self.name),
                    });
                }
            }
            JobType::Deploy => {
                if self.artifact_from.is_none() {
                    return Err(CiCoreError::InvalidConfig {
                        reason: format!("Deploy job '{}' requires artifact_from", self.name),
                    });
                }
            }
        }

        Ok(())
    }
}

/// Configuration for a pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StageConfig {
    /// Stage name (unique within pipeline).
    pub name: String,

    /// Jobs to execute in this stage.
    pub jobs: Vec<JobConfig>,

    /// Run jobs in parallel.
    #[serde(default = "default_true")]
    pub parallel: bool,

    /// Stage dependencies (names of other stages).
    #[serde(default = "default_string_vec")]
    pub depends_on: Vec<String>,

    /// Only run when ref matches this pattern.
    #[serde(default = "default_optional_string")]
    pub when: Option<String>,
}

fn default_true() -> bool {
    true
}

impl StageConfig {
    /// Validate the stage configuration.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.name.is_empty() {
            return Err(CiCoreError::InvalidConfig {
                reason: "Stage name cannot be empty".to_string(),
            });
        }

        if self.jobs.is_empty() {
            return Err(CiCoreError::InvalidConfig {
                reason: format!("Stage '{}' has no jobs", self.name),
            });
        }

        for job in &self.jobs {
            job.validate()?;
        }

        Ok(())
    }

    /// Check if this stage should run for the given ref.
    pub fn should_run(&self, ref_name: &str) -> bool {
        match &self.when {
            Some(pattern) => {
                // Simple glob matching
                if pattern.contains('*') {
                    let prefix = pattern.trim_end_matches('*');
                    ref_name.starts_with(prefix)
                } else {
                    ref_name == pattern
                }
            }
            None => true,
        }
    }
}

/// Trigger configuration for automatic pipeline execution.
///
/// NOTE: `Default` is implemented manually (not derived) so that
/// `refs` defaults to `["refs/heads/main"]` even when the entire
/// `TriggerConfig` is constructed via `Default::default()` — e.g.
/// when `PipelineConfig.triggers` is absent and serde falls back
/// to `#[serde(default)]`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TriggerConfig {
    /// Ref patterns to trigger on.
    #[serde(default = "default_refs")]
    pub refs: Vec<String>,

    /// Path patterns to ignore.
    #[serde(default = "default_string_vec")]
    pub ignore_paths: Vec<String>,

    /// Path patterns to watch (if set, only these trigger).
    #[serde(default = "default_string_vec")]
    pub only_paths: Vec<String>,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            refs: default_refs(),
            ignore_paths: Vec::new(),
            only_paths: Vec::new(),
        }
    }
}

fn default_refs() -> Vec<String> {
    vec!["refs/heads/main".to_string()]
}

impl TriggerConfig {
    /// Check if the given ref should trigger a pipeline.
    ///
    /// This handles both full ref names (`refs/heads/main`) and short ref names
    /// (`heads/main`) since the forge strips the `refs/` prefix when storing refs.
    pub fn should_trigger(&self, ref_name: &str) -> bool {
        // Normalize ref_name to full format if missing refs/ prefix
        let normalized_ref = if ref_name.starts_with("refs/") {
            ref_name.to_string()
        } else {
            format!("refs/{}", ref_name)
        };

        for pattern in &self.refs {
            if pattern.contains('*') {
                let prefix = pattern.trim_end_matches('*');
                if normalized_ref.starts_with(prefix) {
                    return true;
                }
            } else if normalized_ref == *pattern {
                return true;
            }
        }
        false
    }
}

/// Artifact storage configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactConfig {
    /// Storage backend.
    #[serde(default)]
    pub storage: ArtifactStorage,

    /// Retention period in days.
    #[serde(default = "default_retention")]
    pub retention_days: u32,

    /// Compress artifacts.
    #[serde(default = "default_true")]
    pub should_compress: bool,
}

fn default_retention() -> u32 {
    30
}

/// Complete pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PipelineConfig {
    /// Pipeline name.
    pub name: String,

    /// Pipeline description.
    #[serde(default = "default_optional_string")]
    pub description: Option<String>,

    /// Trigger configuration.
    #[serde(default = "default_trigger_config")]
    pub triggers: TriggerConfig,

    /// Pipeline stages.
    pub stages: Vec<StageConfig>,

    /// Artifact configuration.
    #[serde(default = "default_artifact_config")]
    pub artifacts: ArtifactConfig,

    /// Global environment variables.
    #[serde(default = "default_string_map")]
    pub env: HashMap<String, String>,

    /// Pipeline timeout in seconds.
    #[serde(default = "default_pipeline_timeout")]
    pub timeout_secs: u64,

    /// Pipeline priority.
    #[serde(default)]
    pub priority: Priority,
}

fn default_pipeline_timeout() -> u64 {
    7200 // 2 hours
}

fn default_trigger_config() -> TriggerConfig {
    TriggerConfig::default()
}

fn default_artifact_config() -> ArtifactConfig {
    ArtifactConfig::default()
}

impl PipelineConfig {
    /// Validate the complete pipeline configuration.
    pub fn validate(&self) -> crate::error::Result<()> {
        use std::collections::HashSet;

        if self.name.is_empty() {
            return Err(CiCoreError::InvalidConfig {
                reason: "Pipeline name cannot be empty".to_string(),
            });
        }

        if self.stages.is_empty() {
            return Err(CiCoreError::InvalidConfig {
                reason: "Pipeline must have at least one stage".to_string(),
            });
        }

        // Validate stages and check for duplicates
        let mut stage_names = HashSet::new();
        for stage in &self.stages {
            if !stage_names.insert(&stage.name) {
                return Err(CiCoreError::InvalidConfig {
                    reason: format!("Duplicate stage name: {}", stage.name),
                });
            }
            stage.validate()?;
        }

        // Validate stage dependencies exist
        for stage in &self.stages {
            for dep in &stage.depends_on {
                if !stage_names.contains(dep) {
                    return Err(CiCoreError::InvalidConfig {
                        reason: format!("Stage '{}' depends on unknown stage '{}'", stage.name, dep),
                    });
                }
            }
        }

        // Check for circular dependencies
        self.check_circular_dependencies()?;

        // Validate deploy job artifact_from references
        self.validate_deploy_artifact_refs()?;

        Ok(())
    }

    /// Check for circular dependencies between stages.
    fn check_circular_dependencies(&self) -> crate::error::Result<()> {
        use std::collections::HashMap;
        use std::collections::HashSet;

        struct VisitFrame<'a> {
            node: &'a str,
            next_dep_index: usize,
        }

        // Build dependency graph
        let mut deps: HashMap<&str, Vec<&str>> = HashMap::with_capacity(self.stages.len());
        for stage in &self.stages {
            deps.insert(&stage.name, stage.depends_on.iter().map(|name| name.as_str()).collect());
        }

        let mut visited = HashSet::with_capacity(self.stages.len());
        let mut path = Vec::with_capacity(self.stages.len());
        let mut stack = Vec::with_capacity(self.stages.len());

        for stage in &self.stages {
            let start = stage.name.as_str();
            if visited.contains(start) {
                continue;
            }

            path.push(start);
            stack.push(VisitFrame {
                node: start,
                next_dep_index: 0,
            });

            while let Some(frame) = stack.last_mut() {
                let dependencies = deps.get(frame.node).map(Vec::as_slice).unwrap_or(&[]);
                if frame.next_dep_index >= dependencies.len() {
                    let finished = frame.node;
                    stack.pop();
                    path.pop();
                    visited.insert(finished);
                    continue;
                }

                let dependency = dependencies[frame.next_dep_index];
                frame.next_dep_index = frame.next_dep_index.saturating_add(1);

                if let Some(cycle_start) = path.iter().position(|&node| node == dependency) {
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dependency);
                    return Err(CiCoreError::CircularDependency {
                        path: cycle.join(" -> "),
                    });
                }

                if visited.contains(dependency) {
                    continue;
                }

                path.push(dependency);
                stack.push(VisitFrame {
                    node: dependency,
                    next_dep_index: 0,
                });
            }
        }

        Ok(())
    }

    /// Validate that deploy jobs' `artifact_from` references point to
    /// job names in preceding stages (not the same stage or a later one).
    fn validate_deploy_artifact_refs(&self) -> crate::error::Result<()> {
        let ordered_stages = self.stages_in_order();

        // Collect job names from all stages we've seen so far (preceding stages).
        let mut preceding_job_names = std::collections::HashSet::new();

        for stage in &ordered_stages {
            // Check deploy jobs in this stage against preceding jobs
            for job in &stage.jobs {
                if job.job_type == JobType::Deploy
                    && let Some(ref artifact_from) = job.artifact_from
                    && !preceding_job_names.contains(artifact_from.as_str())
                {
                    // Check if it's in the same stage (different error message)
                    let is_in_same_stage = stage.jobs.iter().any(|j| j.name == *artifact_from);
                    if is_in_same_stage {
                        return Err(CiCoreError::InvalidConfig {
                            reason: format!(
                                "Deploy job '{}' references '{}' in the same stage; \
                                 artifact_from must reference a job in a preceding stage",
                                job.name, artifact_from
                            ),
                        });
                    }
                    return Err(CiCoreError::InvalidConfig {
                        reason: format!(
                            "Deploy job '{}' references unknown job '{}' in artifact_from",
                            job.name, artifact_from
                        ),
                    });
                }
            }

            // Add this stage's job names to the preceding set
            for job in &stage.jobs {
                preceding_job_names.insert(job.name.as_str());
            }
        }

        Ok(())
    }

    /// Get stages in topological order (respecting dependencies).
    pub fn stages_in_order(&self) -> Vec<&StageConfig> {
        use std::collections::HashMap;
        use std::collections::HashSet;

        let mut result = Vec::new();
        let mut completed: HashSet<&str> = HashSet::new();
        let stage_map: HashMap<&str, &StageConfig> = self.stages.iter().map(|s| (s.name.as_str(), s)).collect();

        // Simple topological sort
        while result.len() < self.stages.len() {
            for stage in &self.stages {
                if completed.contains(stage.name.as_str()) {
                    continue;
                }

                let is_deps_met = stage.depends_on.iter().all(|d| completed.contains(d.as_str()));

                if is_deps_met {
                    result.push(stage_map[stage.name.as_str()]);
                    completed.insert(&stage.name);
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_default() {
        let job_type: JobType = Default::default();
        assert_eq!(job_type, JobType::Shell);
    }

    #[test]
    fn test_job_type_route() {
        assert_eq!(job_type_route(JobType::Shell), CI_JOB_TYPE_SHELL);
        assert_eq!(job_type_route(JobType::Shell), "shell_command");
        assert_eq!(job_type_route(JobType::Nix), CI_JOB_TYPE_NIX);
        assert_eq!(job_type_route(JobType::Nix), "ci_nix_build");
        assert_eq!(job_type_route(JobType::Vm), CI_JOB_TYPE_VM);
        assert_eq!(job_type_route(JobType::Vm), "ci_vm");
        assert_eq!(job_type_route(JobType::Deploy), CI_JOB_TYPE_DEPLOY);
        assert_eq!(job_type_route(JobType::Deploy), "ci_deploy");
    }

    #[test]
    fn test_priority_conversion_to_jobs_core() {
        assert!(matches!(to_jobs_priority(Priority::High), aspen_jobs_core::Priority::High));
        assert!(matches!(to_jobs_priority(Priority::Normal), aspen_jobs_core::Priority::Normal));
        assert!(matches!(to_jobs_priority(Priority::Low), aspen_jobs_core::Priority::Low));
    }

    #[test]
    fn test_retry_count_to_jobs_policy() {
        assert!(matches!(retry_count_to_jobs_policy(0), aspen_jobs_core::RetryPolicy::None));
        assert_eq!(retry_count_to_jobs_policy(0).max_attempts(), 1);

        match retry_count_to_jobs_policy(3) {
            aspen_jobs_core::RetryPolicy::Exponential {
                max_attempts,
                initial_delay,
                multiplier,
                max_delay,
            } => {
                assert_eq!(max_attempts, 3);
                assert_eq!(initial_delay.as_secs(), 1);
                assert_eq!(multiplier, 2.0);
                assert_eq!(max_delay.map(|duration| duration.as_secs()), Some(300));
            }
            other => panic!("unexpected retry policy: {other:?}"),
        }
    }

    #[test]
    fn test_trigger_should_trigger() {
        let trigger = TriggerConfig {
            refs: vec!["refs/heads/main".to_string(), "refs/heads/feature/*".to_string()],
            ..Default::default()
        };

        assert!(trigger.should_trigger("refs/heads/main"));
        assert!(trigger.should_trigger("refs/heads/feature/foo"));
        assert!(trigger.should_trigger("refs/heads/feature/bar/baz"));
        assert!(!trigger.should_trigger("refs/heads/develop"));
    }

    #[test]
    fn test_deploy_job_requires_artifact_from() {
        let job = JobConfig {
            name: "deploy-node".to_string(),
            job_type: JobType::Deploy,
            command: None,
            args: vec![],
            env: HashMap::new(),
            working_dir: None,
            flake_url: None,
            flake_attr: None,
            binary_hash: None,
            timeout_secs: 3600,
            isolation: IsolationMode::default(),
            cache_key: None,
            artifacts: vec![],
            depends_on: vec![],
            retry_count: 0,
            allow_failure: false,
            tags: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            artifact_from: None,
            strategy: None,
            health_check_timeout_secs: None,
            max_concurrent: None,
            expected_binary: None,
            stateful: None,
            validate_only: None,
            force_cold_boot: false,
            speculative_count: None,
            cached_execution: false,
        };
        let err = job.validate().unwrap_err();
        assert_eq!(job.job_type, JobType::Deploy);
        assert!(job.artifact_from.is_none());
        assert!(err.to_string().contains("requires artifact_from"), "got: {err}");
    }

    #[test]
    fn test_deploy_job_valid_with_artifact_from() {
        let job = JobConfig {
            name: "deploy-node".to_string(),
            job_type: JobType::Deploy,
            command: None,
            args: vec![],
            env: HashMap::new(),
            working_dir: None,
            flake_url: None,
            flake_attr: None,
            binary_hash: None,
            timeout_secs: 3600,
            isolation: IsolationMode::default(),
            cache_key: None,
            artifacts: vec![],
            depends_on: vec![],
            retry_count: 0,
            allow_failure: false,
            tags: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            artifact_from: Some("build-node".to_string()),
            strategy: Some("rolling".to_string()),
            health_check_timeout_secs: None,
            max_concurrent: None,
            expected_binary: None,
            stateful: None,
            validate_only: None,
            force_cold_boot: false,
            speculative_count: None,
            cached_execution: false,
        };
        assert_eq!(job.artifact_from.as_deref(), Some("build-node"));
        assert!(job.validate().is_ok());
    }

    fn make_shell_job(name: &str) -> JobConfig {
        JobConfig {
            name: name.to_string(),
            job_type: JobType::Shell,
            command: Some("echo ok".to_string()),
            args: vec![],
            env: HashMap::new(),
            working_dir: None,
            flake_url: None,
            flake_attr: None,
            binary_hash: None,
            timeout_secs: 300,
            isolation: IsolationMode::default(),
            cache_key: None,
            artifacts: vec![],
            depends_on: vec![],
            retry_count: 0,
            allow_failure: false,
            tags: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            artifact_from: None,
            strategy: None,
            health_check_timeout_secs: None,
            max_concurrent: None,
            expected_binary: None,
            stateful: None,
            validate_only: None,
            force_cold_boot: false,
            speculative_count: None,
            cached_execution: false,
        }
    }

    struct DeployJobFixture<'a> {
        name: &'a str,
        artifact_from: &'a str,
    }

    fn make_deploy_job(fixture: DeployJobFixture<'_>) -> JobConfig {
        JobConfig {
            name: fixture.name.to_string(),
            job_type: JobType::Deploy,
            command: None,
            args: vec![],
            env: HashMap::new(),
            working_dir: None,
            flake_url: None,
            flake_attr: None,
            binary_hash: None,
            timeout_secs: 600,
            isolation: IsolationMode::default(),
            cache_key: None,
            artifacts: vec![],
            depends_on: vec![],
            retry_count: 0,
            allow_failure: false,
            tags: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            artifact_from: Some(fixture.artifact_from.to_string()),
            strategy: Some("rolling".to_string()),
            health_check_timeout_secs: None,
            max_concurrent: None,
            expected_binary: None,
            stateful: None,
            validate_only: None,
            force_cold_boot: false,
            speculative_count: None,
            cached_execution: false,
        }
    }

    #[test]
    fn test_pipeline_deploy_artifact_from_valid() {
        let config = PipelineConfig {
            name: "test".to_string(),
            description: None,
            triggers: TriggerConfig::default(),
            stages: vec![
                StageConfig {
                    name: "build".to_string(),
                    jobs: vec![make_shell_job("build-node")],
                    parallel: true,
                    depends_on: vec![],
                    when: None,
                },
                StageConfig {
                    name: "deploy".to_string(),
                    jobs: vec![make_deploy_job(DeployJobFixture {
                        name: "deploy-node",
                        artifact_from: "build-node",
                    })],
                    parallel: true,
                    depends_on: vec!["build".to_string()],
                    when: None,
                },
            ],
            artifacts: ArtifactConfig::default(),
            env: HashMap::new(),
            timeout_secs: 7200,
            priority: Priority::default(),
        };
        assert_eq!(config.stages.len(), 2);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_pipeline_deploy_artifact_from_unknown_job() {
        let config = PipelineConfig {
            name: "test".to_string(),
            description: None,
            triggers: TriggerConfig::default(),
            stages: vec![
                StageConfig {
                    name: "build".to_string(),
                    jobs: vec![make_shell_job("build-node")],
                    parallel: true,
                    depends_on: vec![],
                    when: None,
                },
                StageConfig {
                    name: "deploy".to_string(),
                    jobs: vec![make_deploy_job(DeployJobFixture {
                        name: "deploy-node",
                        artifact_from: "nonexistent",
                    })],
                    parallel: true,
                    depends_on: vec!["build".to_string()],
                    when: None,
                },
            ],
            artifacts: ArtifactConfig::default(),
            env: HashMap::new(),
            timeout_secs: 7200,
            priority: Priority::default(),
        };
        let err = config.validate().unwrap_err();
        assert_eq!(config.stages.len(), 2);
        assert!(err.to_string().contains("unknown job"), "got: {err}");
    }

    #[test]
    fn test_pipeline_deploy_artifact_from_same_stage() {
        let config = PipelineConfig {
            name: "test".to_string(),
            description: None,
            triggers: TriggerConfig::default(),
            stages: vec![StageConfig {
                name: "all-in-one".to_string(),
                jobs: vec![
                    make_shell_job("build-node"),
                    make_deploy_job(DeployJobFixture {
                        name: "deploy-node",
                        artifact_from: "build-node",
                    }),
                ],
                parallel: true,
                depends_on: vec![],
                when: None,
            }],
            artifacts: ArtifactConfig::default(),
            env: HashMap::new(),
            timeout_secs: 7200,
            priority: Priority::default(),
        };
        let err = config.validate().unwrap_err();
        let err_text = err.to_string();
        assert_eq!(config.stages[0].jobs.len(), 2);
        assert!(!err_text.is_empty());
        assert!(err_text.contains("same stage"), "got: {err_text}");
    }

    #[test]
    fn test_pipeline_validate_rejects_stage_cycle() {
        let config = PipelineConfig {
            name: "test".to_string(),
            description: None,
            triggers: TriggerConfig::default(),
            stages: vec![
                StageConfig {
                    name: "build".to_string(),
                    jobs: vec![make_shell_job("build-node")],
                    parallel: true,
                    depends_on: vec!["deploy".to_string()],
                    when: None,
                },
                StageConfig {
                    name: "deploy".to_string(),
                    jobs: vec![make_shell_job("deploy-node")],
                    parallel: true,
                    depends_on: vec!["build".to_string()],
                    when: None,
                },
            ],
            artifacts: ArtifactConfig::default(),
            env: HashMap::new(),
            timeout_secs: 7200,
            priority: Priority::default(),
        };

        let err = config.validate().unwrap_err();
        let err_text = err.to_string();
        assert!(!err_text.is_empty());
        assert!(err_text.contains("build -> deploy -> build"), "got: {err_text}");
    }

    #[test]
    fn test_stage_should_run() {
        let stage = StageConfig {
            name: "deploy".to_string(),
            jobs: vec![],
            parallel: true,
            depends_on: vec![],
            when: Some("refs/heads/main".to_string()),
        };

        assert!(stage.should_run("refs/heads/main"));
        assert!(!stage.should_run("refs/heads/feature/foo"));

        let stage_wildcard = StageConfig {
            name: "build".to_string(),
            jobs: vec![],
            parallel: true,
            depends_on: vec![],
            when: Some("refs/heads/*".to_string()),
        };

        assert!(stage_wildcard.should_run("refs/heads/main"));
        assert!(stage_wildcard.should_run("refs/heads/feature"));
    }
}
