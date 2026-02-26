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
    #[serde(default)]
    pub command: Option<String>,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Working directory.
    #[serde(default)]
    pub working_dir: Option<String>,

    /// Nix flake URL (for nix jobs).
    #[serde(default)]
    pub flake_url: Option<String>,

    /// Nix flake attribute (for nix jobs).
    #[serde(default)]
    pub flake_attr: Option<String>,

    /// Binary blob hash (for VM jobs).
    #[serde(default)]
    pub binary_hash: Option<String>,

    /// Job timeout in seconds.
    #[serde(default = "default_job_timeout")]
    pub timeout_secs: u64,

    /// Isolation mode.
    #[serde(default)]
    pub isolation: IsolationMode,

    /// Cache key for build caching.
    #[serde(default)]
    pub cache_key: Option<String>,

    /// Artifact glob patterns to collect.
    #[serde(default)]
    pub artifacts: Vec<String>,

    /// Job dependencies (names of other jobs).
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Number of retry attempts.
    #[serde(default)]
    pub retry_count: u32,

    /// Allow pipeline to continue if this job fails.
    #[serde(default)]
    pub allow_failure: bool,

    /// Tags for worker affinity.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Whether to upload build results to blob store (for nix jobs).
    /// Defaults to true.
    #[serde(default = "default_true")]
    pub should_upload_result: bool,
}

fn default_job_timeout() -> u64 {
    3600 // 1 hour
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
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Only run when ref matches this pattern.
    #[serde(default)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct TriggerConfig {
    /// Ref patterns to trigger on.
    #[serde(default = "default_refs")]
    pub refs: Vec<String>,

    /// Path patterns to ignore.
    #[serde(default)]
    pub ignore_paths: Vec<String>,

    /// Path patterns to watch (if set, only these trigger).
    #[serde(default)]
    pub only_paths: Vec<String>,
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
    #[serde(default)]
    pub description: Option<String>,

    /// Trigger configuration.
    #[serde(default)]
    pub triggers: TriggerConfig,

    /// Pipeline stages.
    pub stages: Vec<StageConfig>,

    /// Artifact configuration.
    #[serde(default)]
    pub artifacts: ArtifactConfig,

    /// Global environment variables.
    #[serde(default)]
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

        Ok(())
    }

    /// Check for circular dependencies between stages.
    fn check_circular_dependencies(&self) -> crate::error::Result<()> {
        use std::collections::HashMap;
        use std::collections::HashSet;

        // Build dependency graph
        let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
        for stage in &self.stages {
            deps.insert(&stage.name, stage.depends_on.iter().map(|s| s.as_str()).collect());
        }

        // DFS to detect cycles
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        fn visit<'a>(
            node: &'a str,
            deps: &HashMap<&'a str, Vec<&'a str>>,
            visited: &mut HashSet<&'a str>,
            path: &mut Vec<&'a str>,
        ) -> Result<(), String> {
            if path.contains(&node) {
                path.push(node);
                let cycle_start = path.iter().position(|&n| n == node).unwrap_or(0);
                return Err(path[cycle_start..].join(" -> "));
            }

            if visited.contains(node) {
                return Ok(());
            }

            path.push(node);

            if let Some(dependencies) = deps.get(node) {
                for dep in dependencies {
                    visit(dep, deps, visited, path)?;
                }
            }

            path.pop();
            visited.insert(node);

            Ok(())
        }

        for stage in &self.stages {
            if !visited.contains(stage.name.as_str())
                && let Err(cycle) = visit(&stage.name, &deps, &mut visited, &mut path)
            {
                return Err(CiCoreError::CircularDependency { path: cycle });
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

                let deps_met = stage.depends_on.iter().all(|d| completed.contains(d.as_str()));

                if deps_met {
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
