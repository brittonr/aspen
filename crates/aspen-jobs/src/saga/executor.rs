//! Saga executor for managing saga transactions.

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;
use tracing::warn;

use super::types::COMPENSATION_RETRY_BASE_MS;
use super::types::CompensationResult;
use super::types::MAX_COMPENSATION_RETRIES;
use super::types::MAX_SAGA_LIST_LIMIT;
use super::types::PersistentSagaState;
use super::types::SAGA_STATE_PREFIX;
use super::types::SagaAction;
use super::types::SagaDefinition;
use super::types::SagaExecutionId;
use super::types::SagaState;
use super::types::StepResult;
use crate::error::JobError;
use crate::error::Result;

/// Executor for saga transactions.
///
/// The `SagaExecutor` manages the execution of saga definitions, handling
/// step execution, failure detection, and compensation rollback.
pub struct SagaExecutor<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + Send + Sync + 'static> SagaExecutor<S> {
    /// Create a new saga executor.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// Generate the storage key for a saga.
    fn storage_key(execution_id: &SagaExecutionId) -> String {
        format!("{}{}", SAGA_STATE_PREFIX, execution_id)
    }

    /// Persist the current saga state.
    pub async fn persist_state(&self, state: &PersistentSagaState) -> Result<()> {
        let key = Self::storage_key(&state.execution_id);
        let value = serde_json::to_string(state)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(JobError::from)?;

        debug!(
            execution_id = %state.execution_id,
            saga_type = %state.definition.saga_type,
            state = ?std::mem::discriminant(&state.state),
            "saga state persisted"
        );

        Ok(())
    }

    /// Load saga state by execution ID.
    pub async fn load_state(&self, execution_id: &SagaExecutionId) -> Result<Option<PersistentSagaState>> {
        let key = Self::storage_key(execution_id);

        let response = self.store.read(ReadRequest::new(key)).await.map_err(JobError::from)?;

        match response.kv {
            Some(kv) => {
                let state: PersistentSagaState =
                    serde_json::from_str(&kv.value).map_err(|e| JobError::ExecutionFailed {
                        reason: format!("failed to parse saga state for '{}': {}", execution_id, e),
                    })?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// Delete saga state after completion.
    pub async fn delete_state(&self, execution_id: &SagaExecutionId) -> Result<()> {
        let key = Self::storage_key(execution_id);

        self.store
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await
            .map_err(JobError::from)?;

        Ok(())
    }

    /// Start a new saga execution.
    pub async fn start_saga(&self, definition: SagaDefinition, input: Option<Vec<u8>>) -> Result<PersistentSagaState> {
        let execution_id = SagaExecutionId::new();
        let now = SystemTime::now();

        let state = PersistentSagaState {
            execution_id,
            definition,
            state: SagaState::Executing { current_step: 0 },
            started_at: now,
            updated_at: now,
            input,
        };

        self.persist_state(&state).await?;

        Ok(state)
    }

    /// Mark a step as completed successfully.
    pub async fn complete_step(
        &self,
        state: &mut PersistentSagaState,
        step_index: u32,
        output: Option<String>,
    ) -> Result<()> {
        if let Some(step) = state.definition.get_step_mut(step_index) {
            step.was_executed = true;
            step.action_result = Some(StepResult::Success(output));
        }

        // Move to next step or complete
        let next_step = step_index + 1;
        if next_step as usize >= state.definition.step_count() {
            state.state = SagaState::Completed;
        } else {
            state.state = SagaState::Executing {
                current_step: next_step,
            };
        }

        state.updated_at = SystemTime::now();
        self.persist_state(state).await?;

        debug!(
            execution_id = %state.execution_id,
            step_index,
            step_name = ?state.definition.get_step(step_index).map(|s| &s.name),
            "saga step completed"
        );

        Ok(())
    }

    /// Mark a step as failed and begin compensation.
    pub async fn fail_step(&self, state: &mut PersistentSagaState, step_index: u32, error: String) -> Result<()> {
        if let Some(step) = state.definition.get_step_mut(step_index) {
            step.action_result = Some(StepResult::Failed(error.clone()));
        }

        // Start compensating from the previous step (LIFO order)
        let compensation_start = if step_index > 0 { step_index - 1 } else { 0 };

        // Only enter compensation if there's actually a step to compensate
        if step_index > 0 {
            state.state = SagaState::Compensating {
                failed_step: step_index,
                failure_reason: error.clone(),
                current_compensation: compensation_start,
            };
        } else {
            // First step failed, no compensation needed
            state.state = SagaState::CompensationCompleted {
                failure_reason: error.clone(),
                compensation_results: vec![],
            };
        }

        state.updated_at = SystemTime::now();
        self.persist_state(state).await?;

        warn!(
            execution_id = %state.execution_id,
            step_index,
            error = %error,
            "saga step failed, beginning compensation"
        );

        Ok(())
    }

    /// Complete a compensation step.
    pub async fn complete_compensation(
        &self,
        state: &mut PersistentSagaState,
        step_index: u32,
        result: CompensationResult,
    ) -> Result<()> {
        if let Some(step) = state.definition.get_step_mut(step_index) {
            step.compensation_result = Some(result.clone());
        }

        // Check for compensation failure
        if let CompensationResult::Failed(comp_error) = &result {
            if let SagaState::Compensating { failure_reason, .. } = &state.state {
                state.state = SagaState::CompensationFailed {
                    failure_reason: failure_reason.clone(),
                    failed_compensation_step: step_index,
                    compensation_error: comp_error.clone(),
                };
                state.updated_at = SystemTime::now();
                self.persist_state(state).await?;

                warn!(
                    execution_id = %state.execution_id,
                    step_index,
                    error = %comp_error,
                    "saga compensation failed"
                );

                return Ok(());
            }
        }

        // Move to next compensation or complete
        if let SagaState::Compensating {
            failed_step,
            failure_reason,
            current_compensation,
        } = &state.state
        {
            if *current_compensation == 0 {
                // All compensations done - collect results
                let mut results = Vec::new();
                for step in &state.definition.steps {
                    results.push(step.compensation_result.clone().unwrap_or(CompensationResult::Skipped));
                }

                state.state = SagaState::CompensationCompleted {
                    failure_reason: failure_reason.clone(),
                    compensation_results: results,
                };

                debug!(
                    execution_id = %state.execution_id,
                    "saga compensation completed"
                );
            } else {
                state.state = SagaState::Compensating {
                    failed_step: *failed_step,
                    failure_reason: failure_reason.clone(),
                    current_compensation: current_compensation - 1,
                };
            }
        }

        state.updated_at = SystemTime::now();
        self.persist_state(state).await?;

        Ok(())
    }

    /// Get the next step to execute (either forward or compensation).
    pub fn get_next_action(&self, state: &PersistentSagaState) -> Option<SagaAction> {
        match &state.state {
            SagaState::NotStarted => Some(SagaAction::Execute { step_index: 0 }),
            SagaState::Executing { current_step } => Some(SagaAction::Execute {
                step_index: *current_step,
            }),
            SagaState::Compensating {
                current_compensation, ..
            } => {
                let step = state.definition.get_step(*current_compensation)?;
                // Decomposed: check if step was executed, then check if it needs compensation
                let was_executed = step.was_executed;
                let needs_compensation = step.requires_compensation;
                if was_executed && needs_compensation {
                    Some(SagaAction::Compensate {
                        step_index: *current_compensation,
                    })
                } else {
                    // Skip compensation for unexecuted or no-compensation steps
                    Some(SagaAction::SkipCompensation {
                        step_index: *current_compensation,
                    })
                }
            }
            SagaState::Completed | SagaState::CompensationCompleted { .. } | SagaState::CompensationFailed { .. } => {
                None
            }
        }
    }

    /// Retry a failed compensation with exponential backoff.
    pub async fn retry_compensation(
        &self,
        state: &mut PersistentSagaState,
        step_index: u32,
        error: String,
    ) -> Result<bool> {
        let step = state.definition.get_step_mut(step_index).ok_or_else(|| JobError::NotFound {
            resource: format!("saga step {}", step_index),
        })?;

        let current_attempts = match &step.compensation_result {
            Some(CompensationResult::PendingRetry { attempts, .. }) => *attempts,
            _ => 0,
        };

        if current_attempts >= MAX_COMPENSATION_RETRIES {
            step.compensation_result = Some(CompensationResult::Failed(error));
            self.persist_state(state).await?;
            return Ok(false);
        }

        step.compensation_result = Some(CompensationResult::PendingRetry {
            attempts: current_attempts + 1,
            last_error: error,
        });

        state.updated_at = SystemTime::now();
        self.persist_state(state).await?;

        // Calculate backoff delay
        let delay_ms = COMPENSATION_RETRY_BASE_MS * 2u64.pow(current_attempts);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        Ok(true)
    }

    /// List all active sagas (for monitoring/recovery).
    pub async fn list_active_sagas(&self) -> Result<Vec<PersistentSagaState>> {
        let response = self
            .store
            .scan(ScanRequest {
                prefix: SAGA_STATE_PREFIX.to_string(),
                limit: Some(MAX_SAGA_LIST_LIMIT),
                continuation_token: None,
            })
            .await
            .map_err(JobError::from)?;

        let mut sagas = Vec::new();

        for kv in response.entries {
            if let Ok(state) = serde_json::from_str::<PersistentSagaState>(&kv.value) {
                // Only include active sagas
                match &state.state {
                    SagaState::Completed | SagaState::CompensationCompleted { .. } => continue,
                    _ => sagas.push(state),
                }
            }
        }

        Ok(sagas)
    }
}
