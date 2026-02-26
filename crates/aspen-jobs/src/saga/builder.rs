//! Builder pattern for constructing saga definitions.

use std::collections::HashMap;
use std::time::Duration;

use super::types::MAX_SAGA_STEPS;
use super::types::SagaDefinition;
use super::types::SagaStep;

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
            was_executed: false,
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
