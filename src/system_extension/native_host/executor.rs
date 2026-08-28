#![allow(
    tigerstyle::excessive_file_length,
    reason = "the one-shot callback transaction keeps intent, materialization, execution, publication, observation, and reconciliation ordering visible"
)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use super::super::*;
use crate::fabric_execution::*;

const CALLBACK_DIAGNOSTIC_CODE: &str = "native-callback-process-denied";
const CALLBACK_WIRE_CODE: &str = "native-callback-wire-denied";
const CALLBACK_VALUE_CODE: &str = "native-callback-value-denied";
const CALLBACK_JOURNAL_CODE: &str = "native-callback-journal-failed";
const CALLBACK_LOCK_CODE: &str = "native-callback-state-poisoned";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExecutionTemplate {
    pub host_profile: AdmittedNativeHostProfile,
    pub executable: AdmittedNativeExecutable,
    pub admitted: CanonicalAdmittedSystemExtensionManifest,
    pub request: ExecutionRequest,
    pub authority: ExecutionAuthorityFacts,
    pub resources: ExecutionResourceGrant,
    pub resolved: ResolvedExecutionContext,
    pub context: NativeCallbackContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeInvocationObservation {
    pub envelope_ref: String,
    pub operation_ref: String,
    pub execution_receipt_ref: Option<String>,
    pub lifecycle: ExecutionLifecycleState,
    pub diagnostic_code: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExecutorError {
    Journal(NativeJournalError),
    StatePoisoned,
    Execution(Box<ExecutionPortFailure>),
    ProcessObservation(&'static str),
    Value(NativeValuePortFailure),
    Wire(String),
    Admission(String),
}

impl NativeExecutorError {
    pub const fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::Journal(_) => CALLBACK_JOURNAL_CODE,
            Self::StatePoisoned => CALLBACK_LOCK_CODE,
            Self::Execution(_) | Self::ProcessObservation(_) => CALLBACK_DIAGNOSTIC_CODE,
            Self::Value(_) => CALLBACK_VALUE_CODE,
            Self::Wire(_) | Self::Admission(_) => CALLBACK_WIRE_CODE,
        }
    }
}

pub struct NativeProcessSystemExtensionExecutor<P, J>
where
    P: ExecutionFabricPort,
    J: NativeHostJournal,
{
    port: P,
    journal: Arc<Mutex<J>>,
    values: SharedNativeCallbackValuePort,
    instance: Arc<Mutex<NativeInstanceRecord>>,
    template: NativeExecutionTemplate,
    cancellation: Arc<AtomicBool>,
    observations: Vec<NativeInvocationObservation>,
}

impl<P, J> NativeProcessSystemExtensionExecutor<P, J>
where
    P: ExecutionFabricPort,
    J: NativeHostJournal,
{
    pub fn new(
        port: P,
        journal: Arc<Mutex<J>>,
        values: SharedNativeCallbackValuePort,
        instance: Arc<Mutex<NativeInstanceRecord>>,
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
            cancellation: Arc::new(AtomicBool::new(false)),
            observations: Vec::new(),
        })
    }

    pub fn observations(&self) -> &[NativeInvocationObservation] {
        &self.observations
    }

    pub fn cancellation_handle(&self) -> Arc<AtomicBool> {
        self.cancellation.clone()
    }

    pub fn journal(&self) -> &Arc<Mutex<J>> {
        &self.journal
    }

    pub fn values(&self) -> &SharedNativeCallbackValuePort {
        &self.values
    }

    pub fn instance(&self) -> &Arc<Mutex<NativeInstanceRecord>> {
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
                self.complete_callback(
                    &operation_ref,
                    &operation_ref,
                    invocation,
                    NativeOperationState::Terminal,
                    Some(native_identity_ref(&["native-callback-materialization-failure-v2", &operation_ref])),
                    None,
                    ExecutionLifecycleState::FailedBeforeStart,
                    Some(CALLBACK_VALUE_CODE),
                )?;
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
            self.complete_callback(
                &operation_ref,
                &envelope.envelope_ref,
                invocation,
                NativeOperationState::Terminal,
                Some(native_identity_ref(&["native-callback-envelope-denial-v2", &operation_ref])),
                None,
                ExecutionLifecycleState::FailedBeforeStart,
                Some(CALLBACK_WIRE_CODE),
            )?;
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
            self.complete_callback(
                operation_ref,
                &envelope.envelope_ref,
                &envelope.invocation,
                NativeOperationState::Terminal,
                Some(receipt.receipt_ref.clone()),
                Some(receipt.receipt_ref.clone()),
                receipt.process.lifecycle,
                Some(CALLBACK_DIAGNOSTIC_CODE),
            )?;
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
                self.complete_callback(
                    operation_ref,
                    &envelope.envelope_ref,
                    &envelope.invocation,
                    NativeOperationState::Terminal,
                    Some(receipt.receipt_ref.clone()),
                    Some(receipt.receipt_ref),
                    ExecutionLifecycleState::Exited,
                    Some(CALLBACK_WIRE_CODE),
                )?;
                return Err(NativeExecutorError::Wire(error.to_string()));
            }
        };
        let outcome = materialized.project();
        let issues = validate_callback_outcome(self.template.admitted.manifest(), &envelope.invocation, &outcome);
        if !issues.is_empty() {
            self.complete_callback(
                operation_ref,
                &envelope.envelope_ref,
                &envelope.invocation,
                NativeOperationState::Terminal,
                Some(receipt.receipt_ref.clone()),
                Some(receipt.receipt_ref),
                ExecutionLifecycleState::Exited,
                Some(CALLBACK_WIRE_CODE),
            )?;
            return Err(NativeExecutorError::Admission(format!("callback outcome denied: {issues:?}")));
        }
        if let Err(error) = self.publish_outcome_values(operation_ref, &materialized) {
            let state = match &error {
                NativeExecutorError::Value(failure) if failure.may_have_published() => NativeOperationState::Unknown,
                _ => NativeOperationState::Terminal,
            };
            self.complete_callback(
                operation_ref,
                &envelope.envelope_ref,
                &envelope.invocation,
                state,
                (state == NativeOperationState::Terminal).then(|| receipt.receipt_ref.clone()),
                Some(receipt.receipt_ref),
                ExecutionLifecycleState::Exited,
                Some(CALLBACK_VALUE_CODE),
            )?;
            return Err(error);
        }
        self.complete_callback(
            operation_ref,
            &envelope.envelope_ref,
            &envelope.invocation,
            NativeOperationState::Terminal,
            Some(receipt.receipt_ref.clone()),
            Some(receipt.receipt_ref),
            ExecutionLifecycleState::Exited,
            None,
        )?;
        Ok(outcome)
    }

    fn publish_outcome_values(
        &mut self,
        callback_operation_ref: &str,
        outcome: &NativeMaterializedCallbackOutcome,
    ) -> Result<(), NativeExecutorError> {
        let mut values = BTreeMap::<String, (String, &NativeCallbackValue)>::new();
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
        self.complete_callback(
            operation_ref,
            &envelope.envelope_ref,
            &envelope.invocation,
            next_state,
            terminal_ref.clone(),
            terminal_ref,
            lifecycle,
            Some(failure.diagnostic_code),
        )?;
        Err(NativeExecutorError::Execution(failure))
    }

    fn complete_callback(
        &mut self,
        operation_ref: &str,
        envelope_ref: &str,
        invocation: &CallbackInvocation,
        state: NativeOperationState,
        terminal_ref: Option<String>,
        execution_receipt_ref: Option<String>,
        lifecycle: ExecutionLifecycleState,
        diagnostic_code: Option<&'static str>,
    ) -> Result<(), NativeExecutorError> {
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
    values: &mut BTreeMap<String, (String, &'a NativeCallbackValue)>,
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
