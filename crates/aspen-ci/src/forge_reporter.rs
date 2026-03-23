//! Forge-backed status reporter implementation.
//!
//! Writes commit statuses to Forge's KV namespace so pipeline results
//! are visible in Forge's commit and patch views.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::WriteRequest;
use aspen_forge::CommitStatus;
use aspen_forge::identity::RepoId;

use crate::status_reporter::CommitStatusReport;
use crate::status_reporter::StatusReporter;
use crate::status_reporter::StatusReporterError;

/// Reports pipeline status to Forge by writing `CommitStatus` entries
/// to the shared KV store.
pub struct ForgeStatusReporter<K: KeyValueStore + ?Sized + 'static> {
    kv: Arc<K>,
}

impl<K: KeyValueStore + ?Sized + 'static> ForgeStatusReporter<K> {
    /// Create a new reporter backed by the given KV store.
    pub fn new(kv: Arc<K>) -> Self {
        Self { kv }
    }
}

#[async_trait::async_trait]
impl<K: KeyValueStore + ?Sized + 'static> StatusReporter for ForgeStatusReporter<K> {
    async fn report_status(&self, report: CommitStatusReport) -> Result<(), StatusReporterError> {
        let repo_id = RepoId::from_hex(&report.repo_id_hex).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("invalid repo_id hex: {e}")))
                as StatusReporterError
        })?;

        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let status = CommitStatus {
            repo_id,
            commit: report.commit_hash,
            context: report.context.clone(),
            state: report.state,
            description: report.description,
            pipeline_run_id: Some(report.pipeline_run_id),
            created_at_ms: now_ms,
        };

        let key = format!("forge:status:{}:{}:{}", report.repo_id_hex, hex::encode(report.commit_hash), report.context);

        let value = serde_json::to_string(&status).map_err(|e| {
            Box::new(std::io::Error::other(format!("serialization failed: {e}"))) as StatusReporterError
        })?;

        self.kv
            .write(WriteRequest::set(key, value))
            .await
            .map_err(|e| Box::new(std::io::Error::other(format!("KV write failed: {e}"))) as StatusReporterError)?;

        tracing::info!(
            repo_id = %report.repo_id_hex,
            commit = %hex::encode(report.commit_hash),
            context = %report.context,
            state = %report.state,
            "reported commit status to Forge KV"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_forge::CommitCheckState;

    use super::*;

    #[tokio::test]
    async fn test_forge_reporter_writes_status() {
        let kv = aspen_testing_core::DeterministicKeyValueStore::new();
        let reporter = ForgeStatusReporter::new(kv.clone());

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
        let report = CommitStatusReport {
            repo_id_hex: repo_id.to_hex(),
            commit_hash: [0xab; 32],
            context: "ci/pipeline".to_string(),
            state: CommitCheckState::Success,
            description: "all tests passed".to_string(),
            pipeline_run_id: "run-123".to_string(),
            ref_name: "heads/main".to_string(),
        };

        StatusReporter::report_status(&reporter, report).await.unwrap();

        // Verify KV entry exists via the StatusStore
        let status_store = aspen_forge::StatusStore::new(kv as std::sync::Arc<dyn aspen_core::KeyValueStore>);
        let got = status_store.get_status(&repo_id, &[0xab; 32], "ci/pipeline").await.unwrap();
        let status = got.expect("status should be written");
        assert_eq!(status.state, CommitCheckState::Success);
        assert_eq!(status.context, "ci/pipeline");
        assert_eq!(status.commit, [0xab; 32]);
    }
}
