//! Workflow building and job spec conversion.
//!
//! Converts CI pipeline configurations into aspen-jobs `WorkflowDefinition`
//! and individual `JobSpec` instances for execution.

use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "nix-executor")]
use aspen_ci_executor_nix::NixBuildPayload;
#[cfg(feature = "plugins-vm")]
use aspen_ci_executor_vm::CloudHypervisorPayload;
use aspen_core::KeyValueStore;
use aspen_jobs::JobAffinity;
use aspen_jobs::JobSpec;
use aspen_jobs::TransitionCondition;
use aspen_jobs::WorkflowDefinition;
use aspen_jobs::WorkflowStep;
use aspen_jobs::WorkflowTransition;

use super::DEFAULT_STEP_TIMEOUT_SECS;
use super::PipelineContext;
use super::PipelineOrchestrator;
use crate::config::types::JobConfig;
use crate::config::types::JobType;
use crate::config::types::PipelineConfig;
use crate::error::CiError;
use crate::error::Result;

impl<S: KeyValueStore + ?Sized + 'static> PipelineOrchestrator<S> {
    /// Check if a stage contains deploy jobs. Deploy jobs are executed
    /// in-process on the leader rather than dispatched to the worker queue.
    ///
    /// Returns the deploy job configs for the stage if any exist.
    pub fn deploy_jobs_in_stage(stage: &crate::config::types::StageConfig) -> Vec<&JobConfig> {
        stage.jobs.iter().filter(|j| j.job_type == JobType::Deploy).collect()
    }

    /// Check if a pipeline config has any deploy stages.
    pub fn has_deploy_jobs(config: &PipelineConfig) -> bool {
        config.stages.iter().any(|s| s.jobs.iter().any(|j| j.job_type == JobType::Deploy))
    }

    /// Check if a pipeline config has deploy stages that should run for a given ref.
    ///
    /// Deploy stages guarded by `when` are only counted when the ref matches.
    /// This prevents `has_pending_deploys` from keeping the pipeline in Running
    /// state when the deploy stage's `when` guard excludes the current ref
    /// (e.g., deploy stage has `when = "refs/tags/release/*"` but the push
    /// was to `refs/heads/main`).
    pub fn has_deploy_jobs_for_ref(config: &PipelineConfig, ref_name: &str) -> bool {
        config
            .stages
            .iter()
            .any(|s| s.should_run(ref_name) && s.jobs.iter().any(|j| j.job_type == JobType::Deploy))
    }

    /// Check if a stage contains only deploy jobs.
    ///
    /// Deploy-only stages are excluded from the workflow definition and
    /// run in-process via `DeployExecutor` after the preceding stage.
    pub fn is_deploy_only_stage(stage: &crate::config::types::StageConfig) -> bool {
        !stage.jobs.is_empty() && stage.jobs.iter().all(|j| j.job_type == JobType::Deploy)
    }

    /// Build a workflow definition from pipeline configuration.
    ///
    /// Deploy-only stages are excluded from the workflow — they execute
    /// in-process on the leader via `DeployExecutor` after the preceding
    /// stage completes. Transitions are adjusted to skip over them.
    pub(crate) async fn build_workflow_definition(
        &self,
        config: &PipelineConfig,
        context: &PipelineContext,
    ) -> Result<WorkflowDefinition> {
        let mut steps = HashMap::new();
        let mut terminal_states = std::collections::HashSet::new();

        // Get stages in topological order, filtering out deploy-only stages.
        // Deploy stages are handled by the deploy monitor, not the workflow.
        let ordered_stages = config.stages_in_order();
        let workflow_stages: Vec<&crate::config::types::StageConfig> =
            ordered_stages.iter().filter(|s| !Self::is_deploy_only_stage(s)).copied().collect();

        // Build workflow steps from non-deploy stages
        for (idx, stage) in workflow_stages.iter().enumerate() {
            let step_name = format!("stage_{}", stage.name);

            // Convert jobs to JobSpecs (deploy jobs in mixed stages are skipped)
            let mut job_specs = Vec::new();
            for job in stage.jobs.iter().filter(|job| job.job_type != JobType::Deploy) {
                let spec = self.job_config_to_spec(job, context, &config.env).await?;
                job_specs.push(spec);
            }

            // Determine transitions (using filtered stage list)
            let transitions = build_stage_transitions(stage, workflow_stages.get(idx + 1).copied());

            let timeout_secs = stage.jobs.iter().map(|j| j.timeout_secs).max().unwrap_or(DEFAULT_STEP_TIMEOUT_SECS);

            steps.insert(step_name.clone(), WorkflowStep {
                name: step_name,
                jobs: job_specs,
                transitions,
                parallel: stage.parallel,
                timeout: Some(Duration::from_secs(timeout_secs)),
                retry_on_failure: stage.jobs.iter().any(|j| j.retry_count > 0),
            });
        }

        // Add terminal states
        steps.insert("done".to_string(), WorkflowStep {
            name: "done".to_string(),
            jobs: vec![],
            transitions: vec![],
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        });
        terminal_states.insert("done".to_string());

        steps.insert("failed".to_string(), WorkflowStep {
            name: "failed".to_string(),
            jobs: vec![],
            transitions: vec![],
            parallel: false,
            timeout: None,
            retry_on_failure: false,
        });
        terminal_states.insert("failed".to_string());

        // Initial state is first non-deploy stage
        let initial_state =
            workflow_stages.first().map(|s| format!("stage_{}", s.name)).unwrap_or_else(|| "done".to_string());

        let timeout = Duration::from_secs(config.timeout_secs);

        Ok(WorkflowDefinition {
            name: config.name.clone(),
            initial_state,
            steps,
            terminal_states,
            timeout: Some(timeout),
        })
    }

    /// Convert a JobConfig to a JobSpec for the job system.
    pub(crate) async fn job_config_to_spec(
        &self,
        job: &JobConfig,
        context: &PipelineContext,
        pipeline_env: &HashMap<String, String>,
    ) -> Result<JobSpec> {
        // Merge environment variables: pipeline-level + job-level + CI context
        let mut env = pipeline_env.clone();
        env.extend(job.env.clone());

        // Add CI context variables
        env.insert("CI".to_string(), "true".to_string());
        env.insert("ASPEN_CI".to_string(), "true".to_string());
        env.insert("ASPEN_GIT_COMMIT".to_string(), hex::encode(context.commit_hash));
        env.insert("ASPEN_GIT_REV_SHORT".to_string(), context.short_hash());
        env.insert("ASPEN_GIT_REF".to_string(), context.ref_name.clone());
        env.insert("ASPEN_REPO_ID".to_string(), context.repo_id.to_hex());

        let payload = build_job_payload(job, context, &env, self.kv_store.as_ref()).await?;

        let job_type = match job.job_type {
            JobType::Shell => "shell_command",
            JobType::Nix => "ci_nix_build",
            JobType::Vm => "ci_vm",         // Maps to CloudHypervisorWorker
            JobType::Deploy => "ci_deploy", // Handled in-process, not via worker queue
        };

        let retry_policy = if job.retry_count > 0 {
            aspen_jobs::RetryPolicy::exponential(job.retry_count)
        } else {
            aspen_jobs::RetryPolicy::none()
        };

        let mut spec = JobSpec::new(job_type)
            .payload(payload)
            .map_err(|e| CiError::InvalidConfig {
                reason: format!("Failed to serialize job payload: {}", e),
            })?
            .priority(crate::config::types::to_jobs_priority(crate::config::types::Priority::default()))
            .timeout(Duration::from_secs(job.timeout_secs))
            .retry_policy(retry_policy);

        // Add avoid-leader affinity for CI jobs to prevent resource contention
        // with Raft consensus operations. This helps distribute load across
        // follower nodes and keeps the leader node responsive for cluster
        // coordination.
        if self.config.avoid_leader {
            spec = spec.with_affinity(JobAffinity::avoid_leader()).map_err(|e| CiError::InvalidConfig {
                reason: format!("Failed to set job affinity: {}", e),
            })?;
        }

        Ok(spec)
    }
}

/// Build transitions for a stage based on whether there is a next stage.
fn build_stage_transitions(
    stage: &crate::config::types::StageConfig,
    next_stage: Option<&crate::config::types::StageConfig>,
) -> Vec<WorkflowTransition> {
    let mut transitions = Vec::new();

    if let Some(next) = next_stage {
        // Success -> next stage
        transitions.push(WorkflowTransition {
            condition: TransitionCondition::AllSuccess,
            target: format!("stage_{}", next.name),
        });

        // Failure -> failed state (unless allow_failure is set for all jobs)
        let all_allow_failure = stage.jobs.iter().all(|j| j.allow_failure);
        if !all_allow_failure {
            transitions.push(WorkflowTransition {
                condition: TransitionCondition::AnyFailed,
                target: "failed".to_string(),
            });
        }
    } else {
        // Last stage - success goes to done
        transitions.push(WorkflowTransition {
            condition: TransitionCondition::AllSuccess,
            target: "done".to_string(),
        });

        transitions.push(WorkflowTransition {
            condition: TransitionCondition::AnyFailed,
            target: "failed".to_string(),
        });
    }

    transitions
}

/// Build the job payload JSON based on job type.
async fn build_job_payload<S: KeyValueStore + ?Sized>(
    job: &JobConfig,
    context: &PipelineContext,
    env: &HashMap<String, String>,
    #[allow(unused_variables)] store: &S,
) -> Result<serde_json::Value> {
    match job.job_type {
        JobType::Shell => build_shell_payload(job, context, env),
        #[cfg(feature = "nix-executor")]
        JobType::Nix => build_nix_payload(job, context, store).await,
        #[cfg(not(feature = "nix-executor"))]
        JobType::Nix => Err(CiError::InvalidConfig {
            reason: format!("Nix job type '{}' requires the 'nix-executor' feature to be enabled", job.name),
        }),
        #[cfg(feature = "plugins-vm")]
        JobType::Vm => build_vm_payload(job, context, env),
        #[cfg(not(feature = "plugins-vm"))]
        JobType::Vm => Err(CiError::InvalidConfig {
            reason: format!("VM job type '{}' requires the 'plugins-vm' feature to be enabled", job.name),
        }),
        // Deploy jobs are executed in-process by DeployExecutor on the leader,
        // not dispatched to the worker queue. This arm should not be reached
        // during normal operation — the orchestrator intercepts deploy jobs
        // before they reach job_config_to_spec.
        JobType::Deploy => Err(CiError::InvalidConfig {
            reason: format!(
                "Deploy job '{}' should not be dispatched to worker queue; \
                 it runs in-process on the leader via DeployExecutor",
                job.name
            ),
        }),
    }
}

/// Build payload for shell jobs.
fn build_shell_payload(
    job: &JobConfig,
    context: &PipelineContext,
    env: &HashMap<String, String>,
) -> Result<serde_json::Value> {
    let command = job.command.clone().ok_or_else(|| CiError::InvalidConfig {
        reason: format!("Shell job '{}' missing command", job.name),
    })?;

    // Determine if we need to wrap command in sh -c
    // This is needed when:
    // 1. Command contains shell metacharacters (spaces, quotes, pipes, etc.)
    // 2. And no separate args are provided (command is a full shell expression)
    let needs_shell_wrap = job.args.is_empty()
        && (command.contains(' ')
            || command.contains('\'')
            || command.contains('"')
            || command.contains('|')
            || command.contains('>')
            || command.contains('<')
            || command.contains('&')
            || command.contains(';')
            || command.contains('$'));

    let (final_command, final_args) = if needs_shell_wrap {
        // Wrap in sh -c for shell interpretation
        ("sh".to_string(), vec!["-c".to_string(), command])
    } else {
        // Use command directly with provided args
        (command, job.args.clone())
    };

    // Use job's working_dir if specified, otherwise fall back to checkout_dir
    let working_dir = job
        .working_dir
        .clone()
        .or_else(|| context.checkout_dir.as_ref().map(|p| p.to_string_lossy().to_string()));

    Ok(serde_json::json!({
        "type": "shell",
        "job_name": job.name,
        "run_id": context.run_id,
        "command": final_command,
        "args": final_args,
        "env": env,
        "working_dir": working_dir,
        "timeout_secs": job.timeout_secs,
    }))
}

/// Build payload for Nix jobs.
#[cfg(feature = "nix-executor")]
async fn build_nix_payload<S: KeyValueStore + ?Sized>(
    job: &JobConfig,
    context: &PipelineContext,
    store: &S,
) -> Result<serde_json::Value> {
    let flake_url = job.flake_url.clone().unwrap_or_else(|| ".".to_string());
    let attribute = job.flake_attr.clone().unwrap_or_default();

    // Build flake reference for failure cache check
    let flake_ref = if attribute.is_empty() {
        flake_url.clone()
    } else {
        format!("{}#{}", flake_url, attribute)
    };

    // Check failure cache (advisory only — new commits may fix the issue)
    match crate::failure_cache::check_failure(store, &flake_ref).await {
        Ok(true) => {
            // Cached failure found — log but proceed anyway since the source
            // may have changed (new commit pushed). The failure cache key is
            // based on flake ref, not commit hash, so it can't distinguish
            // "same broken code" from "new code that might work".
            tracing::warn!(
                flake_ref = %flake_ref,
                "Previous build failure cached for this flake ref, retrying anyway (new commit may fix it)"
            );
        }
        Ok(false) => {
            // No cached failure, proceed with build
        }
        Err(e) => {
            // Cache check failed, log warning but proceed with build
            tracing::warn!(
                flake_ref = %flake_ref,
                error = %e,
                "Failed to check build failure cache, proceeding with build"
            );
        }
    }

    // Use job's working_dir if specified, otherwise fall back to checkout_dir
    let working_dir = job.working_dir.as_ref().map(std::path::PathBuf::from).or_else(|| context.checkout_dir.clone());

    let nix_payload = NixBuildPayload {
        job_name: Some(job.name.clone()),
        run_id: Some(context.run_id.clone()),
        flake_url,
        attribute,
        extra_args: job.args.clone(),
        working_dir,
        timeout_secs: job.timeout_secs,
        sandbox: matches!(job.isolation, crate::config::types::IsolationMode::NixSandbox),
        cache_key: job.cache_key.clone(),
        artifacts: job.artifacts.clone(),
        should_upload_result: job.should_upload_result,
        publish_to_cache: job.publish_to_cache,
        cache_outputs: Vec::new(),
        // VM workers cannot access the host checkout_dir, so they need
        // source_hash to download the checkout from blob store.
        source_hash: context.source_hash.clone(),
    };

    serde_json::to_value(&nix_payload).map_err(|e| CiError::InvalidConfig {
        reason: format!("Failed to serialize Nix payload: {}", e),
    })
}

/// Build payload for VM jobs.
#[cfg(feature = "plugins-vm")]
fn build_vm_payload(
    job: &JobConfig,
    context: &PipelineContext,
    env: &HashMap<String, String>,
) -> Result<serde_json::Value> {
    // VM jobs use Cloud Hypervisor microVMs via CloudHypervisorWorker
    let command = job.command.clone().ok_or_else(|| CiError::InvalidConfig {
        reason: format!("VM job '{}' requires a command", job.name),
    })?;

    // For VM jobs, working_dir should be relative to /workspace (the virtiofs mount).
    // Use job's working_dir if specified, otherwise use "." (becomes /workspace in VM).
    // Note: We pass checkout_dir separately so the worker can copy it to workspace.
    let working_dir = job.working_dir.clone().unwrap_or_else(|| ".".to_string());

    // VM jobs use source_hash to download checkout from blob store.
    // VMs cannot access the host's checkout_dir directly since they
    // run in isolated microVMs with only virtiofs mounts to their workspace.
    // The adapter creates the source archive and sets source_hash in context.
    //
    // If source_hash is not set, the VM will fail to find the checkout.
    // checkout_dir is kept as None - it's a host path that VMs can't access.
    let vm_payload = CloudHypervisorPayload {
        job_name: Some(job.name.clone()),
        command,
        args: job.args.clone(),
        working_dir,
        env: env.clone(),
        timeout_secs: job.timeout_secs,
        artifacts: job.artifacts.clone(),
        source_hash: context.source_hash.clone(), // Download checkout from blob store
        checkout_dir: None,                       // VMs can't access host paths
        flake_attr: job.flake_attr.clone(),       // For nix command prefetching
        run_id: if context.run_id.is_empty() {
            None
        } else {
            Some(context.run_id.clone())
        },
    };

    serde_json::to_value(&vm_payload).map_err(|e| CiError::InvalidConfig {
        reason: format!("Failed to serialize VM payload: {}", e),
    })
}

#[cfg(all(test, feature = "nix-executor"))]
mod tests {
    use super::*;
    use crate::config::types::JobType;

    fn test_job_config() -> JobConfig {
        JobConfig {
            name: "build".to_string(),
            job_type: JobType::Nix,
            command: None,
            args: vec![],
            env: std::collections::HashMap::new(),
            working_dir: None,
            flake_url: Some(".".to_string()),
            flake_attr: Some("packages.x86_64-linux.default".to_string()),
            binary_hash: None,
            timeout_secs: 3600,
            isolation: crate::config::types::IsolationMode::NixSandbox,
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
        }
    }

    fn test_context() -> PipelineContext {
        PipelineContext {
            repo_id: aspen_forge::identity::RepoId::from_hash(blake3::Hash::from_bytes([0u8; 32])),
            ref_name: "refs/heads/main".to_string(),
            commit_hash: [0u8; 32],
            triggered_by: "test".to_string(),
            run_id: "test-run-id".to_string(),
            env: std::collections::HashMap::new(),
            checkout_dir: Some(std::path::PathBuf::from("/tmp/ci-test")),
            source_hash: None,
        }
    }

    fn test_store() -> std::sync::Arc<aspen_testing_core::DeterministicKeyValueStore> {
        aspen_testing_core::DeterministicKeyValueStore::new()
    }

    #[tokio::test]
    async fn test_build_nix_payload_publish_to_cache_true() {
        let job = test_job_config();
        let context = test_context();
        let store = test_store();

        let value = build_nix_payload(&job, &context, &store).await.unwrap();
        let payload: NixBuildPayload = serde_json::from_value(value).unwrap();

        assert!(payload.publish_to_cache);
        assert!(payload.should_upload_result);
    }

    #[tokio::test]
    async fn test_build_nix_payload_publish_to_cache_false() {
        let mut job = test_job_config();
        job.publish_to_cache = false;
        let context = test_context();
        let store = test_store();

        let value = build_nix_payload(&job, &context, &store).await.unwrap();
        let payload: NixBuildPayload = serde_json::from_value(value).unwrap();

        assert!(!payload.publish_to_cache);
    }

    #[tokio::test]
    async fn test_build_nix_payload_preserves_flake_fields() {
        let job = test_job_config();
        let context = test_context();
        let store = test_store();

        let value = build_nix_payload(&job, &context, &store).await.unwrap();
        let payload: NixBuildPayload = serde_json::from_value(value).unwrap();

        assert_eq!(payload.flake_url, ".");
        assert_eq!(payload.attribute, "packages.x86_64-linux.default");
    }

    /// Regression: run_id must be preserved in NixBuildPayload.
    ///
    /// The bug was that `start_pipeline_build_updated_context` created a new
    /// PipelineContext with empty run_id, which then got serialized into
    /// NixBuildPayload. Log chunks were written as `_ci:logs::<job>:<chunk>`
    /// (empty run_id), making them invisible to `ci logs` queries.
    #[tokio::test]
    async fn test_build_nix_payload_preserves_run_id() {
        let job = test_job_config();
        let context = test_context();
        let store = test_store();

        let value = build_nix_payload(&job, &context, &store).await.unwrap();
        let payload: NixBuildPayload = serde_json::from_value(value).unwrap();

        assert_eq!(
            payload.run_id,
            Some("test-run-id".to_string()),
            "run_id must be preserved from PipelineContext into NixBuildPayload"
        );
    }

    /// Verify that empty run_id would cause the log streaming bug.
    #[tokio::test]
    async fn test_build_nix_payload_empty_run_id_detected() {
        let job = test_job_config();
        let mut context = test_context();
        context.run_id = String::new(); // Simulates the bug
        let store = test_store();

        let value = build_nix_payload(&job, &context, &store).await.unwrap();
        let payload: NixBuildPayload = serde_json::from_value(value).unwrap();

        // Empty run_id means log keys will have double colon: _ci:logs::<job>:<chunk>
        assert_eq!(
            payload.run_id,
            Some(String::new()),
            "empty run_id propagates to payload — this was the root cause of the log streaming bug"
        );
    }

    #[test]
    fn test_has_deploy_jobs_for_ref_skips_guarded_stages() {
        use std::collections::HashMap;

        use crate::config::types::*;

        let config = PipelineConfig {
            name: "test".to_string(),
            description: None,
            triggers: TriggerConfig::default(),
            stages: vec![
                StageConfig {
                    name: "build".to_string(),
                    jobs: vec![JobConfig {
                        name: "build-node".to_string(),
                        job_type: JobType::Nix,
                        flake_url: Some(".".to_string()),
                        flake_attr: Some("build-node".to_string()),
                        command: None,
                        args: vec![],
                        env: HashMap::new(),
                        working_dir: None,
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
                    }],
                    parallel: true,
                    depends_on: vec![],
                    when: None,
                },
                StageConfig {
                    name: "deploy".to_string(),
                    jobs: vec![JobConfig {
                        name: "deploy-node".to_string(),
                        job_type: JobType::Deploy,
                        artifact_from: Some("build-node".to_string()),
                        strategy: Some("rolling".to_string()),
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
                        should_upload_result: false,
                        publish_to_cache: false,
                        health_check_timeout_secs: None,
                        max_concurrent: None,
                        expected_binary: None,
                        stateful: None,
                        validate_only: None,
                    }],
                    parallel: true,
                    depends_on: vec!["build".to_string()],
                    when: Some("refs/tags/release/*".to_string()),
                },
            ],
            artifacts: ArtifactConfig::default(),
            env: HashMap::new(),
            timeout_secs: 7200,
            priority: Priority::default(),
        };

        // has_deploy_jobs ignores when guard — always true
        assert!(PipelineOrchestrator::<dyn KeyValueStore>::has_deploy_jobs(&config),);

        // has_deploy_jobs_for_ref respects when guard
        assert!(
            !PipelineOrchestrator::<dyn KeyValueStore>::has_deploy_jobs_for_ref(&config, "refs/heads/main"),
            "deploy stage guarded by release tags should not count for main branch"
        );
        assert!(
            PipelineOrchestrator::<dyn KeyValueStore>::has_deploy_jobs_for_ref(&config, "refs/tags/release/v1.0"),
            "deploy stage should count when ref matches the when guard"
        );
    }
}

#[cfg(test)]
mod deploy_ref_tests {
    use aspen_core::KeyValueStore;

    use crate::config::types::*;
    use crate::orchestrator::PipelineOrchestrator;

    fn make_job(name: &str, job_type: JobType) -> JobConfig {
        JobConfig {
            name: name.to_string(),
            job_type,
            command: if job_type == JobType::Shell {
                Some("make".to_string())
            } else {
                None
            },
            args: vec![],
            env: Default::default(),
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
            should_upload_result: false,
            publish_to_cache: false,
            artifact_from: if job_type == JobType::Deploy {
                Some("build-node".to_string())
            } else {
                None
            },
            strategy: None,
            health_check_timeout_secs: None,
            max_concurrent: None,
            expected_binary: None,
            stateful: None,
            validate_only: None,
        }
    }

    fn make_deploy_pipeline_with_when(when: Option<&str>) -> PipelineConfig {
        PipelineConfig {
            name: "test".to_string(),
            description: None,
            triggers: TriggerConfig::default(),
            stages: vec![
                StageConfig {
                    name: "build".to_string(),
                    jobs: vec![make_job("build-node", JobType::Shell)],
                    parallel: true,
                    depends_on: vec![],
                    when: None,
                },
                StageConfig {
                    name: "deploy".to_string(),
                    jobs: vec![make_job("deploy-node", JobType::Deploy)],
                    parallel: true,
                    depends_on: vec!["build".to_string()],
                    when: when.map(String::from),
                },
            ],
            artifacts: ArtifactConfig::default(),
            env: Default::default(),
            timeout_secs: 7200,
            priority: Priority::default(),
        }
    }

    #[test]
    fn test_deploy_no_when_guard_always_pending() {
        let config = make_deploy_pipeline_with_when(None);
        assert!(PipelineOrchestrator::<dyn KeyValueStore>::has_deploy_jobs_for_ref(&config, "refs/heads/main"));
    }

    #[test]
    fn test_deploy_with_release_guard_skips_main() {
        let config = make_deploy_pipeline_with_when(Some("refs/tags/release/*"));
        assert!(!PipelineOrchestrator::<dyn KeyValueStore>::has_deploy_jobs_for_ref(&config, "refs/heads/main"));
    }

    #[test]
    fn test_deploy_with_release_guard_matches_tag() {
        let config = make_deploy_pipeline_with_when(Some("refs/tags/release/*"));
        assert!(PipelineOrchestrator::<dyn KeyValueStore>::has_deploy_jobs_for_ref(
            &config,
            "refs/tags/release/v2.0"
        ));
    }
}
