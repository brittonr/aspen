//! Job workflows with CAS operations for atomic state transitions.
//!
//! This module provides workflow capabilities that leverage Aspen's CAS operations
//! to ensure atomic state transitions and enable complex multi-step job pipelines.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use tracing::info;
use tracing::warn;

use crate::error::Result;
use crate::job::JobId;
use crate::job::JobSpec;
use crate::manager::JobManager;

/// Workflow state with CAS version tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Workflow ID.
    pub id: String,
    /// Current state name.
    pub state: String,
    /// CAS version for atomic updates.
    pub version: u64,
    /// Workflow data.
    pub data: serde_json::Value,
    /// Currently active jobs.
    pub active_jobs: HashSet<JobId>,
    /// Completed jobs in this workflow.
    pub completed_jobs: HashSet<JobId>,
    /// Failed jobs in this workflow.
    pub failed_jobs: HashSet<JobId>,
    /// State transition history.
    ///
    /// Bounded to `MAX_WORKFLOW_HISTORY_SIZE` entries. When the limit is
    /// reached, the oldest 10% of entries are removed.
    pub history: Vec<StateTransition>,
}

impl WorkflowState {
    /// Add a transition to the history with bounded size.
    ///
    /// Tiger Style: Prevents unbounded memory growth by trimming old
    /// history when the limit is reached.
    pub fn add_history(&mut self, transition: StateTransition) {
        let max_size = aspen_core::MAX_WORKFLOW_HISTORY_SIZE as usize;

        // If at capacity, remove oldest 10% to make room
        if self.history.len() >= max_size {
            let keep_count = (max_size * 9) / 10; // Keep 90%
            let remove_count = self.history.len() - keep_count;
            self.history.drain(0..remove_count);
        }

        self.history.push(transition);
    }
}

/// Record of a state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Previous state.
    pub from: String,
    /// New state.
    pub to: String,
    /// Transition timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Trigger (job completion, timeout, manual).
    pub trigger: TransitionTrigger,
}

/// What triggered a state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionTrigger {
    /// Job completed successfully.
    JobSuccess(JobId),
    /// Job failed.
    JobFailed(JobId),
    /// Manual transition.
    Manual,
    /// Timeout expired.
    Timeout,
    /// Condition met.
    Condition(String),
}

/// Workflow step definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name.
    pub name: String,
    /// Jobs to execute in this step.
    pub jobs: Vec<JobSpec>,
    /// Next steps based on outcome.
    pub transitions: Vec<WorkflowTransition>,
    /// Parallel execution allowed.
    pub parallel: bool,
    /// Timeout for this step.
    pub timeout: Option<std::time::Duration>,
    /// Retry policy for jobs in this step.
    pub retry_on_failure: bool,
}

/// Conditional workflow transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransition {
    /// Condition to evaluate.
    pub condition: TransitionCondition,
    /// Target state if condition met.
    pub target: String,
}

/// Transition condition types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    /// All jobs succeeded.
    AllSuccess,
    /// Any job failed.
    AnyFailed,
    /// Specific job succeeded.
    JobSuccess(String),
    /// Specific job failed.
    JobFailed(String),
    /// Percentage of jobs succeeded.
    SuccessRate(f32),
    /// Custom condition evaluated on workflow data.
    Custom(String),
    /// Always transition (default).
    Always,
}

/// Workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Workflow name.
    pub name: String,
    /// Initial state.
    pub initial_state: String,
    /// All workflow steps.
    pub steps: HashMap<String, WorkflowStep>,
    /// Terminal states.
    pub terminal_states: HashSet<String>,
    /// Global timeout.
    pub timeout: Option<std::time::Duration>,
}

/// Workflow manager with CAS-based state management.
pub struct WorkflowManager<S: aspen_core::KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
    store: Arc<S>,
    /// Cached workflow definitions (workflow_id -> definition).
    definitions: tokio::sync::RwLock<std::collections::HashMap<String, WorkflowDefinition>>,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> WorkflowManager<S> {
    /// Create a new workflow manager.
    pub fn new(manager: Arc<JobManager<S>>, store: Arc<S>) -> Self {
        Self {
            manager,
            store,
            definitions: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get a workflow definition by ID.
    pub async fn get_definition(&self, workflow_id: &str) -> Option<WorkflowDefinition> {
        self.definitions.read().await.get(workflow_id).cloned()
    }

    /// Start a new workflow instance.
    pub async fn start_workflow(
        &self,
        definition: &WorkflowDefinition,
        initial_data: serde_json::Value,
    ) -> Result<String> {
        let workflow_id = uuid::Uuid::new_v4().to_string();

        // Cache the definition for later use (e.g., job completion callbacks)
        self.definitions.write().await.insert(workflow_id.clone(), definition.clone());

        let state = WorkflowState {
            id: workflow_id.clone(),
            state: definition.initial_state.clone(),
            version: 0,
            data: initial_data,
            active_jobs: HashSet::new(),
            completed_jobs: HashSet::new(),
            failed_jobs: HashSet::new(),
            history: vec![],
        };

        // Store workflow state with CAS version 0
        let key = format!("__workflow::{}", workflow_id);
        let value = serde_json::to_vec(&state).map_err(|e| crate::error::JobError::SerializationError { source: e })?;

        self.store
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set {
                    key,
                    value: String::from_utf8_lossy(&value).to_string(),
                },
            })
            .await
            .map_err(|e| crate::error::JobError::StorageError { source: e })?;

        info!(
            workflow_id = %workflow_id,
            initial_state = %definition.initial_state,
            "workflow started"
        );

        // Execute initial step
        self.execute_step(&workflow_id, definition, &state).await?;

        Ok(workflow_id)
    }

    /// Execute a workflow step.
    async fn execute_step(
        &self,
        workflow_id: &str,
        definition: &WorkflowDefinition,
        state: &WorkflowState,
    ) -> Result<()> {
        let step = definition.steps.get(&state.state).ok_or_else(|| crate::error::JobError::InvalidJobSpec {
            reason: format!("Unknown workflow state: {}", state.state),
        })?;

        info!(
            workflow_id = %workflow_id,
            step = %step.name,
            "executing workflow step"
        );

        // Submit jobs for this step
        let mut job_ids = Vec::new();
        for job_spec in &step.jobs {
            let mut spec = job_spec.clone();

            // Add workflow metadata
            if spec.payload.is_object() {
                spec.payload["__workflow_id"] = serde_json::Value::String(workflow_id.to_string());
                spec.payload["__workflow_step"] = serde_json::Value::String(step.name.clone());
            }

            let job_id = self.manager.submit(spec).await?;
            job_ids.push(job_id);
        }

        // Update workflow state with active jobs (CAS operation)
        self.update_workflow_state(workflow_id, |mut state| {
            state.active_jobs.extend(job_ids.clone());
            Ok(state)
        })
        .await?;

        Ok(())
    }

    /// Update workflow state using true CAS operation via Aspen's CompareAndSwap.
    ///
    /// This function implements optimistic concurrency control with exponential backoff:
    /// 1. Read current state
    /// 2. Apply updater function
    /// 3. Attempt CompareAndSwap with expected=old_value, new_value=updated_state
    /// 4. On CAS failure, retry with exponential backoff
    ///
    /// The CAS operation is atomic at the Raft consensus level, ensuring linearizability.
    pub async fn update_workflow_state<F>(&self, workflow_id: &str, mut updater: F) -> Result<WorkflowState>
    where F: FnMut(WorkflowState) -> Result<WorkflowState> {
        let key = format!("__workflow::{}", workflow_id);
        let max_retries: u32 = aspen_core::MAX_CAS_RETRIES.min(10); // Cap at 10 for workflows

        for attempt in 0..max_retries {
            // Read current state with version
            let result = self
                .store
                .read(aspen_core::ReadRequest::new(key.clone()))
                .await
                .map_err(|e| crate::error::JobError::StorageError { source: e })?;

            let (current_value, data) = match result {
                aspen_core::ReadResult { kv: Some(kv), .. } => {
                    let value = kv.value.clone();
                    (Some(value.clone()), value.into_bytes())
                }
                _ => {
                    return Err(crate::error::JobError::JobNotFound {
                        id: workflow_id.to_string(),
                    });
                }
            };

            let mut state: WorkflowState =
                serde_json::from_slice(&data).map_err(|e| crate::error::JobError::SerializationError { source: e })?;

            let old_version = state.version;
            state.version += 1;

            // Apply update
            state = updater(state)?;

            // Serialize updated state
            let new_value =
                serde_json::to_string(&state).map_err(|e| crate::error::JobError::SerializationError { source: e })?;

            // Use true CompareAndSwap operation from Aspen
            // This is atomic at the Raft consensus level
            match self
                .store
                .write(aspen_core::WriteRequest {
                    command: aspen_core::WriteCommand::CompareAndSwap {
                        key: key.clone(),
                        expected: current_value.clone(),
                        new_value,
                    },
                })
                .await
            {
                Ok(_) => {
                    info!(
                        workflow_id = %workflow_id,
                        old_version,
                        new_version = state.version,
                        "workflow state updated via CAS"
                    );
                    return Ok(state);
                }
                Err(aspen_core::KeyValueStoreError::CompareAndSwapFailed { .. }) if attempt < max_retries - 1 => {
                    // CAS conflict - another writer modified the state
                    // Use exponential backoff: 1ms, 2ms, 4ms, 8ms, ...
                    let backoff_ms = aspen_core::CAS_RETRY_INITIAL_BACKOFF_MS << attempt.min(7);
                    warn!(
                        workflow_id = %workflow_id,
                        attempt,
                        backoff_ms,
                        "CAS conflict, retrying with backoff"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
                Err(aspen_core::KeyValueStoreError::CompareAndSwapFailed { expected, actual, .. }) => {
                    // Final retry exhausted
                    warn!(
                        workflow_id = %workflow_id,
                        attempt,
                        ?expected,
                        ?actual,
                        "CAS conflict on final retry"
                    );
                    return Err(crate::error::JobError::CasRetryExhausted {
                        workflow_id: workflow_id.to_string(),
                        attempts: max_retries,
                    });
                }
                Err(e) => {
                    return Err(crate::error::JobError::StorageError { source: e });
                }
            }
        }

        Err(crate::error::JobError::CasRetryExhausted {
            workflow_id: workflow_id.to_string(),
            attempts: max_retries,
        })
    }

    /// Process job completion and trigger state transitions.
    pub async fn process_job_completion(
        &self,
        workflow_id: &str,
        job_id: &JobId,
        definition: &WorkflowDefinition,
    ) -> Result<()> {
        // Fetch actual job status from manager BEFORE updating workflow state
        let job_succeeded = match self.manager.get_job(job_id).await {
            Ok(Some(job)) => {
                use crate::job::JobStatus;
                matches!(job.status, JobStatus::Completed)
            }
            Ok(None) => {
                warn!(
                    workflow_id = %workflow_id,
                    job_id = %job_id,
                    "Job not found when processing completion, treating as failed"
                );
                false
            }
            Err(e) => {
                warn!(
                    workflow_id = %workflow_id,
                    job_id = %job_id,
                    error = %e,
                    "Failed to fetch job status, treating as failed"
                );
                false
            }
        };

        info!(
            workflow_id = %workflow_id,
            job_id = %job_id,
            succeeded = job_succeeded,
            "processing job completion in workflow"
        );

        let state = self
            .update_workflow_state(workflow_id, |mut state| {
                // Move job from active to completed/failed based on actual status
                if state.active_jobs.remove(job_id) {
                    if job_succeeded {
                        state.completed_jobs.insert(job_id.clone());
                    } else {
                        state.failed_jobs.insert(job_id.clone());
                    }
                }
                Ok(state)
            })
            .await?;

        info!(
            workflow_id = %workflow_id,
            state = %state.state,
            active = state.active_jobs.len(),
            completed = state.completed_jobs.len(),
            failed = state.failed_jobs.len(),
            "workflow state after job completion"
        );

        // Check if we should transition
        if let Some(step) = definition.steps.get(&state.state) {
            for transition in &step.transitions {
                if self.evaluate_condition(&transition.condition, &state) {
                    self.transition_state(workflow_id, &transition.target, definition).await?;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Evaluate a transition condition.
    fn evaluate_condition(&self, condition: &TransitionCondition, state: &WorkflowState) -> bool {
        match condition {
            TransitionCondition::AllSuccess => state.active_jobs.is_empty() && state.failed_jobs.is_empty(),
            TransitionCondition::AnyFailed => !state.failed_jobs.is_empty(),
            TransitionCondition::SuccessRate(rate) => {
                let total = state.completed_jobs.len() + state.failed_jobs.len();
                if total == 0 {
                    return false;
                }
                let success_rate = state.completed_jobs.len() as f32 / total as f32;
                success_rate >= *rate
            }
            TransitionCondition::Always => true,
            _ => false, // Other conditions would need more context
        }
    }

    /// Transition to a new workflow state.
    async fn transition_state(&self, workflow_id: &str, target: &str, definition: &WorkflowDefinition) -> Result<()> {
        let state = self
            .update_workflow_state(workflow_id, |mut state| {
                let transition = StateTransition {
                    from: state.state.clone(),
                    to: target.to_string(),
                    timestamp: chrono::Utc::now(),
                    trigger: TransitionTrigger::Condition("auto".to_string()),
                };

                state.add_history(transition);
                state.state = target.to_string();
                Ok(state)
            })
            .await?;

        info!(
            workflow_id = %workflow_id,
            new_state = %target,
            "workflow state transitioned"
        );

        // Execute new step if not terminal
        if !definition.terminal_states.contains(target) {
            self.execute_step(workflow_id, definition, &state).await?;
        }

        Ok(())
    }

    /// Get workflow state.
    pub async fn get_workflow_state(&self, workflow_id: &str) -> Result<WorkflowState> {
        let key = format!("__workflow::{}", workflow_id);
        let result = self
            .store
            .read(aspen_core::ReadRequest::new(key))
            .await
            .map_err(|e| crate::error::JobError::StorageError { source: e })?;

        let data = match result {
            aspen_core::ReadResult { kv: Some(kv), .. } => kv.value.into_bytes(),
            _ => {
                return Err(crate::error::JobError::JobNotFound {
                    id: workflow_id.to_string(),
                });
            }
        };

        serde_json::from_slice(&data).map_err(|e| crate::error::JobError::SerializationError { source: e })
    }

    /// Cancel a workflow and all its active jobs.
    ///
    /// This method:
    /// 1. Fetches current workflow state
    /// 2. Cancels all active jobs via JobManager
    /// 3. Updates workflow state to Cancelled
    /// 4. Removes the cached definition
    ///
    /// Jobs that have already completed are not affected.
    pub async fn cancel_workflow(&self, workflow_id: &str) -> Result<Vec<JobId>> {
        let state = self.get_workflow_state(workflow_id).await?;
        let mut cancelled_jobs = Vec::new();

        // Cancel all active jobs
        for job_id in &state.active_jobs {
            match self.manager.cancel_job(job_id).await {
                Ok(()) => {
                    info!(workflow_id = %workflow_id, job_id = %job_id, "cancelled active job");
                    cancelled_jobs.push(job_id.clone());
                }
                Err(e) => {
                    // Log but don't fail - job may have completed in the meantime
                    warn!(
                        workflow_id = %workflow_id,
                        job_id = %job_id,
                        error = %e,
                        "failed to cancel job (may have already completed)"
                    );
                }
            }
        }

        // Update workflow state to cancelled
        self.update_workflow_state(workflow_id, |mut state| {
            let prev_state = state.state.clone();
            state.state = "cancelled".to_string();
            state.add_history(StateTransition {
                from: prev_state,
                to: "cancelled".to_string(),
                timestamp: chrono::Utc::now(),
                trigger: TransitionTrigger::Manual,
            });
            // Move all active jobs to failed (they were cancelled)
            for job_id in std::mem::take(&mut state.active_jobs) {
                state.failed_jobs.insert(job_id);
            }
            Ok(state)
        })
        .await?;

        // Remove the cached definition since workflow is now terminated
        self.definitions.write().await.remove(workflow_id);

        info!(
            workflow_id = %workflow_id,
            cancelled_count = cancelled_jobs.len(),
            "workflow cancelled"
        );

        Ok(cancelled_jobs)
    }

    /// Create a simple linear workflow.
    pub fn create_pipeline(name: &str, steps: Vec<(String, Vec<JobSpec>)>) -> WorkflowDefinition {
        let mut workflow_steps = HashMap::new();
        let mut last_step = None;

        for (i, (step_name, jobs)) in steps.iter().enumerate() {
            let is_last = i == steps.len() - 1;

            let transitions = if is_last {
                vec![] // Terminal state
            } else {
                vec![WorkflowTransition {
                    condition: TransitionCondition::AllSuccess,
                    target: steps[i + 1].0.clone(),
                }]
            };

            workflow_steps.insert(step_name.clone(), WorkflowStep {
                name: step_name.clone(),
                jobs: jobs.clone(),
                transitions,
                parallel: true,
                timeout: None,
                retry_on_failure: false,
            });

            if is_last {
                last_step = Some(step_name.clone());
            }
        }

        let terminal_states = last_step
            .map(|s| {
                let mut set = HashSet::new();
                set.insert(s);
                set
            })
            .unwrap_or_default();

        WorkflowDefinition {
            name: name.to_string(),
            initial_state: steps[0].0.clone(),
            steps: workflow_steps,
            terminal_states,
            timeout: None,
        }
    }

    /// Create a branching workflow with conditions.
    pub fn create_conditional_workflow(name: &str, initial: &str) -> WorkflowBuilder {
        WorkflowBuilder::new(name, initial)
    }
}

/// Builder for creating complex workflows.
pub struct WorkflowBuilder {
    definition: WorkflowDefinition,
}

impl WorkflowBuilder {
    /// Create a new workflow builder.
    pub fn new(name: &str, initial: &str) -> Self {
        Self {
            definition: WorkflowDefinition {
                name: name.to_string(),
                initial_state: initial.to_string(),
                steps: HashMap::new(),
                terminal_states: HashSet::new(),
                timeout: None,
            },
        }
    }

    /// Add a workflow step.
    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.definition.steps.insert(step.name.clone(), step);
        self
    }

    /// Add a terminal state.
    pub fn add_terminal(mut self, state: &str) -> Self {
        self.definition.terminal_states.insert(state.to_string());
        self
    }

    /// Set global timeout.
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.definition.timeout = Some(timeout);
        self
    }

    /// Build the workflow definition.
    pub fn build(self) -> WorkflowDefinition {
        self.definition
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_builder() {
        let workflow = WorkflowBuilder::new("test", "start")
            .add_step(WorkflowStep {
                name: "start".to_string(),
                jobs: vec![JobSpec::new("validate")],
                transitions: vec![
                    WorkflowTransition {
                        condition: TransitionCondition::AllSuccess,
                        target: "process".to_string(),
                    },
                    WorkflowTransition {
                        condition: TransitionCondition::AnyFailed,
                        target: "error".to_string(),
                    },
                ],
                parallel: false,
                timeout: None,
                retry_on_failure: true,
            })
            .add_step(WorkflowStep {
                name: "process".to_string(),
                jobs: vec![JobSpec::new("transform"), JobSpec::new("enrich")],
                transitions: vec![WorkflowTransition {
                    condition: TransitionCondition::AllSuccess,
                    target: "complete".to_string(),
                }],
                parallel: true,
                timeout: None,
                retry_on_failure: false,
            })
            .add_terminal("complete")
            .add_terminal("error")
            .build();

        assert_eq!(workflow.name, "test");
        assert_eq!(workflow.initial_state, "start");
        assert_eq!(workflow.steps.len(), 2);
        assert!(workflow.terminal_states.contains("complete"));
        assert!(workflow.terminal_states.contains("error"));
    }

    #[test]
    fn test_condition_evaluation() {
        let store = Arc::new(aspen_testing::DeterministicKeyValueStore::new());
        let job_manager = Arc::new(JobManager::new(Arc::clone(&store)));
        let manager = WorkflowManager::new(job_manager, store);

        let mut state = WorkflowState {
            id: "test".to_string(),
            state: "processing".to_string(),
            version: 1,
            data: serde_json::json!({}),
            active_jobs: HashSet::new(),
            completed_jobs: HashSet::new(),
            failed_jobs: HashSet::new(),
            history: vec![],
        };

        // Test AllSuccess with no jobs
        assert!(manager.evaluate_condition(&TransitionCondition::AllSuccess, &state));

        // Add completed job
        state.completed_jobs.insert(crate::job::JobId::from_string(uuid::Uuid::new_v4().to_string()));
        assert!(manager.evaluate_condition(&TransitionCondition::AllSuccess, &state));

        // Add failed job
        state.failed_jobs.insert(crate::job::JobId::from_string(uuid::Uuid::new_v4().to_string()));
        assert!(!manager.evaluate_condition(&TransitionCondition::AllSuccess, &state));
        assert!(manager.evaluate_condition(&TransitionCondition::AnyFailed, &state));

        // Test success rate
        state.completed_jobs.insert(crate::job::JobId::from_string(uuid::Uuid::new_v4().to_string()));
        state.completed_jobs.insert(crate::job::JobId::from_string(uuid::Uuid::new_v4().to_string()));
        // 3 completed, 1 failed = 75% success rate
        assert!(manager.evaluate_condition(&TransitionCondition::SuccessRate(0.7), &state));
        assert!(!manager.evaluate_condition(&TransitionCondition::SuccessRate(0.8), &state));
    }
}
