
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
            usage: super::ResourceUsage::zero(),
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
        input: RecoveredState,
    ) -> crate::error::Result<Self> {
        let RecoveredState {
            state,
            usage,
            invocation_sequence,
            event_sequence,
            semantic_state_ref,
            last_lifecycle_ref,
        } = input;
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
            Err(_executor_error) => self.fail_invocation(InvocationFailureInput {
                invocation: &invocation,
                accounted_bytes: event.accounted_bytes,
                decision: super::CallbackExecutionDecision::ExecutorFailed,
                diagnostic: EXECUTOR_ERROR_DIAGNOSTIC,
                outcome: None,
                failure_class: super::FailureClass::Retryable,
            }),
            Ok(outcome) => {
                let outcome_issues = super::validate_callback_outcome(self.admitted.manifest(), &invocation, &outcome);
                if !outcome_issues.is_empty() {
                    return self.fail_invocation(InvocationFailureInput {
                        invocation: &invocation,
                        accounted_bytes: event.accounted_bytes,
                        decision: super::CallbackExecutionDecision::OutcomeDenied,
                        diagnostic: OUTCOME_DENIED_DIAGNOSTIC,
                        outcome: Some(&outcome),
                        failure_class: super::FailureClass::PolicyViolation,
                    });
                }
                let reserved = super::reserve_effect_requests(
                    &self.admitted.manifest().resources,
                    self.usage,
                    outcome.effects.len(),
                );
                let reserved = match reserved {
                    Ok(usage) => usage,
                    Err(_resource_issues) => {
                        return self.fail_invocation(InvocationFailureInput {
                            invocation: &invocation,
                            accounted_bytes: event.accounted_bytes,
                            decision: super::CallbackExecutionDecision::OutcomeDenied,
                            diagnostic: OUTCOME_DENIED_DIAGNOSTIC,
                            outcome: Some(&outcome),
                            failure_class: super::FailureClass::ResourceViolation,
                        });
                    }
                };
                if self.executor.commit_admitted_outcome(&invocation, &outcome).is_err() {
                    return self.fail_invocation(InvocationFailureInput {
                        invocation: &invocation,
                        accounted_bytes: event.accounted_bytes,
                        decision: super::CallbackExecutionDecision::ExecutorFailed,
                        diagnostic: EXECUTOR_ERROR_DIAGNOSTIC,
                        outcome: Some(&outcome),
                        failure_class: super::FailureClass::Retryable,
                    });
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
}
