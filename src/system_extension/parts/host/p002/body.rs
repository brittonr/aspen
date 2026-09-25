
impl<E: SystemExtensionExecutor> SystemExtensionHost<E> {
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
}
