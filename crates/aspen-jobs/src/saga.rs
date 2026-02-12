//! Saga pattern implementation for distributed transactions.
//!
//! This module provides a saga/compensation pattern for managing distributed
//! transactions where multiple steps may need to be rolled back if any step fails.
//! The pattern follows LIFO (last-in-first-out) compensation order.
//!
//! # Example
//!
//! ```ignore
//! use aspen_jobs::{SagaBuilder, SagaExecutor};
//!
//! let saga = SagaBuilder::new("order_saga")
//!     .step("reserve_inventory")
//!         .done()
//!     .step("charge_payment")
//!         .done()
//!     .step("ship_order")
//!         .done()
//!     .build();
//!
//! let executor = SagaExecutor::new(store);
//! let state = executor.start_saga(saga, None).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::warn;
use uuid::Uuid;

use crate::error::JobError;
use crate::error::Result;

/// Maximum number of saga steps allowed.
const MAX_SAGA_STEPS: usize = 100;

/// Maximum number of compensation retries.
const MAX_COMPENSATION_RETRIES: u32 = 5;

/// Base delay for compensation retry backoff.
const COMPENSATION_RETRY_BASE_MS: u64 = 100;

/// Key prefix for saga state persistence.
const SAGA_STATE_PREFIX: &str = "__saga_state::";

/// Maximum number of sagas to list.
const MAX_SAGA_LIST_LIMIT: u32 = 1000;

/// Result of a compensation operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompensationResult {
    /// Compensation succeeded.
    Success,
    /// Compensation failed with an error message.
    Failed(String),
    /// Compensation was skipped (step didn't execute).
    Skipped,
    /// Compensation is pending retry.
    PendingRetry {
        /// Number of attempts made.
        attempts: u32,
        /// Last error message.
        last_error: String,
    },
}

/// State of a saga execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SagaState {
    /// Saga has not started.
    NotStarted,
    /// Saga is executing forward steps.
    Executing {
        /// Index of the current step being executed.
        current_step: usize,
    },
    /// Saga completed successfully.
    Completed,
    /// Saga is compensating (rolling back).
    Compensating {
        /// Index of the step that failed.
        failed_step: usize,
        /// Error that caused the failure.
        failure_reason: String,
        /// Index of the current compensation step.
        current_compensation: usize,
    },
    /// Saga compensation completed.
    CompensationCompleted {
        /// Original failure reason.
        failure_reason: String,
        /// Results of each compensation.
        compensation_results: Vec<CompensationResult>,
    },
    /// Saga compensation failed (poison state).
    CompensationFailed {
        /// Original failure reason.
        failure_reason: String,
        /// Step that failed compensation.
        failed_compensation_step: usize,
        /// Compensation error.
        compensation_error: String,
    },
}

/// Definition of a single saga step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    /// Unique name for this step.
    pub name: String,
    /// Whether this step has been executed.
    pub executed: bool,
    /// Result of the action (if executed).
    pub action_result: Option<StepResult>,
    /// Result of compensation (if compensated).
    pub compensation_result: Option<CompensationResult>,
    /// Timeout for the action.
    pub timeout: Option<Duration>,
    /// Whether compensation is required for this step.
    pub requires_compensation: bool,
}

/// Result of executing a step action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepResult {
    /// Step succeeded with optional output.
    Success(Option<String>),
    /// Step failed with error.
    Failed(String),
}

/// Complete saga definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaDefinition {
    /// Unique identifier for this saga type.
    pub saga_type: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Ordered list of steps.
    pub steps: Vec<SagaStep>,
    /// Maximum time for the entire saga.
    pub timeout: Option<Duration>,
    /// Metadata for the saga.
    pub metadata: HashMap<String, String>,
}

impl SagaDefinition {
    /// Create a new saga definition.
    pub fn new(saga_type: impl Into<String>) -> Self {
        Self {
            saga_type: saga_type.into(),
            description: None,
            steps: Vec::new(),
            timeout: None,
            metadata: HashMap::new(),
        }
    }

    /// Get the number of steps in the saga.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Get a step by index.
    pub fn get_step(&self, index: usize) -> Option<&SagaStep> {
        self.steps.get(index)
    }

    /// Get a mutable step by index.
    pub fn get_step_mut(&mut self, index: usize) -> Option<&mut SagaStep> {
        self.steps.get_mut(index)
    }
}

/// Builder for constructing saga definitions.
pub struct SagaBuilder {
    saga_type: String,
    description: Option<String>,
    steps: Vec<SagaStep>,
    timeout: Option<Duration>,
    metadata: HashMap<String, String>,
}

impl SagaBuilder {
    /// Create a new saga builder.
    pub fn new(saga_type: impl Into<String>) -> Self {
        Self {
            saga_type: saga_type.into(),
            description: None,
            steps: Vec::new(),
            timeout: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the saga description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the saga timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add metadata to the saga.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add a step to the saga.
    pub fn step(self, name: impl Into<String>) -> StepBuilder {
        StepBuilder {
            saga_builder: self,
            name: name.into(),
            timeout: None,
            requires_compensation: true,
        }
    }

    /// Add a pre-built step to the saga.
    pub fn add_step(mut self, step: SagaStep) -> Self {
        if self.steps.len() < MAX_SAGA_STEPS {
            self.steps.push(step);
        }
        self
    }

    /// Build the saga definition.
    pub fn build(self) -> SagaDefinition {
        SagaDefinition {
            saga_type: self.saga_type,
            description: self.description,
            steps: self.steps,
            timeout: self.timeout,
            metadata: self.metadata,
        }
    }
}

/// Builder for saga steps.
pub struct StepBuilder {
    saga_builder: SagaBuilder,
    name: String,
    timeout: Option<Duration>,
    requires_compensation: bool,
}

impl StepBuilder {
    /// Set the step timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Mark this step as not requiring compensation.
    pub fn no_compensation(mut self) -> Self {
        self.requires_compensation = false;
        self
    }

    /// Complete the step and return to the saga builder.
    pub fn done(mut self) -> SagaBuilder {
        let step = SagaStep {
            name: self.name,
            executed: false,
            action_result: None,
            compensation_result: None,
            timeout: self.timeout,
            requires_compensation: self.requires_compensation,
        };

        if self.saga_builder.steps.len() < MAX_SAGA_STEPS {
            self.saga_builder.steps.push(step);
        }

        self.saga_builder
    }
}

/// Unique identifier for a saga execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SagaExecutionId(String);

impl SagaExecutionId {
    /// Create a new saga execution ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from an existing string.
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Default for SagaExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SagaExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Persistent saga state for recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentSagaState {
    /// Saga execution ID.
    pub execution_id: SagaExecutionId,
    /// The saga definition.
    pub definition: SagaDefinition,
    /// Current state.
    pub state: SagaState,
    /// When the saga started.
    pub started_at: SystemTime,
    /// When the saga was last updated.
    pub updated_at: SystemTime,
    /// Input context (serialized).
    pub input: Option<Vec<u8>>,
}

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
                let state: PersistentSagaState = serde_json::from_str(&kv.value)?;
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
        step_index: usize,
        output: Option<String>,
    ) -> Result<()> {
        if let Some(step) = state.definition.get_step_mut(step_index) {
            step.executed = true;
            step.action_result = Some(StepResult::Success(output));
        }

        // Move to next step or complete
        let next_step = step_index + 1;
        if next_step >= state.definition.step_count() {
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
    pub async fn fail_step(&self, state: &mut PersistentSagaState, step_index: usize, error: String) -> Result<()> {
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
        step_index: usize,
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
                if step.executed && step.requires_compensation {
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
        step_index: usize,
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

/// Action to take for a saga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SagaAction {
    /// Execute a forward step.
    Execute {
        /// Index of the step to execute.
        step_index: usize,
    },
    /// Execute compensation for a step.
    Compensate {
        /// Index of the step to compensate.
        step_index: usize,
    },
    /// Skip compensation for a step that wasn't executed.
    SkipCompensation {
        /// Index of the step to skip.
        step_index: usize,
    },
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_saga_builder() {
        let saga = SagaBuilder::new("test_saga")
            .description("A test saga")
            .timeout(Duration::from_secs(60))
            .metadata("version", "1.0")
            .step("step1")
            .timeout(Duration::from_secs(10))
            .done()
            .step("step2")
            .no_compensation()
            .done()
            .step("step3")
            .done()
            .build();

        assert_eq!(saga.saga_type, "test_saga");
        assert_eq!(saga.description, Some("A test saga".to_string()));
        assert_eq!(saga.step_count(), 3);

        let step1 = saga.get_step(0).unwrap();
        assert_eq!(step1.name, "step1");
        assert!(step1.requires_compensation);
        assert!(step1.timeout.is_some());

        let step2 = saga.get_step(1).unwrap();
        assert_eq!(step2.name, "step2");
        assert!(!step2.requires_compensation);

        let step3 = saga.get_step(2).unwrap();
        assert_eq!(step3.name, "step3");
        assert!(step3.requires_compensation);
    }

    #[tokio::test]
    async fn test_saga_state_persistence() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("persist_test").step("step1").done().build();

        // Start saga
        let state = executor.start_saga(saga, None).await.unwrap();
        assert!(matches!(state.state, SagaState::Executing { current_step: 0 }));

        // Load it back
        let loaded = executor.load_state(&state.execution_id).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.execution_id, state.execution_id);
    }

    #[tokio::test]
    async fn test_saga_forward_execution() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("forward_test").step("step1").done().step("step2").done().build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Complete step 1
        executor.complete_step(&mut state, 0, Some("output1".to_string())).await.unwrap();
        assert!(matches!(state.state, SagaState::Executing { current_step: 1 }));

        // Complete step 2
        executor.complete_step(&mut state, 1, Some("output2".to_string())).await.unwrap();
        assert!(matches!(state.state, SagaState::Completed));
    }

    #[tokio::test]
    async fn test_saga_compensation() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("compensation_test")
            .step("step1")
            .done()
            .step("step2")
            .done()
            .step("step3")
            .done()
            .build();

        let mut state = executor.start_saga(saga, None).await.unwrap();

        // Complete steps 1 and 2
        executor.complete_step(&mut state, 0, None).await.unwrap();
        executor.complete_step(&mut state, 1, None).await.unwrap();

        // Fail step 3
        executor.fail_step(&mut state, 2, "step3 failed".to_string()).await.unwrap();

        // Should now be compensating, starting from step 2 (index 1)
        assert!(matches!(state.state, SagaState::Compensating {
            failed_step: 2,
            current_compensation: 1,
            ..
        }));

        // Complete compensation for step 2
        executor.complete_compensation(&mut state, 1, CompensationResult::Success).await.unwrap();

        // Should now be compensating step 1
        assert!(matches!(state.state, SagaState::Compensating {
            current_compensation: 0,
            ..
        }));

        // Complete compensation for step 1
        executor.complete_compensation(&mut state, 0, CompensationResult::Success).await.unwrap();

        // Should now be fully compensated
        assert!(matches!(state.state, SagaState::CompensationCompleted { .. }));
    }

    #[tokio::test]
    async fn test_get_next_action() {
        let store = Arc::new(DeterministicKeyValueStore::default());
        let executor = SagaExecutor::new(store);

        let saga = SagaBuilder::new("action_test").step("step1").done().step("step2").done().build();

        let state = executor.start_saga(saga, None).await.unwrap();

        // Initially should execute step 0
        let action = executor.get_next_action(&state);
        assert_eq!(action, Some(SagaAction::Execute { step_index: 0 }));
    }
}
