
impl<P, J> Supervisor<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    // r[impl molten.system_extension.native_host.operator]
    // r[impl molten.system_extension.native_host.nonclaims]
    pub fn status(&self) -> std::result::Result<CanonicalStatus, Failure> {
        let instance = self.instance()?;
        let operator = self.host.operator_status()?;
        let recovery = classify_native_recovery(&instance);
        let value = crate::preserves_rail::record(STATUS_RECORD, vec![
            crate::preserves_rail::string(NATIVE_STATUS_SCHEMA),
            crate::preserves_rail::string(CLAIM_LEVEL),
            crate::preserves_rail::string(&instance.instance_id),
            crate::preserves_rail::string(&operator.status_ref),
            crate::preserves_rail::u64_value(instance.lifecycle.generation.get()),
            crate::preserves_rail::string(instance.lifecycle.phase.as_str()),
            crate::preserves_rail::u64_value(
                u64::try_from(recovery.len())
                    .map_err(|_| Failure::Host("native recovery count does not fit u64".to_string()))?,
            ),
            crate::preserves_rail::sequence(
                REQUIRED_NATIVE_HOST_NON_CLAIMS
                    .iter()
                    .map(|claim| crate::preserves_rail::string(claim.as_str()))
                    .collect(),
            ),
        ]);
        Ok(CanonicalStatus {
            status_ref: crate::preserves_rail::canonical_hash(&value)?,
            claim_level: CLAIM_LEVEL.to_string(),
            operator,
            recovery,
            non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
        })
    }

    // r[impl molten.system_extension.native_host.semantic_state]
    fn sync_instance(&mut self, is_accepting_ingress: bool) -> std::result::Result<(), Failure> {
        let mut next = self.instance()?;
        next.lifecycle = self.host.state().clone();
        next.usage = self.host.usage();
        next.callback_sequence = self.host.invocation_sequence();
        next.event_sequence = self.host.event_sequence();
        next.state_ref = self.host.semantic_state_ref().map(str::to_string);
        next.checkpoint_ref = self.host.state().checkpoint_ref.clone();
        next.is_accepting_ingress = is_accepting_ingress;
        if let Some(evidence) = self.host.evidence().last() {
            next.evidence_refs.push(evidence.evidence_ref().to_string());
            next.evidence_refs.sort();
            next.evidence_refs.dedup();
        }
        self.replace_instance(next)
    }

    fn set_ingress(&mut self, is_accepting: bool) -> std::result::Result<(), Failure> {
        let mut next = self.instance()?;
        next.is_accepting_ingress = is_accepting;
        self.replace_instance(next)
    }

    fn replace_instance(&mut self, next: NativeInstanceRecord) -> std::result::Result<(), Failure> {
        save_shared(&self.journal, &next)?;
        *lock_instance(&self.instance)? = next;
        Ok(())
    }
}

impl<P, J> NativeServiceIngressPort for Supervisor<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    type Error = Failure;

    fn submit(
        &mut self,
        ingress: &NativeIngressEnvelope,
        logical_tick: u64,
    ) -> std::result::Result<IngressResult, Self::Error> {
        self.ingress(ingress, logical_tick)
    }
}

struct IntentRecordingEffectPort<'a, E, J>
where
    E: FabricEffectPort + ?Sized,
    J: NativeHostJournal,
{
    profile: &'a AdmittedNativeHostProfile,
    journal: std::sync::Arc<std::sync::Mutex<J>>,
    instance: std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>>,
    delegate: &'a mut E,
}

impl<E, J> FabricEffectPort for IntentRecordingEffectPort<'_, E, J>
where
    E: FabricEffectPort + ?Sized,
    J: NativeHostJournal,
{
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalPortBinding,
        effect: &TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<PortEffectOutput> {
        let operation_ref = native_identity_ref(&[
            "native-effect-operation-v2",
            &effect.request_ref,
            &binding.binding_ref,
            &effect.generation.to_string(),
        ]);
        let operation = NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: operation_ref.clone(),
            parent_ref: effect.request_ref.clone(),
            kind: NativeOperationKind::Effect,
            generation: effect.generation,
            state: NativeOperationState::IntentCommitted,
            terminal_ref: None,
            is_retry_permitted: false,
        };
        // The guard stays inside this block so the instance lock is released
        // before the journal write and the later re-lock.
        let next = {
            let current = self
                .instance
                .lock()
                .map_err(|_| crate::fabric::FabricPortError::storage("native effect instance state is unavailable"))?;
            commit_native_operation_intent(self.profile, &current, operation)
                .map_err(|_| crate::fabric::FabricPortError::storage("native effect intent admission failed"))?
        };
        save_shared(&self.journal, &next)
            .map_err(|_| crate::fabric::FabricPortError::storage("native effect intent persistence failed"))?;
        *self
            .instance
            .lock()
            .map_err(|_| crate::fabric::FabricPortError::storage("native effect instance state is unavailable"))? =
            next;
        // The materialized output is admitted before anything terminal is recorded: an invalid output leaves the
        // effect unknown (it may have happened) instead of committing its output ref as a terminal observation.
        let outcome = self
            .delegate
            .route(binding, effect)
            .and_then(|output| admit_materialized_effect_output(self.profile, &output).map(|()| output));
        let (state, terminal_ref) = match &outcome {
            Ok(output) => (NativeOperationState::Terminal, Some(output.output_ref.clone())),
            Err(_) => (NativeOperationState::Unknown, None),
        };
        self.record_routed_outcome(&operation_ref, state, terminal_ref)?;
        outcome
    }
}

impl<E, J> IntentRecordingEffectPort<'_, E, J>
where
    E: FabricEffectPort + ?Sized,
    J: NativeHostJournal,
{
    /// Records what the delegate observed. The delegate has already been called, so the effect may have happened:
    /// any failure to admit, persist, or publish that record is an uncertain outcome, never a storage denial.
    // r[impl molten.system_extension.independent_authority_and_output_gate]
    fn record_routed_outcome(
        &self,
        operation_ref: &str,
        state: NativeOperationState,
        terminal_ref: Option<String>,
    ) -> crate::fabric::FabricPortResult<()> {
        let current = self
            .instance
            .lock()
            .map_err(|_| crate::fabric::FabricPortError::uncertain(EFFECT_UNRECORDED_STATE))?
            .clone();
        let next = observe_native_operation(&current, operation_ref, state, terminal_ref)
            .map_err(|_| crate::fabric::FabricPortError::uncertain(EFFECT_UNRECORDED_ADMISSION))?;
        save_shared(&self.journal, &next)
            .map_err(|_| crate::fabric::FabricPortError::uncertain(EFFECT_UNRECORDED_JOURNAL))?;
        *self.instance.lock().map_err(|_| crate::fabric::FabricPortError::uncertain(EFFECT_UNRECORDED_STATE))? = next;
        Ok(())
    }
}

const EFFECT_UNRECORDED_STATE: &str = "native effect was routed but its instance state is unavailable";
const EFFECT_UNRECORDED_ADMISSION: &str = "native effect was routed but its outcome record was not admitted";
const EFFECT_UNRECORDED_JOURNAL: &str = "native effect was routed but its outcome was not journaled";

// r[impl molten.system_extension.native_host.effect_completion_value]
fn admit_materialized_effect_output(
    profile: &AdmittedNativeHostProfile,
    output: &PortEffectOutput,
) -> crate::fabric::FabricPortResult<()> {
    molten_core::system_extension::admit_materialized_native_effect_output(
        profile,
        &output.output_ref,
        output.materialized_output.as_ref(),
    )
    .map_err(|_| {
        crate::fabric::FabricPortError::malformed(
            "materialized provider output is missing, overbound, or identity-mismatched",
        )
    })
}

fn initial_instance(
    profile: &AdmittedNativeHostProfile,
    executable: &AdmittedNativeExecutable,
    admitted: &AdmittedManifest,
) -> NativeInstanceRecord {
    NativeInstanceRecord {
        schema: NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
        instance_id: native_identity_ref(&[
            "native-instance-v2",
            &admitted.manifest().extension_id,
            &admitted.manifest().service_id,
            admitted.manifest_ref(),
        ]),
        extension_id: admitted.manifest().extension_id.clone(),
        service_id: admitted.manifest().service_id.clone(),
        manifest_ref: admitted.manifest_ref().to_string(),
        executable_ref: executable.executable.executable_ref.clone(),
        profile_ref: profile.profile.profile_ref.clone(),
        state_schema_ref: executable.executable.state_schema_ref.clone(),
        lifecycle: LifecycleState {
            generation: admitted.manifest().initial_generation,
            incarnation: molten_core::system_extension::Incarnation::new(0),
            phase: LifecyclePhase::Installed,
            restart_attempts: 0,
            health: HealthState::Unknown,
            checkpoint_ref: None,
        },
        usage: ResourceUsage::zero(),
        callback_sequence: 0,
        event_sequence: 0,
        state_ref: None,
        checkpoint_ref: None,
        unresolved: Vec::new(),
        completed_operations: Vec::new(),
        completed_operation_refs: Vec::new(),
        evidence_refs: Vec::new(),
        is_accepting_ingress: false,
    }
}

fn lock_instance(
    instance: &std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>>,
) -> std::result::Result<std::sync::MutexGuard<'_, NativeInstanceRecord>, Failure> {
    instance.lock().map_err(|_| Failure::StatePoisoned)
}

fn save_shared<J: NativeHostJournal>(
    journal: &std::sync::Arc<std::sync::Mutex<J>>,
    instance: &NativeInstanceRecord,
) -> std::result::Result<(), Failure> {
    journal
        .lock()
        .map_err(|_| Failure::StatePoisoned)?
        .save_instance(instance)
        .map(|_| ())
        .map_err(Failure::Journal)
}
