//! Core types for the saga pattern implementation.

use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Maximum number of saga steps allowed.
pub const MAX_SAGA_STEPS: usize = 100;

/// Maximum number of compensation retries.
pub const MAX_COMPENSATION_RETRIES: u32 = 5;

/// Base delay for compensation retry backoff.
pub const COMPENSATION_RETRY_BASE_MS: u64 = 100;

/// Key prefix for saga state persistence.
pub const SAGA_STATE_PREFIX: &str = "__saga_state::";

/// Maximum number of sagas to list.
pub const MAX_SAGA_LIST_LIMIT: u32 = 1000;

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
        /// Index of the current step being executed (bounded by MAX_SAGA_STEPS).
        current_step: u32,
    },
    /// Saga completed successfully.
    Completed,
    /// Saga is compensating (rolling back).
    Compensating {
        /// Index of the step that failed (bounded by MAX_SAGA_STEPS).
        failed_step: u32,
        /// Error that caused the failure.
        failure_reason: String,
        /// Index of the current compensation step (bounded by MAX_SAGA_STEPS).
        current_compensation: u32,
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
        /// Step that failed compensation (bounded by MAX_SAGA_STEPS).
        failed_compensation_step: u32,
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
    pub was_executed: bool,
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
    pub fn get_step(&self, index: u32) -> Option<&SagaStep> {
        self.steps.get(index as usize)
    }

    /// Get a mutable step by index.
    pub fn get_step_mut(&mut self, index: u32) -> Option<&mut SagaStep> {
        self.steps.get_mut(index as usize)
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

/// Action to take for a saga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SagaAction {
    /// Execute a forward step.
    Execute {
        /// Index of the step to execute (bounded by MAX_SAGA_STEPS).
        step_index: u32,
    },
    /// Execute compensation for a step.
    Compensate {
        /// Index of the step to compensate (bounded by MAX_SAGA_STEPS).
        step_index: u32,
    },
    /// Skip compensation for a step that wasn't executed.
    SkipCompensation {
        /// Index of the step to skip (bounded by MAX_SAGA_STEPS).
        step_index: u32,
    },
}
