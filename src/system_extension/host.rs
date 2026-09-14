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

impl<E: SystemExtensionExecutor> SystemExtensionHost<E> {
    // r[impl molten.system_extension.execution_profiles]
    pub fn new(admitted: super::CanonicalAdmittedSystemExtensionManifest, executor: E) -> crate::error::Result<Self> {
        let actual = executor.execution_profile();
        let expected = admitted.manifest().execution_profile;
        if actual != expected {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "system-extension execution profile mismatch: executor={} manifest={}",
                actual.as_str(),
                expected.as_str()
            )));
        }
        Ok(Self {
            admitted,
            executor,
            state: super::LifecycleState::absent(),
            usage: super::ResourceUsage::default(),
            invocation_sequence: FIRST_SEQUENCE,
            event_sequence: FIRST_SEQUENCE,
            semantic_state_ref: None,
            observations: std::collections::BTreeMap::new(),
            execution_binding_refs: Vec::new(),
            evidence: Vec::new(),
            last_lifecycle_ref: None,
        })
    }

    pub fn state(&self) -> &super::LifecycleState {
        &self.state
    }

    // r[impl molten.system_extension.native_host.recovery]
    pub fn from_recovered_state(
        admitted: super::CanonicalAdmittedSystemExtensionManifest,
        executor: E,
        state: super::LifecycleState,
        usage: super::ResourceUsage,
        invocation_sequence: u64,
        event_sequence: u64,
        semantic_state_ref: Option<String>,
        last_lifecycle_ref: Option<String>,
    ) -> crate::error::Result<Self> {
        let actual = executor.execution_profile();
        let expected = admitted.manifest().execution_profile;
        if actual != expected {
            return Err(crate::error::MoltenError::invalid_harness(
                "recovered system-extension executor profile mismatch",
            ));
        }
        if state.generation < admitted.manifest().initial_generation || state.phase == super::LifecyclePhase::Absent {
            return Err(crate::error::MoltenError::invalid_harness(
                "recovered system-extension state is not compatible with the admitted manifest",
            ));
        }
        if !usage.is_idle() {
            return Err(crate::error::MoltenError::invalid_harness(
                "recovered system-extension state contains live resource usage",
            ));
        }
        Ok(Self {
            admitted,
            executor,
            state,
            usage,
            invocation_sequence,
            event_sequence,
            semantic_state_ref,
            observations: std::collections::BTreeMap::new(),
            execution_binding_refs: Vec::new(),
            evidence: Vec::new(),
            last_lifecycle_ref,
        })
    }

    pub const fn usage(&self) -> super::ResourceUsage {
        self.usage
    }

    pub fn executor(&self) -> &E {
        &self.executor
    }

    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    pub const fn invocation_sequence(&self) -> u64 {
        self.invocation_sequence
    }

    pub const fn event_sequence(&self) -> u64 {
        self.event_sequence
    }

    pub fn semantic_state_ref(&self) -> Option<&str> {
        self.semantic_state_ref.as_deref()
    }

    pub fn evidence(&self) -> &[HostEvidence] {
        &self.evidence
    }

    pub fn manifest(&self) -> &super::CanonicalAdmittedSystemExtensionManifest {
        &self.admitted
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.callbacks]
    pub fn activate(&mut self, logical_tick: u64) -> crate::error::Result<ActivationArtifacts> {
        let evidence_start = self.evidence.len();
        self.apply_simple_transition(super::LifecycleEventKind::Install)?;
        self.apply_simple_transition(super::LifecycleEventKind::Admit)?;
        self.apply_simple_transition(super::LifecycleEventKind::BeginInitialize)?;
        self.invoke_lifecycle_callback(super::CallbackKind::Initialize, None, logical_tick)?
            .require_executed("initialize")?;
        self.apply_simple_transition(super::LifecycleEventKind::InitializeSucceeded)?;
        self.apply_simple_transition(super::LifecycleEventKind::BeginStart)?;
        self.invoke_lifecycle_callback(super::CallbackKind::Start, None, logical_tick)?
            .require_executed("start")?;
        self.apply_simple_transition(super::LifecycleEventKind::StartSucceeded)?;

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
    ) -> crate::error::Result<HostDispatchResult> {
        let event = self.build_event(super::CallbackKind::Request, Some(payload_ref), accounted_bytes, logical_tick)?;
        self.dispatch(event)
    }

    pub fn dispatch_message(
        &mut self,
        payload_ref: &str,
        accounted_bytes: u64,
        logical_tick: u64,
    ) -> crate::error::Result<HostDispatchResult> {
        let event = self.build_event(super::CallbackKind::Message, Some(payload_ref), accounted_bytes, logical_tick)?;
        self.dispatch(event)
    }

    // r[impl molten.system_extension.callbacks]
    // r[impl molten.system_extension.typed_effects]
    // r[impl molten.system_extension.backpressure]
    pub fn dispatch(&mut self, event: super::CallbackEvent) -> crate::error::Result<HostDispatchResult> {
        self.ensure_evidence_capacity(CALLBACK_FAILURE_EVIDENCE_ITEMS)?;
        let plan = super::plan_callback_dispatch(
            self.admitted.manifest(),
            &self.state,
            self.usage,
            &event,
            self.invocation_sequence,
        )
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
                super::CallbackExecutionDecision::ExecutorFailed,
                EXECUTOR_ERROR_DIAGNOSTIC,
                None,
                super::FailureClass::Retryable,
            ),
            Ok(outcome) => {
                let outcome_issues = super::validate_callback_outcome(self.admitted.manifest(), &invocation, &outcome);
                if !outcome_issues.is_empty() {
                    return self.fail_invocation(
                        &invocation,
                        event.accounted_bytes,
                        super::CallbackExecutionDecision::OutcomeDenied,
                        OUTCOME_DENIED_DIAGNOSTIC,
                        Some(&outcome),
                        super::FailureClass::PolicyViolation,
                    );
                }
                let reserved = super::reserve_effect_requests(
                    &self.admitted.manifest().resources,
                    self.usage,
                    outcome.effects.len(),
                );
                let reserved = match reserved {
                    Ok(usage) => usage,
                    Err(_resource_issues) => {
                        return self.fail_invocation(
                            &invocation,
                            event.accounted_bytes,
                            super::CallbackExecutionDecision::OutcomeDenied,
                            OUTCOME_DENIED_DIAGNOSTIC,
                            Some(&outcome),
                            super::FailureClass::ResourceViolation,
                        );
                    }
                };
                if self.executor.commit_admitted_outcome(&invocation, &outcome).is_err() {
                    return self.fail_invocation(
                        &invocation,
                        event.accounted_bytes,
                        super::CallbackExecutionDecision::ExecutorFailed,
                        EXECUTOR_ERROR_DIAGNOSTIC,
                        Some(&outcome),
                        super::FailureClass::Retryable,
                    );
                }
                self.usage = super::release_callback_resources(reserved, event.accounted_bytes, outcome.effects.len())
                    .map_err(|issues| validation_error("callback resource release", &issues))?;
                if let Some(state_ref) = &outcome.state_ref {
                    self.semantic_state_ref = Some(state_ref.clone());
                }
                self.state.health = outcome.health;
                let receipt = super::canonical_callback_receipt(super::CallbackReceiptInput {
                    manifest_ref: self.admitted.manifest_ref(),
                    extension_id: &self.admitted.manifest().extension_id,
                    service_id: &self.admitted.manifest().service_id,
                    execution_profile: self.admitted.manifest().execution_profile,
                    invocation: &invocation,
                    decision: super::CallbackExecutionDecision::Succeeded,
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
        callback_receipt: &super::CanonicalCallbackReceipt,
        port: &mut P,
    ) -> crate::error::Result<Vec<super::CanonicalEffectCompletion>> {
        let is_receipt_is_host_owned = self.evidence.iter().any(|evidence| {
            matches!(
                evidence,
                HostEvidence::Callback(receipt) if receipt.receipt_ref == callback_receipt.receipt_ref
            )
        });
        if !is_receipt_is_host_owned || callback_receipt.decision != super::CallbackExecutionDecision::Succeeded {
            return Err(crate::error::MoltenError::invalid_harness(
                "system-extension effect routing requires a successful host-owned callback receipt",
            ));
        }
        let effect_issues = super::validate_typed_effects(
            self.admitted.manifest(),
            self.state.generation,
            &callback_receipt.approved_effects,
        );
        if !effect_issues.is_empty() {
            return Err(validation_error("effect routing", &effect_issues));
        }
        self.ensure_evidence_capacity(callback_receipt.approved_effects.len())?;
        self.usage = super::reserve_effect_requests(
            &self.admitted.manifest().resources,
            self.usage,
            callback_receipt.approved_effects.len(),
        )
        .map_err(|issues| validation_error("effect routing resources", &issues))?;

        let routing_result = (|| {
            let mut completions = Vec::with_capacity(callback_receipt.approved_effects.len());
            for effect in &callback_receipt.approved_effects {
                let key = match &effect.target {
                    super::EffectTarget::FabricPort(key) => key,
                    super::EffectTarget::Ambient(ambient) => {
                        return Err(crate::error::MoltenError::invalid_harness(format!(
                            "ambient effect reached routing after validation: {}",
                            ambient.as_str()
                        )));
                    }
                };
                let binding = self.admitted.binding_for(key).ok_or_else(|| {
                    crate::error::MoltenError::invalid_harness(format!(
                        "system-extension effect port {}@{} is not canonically bound",
                        key.port_id, key.version
                    ))
                })?;
                let output = port.route(binding, effect).map_err(|_error| {
                    crate::error::MoltenError::invalid_harness("system-extension bound fabric-port routing failed")
                })?;
                let completion =
                    super::canonical_effect_completion(&callback_receipt.receipt_ref, binding, effect, &output)?;
                self.record_evidence(HostEvidence::EffectCompletion(completion.clone()))?;
                completions.push(completion);
            }
            Ok(completions)
        })();
        self.usage = super::release_effect_requests(self.usage, callback_receipt.approved_effects.len())
            .map_err(|issues| validation_error("effect routing resource release", &issues))?;
        routing_result
    }

    pub fn health(&mut self, logical_tick: u64) -> crate::error::Result<HostDispatchResult> {
        self.invoke_lifecycle_callback(super::CallbackKind::Health, None, logical_tick)
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn checkpoint(&mut self, logical_tick: u64) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.apply_simple_transition(super::LifecycleEventKind::BeginCheckpoint)?;
        let (_, outcome) = self
            .invoke_lifecycle_callback(super::CallbackKind::Checkpoint, None, logical_tick)?
            .require_executed("checkpoint")?;
        let checkpoint_ref = outcome.checkpoint_ref.ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("validated checkpoint callback returned no checkpoint ref")
        })?;
        self.apply_transition(super::LifecycleEvent {
            kind: super::LifecycleEventKind::CheckpointSucceeded,
            generation: self.state.generation,
            next_generation: None,
            checkpoint_ref: Some(checkpoint_ref),
            failure_class: None,
        })
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn recover(
        &mut self,
        checkpoint_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.apply_simple_transition(super::LifecycleEventKind::BeginRecovery)?;
        self.invoke_lifecycle_callback(super::CallbackKind::Recover, Some(checkpoint_ref), logical_tick)?
            .require_executed("recover")?;
        self.apply_transition(super::LifecycleEvent {
            kind: super::LifecycleEventKind::RecoverySucceeded,
            generation: self.state.generation,
            next_generation: None,
            checkpoint_ref: Some(checkpoint_ref.to_string()),
            failure_class: None,
        })
    }

    pub fn drain(&mut self, logical_tick: u64) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.apply_simple_transition(super::LifecycleEventKind::BeginDrain)?;
        self.invoke_lifecycle_callback(super::CallbackKind::Drain, None, logical_tick)?
            .require_executed("drain")?;
        self.apply_simple_transition(super::LifecycleEventKind::DrainSucceeded)
    }

    pub fn shutdown(&mut self, logical_tick: u64) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.apply_simple_transition(super::LifecycleEventKind::BeginShutdown)?;
        self.invoke_lifecycle_callback(super::CallbackKind::Shutdown, None, logical_tick)?
            .require_executed("shutdown")?;
        self.apply_simple_transition(super::LifecycleEventKind::ShutdownSucceeded)
    }

    pub fn remove(&mut self) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.apply_simple_transition(super::LifecycleEventKind::Remove)
    }

    // r[impl molten.system_extension.native_host.recovery]
    pub fn observe_host_loss(&mut self) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.apply_transition(super::LifecycleEvent {
            kind: super::LifecycleEventKind::Failure,
            generation: self.state.generation,
            next_generation: None,
            checkpoint_ref: None,
            failure_class: Some(super::FailureClass::Retryable),
        })
    }

    // r[impl molten.system_extension.supervision]
    pub fn restart(&mut self, logical_tick: u64) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.apply_simple_transition(super::LifecycleEventKind::BeginRestart)?;
        if self.admitted.manifest().declares_callback(super::CallbackKind::Recover) {
            let checkpoint_ref = self.state.checkpoint_ref.clone().ok_or_else(|| {
                crate::error::MoltenError::invalid_harness("restart recovery requires a canonical checkpoint ref")
            })?;
            return self.recover(&checkpoint_ref, logical_tick);
        }
        self.apply_simple_transition(super::LifecycleEventKind::BeginStart)?;
        self.invoke_lifecycle_callback(super::CallbackKind::Start, None, logical_tick)?
            .require_executed("restart-start")?;
        self.apply_simple_transition(super::LifecycleEventKind::StartSucceeded)
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn upgrade(
        &mut self,
        next: super::CanonicalAdmittedSystemExtensionManifest,
        next_executor: E,
        checkpoint_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<GenerationReplacementArtifacts> {
        self.replace_generation(super::MigrationOperation::Upgrade, next, next_executor, checkpoint_ref, logical_tick)
    }

    // r[impl molten.system_extension.lifecycle]
    // r[impl molten.system_extension.evidence]
    pub fn rollback(
        &mut self,
        next: super::CanonicalAdmittedSystemExtensionManifest,
        next_executor: E,
        checkpoint_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<GenerationReplacementArtifacts> {
        self.replace_generation(super::MigrationOperation::Rollback, next, next_executor, checkpoint_ref, logical_tick)
    }

    // r[impl molten.system_extension.operator_readback]
    pub fn operator_status(&self) -> crate::error::Result<super::CanonicalOperatorStatus> {
        let port_binding_refs = self.admitted.all_binding_refs().map(str::to_string).collect();
        super::canonical_operator_status(super::OperatorStatus {
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
    pub fn executable_conformance_input(
        &self,
        required_callbacks: Vec<super::CallbackKind>,
    ) -> super::ExecutableConformanceInput {
        let observations = self
            .observations
            .iter()
            .map(|(callback, invocation_count)| super::CallbackObservation {
                callback: *callback,
                invocation_count: *invocation_count,
            })
            .collect();
        super::ExecutableConformanceInput {
            execution_profile: self.admitted.manifest().execution_profile,
            required_callbacks,
            observations,
            execution_binding_refs: self.execution_binding_refs.clone(),
        }
    }

    fn replace_generation(
        &mut self,
        operation: super::MigrationOperation,
        next: super::CanonicalAdmittedSystemExtensionManifest,
        next_executor: E,
        checkpoint_ref: &str,
        logical_tick: u64,
    ) -> crate::error::Result<GenerationReplacementArtifacts> {
        self.ensure_evidence_capacity(REPLACEMENT_EVIDENCE_ITEMS)?;
        if self.admitted.manifest().extension_id != next.manifest().extension_id
            || self.admitted.manifest().service_id != next.manifest().service_id
        {
            return Err(crate::error::MoltenError::invalid_harness(
                "system-extension generation replacement changed extension or service identity",
            ));
        }
        if next_executor.execution_profile() != next.manifest().execution_profile {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "system-extension replacement profile mismatch: executor={} manifest={}",
                next_executor.execution_profile().as_str(),
                next.manifest().execution_profile.as_str()
            )));
        }
        if !next.manifest().declares_callback(super::CallbackKind::Recover) {
            return Err(crate::error::MoltenError::invalid_harness(
                "system-extension replacement manifest must declare recover callback",
            ));
        }
        if self.state.checkpoint_ref.as_deref() != Some(checkpoint_ref) {
            return Err(crate::error::MoltenError::invalid_harness(
                "system-extension replacement checkpoint does not match active state",
            ));
        }
        let source_schema = self.admitted.manifest().state_schema.clone();
        let target_schema = next.manifest().state_schema.clone();
        super::plan_state_migration(next.manifest(), &source_schema, &target_schema)
            .map_err(|issues| validation_error("state migration", &issues))?;
        let previous_manifest_ref = self.admitted.manifest_ref().to_string();
        let next_manifest_ref = next.manifest_ref().to_string();
        let next_generation = self
            .state
            .generation
            .checked_add(GENERATION_INCREMENT)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("system-extension generation overflow"))?;
        let begin_kind = match operation {
            super::MigrationOperation::Upgrade => super::LifecycleEventKind::BeginUpgrade,
            super::MigrationOperation::Rollback => super::LifecycleEventKind::BeginRollback,
        };
        let completion_kind = match operation {
            super::MigrationOperation::Upgrade => super::LifecycleEventKind::UpgradeSucceeded,
            super::MigrationOperation::Rollback => super::LifecycleEventKind::RollbackSucceeded,
        };
        let begin = self.apply_transition(super::LifecycleEvent {
            kind: begin_kind,
            generation: self.state.generation,
            next_generation: Some(next_generation),
            checkpoint_ref: None,
            failure_class: None,
        })?;
        let migration = super::canonical_state_migration_receipt(super::StateMigrationReceiptInput {
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
            .invoke_lifecycle_callback(super::CallbackKind::Recover, Some(checkpoint_ref), logical_tick)?
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
        callback: super::CallbackKind,
        payload_ref: Option<&str>,
        logical_tick: u64,
    ) -> crate::error::Result<HostDispatchResult> {
        let event = self.build_event(callback, payload_ref, LIFECYCLE_CALLBACK_BYTES, logical_tick)?;
        self.dispatch(event)
    }

    fn build_event(
        &mut self,
        callback: super::CallbackKind,
        payload_ref: Option<&str>,
        accounted_bytes: u64,
        logical_tick: u64,
    ) -> crate::error::Result<super::CallbackEvent> {
        let event_sequence = self
            .event_sequence
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("system-extension event sequence overflow"))?;
        let deadline_tick = logical_tick
            .checked_add(self.admitted.manifest().resources.callback_deadline_ticks)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("system-extension callback deadline overflow"))?;
        let value = super::callback_event_value(
            callback,
            self.state.generation,
            event_sequence,
            payload_ref,
            logical_tick,
            deadline_tick,
        );
        let event_ref = crate::preserves_rail::canonical_hash(&value)?;
        self.event_sequence = event_sequence;
        Ok(super::CallbackEvent {
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
        invocation: &super::CallbackInvocation,
        accounted_bytes: u64,
        decision: super::CallbackExecutionDecision,
        diagnostic: &'static str,
        outcome: Option<&super::CallbackOutcome>,
        failure_class: super::FailureClass,
    ) -> crate::error::Result<HostDispatchResult> {
        self.usage = super::release_callback_resources(self.usage, accounted_bytes, 0)
            .map_err(|issues| validation_error("failed callback resource release", &issues))?;
        let receipt = super::canonical_callback_receipt(super::CallbackReceiptInput {
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
        let lifecycle = self.apply_transition(super::LifecycleEvent {
            kind: super::LifecycleEventKind::Failure,
            generation: self.state.generation,
            next_generation: None,
            checkpoint_ref: None,
            failure_class: Some(failure_class),
        })?;
        Ok(HostDispatchResult::Failed { receipt, lifecycle })
    }

    fn apply_simple_transition(
        &mut self,
        kind: super::LifecycleEventKind,
    ) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        let generation = if kind == super::LifecycleEventKind::Install {
            self.admitted.manifest().initial_generation
        } else {
            self.state.generation
        };
        self.apply_transition(super::LifecycleEvent::simple(kind, generation))
    }

    fn apply_transition(
        &mut self,
        event: super::LifecycleEvent,
    ) -> crate::error::Result<super::CanonicalLifecycleReceipt> {
        self.ensure_evidence_capacity(TRANSITION_EVIDENCE_ITEMS)?;
        let previous = self.state.clone();
        let next = super::plan_lifecycle_transition(
            &previous,
            &event,
            self.usage,
            self.admitted.manifest().resources.max_restart_attempts,
        )
        .map_err(|issues| validation_error("lifecycle transition", &issues))?;
        let receipt = super::canonical_lifecycle_receipt(
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

    fn record_observed_invocation(&mut self, callback: super::CallbackKind) -> crate::error::Result<()> {
        let current = self.observations.get(&callback).copied().unwrap_or(0);
        let next = current.checked_add(1).ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("system-extension callback observation overflow")
        })?;
        self.observations.insert(callback, next);
        Ok(())
    }

    fn record_callback_receipt(&mut self, receipt: super::CanonicalCallbackReceipt) -> crate::error::Result<()> {
        self.push_execution_binding_ref(receipt.execution_binding_ref.clone())?;
        self.record_evidence(HostEvidence::Callback(receipt))
    }

    fn record_current_readiness(&mut self, boundary_ref: &str) -> crate::error::Result<()> {
        let readiness = super::canonical_service_readiness(
            self.admitted.manifest_ref(),
            &self.admitted.manifest().extension_id,
            &self.admitted.manifest().service_id,
            &self.state,
            boundary_ref,
        )?;
        self.record_evidence(HostEvidence::Readiness(readiness))
    }

    fn push_execution_binding_ref(&mut self, execution_binding_ref: String) -> crate::error::Result<()> {
        if self.execution_binding_refs.len() >= MAX_HOST_EVIDENCE_ITEMS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "system-extension execution binding count exceeds {MAX_HOST_EVIDENCE_ITEMS}"
            )));
        }
        self.execution_binding_refs.push(execution_binding_ref);
        Ok(())
    }

    fn ensure_evidence_capacity(&self, additional: usize) -> crate::error::Result<()> {
        let total =
            self.evidence.len().checked_add(additional).ok_or_else(|| {
                crate::error::MoltenError::invalid_harness("system-extension evidence count overflow")
            })?;
        if total > MAX_HOST_EVIDENCE_ITEMS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "system-extension evidence count {total} exceeds {MAX_HOST_EVIDENCE_ITEMS}"
            )));
        }
        Ok(())
    }

    fn record_evidence(&mut self, evidence: HostEvidence) -> crate::error::Result<()> {
        if self.evidence.len() >= MAX_HOST_EVIDENCE_ITEMS {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "system-extension evidence count exceeds {MAX_HOST_EVIDENCE_ITEMS}"
            )));
        }
        self.evidence.push(evidence);
        Ok(())
    }
}

fn collect_artifacts(
    evidence: &[HostEvidence],
) -> (Vec<super::CanonicalLifecycleReceipt>, Vec<super::CanonicalCallbackReceipt>) {
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

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("system-extension {label} denied: {issues:?}"))
}
