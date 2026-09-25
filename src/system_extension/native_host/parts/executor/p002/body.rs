
impl<P, J> NativeProcessSystemExtensionExecutor<P, J>
where
    P: ExecutionFabricPort,
    J: NativeHostJournal,
{
    fn accept_execution_receipt(
        &mut self,
        operation_ref: &str,
        envelope: &CanonicalNativeCallbackEnvelope,
        receipt: CanonicalExecutionReceipt,
    ) -> Result<CallbackOutcome, NativeExecutorError> {
        let is_accepted = receipt.process.lifecycle == ExecutionLifecycleState::Exited
            && receipt.process.disposition == ExecutionObservedDisposition::ExitPolicyAccepted
            && !receipt.process.stdout.truncated;
        if !is_accepted {
            self.complete_callback(CallbackCompletionInput {
                operation_ref,
                envelope_ref: &envelope.envelope_ref,
                invocation: &envelope.invocation,
                state: NativeOperationState::Terminal,
                terminal_ref: Some(receipt.receipt_ref.clone()),
                execution_receipt_ref: Some(receipt.receipt_ref.clone()),
                lifecycle: receipt.process.lifecycle,
                diagnostic_code: Some(CALLBACK_DIAGNOSTIC_CODE),
            })?;
            return Err(NativeExecutorError::ProcessObservation(CALLBACK_DIAGNOSTIC_CODE));
        }
        let maximum_values = self.maximum_materialized_values()?;
        let materialized = match decode_native_callback_outcome(
            &receipt.process.stdout.retained_bytes,
            self.template.host_profile.profile.max_callback_output_bytes,
            self.template.host_profile.profile.max_materialized_value_bytes,
            maximum_values,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.complete_callback(CallbackCompletionInput {
                    operation_ref,
                    envelope_ref: &envelope.envelope_ref,
                    invocation: &envelope.invocation,
                    state: NativeOperationState::Terminal,
                    terminal_ref: Some(receipt.receipt_ref.clone()),
                    execution_receipt_ref: Some(receipt.receipt_ref),
                    lifecycle: ExecutionLifecycleState::Exited,
                    diagnostic_code: Some(CALLBACK_WIRE_CODE),
                })?;
                return Err(NativeExecutorError::Wire(error.to_string()));
            }
        };
        let outcome = materialized.project();
        let issues = validate_callback_outcome(self.template.admitted.manifest(), &envelope.invocation, &outcome);
        if !issues.is_empty() {
            self.complete_callback(CallbackCompletionInput {
                operation_ref,
                envelope_ref: &envelope.envelope_ref,
                invocation: &envelope.invocation,
                state: NativeOperationState::Terminal,
                terminal_ref: Some(receipt.receipt_ref.clone()),
                execution_receipt_ref: Some(receipt.receipt_ref),
                lifecycle: ExecutionLifecycleState::Exited,
                diagnostic_code: Some(CALLBACK_WIRE_CODE),
            })?;
            return Err(NativeExecutorError::Admission(format!("callback outcome denied: {issues:?}")));
        }
        if let Err(error) = self.publish_outcome_values(operation_ref, &materialized) {
            let state = match &error {
                NativeExecutorError::Value(failure) if failure.may_have_published() => NativeOperationState::Unknown,
                _ => NativeOperationState::Terminal,
            };
            self.complete_callback(CallbackCompletionInput {
                operation_ref,
                envelope_ref: &envelope.envelope_ref,
                invocation: &envelope.invocation,
                state,
                terminal_ref: (state == NativeOperationState::Terminal).then(|| receipt.receipt_ref.clone()),
                execution_receipt_ref: Some(receipt.receipt_ref),
                lifecycle: ExecutionLifecycleState::Exited,
                diagnostic_code: Some(CALLBACK_VALUE_CODE),
            })?;
            return Err(error);
        }
        self.complete_callback(CallbackCompletionInput {
            operation_ref,
            envelope_ref: &envelope.envelope_ref,
            invocation: &envelope.invocation,
            state: NativeOperationState::Terminal,
            terminal_ref: Some(receipt.receipt_ref.clone()),
            execution_receipt_ref: Some(receipt.receipt_ref),
            lifecycle: ExecutionLifecycleState::Exited,
            diagnostic_code: None,
        })?;
        Ok(outcome)
    }

    fn publish_outcome_values(
        &mut self,
        callback_operation_ref: &str,
        outcome: &NativeMaterializedCallbackOutcome,
    ) -> Result<(), NativeExecutorError> {
        let mut values = std::collections::BTreeMap::<String, (String, &NativeCallbackValue)>::new();
        for (position, value) in outcome.outputs.iter().enumerate() {
            let role = format!("output-{position}");
            insert_publication_value(&mut values, role, value)?;
        }
        for (position, effect) in outcome.effects.iter().enumerate() {
            let role = format!("effect-request-{position}");
            insert_publication_value(&mut values, role, &effect.request)?;
        }
        if let Some(state) = &outcome.state {
            insert_publication_value(&mut values, "state".to_string(), state)?;
        }
        if let Some(checkpoint) = &outcome.checkpoint {
            insert_publication_value(&mut values, "checkpoint".to_string(), checkpoint)?;
        }
        for (_value_ref, (role, value)) in values {
            self.publish_external_value(callback_operation_ref, &role, value)?;
        }
        Ok(())
    }

    fn record_execution_failure(
        &mut self,
        operation_ref: &str,
        envelope: &CanonicalNativeCallbackEnvelope,
        failure: Box<ExecutionPortFailure>,
    ) -> Result<CallbackOutcome, NativeExecutorError> {
        let next_state = if failure.kind == ExecutionPortFailureKind::RejectedBeforeStart {
            NativeOperationState::Terminal
        } else {
            NativeOperationState::Unknown
        };
        let terminal_ref = failure.receipt.as_ref().map(|receipt| receipt.receipt_ref.clone()).or_else(|| {
            (next_state == NativeOperationState::Terminal)
                .then(|| native_identity_ref(&["native-callback-prestart-failure-v2", operation_ref]))
        });
        let lifecycle = failure
            .process_observation
            .as_ref()
            .map_or(ExecutionLifecycleState::FailedBeforeStart, |process| process.lifecycle);
        self.complete_callback(CallbackCompletionInput {
            operation_ref,
            envelope_ref: &envelope.envelope_ref,
            invocation: &envelope.invocation,
            state: next_state,
            terminal_ref: terminal_ref.clone(),
            execution_receipt_ref: terminal_ref,
            lifecycle,
            diagnostic_code: Some(failure.diagnostic_code),
        })?;
        Err(NativeExecutorError::Execution(failure))
    }

    fn complete_callback(&mut self, input: CallbackCompletionInput<'_>) -> Result<(), NativeExecutorError> {
        let CallbackCompletionInput {
            operation_ref,
            envelope_ref,
            invocation,
            state,
            terminal_ref,
            execution_receipt_ref,
            lifecycle,
            diagnostic_code,
        } = input;
        self.observe_operation(operation_ref, state, terminal_ref)?;
        self.observations.push(NativeInvocationObservation {
            envelope_ref: envelope_ref.to_string(),
            operation_ref: operation_ref.to_string(),
            execution_receipt_ref,
            lifecycle,
            diagnostic_code,
        });
        if invocation.generation
            != self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?.lifecycle.generation
        {
            return Err(NativeExecutorError::Admission(
                "callback completion generation differs from durable instance".to_string(),
            ));
        }
        Ok(())
    }

    fn observe_operation(
        &self,
        operation_ref: &str,
        state: NativeOperationState,
        terminal_ref: Option<String>,
    ) -> Result<(), NativeExecutorError> {
        let current = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?.clone();
        let next = observe_native_operation(&current, operation_ref, state, terminal_ref).map_err(|issues| {
            NativeExecutorError::Admission(format!("native operation observation denied: {issues:?}"))
        })?;
        self.replace_instance(next)
    }

    fn replace_instance(&self, instance: NativeInstanceRecord) -> Result<(), NativeExecutorError> {
        self.save_instance(&instance)?;
        *self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)? = instance;
        Ok(())
    }

    fn save_instance(&self, instance: &NativeInstanceRecord) -> Result<(), NativeExecutorError> {
        self.journal
            .lock()
            .map_err(|_| NativeExecutorError::StatePoisoned)?
            .save_instance(instance)
            .map(|_| ())
            .map_err(NativeExecutorError::Journal)
    }
}

impl<P, J> SystemExtensionExecutor for NativeProcessSystemExtensionExecutor<P, J>
where
    P: ExecutionFabricPort,
    J: NativeHostJournal,
{
    fn execution_profile(&self) -> ExecutionProfile {
        ExecutionProfile::NativeProcess
    }

    fn invoke(&mut self, invocation: &CallbackInvocation) -> std::result::Result<CallbackOutcome, String> {
        self.invoke_native(invocation).map_err(|error| error.diagnostic_code().to_string())
    }

    fn commit_admitted_outcome(
        &mut self,
        invocation: &CallbackInvocation,
        outcome: &CallbackOutcome,
    ) -> std::result::Result<(), String> {
        let Some(state_ref) = &outcome.state_ref else {
            return Ok(());
        };
        let mut next = self.instance.lock().map_err(|_| CALLBACK_LOCK_CODE.to_string())?.clone();
        if next.lifecycle.generation != invocation.generation {
            return Err("native callback semantic state generation mismatch".to_string());
        }
        next.state_ref = Some(state_ref.clone());
        self.replace_instance(next).map_err(|error| error.diagnostic_code().to_string())
    }
}

fn callback_operation_ref(context: &NativeCallbackContext, invocation: &CallbackInvocation) -> String {
    native_identity_ref(&[
        "native-callback-operation-v2",
        &context.instance_id,
        &context.manifest_ref,
        invocation.callback.as_str(),
        &invocation.generation.to_string(),
        &invocation.sequence.to_string(),
        &invocation.event_ref,
    ])
}

fn insert_publication_value<'a>(
    values: &mut std::collections::BTreeMap<String, (String, &'a NativeCallbackValue)>,
    role: String,
    value: &'a NativeCallbackValue,
) -> Result<(), NativeExecutorError> {
    if let Some((_existing_role, existing)) = values.get(&value.value_ref) {
        if existing.bytes != value.bytes {
            return Err(NativeExecutorError::Admission(
                "equal native value references carry different bytes".to_string(),
            ));
        }
        return Ok(());
    }
    values.insert(value.value_ref.clone(), (role, value));
    Ok(())
}
