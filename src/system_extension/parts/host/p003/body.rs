
impl<E: SystemExtensionExecutor> SystemExtensionHost<E> {
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
        let value = super::callback_event_value(super::CallbackEventInput {
            callback,
            generation: self.state.generation,
            sequence: event_sequence,
            payload_ref,
            logical_tick,
            deadline_tick,
        });
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

    fn fail_invocation(&mut self, input: InvocationFailureInput<'_>) -> crate::error::Result<HostDispatchResult> {
        let InvocationFailureInput {
            invocation,
            accounted_bytes,
            decision,
            diagnostic,
            outcome,
            failure_class,
        } = input;
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
        let receipt = super::canonical_lifecycle_receipt(super::LifecycleReceiptInput {
            manifest_ref: self.admitted.manifest_ref(),
            extension_id: &self.admitted.manifest().extension_id,
            service_id: &self.admitted.manifest().service_id,
            previous: &previous,
            next: &next,
            event: &event,
            usage: self.usage,
        })?;
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
    let lifecycle = evidence
        .iter()
        .filter_map(|item| match item {
            HostEvidence::Lifecycle(receipt) => Some(receipt.clone()),
            _ => None,
        })
        .collect();
    let callbacks = evidence
        .iter()
        .filter_map(|item| match item {
            HostEvidence::Callback(receipt) => Some(receipt.clone()),
            _ => None,
        })
        .collect();
    (lifecycle, callbacks)
}
