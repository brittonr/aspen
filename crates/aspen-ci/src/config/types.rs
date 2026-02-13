//! Configuration types for CI pipelines.
//!
//! This module re-exports types from `aspen-ci-core` and adds `aspen-jobs` integration.
//! The core types are defined in `aspen-ci-core::config`.

// Re-export all types from aspen-ci-core
pub use aspen_ci_core::config::ArtifactConfig;
pub use aspen_ci_core::config::ArtifactStorage;
pub use aspen_ci_core::config::IsolationMode;
pub use aspen_ci_core::config::JobConfig;
pub use aspen_ci_core::config::JobType;
pub use aspen_ci_core::config::PipelineConfig;
pub use aspen_ci_core::config::Priority;
pub use aspen_ci_core::config::StageConfig;
pub use aspen_ci_core::config::TriggerConfig;

/// Convert a CI Priority to an aspen-jobs Priority.
///
/// This is a free function instead of a From impl to avoid the orphan rule
/// since both types are defined in external crates.
pub fn to_jobs_priority(p: Priority) -> aspen_jobs::Priority {
    match p {
        Priority::High => aspen_jobs::Priority::High,
        Priority::Normal => aspen_jobs::Priority::Normal,
        Priority::Low => aspen_jobs::Priority::Low,
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

    #[test]
    fn test_priority_conversion() {
        assert!(matches!(to_jobs_priority(Priority::High), aspen_jobs::Priority::High));
        assert!(matches!(to_jobs_priority(Priority::Normal), aspen_jobs::Priority::Normal));
        assert!(matches!(to_jobs_priority(Priority::Low), aspen_jobs::Priority::Low));
    }
}
