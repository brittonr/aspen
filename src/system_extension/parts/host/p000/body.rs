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
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &super::TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<super::PortEffectOutput>;
}

impl<T: SystemExtensionExecutor + ?Sized> SystemExtensionExecutor for Box<T> {
    fn execution_profile(&self) -> super::ExecutionProfile {
        (**self).execution_profile()
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
pub enum HostEvidence {
    Lifecycle(super::CanonicalLifecycleReceipt),
    Callback(super::CanonicalCallbackReceipt),
    EffectCompletion(super::CanonicalEffectCompletion),
    Migration(super::CanonicalStateMigrationReceipt),
    Readiness(super::CanonicalServiceReadiness),
}

impl HostEvidence {
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
pub enum HostDispatchResult {
    Executed {
        receipt: super::CanonicalCallbackReceipt,
        outcome: super::CallbackOutcome,
        approved_effects: Vec<super::TypedEffectRequest>,
    },
    Deferred {
        decision: super::AdmissionDecision,
        usage: super::ResourceUsage,
    },
    Failed {
        receipt: super::CanonicalCallbackReceipt,
        lifecycle: super::CanonicalLifecycleReceipt,
    },
}

impl HostDispatchResult {
    pub fn require_executed(
        self,
        label: &str,
    ) -> crate::error::Result<(super::CanonicalCallbackReceipt, super::CallbackOutcome)> {
        match self {
            Self::Executed { receipt, outcome, .. } => Ok((receipt, outcome)),
            Self::Deferred { decision, .. } => Err(crate::error::MoltenError::invalid_harness(format!(
                "system-extension {label} callback was deferred: {decision:?}"
            ))),
            Self::Failed { receipt, lifecycle } => Err(crate::error::MoltenError::invalid_harness(format!(
                "system-extension {label} callback failed: callback={} lifecycle={}",
                receipt.receipt_ref, lifecycle.receipt_ref
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationArtifacts {
    pub lifecycle_receipts: Vec<super::CanonicalLifecycleReceipt>,
    pub callback_receipts: Vec<super::CanonicalCallbackReceipt>,
    pub status: super::CanonicalOperatorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationReplacementArtifacts {
    pub begin: super::CanonicalLifecycleReceipt,
    pub migration: super::CanonicalStateMigrationReceipt,
    pub callback: super::CanonicalCallbackReceipt,
    pub completion: super::CanonicalLifecycleReceipt,
    pub status: super::CanonicalOperatorStatus,
}

pub struct SystemExtensionHost<E: SystemExtensionExecutor> {
    admitted: super::CanonicalAdmittedSystemExtensionManifest,
    executor: E,
    state: super::LifecycleState,
    usage: super::ResourceUsage,
    invocation_sequence: u64,
    event_sequence: u64,
    semantic_state_ref: Option<String>,
    observations: std::collections::BTreeMap<super::CallbackKind, u64>,
    execution_binding_refs: Vec<String>,
    evidence: Vec<HostEvidence>,
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
