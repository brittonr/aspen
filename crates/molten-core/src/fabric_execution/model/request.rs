use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnvironmentEntry {
    pub name: String,
    pub value: String,
    pub value_class: EnvironmentValueClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionRequestLimits {
    pub timeout_ms: u64,
    pub stdin_max_bytes: u64,
    pub stdout_max_bytes: u64,
    pub stderr_max_bytes: u64,
    pub poll_interval_ms: u64,
    pub teardown_timeout_ms: u64,
    pub concurrency_units: u64,
    pub queue_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub schema: String,
    pub operation_ref: String,
    pub idempotency_ref: String,
    pub extension_id: String,
    pub service_id: String,
    pub callback_ref: String,
    pub effect_ref: String,
    pub generation: u64,
    pub profile_ref: String,
    pub executable_artifact_ref: String,
    pub executable_identity_ref: String,
    pub arguments: Vec<String>,
    pub environment: Vec<EnvironmentEntry>,
    pub environment_mode: ExecutionEnvironmentMode,
    pub invocation_mode: ExecutionInvocationMode,
    pub executable_resolution: ExecutableResolutionMode,
    pub workspace_ref: String,
    pub workspace_mode: WorkspaceMode,
    pub stdin_ref: Option<String>,
    pub limits: ExecutionRequestLimits,
    pub termination_scope: ExecutionTerminationScope,
    pub accepted_exit_codes: Vec<i32>,
    pub reject_stdout_truncation: bool,
    pub reject_stderr_truncation: bool,
    pub authority_ref: String,
    pub resource_grant_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionAuthorityFacts {
    pub authority_ref: String,
    pub executable_authority_ref: String,
    pub provenance_ref: String,
    pub effect_admission_ref: String,
    pub workspace_authority_ref: String,
    pub process_authority_ref: String,
    pub resource_grant_ref: String,
    pub policy_ref: String,
    pub executable_artifact_ref: String,
    pub executable_identity_ref: String,
    pub workspace_ref: String,
    pub operation_ref: String,
    pub extension_id: String,
    pub service_id: String,
    pub generation: u64,
    pub profile_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResourceGrant {
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub diagnostic_bytes: u64,
    pub logical_deadline_ticks: u64,
    pub concurrency_units: u64,
    pub queue_units: u64,
}

// r[impl molten.fabric_execution.output]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityResolutionPlan {
    pub executable_artifact_ref: String,
    pub executable_identity_ref: String,
    pub workspace_ref: String,
    pub stdin_ref: Option<String>,
    pub stdout_role: String,
    pub stderr_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedExecutionPlan {
    pub profile: AdmittedExecutionProfile,
    pub request: ExecutionRequest,
    pub authority: ExecutionAuthorityFacts,
    pub resources: ExecutionResourceGrant,
    pub resolution: CapabilityResolutionPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionIdentity {
    pub extension_id: String,
    pub service_id: String,
    pub generation: u64,
    pub callback_ref: String,
    pub effect_ref: String,
    pub operation_ref: String,
    pub executable_identity_ref: String,
    pub profile_ref: String,
    pub idempotency_ref: String,
}

impl ExecutionRequest {
    #[must_use]
    pub fn identity(&self) -> ExecutionIdentity {
        ExecutionIdentity {
            extension_id: self.extension_id.clone(),
            service_id: self.service_id.clone(),
            generation: self.generation,
            callback_ref: self.callback_ref.clone(),
            effect_ref: self.effect_ref.clone(),
            operation_ref: self.operation_ref.clone(),
            executable_identity_ref: self.executable_identity_ref.clone(),
            profile_ref: self.profile_ref.clone(),
            idempotency_ref: self.idempotency_ref.clone(),
        }
    }
}
