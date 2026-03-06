//! KV store persistence for pipeline runs.
//!
//! Handles storing and loading pipeline runs from the distributed KV store,
//! including both the primary run data and the per-repository index.

use aspen_core::KeyValueStore;
use aspen_core::ReadConsistency;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_forge::identity::RepoId;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::warn;

use super::KV_PREFIX_CI_REF_STATUS;
use super::KV_PREFIX_CI_RUNS;
use super::KV_PREFIX_CI_RUNS_BY_REPO;
use super::PipelineOrchestrator;
use super::PipelineRun;
use super::PipelineStatus;
use crate::error::CiError;
use crate::error::Result;

/// Ref status stored in KV at `_ci:ref-status:{repo_hex}:{ref_name}`.
///
/// Tracks the latest pipeline run for a given ref, enabling fast
/// `ci status <repo> <ref>` lookups without scanning all runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefStatus {
    /// Latest run ID for this ref.
    pub run_id: String,
    /// Current pipeline status.
    pub status: PipelineStatus,
    /// Pipeline name.
    pub pipeline_name: String,
}

impl<S: KeyValueStore + ?Sized + 'static> PipelineOrchestrator<S> {
    /// Check and enforce run limits.
    pub(crate) async fn check_run_limits(&self, repo_id: &RepoId) -> Result<()> {
        let active_count = self.active_runs.read().await.len() as u32;
        if active_count >= self.config.max_total_runs {
            return Err(CiError::InvalidConfig {
                reason: format!("Maximum total concurrent runs ({}) reached", self.config.max_total_runs),
            });
        }

        let repo_count = self.runs_per_repo.read().await.get(repo_id).copied().unwrap_or(0);

        if repo_count >= self.config.max_runs_per_repo as usize {
            return Err(CiError::InvalidConfig {
                reason: format!("Maximum concurrent runs per repository ({}) reached", self.config.max_runs_per_repo),
            });
        }

        Ok(())
    }

    /// Track a new pipeline run.
    ///
    /// Updates both in-memory cache and persists to KV store.
    pub(crate) async fn track_run(&self, run: &PipelineRun) {
        // Add to in-memory cache
        self.active_runs.write().await.insert(run.id.clone(), run.clone());

        let mut repo_runs = self.runs_per_repo.write().await;
        *repo_runs.entry(run.context.repo_id).or_insert(0) += 1;

        // Persist to KV store
        if let Err(e) = self.persist_run(run).await {
            warn!(run_id = %run.id, error = %e, "Failed to persist pipeline run to KV store");
        }
    }

    /// Persist a pipeline run to the KV store.
    ///
    /// Stores:
    /// - The full run data at `_ci:runs:{run_id}`
    /// - A repo index entry at `_ci:runs:by-repo:{repo_id}:{created_at_ms}:{run_id}`
    pub(crate) async fn persist_run(&self, run: &PipelineRun) -> std::result::Result<(), CiError> {
        // Serialize the run to JSON string
        let run_json = serde_json::to_string(run).map_err(|e| CiError::InvalidConfig {
            reason: format!("Failed to serialize pipeline run: {}", e),
        })?;

        // Build keys
        let run_key = format!("{}{}", KV_PREFIX_CI_RUNS, run.id);
        let created_at_ms = run.created_at.timestamp_millis() as u64;
        let index_key =
            format!("{}{}:{}:{}", KV_PREFIX_CI_RUNS_BY_REPO, run.context.repo_id.to_hex(), created_at_ms, run.id);

        // Build ref-status index entry
        let ref_status = RefStatus {
            run_id: run.id.clone(),
            status: run.status,
            pipeline_name: run.pipeline_name.clone(),
        };
        let ref_status_key =
            format!("{}{}:{}", KV_PREFIX_CI_REF_STATUS, run.context.repo_id.to_hex(), run.context.ref_name);
        let ref_status_json = serde_json::to_string(&ref_status).map_err(|e| CiError::InvalidConfig {
            reason: format!("Failed to serialize ref status: {}", e),
        })?;

        // Write all keys atomically using SetMulti
        // Values are String in the KV store
        let write_request = WriteRequest {
            command: WriteCommand::SetMulti {
                pairs: vec![
                    (run_key.clone(), run_json),
                    (index_key.clone(), run.id.clone()),
                    (ref_status_key, ref_status_json),
                ],
            },
        };

        self.kv_store.write(write_request).await.map_err(|e| CiError::InvalidConfig {
            reason: format!("Failed to write pipeline run to KV store: {}", e),
        })?;

        debug!(
            run_id = %run.id,
            run_key = %run_key,
            index_key = %index_key,
            "Persisted pipeline run to KV store"
        );

        Ok(())
    }

    /// Get the latest pipeline run for a ref.
    ///
    /// Looks up the ref-status index to find the latest run ID,
    /// then loads the full run from KV.
    pub async fn get_latest_run_for_ref(&self, repo_id: &RepoId, ref_name: &str) -> Option<PipelineRun> {
        let key = format!("{}{}:{}", KV_PREFIX_CI_REF_STATUS, repo_id.to_hex(), ref_name);

        let read_request = ReadRequest {
            key: key.clone(),
            consistency: ReadConsistency::Linearizable,
        };

        let ref_status: RefStatus = match self.kv_store.read(read_request).await {
            Ok(result) => match result.kv {
                Some(kv_entry) => match serde_json::from_str(&kv_entry.value) {
                    Ok(status) => status,
                    Err(e) => {
                        debug!(key = %key, error = %e, "Failed to parse ref status from KV store");
                        return None;
                    }
                },
                None => return None,
            },
            Err(e) => {
                debug!(key = %key, error = %e, "Failed to read ref status from KV store");
                return None;
            }
        };

        // Load the full run
        self.load_run_from_kv(&ref_status.run_id).await
    }

    /// Get the ref status (without loading the full run).
    ///
    /// Returns the latest run ID and status for a ref.
    pub async fn get_ref_status(&self, repo_id: &RepoId, ref_name: &str) -> Option<RefStatus> {
        let key = format!("{}{}:{}", KV_PREFIX_CI_REF_STATUS, repo_id.to_hex(), ref_name);

        let read_request = ReadRequest {
            key,
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv_store.read(read_request).await {
            Ok(result) => result.kv.and_then(|kv| serde_json::from_str(&kv.value).ok()),
            Err(_) => None,
        }
    }

    /// Load a pipeline run from the KV store.
    pub(crate) async fn load_run_from_kv(&self, run_id: &str) -> Option<PipelineRun> {
        let key = format!("{}{}", KV_PREFIX_CI_RUNS, run_id);

        let read_request = ReadRequest {
            key: key.clone(),
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv_store.read(read_request).await {
            Ok(result) => {
                // ReadResult has a `kv` field with Option<KeyValueWithRevision>
                if let Some(kv_entry) = result.kv {
                    match serde_json::from_str::<PipelineRun>(&kv_entry.value) {
                        Ok(run) => Some(run),
                        Err(e) => {
                            debug!(key = %key, error = %e, "Failed to parse pipeline run from KV store");
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Err(e) => {
                debug!(key = %key, error = %e, "Failed to read pipeline run from KV store");
                None
            }
        }
    }
}
