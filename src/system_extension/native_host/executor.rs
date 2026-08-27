#![allow(
    tigerstyle::excessive_file_length,
    reason = "the one-shot callback transaction keeps intent, execution, observation, and reconciliation ordering visible"
)]

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use super::super::*;
use crate::fabric_execution::*;

const CALLBACK_DIAGNOSTIC_CODE: &str = "native-callback-process-denied";
const CALLBACK_WIRE_CODE: &str = "native-callback-wire-denied";
const CALLBACK_JOURNAL_CODE: &str = "native-callback-journal-failed";
const CALLBACK_LOCK_CODE: &str = "native-callback-state-poisoned";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExecutionTemplate {
    pub host_profile: AdmittedNativeHostProfile,
    pub executable: AdmittedNativeExecutable,
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
    Wire(String),
    Admission(String),
}

impl NativeExecutorError {
    pub const fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::Journal(_) => CALLBACK_JOURNAL_CODE,
            Self::StatePoisoned => CALLBACK_LOCK_CODE,
            Self::Execution(_) | Self::ProcessObservation(_) => CALLBACK_DIAGNOSTIC_CODE,
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
        Ok(Self {
            port,
            journal,
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

    pub fn instance(&self) -> &Arc<Mutex<NativeInstanceRecord>> {
        &self.instance
    }

    // r[impl molten.system_extension.native_host.execution]
    // r[impl molten.system_extension.native_host.intent]
    pub fn invoke_native(&mut self, invocation: &CallbackInvocation) -> Result<CallbackOutcome, NativeExecutorError> {
        let context = self.callback_context()?;
        let envelope = canonical_native_callback_envelope(&context, invocation)
            .map_err(|error| NativeExecutorError::Wire(error.to_string()))?;
        self.commit_callback_intent(&envelope)?;
        let request = self.execution_request(&envelope)?;
        let resolved = self.resolved_context(&envelope);
        let execution = self.port.execute(&request, &resolved, Some(&self.cancellation));
        match execution {
            Ok(receipt) => self.accept_execution_receipt(&envelope, receipt),
            Err(failure) => self.record_execution_failure(&envelope, failure),
        }
    }

    fn callback_context(&self) -> Result<NativeCallbackContext, NativeExecutorError> {
        let instance = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?;
        let mut context = self.template.context.clone();
        context.state_ref = instance.lifecycle.checkpoint_ref.clone();
        Ok(context)
    }

    fn commit_callback_intent(
        &mut self,
        envelope: &CanonicalNativeCallbackEnvelope,
    ) -> Result<(), NativeExecutorError> {
        let operation = NativeOperationRecord {
            schema: NATIVE_OPERATION_SCHEMA.to_string(),
            operation_ref: envelope.envelope_ref.clone(),
            parent_ref: envelope.invocation.event_ref.clone(),
            kind: NativeOperationKind::Callback,
            generation: envelope.invocation.generation,
            state: NativeOperationState::IntentCommitted,
            terminal_ref: None,
            is_retry_permitted: false,
        };
        let current = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?.clone();
        let next = commit_native_operation_intent(&self.template.host_profile, &current, operation)
            .map_err(|issues| NativeExecutorError::Admission(format!("callback intent denied: {issues:?}")))?;
        self.save_instance(&next)?;
        *self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)? = next;
        Ok(())
    }

    fn execution_request(
        &self,
        envelope: &CanonicalNativeCallbackEnvelope,
    ) -> Result<CanonicalExecutionRequest, NativeExecutorError> {
        let mut request = self.template.request.clone();
        request.operation_ref = envelope.envelope_ref.clone();
        request.idempotency_ref = native_identity_ref(&[
            "native-callback-idempotency-v1",
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
        envelope: &CanonicalNativeCallbackEnvelope,
        receipt: CanonicalExecutionReceipt,
    ) -> Result<CallbackOutcome, NativeExecutorError> {
        let is_accepted = receipt.process.lifecycle == ExecutionLifecycleState::Exited
            && receipt.process.disposition == ExecutionObservedDisposition::ExitPolicyAccepted
            && !receipt.process.stdout.truncated;
        if !is_accepted {
            self.complete_callback(
                envelope,
                NativeOperationState::Terminal,
                Some(receipt.receipt_ref.clone()),
                Some(receipt.receipt_ref.clone()),
                receipt.process.lifecycle,
                Some(CALLBACK_DIAGNOSTIC_CODE),
            )?;
            return Err(NativeExecutorError::ProcessObservation(CALLBACK_DIAGNOSTIC_CODE));
        }
        let outcome = match decode_native_callback_outcome(
            &receipt.process.stdout.retained_bytes,
            self.template.host_profile.profile.max_callback_output_bytes,
            self.template.host_profile.profile.max_unresolved_operations,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.complete_callback(
                    envelope,
                    NativeOperationState::Terminal,
                    Some(receipt.receipt_ref.clone()),
                    Some(receipt.receipt_ref),
                    ExecutionLifecycleState::Exited,
                    Some(CALLBACK_WIRE_CODE),
                )?;
                return Err(NativeExecutorError::Wire(error.to_string()));
            }
        };
        self.complete_callback(
            envelope,
            NativeOperationState::Terminal,
            Some(receipt.receipt_ref.clone()),
            Some(receipt.receipt_ref),
            ExecutionLifecycleState::Exited,
            None,
        )?;
        Ok(outcome)
    }

    fn record_execution_failure(
        &mut self,
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
                .then(|| native_identity_ref(&["native-callback-prestart-failure-v1", &envelope.envelope_ref]))
        });
        let lifecycle = failure
            .process_observation
            .as_ref()
            .map_or(ExecutionLifecycleState::FailedBeforeStart, |process| process.lifecycle);
        self.complete_callback(
            envelope,
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
        envelope: &CanonicalNativeCallbackEnvelope,
        state: NativeOperationState,
        terminal_ref: Option<String>,
        execution_receipt_ref: Option<String>,
        lifecycle: ExecutionLifecycleState,
        diagnostic_code: Option<&'static str>,
    ) -> Result<(), NativeExecutorError> {
        let current = self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)?.clone();
        let next = observe_native_operation(&current, &envelope.envelope_ref, state, terminal_ref)
            .map_err(|issues| NativeExecutorError::Admission(format!("callback observation denied: {issues:?}")))?;
        self.save_instance(&next)?;
        *self.instance.lock().map_err(|_| NativeExecutorError::StatePoisoned)? = next;
        self.observations.push(NativeInvocationObservation {
            envelope_ref: envelope.envelope_ref.clone(),
            operation_ref: envelope.envelope_ref.clone(),
            execution_receipt_ref,
            lifecycle,
            diagnostic_code,
        });
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
}
