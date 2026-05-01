use std::time::Duration;

use aspen_ci_core::ArtifactConfig;
use aspen_ci_core::JobConfig as CiJobConfig;
use aspen_ci_core::JobType;
use aspen_ci_core::PipelineConfig;
use aspen_ci_core::Priority as CiPriority;
use aspen_ci_core::StageConfig;
use aspen_ci_core::TriggerConfig;
use aspen_ci_core::are_dependencies_met;
use aspen_ci_core::compute_effective_timeout_secs;
use aspen_jobs_core::DependencyFailurePolicy;
use aspen_jobs_core::DependencyState;
use aspen_jobs_core::JobEvent;
use aspen_jobs_core::JobId;
use aspen_jobs_core::JobSpec;
use aspen_jobs_core::JobStatus;
use aspen_jobs_core::Priority as JobPriority;
use aspen_jobs_core::RetryPolicy;
use aspen_jobs_core::transition_status;
use aspen_jobs_protocol::JobDetails;
use aspen_jobs_protocol::JobSubmitResultResponse;

fn main() {
    let pipeline = PipelineConfig {
        name: "portable".to_string(),
        description: Some("portable jobs/CI core fixture".to_string()),
        triggers: TriggerConfig::default(),
        stages: vec![StageConfig {
            name: "build".to_string(),
            jobs: vec![CiJobConfig {
                name: "cargo-check".to_string(),
                job_type: JobType::Shell,
                command: Some("cargo".to_string()),
                args: vec!["check".to_string(), "-p".to_string(), "aspen-jobs-core".to_string()],
                env: Default::default(),
                working_dir: None,
                flake_url: None,
                flake_attr: None,
                binary_hash: None,
                timeout_secs: 300,
                isolation: Default::default(),
                cache_key: None,
                artifacts: Vec::new(),
                depends_on: Vec::new(),
                retry_count: 1,
                allow_failure: false,
                tags: vec!["portable".to_string()],
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
                cached_execution: false,
                speculative_count: None,
            }],
            parallel: true,
            depends_on: Vec::new(),
            when: None,
        }],
        artifacts: ArtifactConfig::default(),
        env: Default::default(),
        timeout_secs: 600,
        priority: CiPriority::Normal,
    };
    pipeline.validate().expect("portable pipeline config validates");
    assert!(are_dependencies_met(&[], &["build"]));
    assert_eq!(compute_effective_timeout_secs(Some(300), 60, 600), 300);

    let parent = JobId::from_string("parent-job".to_string());
    let retry_policy = RetryPolicy::fixed(2, Duration::from_secs(1));
    let job = JobSpec::new("ci/build").priority(JobPriority::High).depends_on(parent.clone());
    assert_eq!(job.config.priority.queue_name(), "high");
    assert_eq!(job.config.dependencies[0].as_str(), parent.as_str());
    assert_eq!(
        transition_status(JobStatus::Running, JobEvent::FailRetryable, &retry_policy, 1),
        JobStatus::Retrying
    );
    assert_eq!(
        DependencyState::Waiting(vec![parent]).clone(),
        DependencyState::Waiting(vec![JobId::from_string("parent-job".to_string())])
    );
    assert_eq!(DependencyFailurePolicy::default(), DependencyFailurePolicy::FailJob);

    let submit = JobSubmitResultResponse {
        is_success: true,
        job_id: Some("job-1".to_string()),
        error: None,
    };
    let details = JobDetails {
        job_id: "job-1".to_string(),
        job_type: "ci/build".to_string(),
        status: "running".to_string(),
        priority: 2,
        progress: 50,
        progress_message: Some("checking".to_string()),
        payload: serde_json::to_string(&job).expect("serialize core job spec"),
        tags: vec!["portable".to_string()],
        submitted_at: "2026-05-01T00:00:00Z".to_string(),
        started_at: Some("2026-05-01T00:00:01Z".to_string()),
        completed_at: None,
        worker_id: None,
        attempts: 1,
        result: None,
        error_message: None,
    };
    assert!(submit.is_success);
    assert_eq!(details.job_id, submit.job_id.unwrap());
}
