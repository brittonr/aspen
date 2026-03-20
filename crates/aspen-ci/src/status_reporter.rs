//! Status reporter trait for publishing commit status updates.
//!
//! CI invokes the reporter on pipeline state transitions (created → pending,
//! completed → success/failure/error). The concrete implementation writes
//! commit statuses to Forge's KV namespace and optionally broadcasts gossip.

use aspen_forge::CommitCheckState;

use crate::orchestrator::PipelineStatus;

/// Error type for status reporting operations.
pub type StatusReporterError = Box<dyn std::error::Error + Send + Sync>;

/// A commit status report to be published to Forge.
#[derive(Debug, Clone)]
pub struct CommitStatusReport {
    /// Repository ID (hex).
    pub repo_id_hex: String,
    /// Commit hash (32 bytes).
    pub commit_hash: [u8; 32],
    /// Context string identifying the check (e.g., "ci/pipeline").
    pub context: String,
    /// Current state of the check.
    pub state: CommitCheckState,
    /// Human-readable description.
    pub description: String,
    /// Pipeline run ID that produced this status.
    pub pipeline_run_id: String,
    /// Ref name (e.g., "heads/main").
    pub ref_name: String,
}

/// Trait for reporting commit status updates from CI pipelines.
///
/// Implementations write commit statuses to a persistent store and
/// optionally broadcast them over gossip for real-time notification.
#[async_trait::async_trait]
pub trait StatusReporter: Send + Sync + 'static {
    /// Report a commit status update.
    ///
    /// Errors are logged and swallowed by the caller — status reporting
    /// must not block or fail pipeline execution.
    async fn report_status(&self, report: CommitStatusReport) -> Result<(), StatusReporterError>;
}

/// Convert a CI `PipelineStatus` to a Forge `CommitCheckState`.
///
/// Only meaningful transitions are mapped:
/// - Initializing/CheckingOut/Pending/Running → Pending
/// - Success → Success
/// - Failed/CheckoutFailed → Failure
/// - Cancelled → Error
pub fn pipeline_status_to_check_state(status: &PipelineStatus) -> CommitCheckState {
    match status {
        PipelineStatus::Initializing
        | PipelineStatus::CheckingOut
        | PipelineStatus::Pending
        | PipelineStatus::Running => CommitCheckState::Pending,
        PipelineStatus::Success => CommitCheckState::Success,
        PipelineStatus::Failed | PipelineStatus::CheckoutFailed => CommitCheckState::Failure,
        PipelineStatus::Cancelled => CommitCheckState::Error,
    }
}

/// No-op reporter for when status reporting is disabled.
pub struct NoOpStatusReporter;

#[async_trait::async_trait]
impl StatusReporter for NoOpStatusReporter {
    async fn report_status(&self, _report: CommitStatusReport) -> Result<(), StatusReporterError> {
        Ok(())
    }
}

/// Mock reporter that records all calls for testing.
#[cfg(test)]
pub(crate) struct MockStatusReporter {
    pub reports: tokio::sync::Mutex<Vec<CommitStatusReport>>,
}

#[cfg(test)]
impl MockStatusReporter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            reports: tokio::sync::Mutex::new(Vec::new()),
        })
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl StatusReporter for MockStatusReporter {
    async fn report_status(&self, report: CommitStatusReport) -> Result<(), StatusReporterError> {
        self.reports.lock().await.push(report);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_status_to_check_state() {
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::Initializing), CommitCheckState::Pending);
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::CheckingOut), CommitCheckState::Pending);
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::Pending), CommitCheckState::Pending);
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::Running), CommitCheckState::Pending);
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::Success), CommitCheckState::Success);
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::Failed), CommitCheckState::Failure);
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::CheckoutFailed), CommitCheckState::Failure);
        assert_eq!(pipeline_status_to_check_state(&PipelineStatus::Cancelled), CommitCheckState::Error);
    }

    #[tokio::test]
    async fn test_mock_reporter_records_calls() {
        let reporter = MockStatusReporter::new();
        let report = CommitStatusReport {
            repo_id_hex: "abc123".to_string(),
            commit_hash: [1u8; 32],
            context: "ci/pipeline".to_string(),
            state: CommitCheckState::Success,
            description: "all tests passed".to_string(),
            pipeline_run_id: "run-1".to_string(),
            ref_name: "heads/main".to_string(),
        };

        StatusReporter::report_status(reporter.as_ref(), report).await.unwrap();

        let reports = reporter.reports.lock().await;
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].state, CommitCheckState::Success);
    }

    #[tokio::test]
    async fn test_noop_reporter() {
        let reporter = NoOpStatusReporter;
        let report = CommitStatusReport {
            repo_id_hex: "abc123".to_string(),
            commit_hash: [1u8; 32],
            context: "ci/pipeline".to_string(),
            state: CommitCheckState::Pending,
            description: "starting".to_string(),
            pipeline_run_id: "run-1".to_string(),
            ref_name: "heads/main".to_string(),
        };

        StatusReporter::report_status(&reporter, report).await.unwrap();
    }
}
