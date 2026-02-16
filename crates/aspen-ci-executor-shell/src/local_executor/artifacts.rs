//! Artifact collection and upload for completed jobs.

use tracing::warn;

use super::LocalExecutorPayload;
use super::LocalExecutorWorker;
use crate::agent::protocol::ExecutionResult;
use crate::common::ArtifactCollectionResult;
use crate::common::ArtifactUploadResult;
use crate::common::collect_artifacts;
use crate::common::upload_artifacts_to_blob_store;

impl LocalExecutorWorker {
    /// Collect and upload artifacts if the job succeeded.
    pub(super) async fn collect_and_upload_artifacts(
        &self,
        job_id: &str,
        result: &ExecutionResult,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
    ) -> (ArtifactCollectionResult, Option<ArtifactUploadResult>) {
        // Early return: skip artifact collection on failure
        if result.exit_code != 0 {
            return (ArtifactCollectionResult::default(), None);
        }
        if result.error.is_some() {
            return (ArtifactCollectionResult::default(), None);
        }
        // Early return: no artifacts requested
        if payload.artifacts.is_empty() {
            return (ArtifactCollectionResult::default(), None);
        }

        match collect_artifacts(job_workspace, &payload.artifacts).await {
            Ok(collected) => {
                let upload = if let Some(ref blob_store) = self.blob_store {
                    if !collected.artifacts.is_empty() {
                        Some(upload_artifacts_to_blob_store(&collected, blob_store, job_id).await)
                    } else {
                        None
                    }
                } else {
                    None
                };
                (collected, upload)
            }
            Err(e) => {
                warn!(job_id = %job_id, error = ?e, "artifact collection failed");
                (ArtifactCollectionResult::default(), None)
            }
        }
    }
}
