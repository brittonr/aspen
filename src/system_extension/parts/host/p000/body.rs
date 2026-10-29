const MAX_HOST_EVIDENCE_ITEMS: usize = 128;
const LIFECYCLE_CALLBACK_BYTES: u64 = 0;
const FIRST_SEQUENCE: u64 = 0;
const EXECUTOR_ERROR_DIAGNOSTIC: &str = "executor-error";
const OUTCOME_DENIED_DIAGNOSTIC: &str = "callback-outcome-denied";
const GENERATION_INCREMENT: u64 = 1;
const TRANSITION_EVIDENCE_ITEMS: usize = 2;
const CALLBACK_FAILURE_EVIDENCE_ITEMS: usize = 3;
const REPLACEMENT_EVIDENCE_ITEMS: usize = 7;

pub trait SystemExtensionExecutor {
    fn execution_profile(&self) -> super::ExecutionProfile;

    /// Classification of the latest executor error. An accepted, unresolved
    /// effect must return a non-retryable class rather than request a restart.
    fn failure_class(&self) -> super::FailureClass {
        super::FailureClass::Retryable
    }

    /// Invoke admitted code. The host passes only canonical callback context;
    /// execution profiles own any stronger isolation or process boundary.
    fn invoke(&mut self, invocation: &super::CallbackInvocation)
    -> std::result::Result<super::CallbackOutcome, String>;

    fn commit_admitted_outcome(
        &mut self,
        _invocation: &super::CallbackInvocation,
        _outcome: &super::CallbackOutcome,
    ) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub trait FabricEffectPort {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &super::TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<super::PortEffectOutput>;
}

impl<T: SystemExtensionExecutor + ?Sized> SystemExtensionExecutor for Box<T> {
    fn execution_profile(&self) -> super::ExecutionProfile {
        (**self).execution_profile()
    }

    fn failure_class(&self) -> super::FailureClass {
        (**self).failure_class()
    }

    fn invoke(
        &mut self,
        invocation: &super::CallbackInvocation,
    ) -> std::result::Result<super::CallbackOutcome, String> {
        (**self).invoke(invocation)
    }

    fn commit_admitted_outcome(
        &mut self,
        invocation: &super::CallbackInvocation,
        outcome: &super::CallbackOutcome,
    ) -> std::result::Result<(), String> {
        (**self).commit_admitted_outcome(invocation, outcome)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    Lifecycle(super::LifecycleReceipt),
    Callback(super::CallbackReceipt),
    EffectCompletion(super::EffectCompletion),
    Migration(super::StateMigrationReceipt),
    Readiness(super::ServiceReadiness),
}

impl Evidence {
    pub fn evidence_ref(&self) -> &str {
        match self {
            Self::Lifecycle(receipt) => &receipt.receipt_ref,
            Self::Callback(receipt) => &receipt.receipt_ref,
            Self::EffectCompletion(receipt) => &receipt.completion_ref,
            Self::Migration(receipt) => &receipt.receipt_ref,
            Self::Readiness(receipt) => &receipt.readiness_ref,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchResult {
    Executed {
        receipt: super::CallbackReceipt,
        outcome: super::CallbackOutcome,
        approved_effects: Vec<super::TypedEffectRequest>,
    },
    Deferred {
        decision: super::AdmissionDecision,
        usage: super::ResourceUsage,
    },
    Failed {
        receipt: super::CallbackReceipt,
        lifecycle: super::LifecycleReceipt,
    },
}

impl DispatchResult {
    pub fn require_executed(
        self,
        label: &str,
    ) -> crate::error::Result<(super::CallbackReceipt, super::CallbackOutcome)> {
        match self {
            Self::Executed { receipt, outcome, .. } => Ok((receipt, outcome)),
            Self::Deferred { decision, .. } => Err(crate::error::Failure::invalid_harness(format!(
                "system-extension {label} callback was deferred: {decision:?}"
            ))),
            Self::Failed { receipt, lifecycle } => Err(crate::error::Failure::invalid_harness(format!(
                "system-extension {label} callback failed: callback={} lifecycle={}",
                receipt.receipt_ref, lifecycle.receipt_ref
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationArtifacts {
    pub lifecycle_receipts: Vec<super::LifecycleReceipt>,
    pub callback_receipts: Vec<super::CallbackReceipt>,
    pub status: super::OperatorStatusRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationReplacementArtifacts {
    pub begin: super::LifecycleReceipt,
    pub migration: super::StateMigrationReceipt,
    pub callback: super::CallbackReceipt,
    pub completion: super::LifecycleReceipt,
    pub status: super::OperatorStatusRecord,
}

pub struct Dispatcher<E: SystemExtensionExecutor> {
    admitted: super::AdmittedManifest,
    executor: E,
    state: super::LifecycleState,
    usage: super::ResourceUsage,
    invocation_sequence: u64,
    event_sequence: u64,
    semantic_state_ref: Option<String>,
    observations: std::collections::BTreeMap<super::CallbackKind, u64>,
    execution_binding_refs: Vec<String>,
    evidence: Vec<Evidence>,
    last_lifecycle_ref: Option<String>,
}

pub struct RecoveredState {
    pub state: super::LifecycleState,
    pub usage: super::ResourceUsage,
    pub invocation_sequence: u64,
    pub event_sequence: u64,
    pub semantic_state_ref: Option<String>,
    pub last_lifecycle_ref: Option<String>,
}

struct InvocationFailureInput<'a> {
    invocation: &'a super::CallbackInvocation,
    accounted_bytes: u64,
    decision: super::CallbackExecutionDecision,
    diagnostic: &'static str,
    outcome: Option<&'a super::CallbackOutcome>,
    failure_class: super::FailureClass,
}
