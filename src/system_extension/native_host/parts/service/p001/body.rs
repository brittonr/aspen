
impl<P, J> NativeSystemExtensionService<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    // r[impl molten.system_extension.native_host.profile]
    // r[impl molten.system_extension.native_host.operator]
    pub fn install(
        admission: AdmissionSet,
        port: P,
        journal: std::sync::Arc<std::sync::Mutex<J>>,
        values: SharedNativeCallbackValuePort,
        template: NativeExecutionTemplate,
    ) -> std::result::Result<Self, NativeServiceError> {
        let AdmissionSet {
            profile,
            executable,
            admitted,
        } = admission;
        if admitted.manifest().execution_profile != ExecutionProfile::NativeProcess {
            return Err(NativeServiceError::Host("native service manifest does not select native-process".to_string()));
        }
        if admitted.manifest_ref() != executable.executable.manifest_ref {
            return Err(NativeServiceError::Host(
                "native executable evidence does not match the admitted manifest".to_string(),
            ));
        }
        let instance = std::sync::Arc::new(std::sync::Mutex::new(initial_instance(&profile, &executable, &admitted)));
        let record = lock_instance(&instance)?.clone();
        save_shared(&journal, &record)?;
        let executor =
            NativeProcessSystemExtensionExecutor::new(port, journal.clone(), values, instance.clone(), template)
                .map_err(|error| NativeServiceError::Host(format!("native executor construction failed: {error:?}")))?;
        let host = SystemExtensionHost::new(admitted, executor)?;
        Ok(Self {
            profile,
            executable,
            journal,
            instance,
            host,
        })
    }

    // r[impl molten.system_extension.native_host.recovery]
    pub fn from_recovered(
        admission: AdmissionSet,
        executor: NativeProcessSystemExtensionExecutor<P, J>,
        journal: std::sync::Arc<std::sync::Mutex<J>>,
        instance: std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>>,
    ) -> std::result::Result<Self, NativeServiceError> {
        let AdmissionSet {
            profile,
            executable,
            admitted,
        } = admission;
        let restored = lock_instance(&instance)?.clone();
        admit_native_instance_recovery(&profile, &executable, &restored).map_err(NativeServiceError::Admission)?;
        if restored.manifest_ref != admitted.manifest_ref() {
            return Err(NativeServiceError::Host(
                "durable native instance manifest differs from the admitted manifest".to_string(),
            ));
        }
        let host = SystemExtensionHost::from_recovered_state(admitted, executor, RecoveredState {
            state: restored.lifecycle.clone(),
            usage: restored.usage,
            invocation_sequence: restored.callback_sequence,
            event_sequence: restored.event_sequence,
            semantic_state_ref: restored.state_ref.clone(),
            last_lifecycle_ref: restored.evidence_refs.last().cloned(),
        })?;
        Ok(Self {
            profile,
            executable,
            journal,
            instance,
            host,
        })
    }

    pub fn host(&self) -> &SystemExtensionHost<NativeProcessSystemExtensionExecutor<P, J>> {
        &self.host
    }

    pub fn executable(&self) -> &AdmittedNativeExecutable {
        &self.executable
    }

    pub fn host_mut(&mut self) -> &mut SystemExtensionHost<NativeProcessSystemExtensionExecutor<P, J>> {
        &mut self.host
    }

    pub fn instance(&self) -> std::result::Result<NativeInstanceRecord, NativeServiceError> {
        Ok(lock_instance(&self.instance)?.clone())
    }

    // r[impl molten.system_extension.native_host.operator]
    pub fn start(&mut self, logical_tick: u64) -> std::result::Result<ActivationArtifacts, NativeServiceError> {
        let artifacts = self.host.activate(logical_tick)?;
        self.sync_instance(true)?;
        Ok(artifacts)
    }

    // r[impl molten.system_extension.native_host.ingress]
    pub fn ingress(
        &mut self,
        ingress: &NativeIngressEnvelope,
        logical_tick: u64,
    ) -> std::result::Result<NativeServiceIngressResult, NativeServiceError> {
        let current = self.instance()?;
        let admission =
            admit_native_ingress(&self.profile, &current, ingress).map_err(NativeServiceError::Admission)?;
        let operation = NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: ingress.request_ref.clone(),
            parent_ref: ingress.payload.value_ref.clone(),
            kind: NativeOperationKind::Ingress,
            generation: ingress.generation,
            state: NativeOperationState::IntentCommitted,
            terminal_ref: None,
            is_retry_permitted: false,
        };
        let with_intent = commit_native_operation_intent(&self.profile, &current, operation)
            .map_err(NativeServiceError::Admission)?;
        self.replace_instance(with_intent)?;
        if let Err(error) =
            self.host
                .executor_mut()
                .publish_external_value(&ingress.request_ref, "ingress-payload", &ingress.payload)
        {
            let state = match &error {
                NativeExecutorError::Value(failure) if failure.may_have_published() => NativeOperationState::Unknown,
                _ => NativeOperationState::Terminal,
            };
            let terminal_ref = (state == NativeOperationState::Terminal)
                .then(|| native_identity_ref(&["native-ingress-publication-rejection-v2", &ingress.request_ref]));
            let next = observe_native_operation(&self.instance()?, &ingress.request_ref, state, terminal_ref)
                .map_err(NativeServiceError::Admission)?;
            self.replace_instance(next)?;
            return Err(NativeServiceError::Executor(error));
        }
        let dispatch = self.host.dispatch_request(&ingress.payload.value_ref, ingress.accounted_bytes, logical_tick)?;
        let terminal_ref = match &dispatch {
            HostDispatchResult::Executed { receipt, .. } | HostDispatchResult::Failed { receipt, .. } => {
                receipt.receipt_ref.clone()
            }
            HostDispatchResult::Deferred { .. } => admission.acknowledgement_ref.clone(),
        };
        let terminal = observe_native_operation(
            &self.instance()?,
            &ingress.request_ref,
            NativeOperationState::Terminal,
            Some(terminal_ref),
        )
        .map_err(NativeServiceError::Admission)?;
        self.replace_instance(terminal)?;
        self.sync_instance(true)?;
        Ok(NativeServiceIngressResult { admission, dispatch })
    }

    // r[impl molten.system_extension.native_host.effects]
    // r[impl molten.system_extension.native_host.intent]
    pub fn route_effects<E: FabricEffectPort>(
        &mut self,
        callback: &CanonicalCallbackReceipt,
        delegate: &mut E,
    ) -> std::result::Result<Vec<CanonicalEffectCompletion>, NativeServiceError> {
        let mut recording = IntentRecordingEffectPort {
            profile: &self.profile,
            journal: self.journal.clone(),
            instance: self.instance.clone(),
            delegate,
        };
        let completions = self.host.route_approved_effects(callback, &mut recording)?;
        self.sync_instance(true)?;
        Ok(completions)
    }

    // r[impl molten.system_extension.native_host.effect_completion]
    pub fn deliver_effect_completion(
        &mut self,
        completion: &CanonicalEffectCompletion,
        logical_tick: u64,
    ) -> std::result::Result<HostDispatchResult, NativeServiceError> {
        let operation_ref = native_identity_ref(&[
            "native-effect-operation-v2",
            &completion.request_ref,
            &completion.binding_ref,
            &completion.generation.to_string(),
        ]);
        let input = NativeEffectCompletionInput {
            completion_ref: completion.completion_ref.clone(),
            effect_ref: completion.request_ref.clone(),
            operation_ref,
            port_binding_ref: completion.binding_ref.clone(),
            generation: completion.generation,
        };
        let plan = admit_native_effect_completion(&self.instance()?, &input).map_err(NativeServiceError::Admission)?;
        let completion_bytes = crate::preserves_rail::canonical_bytes(&completion.value)?;
        let completion_value = NativeCallbackValue {
            value_ref: completion.completion_ref.clone(),
            bytes: completion_bytes,
        };
        self.host
            .executor_mut()
            .publish_external_value(&input.operation_ref, "effect-completion", &completion_value)
            .map_err(NativeServiceError::Executor)?;
        let completion_bytes = u64::try_from(completion_value.bytes.len())
            .map_err(|_| NativeServiceError::Host("effect completion byte count does not fit u64".to_string()))?;
        let dispatch = self.host.dispatch_message(&plan.payload_ref, completion_bytes, logical_tick)?;
        if matches!(dispatch, HostDispatchResult::Executed { .. }) {
            let consumed =
                consume_native_effect_completion(&self.instance()?, &input).map_err(NativeServiceError::Admission)?;
            self.replace_instance(consumed)?;
        }
        self.sync_instance(true)?;
        Ok(dispatch)
    }

    // r[impl molten.system_extension.native_host.operator]
    pub fn checkpoint(
        &mut self,
        logical_tick: u64,
    ) -> std::result::Result<CanonicalLifecycleReceipt, NativeServiceError> {
        let receipt = self.host.checkpoint(logical_tick)?;
        self.sync_instance(true)?;
        Ok(receipt)
    }

    // r[impl molten.system_extension.native_host.recovery]
    pub fn recover(&mut self, logical_tick: u64) -> std::result::Result<CanonicalLifecycleReceipt, NativeServiceError> {
        let checkpoint = self.instance()?.checkpoint_ref.ok_or(NativeServiceError::MissingCheckpoint)?;
        let receipt = self.host.recover(&checkpoint, logical_tick)?;
        self.sync_instance(true)?;
        Ok(receipt)
    }

    // r[impl molten.system_extension.native_host.recovery]
    pub fn restart(&mut self, logical_tick: u64) -> std::result::Result<CanonicalLifecycleReceipt, NativeServiceError> {
        if self.host.state().phase == LifecyclePhase::Running {
            self.host.observe_host_loss()?;
        }
        let receipt = self.host.restart(logical_tick)?;
        self.sync_instance(true)?;
        Ok(receipt)
    }

    // r[impl molten.system_extension.native_host.operator]
    pub fn drain(&mut self, logical_tick: u64) -> std::result::Result<CanonicalLifecycleReceipt, NativeServiceError> {
        self.set_ingress(false)?;
        let receipt = self.host.drain(logical_tick)?;
        self.sync_instance(false)?;
        Ok(receipt)
    }

    pub fn stop(&mut self, logical_tick: u64) -> std::result::Result<CanonicalLifecycleReceipt, NativeServiceError> {
        self.set_ingress(false)?;
        let receipt = self.host.shutdown(logical_tick)?;
        self.sync_instance(false)?;
        Ok(receipt)
    }

    pub fn remove(&mut self) -> std::result::Result<CanonicalLifecycleReceipt, NativeServiceError> {
        admit_native_removal(&self.instance()?).map_err(NativeServiceError::Admission)?;
        let receipt = self.host.remove()?;
        self.sync_instance(false)?;
        Ok(receipt)
    }
}
