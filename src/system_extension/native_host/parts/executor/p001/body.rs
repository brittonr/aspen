
impl<P, J> NativeProcessSystemExtensionExecutor<P, J>
where
    P: ExecutionFabricPort,
    J: NativeHostJournal,
{
    pub fn new(
        port: P,
        journal: std::sync::Arc<std::sync::Mutex<J>>,
        values: SharedNativeCallbackValuePort,
        instance: std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>>,
        template: NativeExecutionTemplate,
    ) -> Result<Self, NativeExecutorError> {
        if template.request.profile_ref != port.profile().profile.descriptor.profile_ref {
            return Err(NativeExecutorError::Admission(
                "native executor request profile differs from the selected execution port".to_string(),
            ));
        }
        if template.executable.executable.execution_profile_ref != template.host_profile.profile.execution_profile_ref {
            return Err(NativeExecutorError::Admission(
                "native executable and host execution profiles differ".to_string(),
            ));
        }
        if template.admitted.manifest_ref() != template.executable.executable.manifest_ref
            || template.admitted.manifest_ref() != template.context.manifest_ref
        {
            return Err(NativeExecutorError::Admission(
                "native executor manifest snapshot differs from executable or callback context".to_string(),
            ));
        }
        Ok(Self {
            port,
            journal,
            values,
            instance,
            template,
            cancellation: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            observations: Vec::new(),
        })
    }

    pub fn observations(&self) -> &[NativeInvocationObservation] {
        &self.observations
    }

    pub fn cancellation_handle(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.cancellation.clone()
    }

    pub fn journal(&self) -> &std::sync::Arc<std::sync::Mutex<J>> {
        &self.journal
    }

    pub fn values(&self) -> &SharedNativeCallbackValuePort {
        &self.values
    }

    pub fn instance(&self) -> &std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>> {
        &self.instance
    }

    // r[impl molten.system_extension.native_host.value_intent]
    // r[impl molten.system_extension.native_host.value_materialization]
    pub fn invoke_native(&mut self, invocation: &CallbackInvocation) -> Result<CallbackOutcome, NativeExecutorError> {
        let context = self.callback_context()?;
        let operation_ref = callback_operation_ref(&context, invocation);
        self.commit_callback_intent(&operation_ref, invocation)?;
        let inputs = match self.materialize_inputs(&context, invocation) {
            Ok(inputs) => inputs,
            Err(error) => {
                self.complete_callback(CallbackCompletionInput {
                    operation_ref: &operation_ref,
                    envelope_ref: &operation_ref,
                    invocation,
                    state: NativeOperationState::Terminal,
                    terminal_ref: Some(native_identity_ref(&[
                        "native-callback-materialization-failure-v2",
                        &operation_ref,
                    ])),
                    execution_receipt_ref: None,
                    lifecycle: ExecutionLifecycleState::FailedBeforeStart,
                    diagnostic_code: Some(CALLBACK_VALUE_CODE),
                })?;
                return Err(error);
            }
        };
        let envelope = canonical_native_callback_envelope(&context, invocation, &inputs)
            .map_err(|error| NativeExecutorError::Wire(error.to_string()))?;
        let maximum_values = self.maximum_materialized_values()?;
        if let Err(error) = decode_native_callback_envelope(
            &envelope.bytes,
            self.template.host_profile.profile.max_callback_input_bytes,
            self.template.host_profile.profile.max_materialized_value_bytes,
            maximum_values,
        ) {
            self.complete_callback(CallbackCompletionInput {
                operation_ref: &operation_ref,
                envelope_ref: &envelope.envelope_ref,
                invocation,
                state: NativeOperationState::Terminal,
                terminal_ref: Some(native_identity_ref(&["native-callback-envelope-denial-v2", &operation_ref])),
                execution_receipt_ref: None,
                lifecycle: ExecutionLifecycleState::FailedBeforeStart,
                diagnostic_code: Some(CALLBACK_WIRE_CODE),
            })?;
            return Err(NativeExecutorError::Wire(error.to_string()));
        }
        let request = self.execution_request(&operation_ref, &envelope)?;
        let resolved = self.resolved_context(&envelope);
        let execution = self.port.execute(&request, &resolved, Some(&self.cancellation));
        match execution {
            Ok(receipt) => self.accept_execution_receipt(&operation_ref, &envelope, receipt),
            Err(failure) => self.record_execution_failure(&operation_ref, &envelope, failure),
        }
    }

    // r[impl molten.system_extension.native_host.value_intent]
    // r[impl molten.system_extension.native_host.value_publication]
    pub fn publish_external_value(
        &mut self,
        parent_ref: &str,
        role: &str,
        value: &NativeCallbackValue,
    ) -> Result<NativeValuePublicationReceipt, NativeExecutorError> {
        admit_native_callback_value(value, self.template.host_profile.profile.max_materialized_value_bytes)
            .map_err(NativeExecutorError::Value)?;
        let generation = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?.lifecycle.generation;
        let operation_ref = native_identity_ref(&[
            "native-value-publication-operation-v2",
            parent_ref,
            role,
            &value.value_ref,
            &generation.to_string(),
        ]);
        let operation = NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: operation_ref.clone(),
            parent_ref: parent_ref.to_string(),
            kind: NativeOperationKind::ValuePublication,
            generation,
            state: NativeOperationState::IntentCommitted,
            terminal_ref: None,
            is_retry_permitted: false,
        };
        let current = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?.clone();
        let next = commit_native_operation_intent(&self.template.host_profile, &current, operation)
            .map_err(|issues| NativeExecutorError::Admission(format!("value publication intent denied: {issues:?}")))?;
        self.replace_instance(next)?;
        let publication = self
            .values
            .lock()
            .map_err(|_| NativeExecutorError::StatePoisoned)?
            .publish(value, self.template.host_profile.profile.max_materialized_value_bytes);
        match publication {
            Ok(receipt) => {
                let terminal_ref = native_identity_ref(&[
                    "native-value-publication-observation-v2",
                    &operation_ref,
                    &receipt.publication_ref,
                ]);
                self.observe_operation(&operation_ref, NativeOperationState::Terminal, Some(terminal_ref))?;
                Ok(receipt)
            }
            Err(failure) => {
                let state = if failure.may_have_published() {
                    NativeOperationState::Unknown
                } else {
                    NativeOperationState::Terminal
                };
                let terminal_ref = (!failure.may_have_published())
                    .then(|| native_identity_ref(&["native-value-publication-rejection-v2", &operation_ref]));
                self.observe_operation(&operation_ref, state, terminal_ref)?;
                Err(NativeExecutorError::Value(failure))
            }
        }
    }

    fn callback_context(&self) -> Result<NativeCallbackContext, NativeExecutorError> {
        let instance = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?;
        let mut context = self.template.context.clone();
        context.state_ref.clone_from(&instance.state_ref);
        Ok(context)
    }

    fn maximum_materialized_values(&self) -> Result<u64, NativeExecutorError> {
        u64::try_from(self.template.host_profile.profile.max_materialized_values)
            .map_err(|_| NativeExecutorError::Admission("native materialized value count does not fit u64".to_string()))
    }

    fn materialize_inputs(
        &self,
        context: &NativeCallbackContext,
        invocation: &CallbackInvocation,
    ) -> Result<NativeCallbackInputs, NativeExecutorError> {
        let mut values = self.values.lock().map_err(|_| NativeExecutorError::StatePoisoned)?;
        let maximum = self.template.host_profile.profile.max_materialized_value_bytes;
        let payload = invocation
            .payload_ref
            .as_deref()
            .map(|value_ref| values.materialize(value_ref, maximum))
            .transpose()
            .map_err(NativeExecutorError::Value)?;
        let state = context
            .state_ref
            .as_deref()
            .map(|value_ref| values.materialize(value_ref, maximum))
            .transpose()
            .map_err(NativeExecutorError::Value)?;
        Ok(NativeCallbackInputs { payload, state })
    }

    fn commit_callback_intent(
        &mut self,
        operation_ref: &str,
        invocation: &CallbackInvocation,
    ) -> Result<(), NativeExecutorError> {
        let operation = NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: operation_ref.to_string(),
            parent_ref: invocation.event_ref.clone(),
            kind: NativeOperationKind::Callback,
            generation: invocation.generation,
            state: NativeOperationState::IntentCommitted,
            terminal_ref: None,
            is_retry_permitted: false,
        };
        let current = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?.clone();
        let next = commit_native_operation_intent(&self.template.host_profile, &current, operation)
            .map_err(|issues| NativeExecutorError::Admission(format!("callback intent denied: {issues:?}")))?;
        self.replace_instance(next)
    }

    fn execution_request(
        &self,
        operation_ref: &str,
        envelope: &CanonicalNativeCallbackEnvelope,
    ) -> Result<CanonicalExecutionRequest, NativeExecutorError> {
        let mut request = self.template.request.clone();
        request.operation_ref = operation_ref.to_string();
        request.idempotency_ref = native_identity_ref(&[
            "native-callback-idempotency-v2",
            operation_ref,
            &envelope.envelope_ref,
            &self.template.executable.executable.executable_ref,
        ]);
        request.extension_id = envelope.context.extension_id.clone();
        request.service_id = envelope.context.service_id.clone();
        request.callback_ref = envelope.invocation.event_ref.clone();
        request.effect_ref = envelope.envelope_ref.clone();
        request.generation = envelope.invocation.generation;
        request.executable_artifact_ref = self.template.executable.executable.executable_ref.clone();
        request.executable_identity_ref = self.template.executable.executable.executable_bytes_ref.clone();
        request.workspace_ref = self.template.resolved.workspace_ref.clone();
        request.stdin_ref = Some(envelope.envelope_ref.clone());
        request.authority_ref = self.template.executable.executable.authority_ref.clone();
        request.resource_grant_ref = self.template.executable.executable.resource_ref.clone();

        let mut authority = self.template.authority.clone();
        authority.authority_ref = request.authority_ref.clone();
        authority.resource_grant_ref = request.resource_grant_ref.clone();
        authority.executable_artifact_ref = request.executable_artifact_ref.clone();
        authority.executable_identity_ref = request.executable_identity_ref.clone();
        authority.workspace_ref = request.workspace_ref.clone();
        authority.operation_ref = request.operation_ref.clone();
        authority.extension_id = request.extension_id.clone();
        authority.service_id = request.service_id.clone();
        authority.generation = request.generation;
        authority.profile_ref = request.profile_ref.clone();
        canonical_admit_execution_request(
            self.port.profile(),
            &request,
            &authority,
            self.template.resources,
            request.generation,
        )
        .map_err(|error| NativeExecutorError::Admission(error.to_string()))
    }

    fn resolved_context(&self, envelope: &CanonicalNativeCallbackEnvelope) -> ResolvedExecutionContext {
        let mut resolved = self.template.resolved.clone();
        resolved.executable_artifact_ref = self.template.executable.executable.executable_ref.clone();
        resolved.executable_identity_ref = self.template.executable.executable.executable_bytes_ref.clone();
        resolved.stdin_ref = Some(envelope.envelope_ref.clone());
        resolved.stdin_bytes = Some(envelope.bytes.clone());
        resolved
    }
}
