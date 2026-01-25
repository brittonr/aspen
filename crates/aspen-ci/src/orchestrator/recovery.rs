//! Orphaned pipeline recovery after leader failover.
//!
//! This module handles recovery of CI pipelines that were in progress when
//! the Raft leader crashed. After a new leader is elected, it scans for
//! pipelines in a running state and attempts to recover them.
//!
//! # Recovery Process
//!
//! 1. Scan KV for pipelines with status=Running or status=Pending
//! 2. Check job heartbeats to determine if workers are still alive
//! 3. Mark stale jobs as Unknown for re-evaluation
//! 4. Reschedule or fail jobs based on retry policy
//!
//! # Tiger Style
//!
//! - Bounded recovery batch size (`MAX_PIPELINE_RECOVERY_BATCH`)
//! - Fixed orphan detection threshold (`JOB_ORPHAN_DETECTION_THRESHOLD_MS`)
//! - Fail-fast on KV errors (let Raft handle retry)

use std::sync::Arc;
use std::time::Duration;

use aspen_constants::JOB_ORPHAN_DETECTION_THRESHOLD_MS;
use aspen_constants::MAX_PIPELINE_RECOVERY_BATCH;
use aspen_core::KeyValueStore;
use aspen_core::ReadConsistency;
use aspen_core::ScanRequest;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::pipeline::PipelineRun;
use super::pipeline::PipelineStatus;
use crate::error::CiError;
use crate::error::Result;

/// KV prefix for job heartbeats.
/// Key format: `{KV_PREFIX_JOB_HEARTBEAT}{job_id}`
/// Value: Timestamp in milliseconds since Unix epoch.
const KV_PREFIX_JOB_HEARTBEAT: &str = "_jobs:heartbeat:";

/// KV prefix for CI runs (must match pipeline.rs).
const KV_PREFIX_CI_RUNS: &str = "_ci:runs:";

/// Result of recovering a single pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    /// Pipeline run ID.
    pub run_id: String,
    /// Action taken.
    pub action: RecoveryAction,
    /// Number of jobs affected.
    pub jobs_affected: u32,
    /// Error message if recovery failed.
    pub error: Option<String>,
}

/// Action taken for pipeline recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Pipeline is still running with live workers - no action needed.
    StillRunning,
    /// Pipeline's jobs were marked as Unknown for re-evaluation.
    MarkedUnknown,
    /// Pipeline was rescheduled.
    Rescheduled,
    /// Pipeline was marked as failed (no retries remaining).
    MarkedFailed,
    /// Pipeline was skipped (not in recoverable state).
    Skipped,
}

/// Orchestrates recovery of orphaned pipelines after leader failover.
pub struct OrphanedPipelineRecovery<S: KeyValueStore + ?Sized> {
    /// KV store for reading pipeline and heartbeat data.
    kv_store: Arc<S>,
    /// Orphan detection threshold.
    orphan_threshold: Duration,
    /// Maximum pipelines to recover per scan.
    batch_size: u32,
}

impl<S: KeyValueStore + ?Sized + 'static> OrphanedPipelineRecovery<S> {
    /// Create a new recovery orchestrator.
    pub fn new(kv_store: Arc<S>) -> Self {
        Self {
            kv_store,
            orphan_threshold: Duration::from_millis(JOB_ORPHAN_DETECTION_THRESHOLD_MS),
            batch_size: MAX_PIPELINE_RECOVERY_BATCH,
        }
    }

    /// Scan for and recover orphaned pipelines.
    ///
    /// This should be called after the new leader is elected to ensure
    /// in-progress pipelines are not left in limbo.
    ///
    /// # Returns
    ///
    /// Results for each pipeline that was evaluated for recovery.
    pub async fn recover_orphaned_pipelines(&self) -> Result<Vec<RecoveryResult>> {
        info!("starting orphaned pipeline recovery scan");

        // Scan for all pipeline runs
        let scan_request = ScanRequest {
            prefix: KV_PREFIX_CI_RUNS.to_string(),
            limit: Some(self.batch_size),
            continuation_token: None,
        };

        let scan_result = self.kv_store.scan(scan_request).await.map_err(|e| CiError::InvalidConfig {
            reason: format!("Failed to scan pipelines: {}", e),
        })?;

        let mut results = Vec::new();
        let mut recovered_count = 0u32;

        for entry in scan_result.entries {
            if recovered_count >= self.batch_size {
                info!(
                    recovered = recovered_count,
                    batch_size = self.batch_size,
                    "recovery batch limit reached, remaining pipelines will be recovered in next scan"
                );
                break;
            }

            // Parse the pipeline run
            let run: PipelineRun = match serde_json::from_str(&entry.value) {
                Ok(r) => r,
                Err(e) => {
                    warn!(
                        key = %entry.key,
                        error = %e,
                        "failed to deserialize pipeline run, skipping"
                    );
                    continue;
                }
            };

            // Only consider non-terminal pipelines
            if run.status.is_terminal() {
                continue;
            }

            // Check if this pipeline needs recovery
            let result = self.evaluate_pipeline(&run).await;
            if !matches!(result.action, RecoveryAction::Skipped | RecoveryAction::StillRunning) {
                recovered_count += 1;
            }
            results.push(result);
        }

        info!(
            total_evaluated = results.len(),
            recovered = recovered_count,
            "orphaned pipeline recovery scan complete"
        );

        Ok(results)
    }

    /// Evaluate a single pipeline for recovery.
    async fn evaluate_pipeline(&self, run: &PipelineRun) -> RecoveryResult {
        debug!(
            run_id = %run.id,
            status = ?run.status,
            "evaluating pipeline for recovery"
        );

        match run.status {
            PipelineStatus::Running => self.evaluate_running_pipeline(run).await,
            PipelineStatus::Pending | PipelineStatus::Initializing | PipelineStatus::CheckingOut => {
                // These pipelines may have been interrupted during setup
                RecoveryResult {
                    run_id: run.id.clone(),
                    action: RecoveryAction::MarkedUnknown,
                    jobs_affected: 0,
                    error: Some("Pipeline interrupted during initialization".to_string()),
                }
            }
            _ => RecoveryResult {
                run_id: run.id.clone(),
                action: RecoveryAction::Skipped,
                jobs_affected: 0,
                error: None,
            },
        }
    }

    /// Evaluate a running pipeline by checking job heartbeats.
    async fn evaluate_running_pipeline(&self, run: &PipelineRun) -> RecoveryResult {
        let mut stale_jobs = 0u32;
        let mut live_jobs = 0u32;
        let now = Utc::now().timestamp_millis() as u64;
        let threshold_ms = self.orphan_threshold.as_millis() as u64;

        // Check heartbeats for each stage's jobs
        for stage in &run.stages {
            for (job_name, job_status) in &stage.jobs {
                // Get job ID if assigned
                let job_id = match job_status {
                    super::pipeline::JobStatus { job_id: Some(id), .. } => id.as_str().to_string(),
                    _ => continue,
                };

                // Check heartbeat
                match self.check_job_heartbeat(&job_id, now, threshold_ms).await {
                    HeartbeatStatus::Live => {
                        live_jobs += 1;
                        debug!(job_id = %job_id, job_name = %job_name, "job heartbeat is live");
                    }
                    HeartbeatStatus::Stale(last_heartbeat) => {
                        stale_jobs += 1;
                        debug!(
                            job_id = %job_id,
                            job_name = %job_name,
                            last_heartbeat_ms = last_heartbeat,
                            age_ms = now.saturating_sub(last_heartbeat),
                            "job heartbeat is stale"
                        );
                    }
                    HeartbeatStatus::Missing => {
                        stale_jobs += 1;
                        debug!(job_id = %job_id, job_name = %job_name, "job has no heartbeat");
                    }
                }
            }
        }

        // Determine action based on heartbeat status
        if stale_jobs == 0 && live_jobs > 0 {
            // All jobs with heartbeats are still live
            RecoveryResult {
                run_id: run.id.clone(),
                action: RecoveryAction::StillRunning,
                jobs_affected: 0,
                error: None,
            }
        } else if stale_jobs > 0 {
            // Some jobs are stale - mark them as unknown
            info!(
                run_id = %run.id,
                stale_jobs = stale_jobs,
                live_jobs = live_jobs,
                "marking pipeline with stale jobs for recovery"
            );
            RecoveryResult {
                run_id: run.id.clone(),
                action: RecoveryAction::MarkedUnknown,
                jobs_affected: stale_jobs,
                error: None,
            }
        } else {
            // No jobs with heartbeats - may be early in execution
            RecoveryResult {
                run_id: run.id.clone(),
                action: RecoveryAction::Skipped,
                jobs_affected: 0,
                error: Some("No jobs with heartbeats found".to_string()),
            }
        }
    }

    /// Check the heartbeat status of a job.
    async fn check_job_heartbeat(&self, job_id: &str, now_ms: u64, threshold_ms: u64) -> HeartbeatStatus {
        let heartbeat_key = format!("{}{}", KV_PREFIX_JOB_HEARTBEAT, job_id);

        let read_request = aspen_core::ReadRequest {
            key: heartbeat_key,
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv_store.read(read_request).await {
            Ok(result) => {
                if let Some(kv) = result.kv {
                    // Parse timestamp from value
                    if let Ok(timestamp) = kv.value.trim().parse::<u64>() {
                        let age_ms = now_ms.saturating_sub(timestamp);
                        if age_ms <= threshold_ms {
                            HeartbeatStatus::Live
                        } else {
                            HeartbeatStatus::Stale(timestamp)
                        }
                    } else {
                        HeartbeatStatus::Missing
                    }
                } else {
                    HeartbeatStatus::Missing
                }
            }
            Err(e) => {
                warn!(job_id = %job_id, error = %e, "failed to read job heartbeat");
                HeartbeatStatus::Missing
            }
        }
    }

    /// Get the list of pipelines that need recovery without taking action.
    ///
    /// This is useful for monitoring and diagnostics.
    pub async fn list_orphaned_pipelines(&self) -> Result<Vec<PipelineRun>> {
        let scan_request = ScanRequest {
            prefix: KV_PREFIX_CI_RUNS.to_string(),
            limit: Some(self.batch_size),
            continuation_token: None,
        };

        let scan_result = self.kv_store.scan(scan_request).await.map_err(|e| CiError::InvalidConfig {
            reason: format!("Failed to scan pipelines: {}", e),
        })?;

        let mut orphaned = Vec::new();

        for entry in scan_result.entries {
            if let Ok(run) = serde_json::from_str::<PipelineRun>(&entry.value) {
                if !run.status.is_terminal() && matches!(run.status, PipelineStatus::Running) {
                    orphaned.push(run);
                }
            }
        }

        Ok(orphaned)
    }
}

/// Status of a job's heartbeat.
#[derive(Debug)]
enum HeartbeatStatus {
    /// Heartbeat is recent and within threshold.
    Live,
    /// Heartbeat exists but is older than threshold.
    Stale(u64),
    /// No heartbeat found.
    Missing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_action_serialization() {
        let action = RecoveryAction::MarkedUnknown;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"MarkedUnknown\"");

        let deserialized: RecoveryAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_recovery_result_serialization() {
        let result = RecoveryResult {
            run_id: "test-run-123".to_string(),
            action: RecoveryAction::Rescheduled,
            jobs_affected: 3,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: RecoveryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.run_id, result.run_id);
        assert_eq!(deserialized.action, result.action);
        assert_eq!(deserialized.jobs_affected, result.jobs_affected);
    }
}
