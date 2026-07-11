use std::collections::BTreeMap;

use super::AdmissionDecision;
use super::CallbackEvent;
use super::CallbackExecutionDecision;
use super::CallbackInvocation;
use super::CallbackKind;
use super::CallbackObservation;
use super::CallbackOutcome;
use super::CanonicalAdmittedSystemExtensionManifest;
use super::CanonicalCallbackReceipt;
use super::CanonicalEffectCompletion;
use super::CanonicalLifecycleReceipt;
use super::CanonicalOperatorStatus;
use super::CanonicalServiceReadiness;
use super::CanonicalStateMigrationReceipt;
use super::EffectTarget;
use super::ExecutableConformanceInput;
use super::ExecutionProfile;
use super::FailureClass;
use super::LifecycleEvent;
use super::LifecycleEventKind;
use super::LifecycleState;
use super::MigrationOperation;
use super::OperatorStatus;
use super::PortEffectOutput;
use super::ResourceUsage;
use super::TypedEffectRequest;
use super::callback_event_value;
use super::canonical_callback_receipt;
use super::canonical_effect_completion;
use super::canonical_lifecycle_receipt;
use super::canonical_operator_status;
use super::canonical_service_readiness;
use super::canonical_state_migration_receipt;
use super::plan_callback_dispatch;
use super::plan_lifecycle_transition;
use super::plan_state_migration;
use super::release_callback_resources;
use super::release_effect_requests;
use super::reserve_effect_requests;
use super::validate_callback_outcome;
use super::validate_typed_effects;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_hash;

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
    fn execution_profile(&self) -> ExecutionProfile;

    /// Invoke admitted code. The host passes only canonical callback context;
    /// execution profiles own any stronger isolation or process boundary.
    fn invoke(&mut self, invocation: &CallbackInvocation) -> std::result::Result<CallbackOutcome, String>;
}

pub trait FabricEffectPort {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &TypedEffectRequest,
    ) -> std::result::Result<PortEffectOutput, String>;
}

impl<T: SystemExtensionExecutor + ?Sized> SystemExtensionExecutor for Box<T> {
    fn execution_profile(&self) -> ExecutionProfile {
        (**self).execution_profile()
    }

    fn invoke(&mut self, invocation: &CallbackInvocation) -> std::result::Result<CallbackOutcome, String> {
        (**self).invoke(invocation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEvidence {
    Lifecycle(CanonicalLifecycleReceipt),
    Callback(CanonicalCallbackReceipt),
    EffectCompletion(CanonicalEffectCompletion),
    Migration(CanonicalStateMigrationReceipt),
    Readiness(CanonicalServiceReadiness),
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
        receipt: CanonicalCallbackReceipt,
        outcome: CallbackOutcome,
        approved_effects: Vec<TypedEffectRequest>,
    },
    Deferred {
        decision: AdmissionDecision,
        usage: ResourceUsage,
    },
    Failed {
        receipt: CanonicalCallbackReceipt,
        lifecycle: CanonicalLifecycleReceipt,
    },
}

impl HostDispatchResult {
    pub fn require_executed(self, label: &str) -> Result<(CanonicalCallbackReceipt, CallbackOutcome)> {
        match self {
            Self::Executed { receipt, outcome, .. } => Ok((receipt, outcome)),
            Self::Deferred { decision, .. } => Err(MoltenError::invalid_harness(format!(
                "system-extension {label} callback was deferred: {decision:?}"
            ))),
            Self::Failed { receipt, lifecycle } => Err(MoltenError::invalid_harness(format!(
                "system-extension {label} callback failed: callback={} lifecycle={}",
                receipt.receipt_ref, lifecycle.receipt_ref
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationArtifacts {
    pub lifecycle_receipts: Vec<CanonicalLifecycleReceipt>,
    pub callback_receipts: Vec<CanonicalCallbackReceipt>,
    pub status: CanonicalOperatorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationReplacementArtifacts {
    pub begin: CanonicalLifecycleReceipt,
    pub migration: CanonicalStateMigrationReceipt,
    pub callback: CanonicalCallbackReceipt,
    pub completion: CanonicalLifecycleReceipt,
    pub status: CanonicalOperatorStatus,
}

pub struct SystemExtensionHost<E: SystemExtensionExecutor> {
    admitted: CanonicalAdmittedSystemExtensionManifest,
    executor: E,
    state: LifecycleState,
    usage: ResourceUsage,
    invocation_sequence: u64,
    event_sequence: u64,
    observations: BTreeMap<CallbackKind, u64>,
    execution_binding_refs: Vec<String>,
    evidence: Vec<HostEvidence>,
    last_lifecycle_ref: Option<String>,
}

impl<E: SystemExtensionExecutor> SystemExtensionHost<E> {
    // r[impl molten.system_extension.execution_profiles]
    pub fn new(admitted: CanonicalAdmittedSystemExtensionManifest, executor: E) -> Result<Self> {
        let actual = executor.execution_profile();
        let expected = admitted.manifest().execution_profile;
        if actual != expected {
            return Err(MoltenError::invalid_harness(format!(
                "system-extension execution profile mismatch: executor={} manifest={}",
                actual.as_str(),
                expected.as_str()
            )));
        }
        Ok(Self {
            admitted,
            executor,
            state: LifecycleState::absent(),
            usage: ResourceUsage::default(),
            invocation_sequence: FIRST_SEQUENCE,
            event_sequence: FIRST_SEQUENCE,
            observations: BTreeMap::new(),
            execution_binding_refs: Vec::new(),
            evidence: Vec::new(),
            last_lifecycle_ref: None,
        })
    }

    pub fn state(&self) -> &LifecycleState {
        &self.state
    }

    pub const fn usage(&self) -> ResourceUsage {
        self.usage
    }

    pub fn evidence(&self) -> &[HostEvidence] {
        &self.evidence
    }

    pub fn manifest(&self) -> &CanonicalAdmittedSystemExtensionManifest {
        &self.admitted
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.callbacks]
    pub fn activate(&mut self, logical_tick: u64) -> Result<ActivationArtifacts> {
        let evidence_start = self.evidence.len();
        self.apply_simple_transition(LifecycleEventKind::Install)?;
        self.apply_simple_transition(LifecycleEventKind::Admit)?;
        self.apply_simple_transition(LifecycleEventKind::BeginInitialize)?;
        self.invoke_lifecycle_callback(CallbackKind::Initialize, None, logical_tick)?
            .require_executed("initialize")?;
        self.apply_simple_transition(LifecycleEventKind::InitializeSucceeded)?;
        self.apply_simple_transition(LifecycleEventKind::BeginStart)?;
        self.invoke_lifecycle_callback(CallbackKind::Start, None, logical_tick)?.require_executed("start")?;
        self.apply_simple_transition(LifecycleEventKind::StartSucceeded)?;

        let (lifecycle_receipts, callback_receipts) = collect_artifacts(&self.evidence[evidence_start..]);
        Ok(ActivationArtifacts {
            lifecycle_receipts,
            callback_receipts,
            status: self.operator_status()?,
        })
    }

    pub fn dispatch_request(
        &mut self,
        payload_ref: &str,
        accounted_bytes: u64,
        logical_tick: u64,
    ) -> Result<HostDispatchResult> {
        let event = self.build_event(CallbackKind::Request, Some(payload_ref), accounted_bytes, logical_tick)?;
        self.dispatch(event)
    }

    // r[impl molten.system_extension.callbacks]
    // r[impl molten.system_extension.typed_effects]
    // r[impl molten.system_extension.backpressure]
    pub fn dispatch(&mut self, event: CallbackEvent) -> Result<HostDispatchResult> {
        self.ensure_evidence_capacity(CALLBACK_FAILURE_EVIDENCE_ITEMS)?;
        let plan =
            plan_callback_dispatch(self.admitted.manifest(), &self.state, self.usage, &event, self.invocation_sequence)
                .map_err(|issues| validation_error("callback dispatch", &issues))?;
        self.usage = plan.next_usage;
        let Some(invocation) = plan.invocation else {
            return Ok(HostDispatchResult::Deferred {
                decision: plan.decision,
                usage: self.usage,
            });
        };
        self.invocation_sequence = invocation.sequence;
        self.record_observed_invocation(invocation.callback)?;

        let execution = self.executor.invoke(&invocation);
        match execution {
            Err(_executor_error) => self.fail_invocation(
                &invocation,
                event.accounted_bytes,
                CallbackExecutionDecision::ExecutorFailed,
                EXECUTOR_ERROR_DIAGNOSTIC,
                None,
                FailureClass::Retryable,
            ),
            Ok(outcome) => {
                let outcome_issues = validate_callback_outcome(self.admitted.manifest(), &invocation, &outcome);
                if !outcome_issues.is_empty() {
                    return self.fail_invocation(
                        &invocation,
                        event.accounted_bytes,
                        CallbackExecutionDecision::OutcomeDenied,
                        OUTCOME_DENIED_DIAGNOSTIC,
                        Some(&outcome),
                        FailureClass::PolicyViolation,
                    );
                }
                let reserved =
                    reserve_effect_requests(&self.admitted.manifest().resources, self.usage, outcome.effects.len());
                let reserved = match reserved {
                    Ok(usage) => usage,
                    Err(_resource_issues) => {
                        return self.fail_invocation(
                            &invocation,
                            event.accounted_bytes,
                            CallbackExecutionDecision::OutcomeDenied,
                            OUTCOME_DENIED_DIAGNOSTIC,
                            Some(&outcome),
                            FailureClass::ResourceViolation,
                        );
                    }
                };
                self.usage = release_callback_resources(reserved, event.accounted_bytes, outcome.effects.len())
                    .map_err(|issues| validation_error("callback resource release", &issues))?;
                self.state.health = outcome.health;
                let receipt = canonical_callback_receipt(super::CallbackReceiptInput {
                    manifest_ref: self.admitted.manifest_ref(),
                    extension_id: &self.admitted.manifest().extension_id,
                    service_id: &self.admitted.manifest().service_id,
                    execution_profile: self.admitted.manifest().execution_profile,
                    invocation: &invocation,
                    decision: CallbackExecutionDecision::Succeeded,
                    outcome: Some(&outcome),
                    diagnostic: None,
                })?;
                self.record_callback_receipt(receipt.clone())?;
                self.record_current_readiness(&receipt.receipt_ref)?;
                Ok(HostDispatchResult::Executed {
                    approved_effects: receipt.approved_effects.clone(),
                    receipt,
                    outcome,
                })
            }
        }
    }

    // r[impl molten.system_extension.typed_effects]
    pub fn route_approved_effects<P: FabricEffectPort>(
        &mut self,
        callback_receipt: &CanonicalCallbackReceipt,
        port: &mut P,
    ) -> Result<Vec<CanonicalEffectCompletion>> {
        let receipt_is_host_owned = self.evidence.iter().any(|evidence| {
            matches!(
                evidence,
                HostEvidence::Callback(receipt) if receipt.receipt_ref == callback_receipt.receipt_ref
            )
        });
        if !receipt_is_host_owned || callback_receipt.decision != CallbackExecutionDecision::Succeeded {
            return Err(MoltenError::invalid_harness(
                "system-extension effect routing requires a successful host-owned callback receipt",
            ));
        }
        let effect_issues =
            validate_typed_effects(self.admitted.manifest(), self.state.generation, &callback_receipt.approved_effects);
        if !effect_issues.is_empty() {
            return Err(validation_error("effect routing", &effect_issues));
        }
        self.ensure_evidence_capacity(callback_receipt.approved_effects.len())?;
        self.usage = reserve_effect_requests(
            &self.admitted.manifest().resources,
            self.usage,
            callback_receipt.approved_effects.len(),
        )
        .map_err(|issues| validation_error("effect routing resources", &issues))?;

        let routing_result = (|| {
            let mut completions = Vec::with_capacity(callback_receipt.approved_effects.len());
            for effect in &callback_receipt.approved_effects {
                let key = match &effect.target {
                    EffectTarget::FabricPort(key) => key,
                    EffectTarget::Ambient(ambient) => {
                        return Err(MoltenError::invalid_harness(format!(
                            "ambient effect reached routing after validation: {}",
                            ambient.as_str()
                        )));
                    }
                };
                let binding = self.admitted.binding_for(key).ok_or_else(|| {
                    MoltenError::invalid_harness(format!(
                        "system-extension effect port {}@{} is not canonically bound",
                        key.port_id, key.version
                    ))
                })?;
                let output = port.route(binding, effect).map_err(|_error| {
                    MoltenError::invalid_harness("system-extension bound fabric-port routing failed")
                })?;
                let completion = canonical_effect_completion(&callback_receipt.receipt_ref, binding, effect, &output)?;
                self.record_evidence(HostEvidence::EffectCompletion(completion.clone()))?;
                completions.push(completion);
            }
            Ok(completions)
        })();
        self.usage = release_effect_requests(self.usage, callback_receipt.approved_effects.len())
            .map_err(|issues| validation_error("effect routing resource release", &issues))?;
        routing_result
    }

    pub fn health(&mut self, logical_tick: u64) -> Result<HostDispatchResult> {
        self.invoke_lifecycle_callback(CallbackKind::Health, None, logical_tick)
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn checkpoint(&mut self, logical_tick: u64) -> Result<CanonicalLifecycleReceipt> {
        self.apply_simple_transition(LifecycleEventKind::BeginCheckpoint)?;
        let (_, outcome) = self
            .invoke_lifecycle_callback(CallbackKind::Checkpoint, None, logical_tick)?
            .require_executed("checkpoint")?;
        let checkpoint_ref = outcome
            .checkpoint_ref
            .ok_or_else(|| MoltenError::invalid_harness("validated checkpoint callback returned no checkpoint ref"))?;
        self.apply_transition(LifecycleEvent {
            kind: LifecycleEventKind::CheckpointSucceeded,
            generation: self.state.generation,
            next_generation: None,
            checkpoint_ref: Some(checkpoint_ref),
            failure_class: None,
        })
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn recover(&mut self, checkpoint_ref: &str, logical_tick: u64) -> Result<CanonicalLifecycleReceipt> {
        self.apply_simple_transition(LifecycleEventKind::BeginRecovery)?;
        self.invoke_lifecycle_callback(CallbackKind::Recover, Some(checkpoint_ref), logical_tick)?
            .require_executed("recover")?;
        self.apply_transition(LifecycleEvent {
            kind: LifecycleEventKind::RecoverySucceeded,
            generation: self.state.generation,
            next_generation: None,
            checkpoint_ref: Some(checkpoint_ref.to_string()),
            failure_class: None,
        })
    }

    pub fn drain(&mut self, logical_tick: u64) -> Result<CanonicalLifecycleReceipt> {
        self.apply_simple_transition(LifecycleEventKind::BeginDrain)?;
        self.invoke_lifecycle_callback(CallbackKind::Drain, None, logical_tick)?.require_executed("drain")?;
        self.apply_simple_transition(LifecycleEventKind::DrainSucceeded)
    }

    pub fn shutdown(&mut self, logical_tick: u64) -> Result<CanonicalLifecycleReceipt> {
        self.apply_simple_transition(LifecycleEventKind::BeginShutdown)?;
        self.invoke_lifecycle_callback(CallbackKind::Shutdown, None, logical_tick)?
            .require_executed("shutdown")?;
        self.apply_simple_transition(LifecycleEventKind::ShutdownSucceeded)
    }

    pub fn remove(&mut self) -> Result<CanonicalLifecycleReceipt> {
        self.apply_simple_transition(LifecycleEventKind::Remove)
    }

    // r[impl molten.system_extension.supervision]
    pub fn restart(&mut self, logical_tick: u64) -> Result<CanonicalLifecycleReceipt> {
        self.apply_simple_transition(LifecycleEventKind::BeginRestart)?;
        if self.admitted.manifest().declares_callback(CallbackKind::Recover) {
            let checkpoint_ref =
                self.state.checkpoint_ref.clone().ok_or_else(|| {
                    MoltenError::invalid_harness("restart recovery requires a canonical checkpoint ref")
                })?;
            return self.recover(&checkpoint_ref, logical_tick);
        }
        self.apply_simple_transition(LifecycleEventKind::BeginStart)?;
        self.invoke_lifecycle_callback(CallbackKind::Start, None, logical_tick)?
            .require_executed("restart-start")?;
        self.apply_simple_transition(LifecycleEventKind::StartSucceeded)
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn upgrade(
        &mut self,
        next: CanonicalAdmittedSystemExtensionManifest,
        next_executor: E,
        checkpoint_ref: &str,
        logical_tick: u64,
    ) -> Result<GenerationReplacementArtifacts> {
        self.replace_generation(MigrationOperation::Upgrade, next, next_executor, checkpoint_ref, logical_tick)
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn rollback(
        &mut self,
        next: CanonicalAdmittedSystemExtensionManifest,
        next_executor: E,
        checkpoint_ref: &str,
        logical_tick: u64,
    ) -> Result<GenerationReplacementArtifacts> {
        self.replace_generation(MigrationOperation::Rollback, next, next_executor, checkpoint_ref, logical_tick)
    }

    // r[impl molten.system_extension.operator_readback]
    pub fn operator_status(&self) -> Result<CanonicalOperatorStatus> {
        let port_binding_refs = self.admitted.all_binding_refs().map(str::to_string).collect();
        canonical_operator_status(OperatorStatus {
            extension_id: self.admitted.manifest().extension_id.clone(),
            service_id: self.admitted.manifest().service_id.clone(),
            manifest_ref: self.admitted.manifest_ref().to_string(),
            generation: self.state.generation,
            phase: self.state.phase,
            execution_profile: self.admitted.manifest().execution_profile,
            port_binding_refs,
            resources: self.admitted.manifest().resources.clone(),
            usage: self.usage,
            health: self.state.health,
            restart_attempts: self.state.restart_attempts,
            checkpoint_ref: self.state.checkpoint_ref.clone(),
            last_lifecycle_ref: self.last_lifecycle_ref.clone(),
            invocation_count: self.invocation_sequence,
        })
    }

    // r[impl molten.system_extension.final_validation]
    pub fn executable_conformance_input(&self, required_callbacks: Vec<CallbackKind>) -> ExecutableConformanceInput {
        let observations = self
            .observations
            .iter()
            .map(|(callback, invocation_count)| CallbackObservation {
                callback: *callback,
                invocation_count: *invocation_count,
            })
            .collect();
        ExecutableConformanceInput {
            execution_profile: self.admitted.manifest().execution_profile,
            required_callbacks,
            observations,
            execution_binding_refs: self.execution_binding_refs.clone(),
        }
    }

    fn replace_generation(
        &mut self,
        operation: MigrationOperation,
        next: CanonicalAdmittedSystemExtensionManifest,
        next_executor: E,
        checkpoint_ref: &str,
        logical_tick: u64,
    ) -> Result<GenerationReplacementArtifacts> {
        self.ensure_evidence_capacity(REPLACEMENT_EVIDENCE_ITEMS)?;
        if self.admitted.manifest().extension_id != next.manifest().extension_id
            || self.admitted.manifest().service_id != next.manifest().service_id
        {
            return Err(MoltenError::invalid_harness(
                "system-extension generation replacement changed extension or service identity",
            ));
        }
        if next_executor.execution_profile() != next.manifest().execution_profile {
            return Err(MoltenError::invalid_harness(format!(
                "system-extension replacement profile mismatch: executor={} manifest={}",
                next_executor.execution_profile().as_str(),
                next.manifest().execution_profile.as_str()
            )));
        }
        if !next.manifest().declares_callback(CallbackKind::Recover) {
            return Err(MoltenError::invalid_harness(
                "system-extension replacement manifest must declare recover callback",
            ));
        }
        if self.state.checkpoint_ref.as_deref() != Some(checkpoint_ref) {
            return Err(MoltenError::invalid_harness(
                "system-extension replacement checkpoint does not match active state",
            ));
        }
        let source_schema = self.admitted.manifest().state_schema.clone();
        let target_schema = next.manifest().state_schema.clone();
        plan_state_migration(next.manifest(), &source_schema, &target_schema)
            .map_err(|issues| validation_error("state migration", &issues))?;
        let previous_manifest_ref = self.admitted.manifest_ref().to_string();
        let next_manifest_ref = next.manifest_ref().to_string();
        let next_generation = self
            .state
            .generation
            .checked_add(GENERATION_INCREMENT)
            .ok_or_else(|| MoltenError::invalid_harness("system-extension generation overflow"))?;
        let begin_kind = match operation {
            MigrationOperation::Upgrade => LifecycleEventKind::BeginUpgrade,
            MigrationOperation::Rollback => LifecycleEventKind::BeginRollback,
        };
        let completion_kind = match operation {
            MigrationOperation::Upgrade => LifecycleEventKind::UpgradeSucceeded,
            MigrationOperation::Rollback => LifecycleEventKind::RollbackSucceeded,
        };
        let begin = self.apply_transition(LifecycleEvent {
            kind: begin_kind,
            generation: self.state.generation,
            next_generation: Some(next_generation),
            checkpoint_ref: None,
            failure_class: None,
        })?;
        let migration = canonical_state_migration_receipt(super::StateMigrationReceiptInput {
            operation,
            extension_id: &next.manifest().extension_id,
            service_id: &next.manifest().service_id,
            previous_manifest_ref: &previous_manifest_ref,
            next_manifest_ref: &next_manifest_ref,
            source_schema: &source_schema,
            target_schema: &target_schema,
            checkpoint_ref,
            generation: next_generation,
        })?;
        self.record_evidence(HostEvidence::Migration(migration.clone()))?;
        self.admitted = next;
        self.executor = next_executor;
        let (callback, _outcome) = self
            .invoke_lifecycle_callback(CallbackKind::Recover, Some(checkpoint_ref), logical_tick)?
            .require_executed("generation replacement recovery")?;
        let completion = self.apply_simple_transition(completion_kind)?;
        Ok(GenerationReplacementArtifacts {
            begin,
            migration,
            callback,
            completion,
            status: self.operator_status()?,
        })
    }

    fn invoke_lifecycle_callback(
        &mut self,
        callback: CallbackKind,
        payload_ref: Option<&str>,
        logical_tick: u64,
    ) -> Result<HostDispatchResult> {
        let event = self.build_event(callback, payload_ref, LIFECYCLE_CALLBACK_BYTES, logical_tick)?;
        self.dispatch(event)
    }

    fn build_event(
        &mut self,
        callback: CallbackKind,
        payload_ref: Option<&str>,
        accounted_bytes: u64,
        logical_tick: u64,
    ) -> Result<CallbackEvent> {
        let event_sequence = self
            .event_sequence
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("system-extension event sequence overflow"))?;
        let deadline_tick = logical_tick
            .checked_add(self.admitted.manifest().resources.callback_deadline_ticks)
            .ok_or_else(|| MoltenError::invalid_harness("system-extension callback deadline overflow"))?;
        let value = callback_event_value(
            callback,
            self.state.generation,
            event_sequence,
            payload_ref,
            logical_tick,
            deadline_tick,
        );
        let event_ref = canonical_hash(&value)?;
        self.event_sequence = event_sequence;
        Ok(CallbackEvent {
            callback,
            generation: self.state.generation,
            event_ref,
            payload_ref: payload_ref.map(str::to_string),
            accounted_bytes,
            logical_tick,
            deadline_tick: Some(deadline_tick),
            cancellation_requested: false,
        })
    }

    fn fail_invocation(
        &mut self,
        invocation: &CallbackInvocation,
        accounted_bytes: u64,
        decision: CallbackExecutionDecision,
        diagnostic: &'static str,
        outcome: Option<&CallbackOutcome>,
        failure_class: FailureClass,
    ) -> Result<HostDispatchResult> {
        self.usage = release_callback_resources(self.usage, accounted_bytes, 0)
            .map_err(|issues| validation_error("failed callback resource release", &issues))?;
        let receipt = canonical_callback_receipt(super::CallbackReceiptInput {
            manifest_ref: self.admitted.manifest_ref(),
            extension_id: &self.admitted.manifest().extension_id,
            service_id: &self.admitted.manifest().service_id,
            execution_profile: self.admitted.manifest().execution_profile,
            invocation,
            decision,
            outcome,
            diagnostic: Some(diagnostic),
        })?;
        self.record_callback_receipt(receipt.clone())?;
        let lifecycle = self.apply_transition(LifecycleEvent {
            kind: LifecycleEventKind::Failure,
            generation: self.state.generation,
            next_generation: None,
            checkpoint_ref: None,
            failure_class: Some(failure_class),
        })?;
        Ok(HostDispatchResult::Failed { receipt, lifecycle })
    }

    fn apply_simple_transition(&mut self, kind: LifecycleEventKind) -> Result<CanonicalLifecycleReceipt> {
        let generation = if kind == LifecycleEventKind::Install {
            self.admitted.manifest().initial_generation
        } else {
            self.state.generation
        };
        self.apply_transition(LifecycleEvent::simple(kind, generation))
    }

    fn apply_transition(&mut self, event: LifecycleEvent) -> Result<CanonicalLifecycleReceipt> {
        self.ensure_evidence_capacity(TRANSITION_EVIDENCE_ITEMS)?;
        let previous = self.state.clone();
        let next = plan_lifecycle_transition(
            &previous,
            &event,
            self.usage,
            self.admitted.manifest().resources.max_restart_attempts,
        )
        .map_err(|issues| validation_error("lifecycle transition", &issues))?;
        let receipt = canonical_lifecycle_receipt(
            self.admitted.manifest_ref(),
            &self.admitted.manifest().extension_id,
            &self.admitted.manifest().service_id,
            &previous,
            &next,
            &event,
            self.usage,
        )?;
        self.state = next;
        self.last_lifecycle_ref = Some(receipt.receipt_ref.clone());
        self.record_evidence(HostEvidence::Lifecycle(receipt.clone()))?;
        self.record_current_readiness(&receipt.receipt_ref)?;
        Ok(receipt)
    }

    fn record_observed_invocation(&mut self, callback: CallbackKind) -> Result<()> {
        let current = self.observations.get(&callback).copied().unwrap_or(0);
        let next = current
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("system-extension callback observation overflow"))?;
        self.observations.insert(callback, next);
        Ok(())
    }

    fn record_callback_receipt(&mut self, receipt: CanonicalCallbackReceipt) -> Result<()> {
        self.push_execution_binding_ref(receipt.execution_binding_ref.clone())?;
        self.record_evidence(HostEvidence::Callback(receipt))
    }

    fn record_current_readiness(&mut self, boundary_ref: &str) -> Result<()> {
        let readiness = canonical_service_readiness(
            self.admitted.manifest_ref(),
            &self.admitted.manifest().extension_id,
            &self.admitted.manifest().service_id,
            &self.state,
            boundary_ref,
        )?;
        self.record_evidence(HostEvidence::Readiness(readiness))
    }

    fn push_execution_binding_ref(&mut self, execution_binding_ref: String) -> Result<()> {
        if self.execution_binding_refs.len() >= MAX_HOST_EVIDENCE_ITEMS {
            return Err(MoltenError::invalid_harness(format!(
                "system-extension execution binding count exceeds {MAX_HOST_EVIDENCE_ITEMS}"
            )));
        }
        self.execution_binding_refs.push(execution_binding_ref);
        Ok(())
    }

    fn ensure_evidence_capacity(&self, additional: usize) -> Result<()> {
        let total = self
            .evidence
            .len()
            .checked_add(additional)
            .ok_or_else(|| MoltenError::invalid_harness("system-extension evidence count overflow"))?;
        if total > MAX_HOST_EVIDENCE_ITEMS {
            return Err(MoltenError::invalid_harness(format!(
                "system-extension evidence count {total} exceeds {MAX_HOST_EVIDENCE_ITEMS}"
            )));
        }
        Ok(())
    }

    fn record_evidence(&mut self, evidence: HostEvidence) -> Result<()> {
        if self.evidence.len() >= MAX_HOST_EVIDENCE_ITEMS {
            return Err(MoltenError::invalid_harness(format!(
                "system-extension evidence count exceeds {MAX_HOST_EVIDENCE_ITEMS}"
            )));
        }
        self.evidence.push(evidence);
        Ok(())
    }
}

fn collect_artifacts(evidence: &[HostEvidence]) -> (Vec<CanonicalLifecycleReceipt>, Vec<CanonicalCallbackReceipt>) {
    let mut lifecycle = Vec::new();
    let mut callbacks = Vec::new();
    for item in evidence {
        match item {
            HostEvidence::Lifecycle(receipt) => lifecycle.push(receipt.clone()),
            HostEvidence::Callback(receipt) => callbacks.push(receipt.clone()),
            HostEvidence::EffectCompletion(_) | HostEvidence::Migration(_) | HostEvidence::Readiness(_) => {}
        }
    }
    (lifecycle, callbacks)
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("system-extension {label} denied: {issues:?}"))
}
