use std::collections::HashMap;

use aspen_ci_core::{
    are_dependencies_met, check_pipeline_limits, compute_effective_timeout_secs,
    find_missing_dependency, find_ready_stages, ArtifactConfig, ArtifactStorage, CiLogChunk,
    IsolationMode, JobConfig, JobType, PipelineConfig, PipelineLimits, Priority, StageConfig,
    TriggerConfig,
};
use aspen_jobs_protocol::{JobDetails, WorkerPollJobsResultResponse};

fn shell_job(name: &str) -> JobConfig {
    JobConfig {
        name: name.to_string(),
        job_type: JobType::Shell,
        command: Some("cargo".to_string()),
        args: vec!["check".to_string()],
        env: HashMap::new(),
        working_dir: None,
        flake_url: None,
        flake_attr: None,
        binary_hash: None,
        timeout_secs: 300,
        isolation: IsolationMode::None,
        cache_key: None,
        artifacts: vec!["target/debug".to_string()],
        depends_on: vec![],
        retry_count: 0,
        allow_failure: false,
        tags: vec!["portable".to_string()],
        should_upload_result: false,
        publish_to_cache: false,
        artifact_from: None,
        strategy: None,
        health_check_timeout_secs: None,
        max_concurrent: None,
        expected_binary: None,
        stateful: Some(false),
        validate_only: Some(true),
        force_cold_boot: false,
        cached_execution: false,
        speculative_count: None,
    }
}

pub fn sample_pipeline() -> PipelineConfig {
    PipelineConfig {
        name: "portable-ci".to_string(),
        description: Some("portable fixture for jobs-ci-core".to_string()),
        triggers: TriggerConfig::default(),
        stages: vec![
            StageConfig {
                name: "build".to_string(),
                jobs: vec![shell_job("cargo-check")],
                parallel: false,
                depends_on: vec![],
                when: None,
            },
            StageConfig {
                name: "test".to_string(),
                jobs: vec![shell_job("cargo-test")],
                parallel: true,
                depends_on: vec!["build".to_string()],
                when: None,
            },
        ],
        artifacts: ArtifactConfig {
            storage: ArtifactStorage::None,
            retention_days: 1,
            should_compress: false,
        },
        env: HashMap::new(),
        timeout_secs: 600,
        priority: Priority::Normal,
    }
}

pub fn prove_pure_ci_helpers() {
    let config = sample_pipeline();
    config.validate().expect("portable pipeline validates");
    check_pipeline_limits(PipelineLimits {
        stage_count: 2,
        job_count: 2,
        max_stages: 8,
        max_jobs: 16,
    })
    .expect("limits pass");
    assert_eq!(compute_effective_timeout_secs(Some(10), 20, 60), 10);
    assert_eq!(find_missing_dependency(&["build"], &["build", "test"]), None);
    assert!(are_dependencies_met(&["build"], &["build"]));
    assert_eq!(find_ready_stages(&[("build", vec![]), ("test", vec!["build"])], &[], &[]), vec!["build"]);
}

pub fn prove_protocol_and_log_types_serialize() {
    let chunk = CiLogChunk {
        index: 1,
        content: "portable log chunk".to_string(),
        timestamp_ms: 1_700_000_000,
    };
    let chunk_json = serde_json::to_string(&chunk).expect("log chunk serializes");
    let parsed_chunk: CiLogChunk = serde_json::from_str(&chunk_json).expect("log chunk parses");
    assert_eq!(parsed_chunk.index, 1);

    let details = JobDetails {
        job_id: "job-1".to_string(),
        job_type: "ci".to_string(),
        status: "queued".to_string(),
        priority: 1,
        progress: 0,
        progress_message: None,
        payload: "{}".to_string(),
        tags: vec!["portable".to_string()],
        submitted_at: "2026-04-30T21:40:00Z".to_string(),
        started_at: None,
        completed_at: None,
        worker_id: None,
        attempts: 0,
        result: None,
        error_message: None,
    };
    let poll = WorkerPollJobsResultResponse {
        is_success: true,
        worker_id: "worker-1".to_string(),
        jobs: vec![],
        error: None,
    };
    assert_eq!(details.status, "queued");
    assert!(poll.is_success);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_jobs_ci_core_surface_works() {
        prove_pure_ci_helpers();
        prove_protocol_and_log_types_serialize();
    }
}
