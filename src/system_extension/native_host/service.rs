#![allow(
    tigerstyle::excessive_file_length,
    reason = "operator methods keep lifecycle, ingress, effect, journal, and status ordering in one explicit shell"
)]

use std::sync::Arc;
use std::sync::Mutex;

use super::super::*;
use crate::error::MoltenError;
use crate::fabric::CanonicalFabricPortBinding;
use crate::fabric::FabricPortResult;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

const STATUS_RECORD: &str = "native-host-status-v1";
const CLAIM_LEVEL: &str = "local-live-pilot";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeServiceError {
    Host(String),
    Journal(NativeJournalError),
    StatePoisoned,
    Admission(Vec<NativeHostIssue>),
    MissingCheckpoint,
}

impl From<MoltenError> for NativeServiceError {
    fn from(error: MoltenError) -> Self {
        Self::Host(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeServiceIngressResult {
    pub admission: NativeIngressAdmission,
    pub dispatch: HostDispatchResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNativeServiceStatus {
    pub status_ref: String,
    pub claim_level: String,
    pub operator: CanonicalOperatorStatus,
    pub recovery: Vec<NativeRecoveryInventory>,
    pub non_claims: Vec<NativeHostNonClaim>,
}

pub trait NativeServiceIngressPort {
    type Error;

    fn submit(
        &mut self,
        ingress: &NativeIngressEnvelope,
        logical_tick: u64,
    ) -> std::result::Result<NativeServiceIngressResult, Self::Error>;
}

pub struct NativeServiceClient<'a, T: NativeServiceIngressPort> {
    port: &'a mut T,
}

impl<'a, T: NativeServiceIngressPort> NativeServiceClient<'a, T> {
    pub fn new(port: &'a mut T) -> Self {
        Self { port }
    }

    pub fn submit(
        &mut self,
        ingress: &NativeIngressEnvelope,
        logical_tick: u64,
    ) -> std::result::Result<NativeServiceIngressResult, T::Error> {
        self.port.submit(ingress, logical_tick)
    }
}

pub struct NativeSystemExtensionService<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    profile: AdmittedNativeHostProfile,
    executable: AdmittedNativeExecutable,
    journal: Arc<Mutex<J>>,
    instance: Arc<Mutex<NativeInstanceRecord>>,
    host: SystemExtensionHost<NativeProcessSystemExtensionExecutor<P, J>>,
}

impl<P, J> NativeSystemExtensionService<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    // r[impl molten.system_extension.native_host.profile]
    // r[impl molten.system_extension.native_host.operator]
    pub fn install(
        profile: AdmittedNativeHostProfile,
        executable: AdmittedNativeExecutable,
        admitted: CanonicalAdmittedSystemExtensionManifest,
        port: P,
        journal: Arc<Mutex<J>>,
        template: NativeExecutionTemplate,
    ) -> std::result::Result<Self, NativeServiceError> {
        if admitted.manifest().execution_profile != ExecutionProfile::NativeProcess {
            return Err(NativeServiceError::Host("native service manifest does not select native-process".to_string()));
        }
        if admitted.manifest_ref() != executable.executable.manifest_ref {
            return Err(NativeServiceError::Host(
                "native executable evidence does not match the admitted manifest".to_string(),
            ));
        }
        let instance = Arc::new(Mutex::new(initial_instance(&profile, &executable, &admitted)));
        save_shared(&journal, &lock_instance(&instance)?.clone())?;
        let executor = NativeProcessSystemExtensionExecutor::new(port, journal.clone(), instance.clone(), template)
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
        profile: AdmittedNativeHostProfile,
        executable: AdmittedNativeExecutable,
        admitted: CanonicalAdmittedSystemExtensionManifest,
        executor: NativeProcessSystemExtensionExecutor<P, J>,
        journal: Arc<Mutex<J>>,
        instance: Arc<Mutex<NativeInstanceRecord>>,
    ) -> std::result::Result<Self, NativeServiceError> {
        let restored = lock_instance(&instance)?.clone();
        admit_native_instance_recovery(&profile, &executable, &restored).map_err(NativeServiceError::Admission)?;
        if restored.manifest_ref != admitted.manifest_ref() {
            return Err(NativeServiceError::Host(
                "durable native instance manifest differs from the admitted manifest".to_string(),
            ));
        }
        let host = SystemExtensionHost::from_recovered_state(
            admitted,
            executor,
            restored.lifecycle.clone(),
            restored.usage,
            restored.callback_sequence,
            restored.event_sequence,
            restored.evidence_refs.last().cloned(),
        )?;
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
            parent_ref: ingress.payload_ref.clone(),
            kind: NativeOperationKind::Ingress,
            generation: ingress.generation,
            state: NativeOperationState::IntentCommitted,
            terminal_ref: None,
            is_retry_permitted: false,
        };
        let with_intent = commit_native_operation_intent(&self.profile, &current, operation)
            .map_err(NativeServiceError::Admission)?;
        self.replace_instance(with_intent)?;
        let dispatch = self.host.dispatch_request(&ingress.payload_ref, ingress.accounted_bytes, logical_tick)?;
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
            "native-effect-operation-v1",
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
        let dispatch = self.host.dispatch_message(&plan.payload_ref, 0, logical_tick)?;
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

    // r[impl molten.system_extension.native_host.operator]
    // r[impl molten.system_extension.native_host.nonclaims]
    pub fn status(&self) -> std::result::Result<CanonicalNativeServiceStatus, NativeServiceError> {
        let instance = self.instance()?;
        let operator = self.host.operator_status()?;
        let recovery = classify_native_recovery(&instance);
        let value = record(STATUS_RECORD, vec![
            string(NATIVE_STATUS_SCHEMA),
            string(CLAIM_LEVEL),
            string(&instance.instance_id),
            string(&operator.status_ref),
            u64_value(instance.lifecycle.generation),
            string(instance.lifecycle.phase.as_str()),
            u64_value(
                u64::try_from(recovery.len())
                    .map_err(|_| NativeServiceError::Host("native recovery count does not fit u64".to_string()))?,
            ),
            sequence(REQUIRED_NATIVE_HOST_NON_CLAIMS.iter().map(|claim| string(claim.as_str())).collect()),
        ]);
        Ok(CanonicalNativeServiceStatus {
            status_ref: canonical_hash(&value)?,
            claim_level: CLAIM_LEVEL.to_string(),
            operator,
            recovery,
            non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
        })
    }

    fn sync_instance(&mut self, is_accepting_ingress: bool) -> std::result::Result<(), NativeServiceError> {
        let mut next = self.instance()?;
        next.lifecycle = self.host.state().clone();
        next.usage = self.host.usage();
        next.callback_sequence = self.host.invocation_sequence();
        next.event_sequence = self.host.event_sequence();
        next.checkpoint_ref = self.host.state().checkpoint_ref.clone();
        next.is_accepting_ingress = is_accepting_ingress;
        if let Some(evidence) = self.host.evidence().last() {
            next.evidence_refs.push(evidence.evidence_ref().to_string());
            next.evidence_refs.sort();
            next.evidence_refs.dedup();
        }
        self.replace_instance(next)
    }

    fn set_ingress(&mut self, is_accepting: bool) -> std::result::Result<(), NativeServiceError> {
        let mut next = self.instance()?;
        next.is_accepting_ingress = is_accepting;
        self.replace_instance(next)
    }

    fn replace_instance(&mut self, next: NativeInstanceRecord) -> std::result::Result<(), NativeServiceError> {
        save_shared(&self.journal, &next)?;
        *lock_instance(&self.instance)? = next;
        Ok(())
    }
}

impl<P, J> NativeServiceIngressPort for NativeSystemExtensionService<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    type Error = NativeServiceError;

    fn submit(
        &mut self,
        ingress: &NativeIngressEnvelope,
        logical_tick: u64,
    ) -> std::result::Result<NativeServiceIngressResult, Self::Error> {
        self.ingress(ingress, logical_tick)
    }
}

struct IntentRecordingEffectPort<'a, E, J>
where
    E: FabricEffectPort,
    J: NativeHostJournal,
{
    profile: &'a AdmittedNativeHostProfile,
    journal: Arc<Mutex<J>>,
    instance: Arc<Mutex<NativeInstanceRecord>>,
    delegate: &'a mut E,
}

impl<E, J> FabricEffectPort for IntentRecordingEffectPort<'_, E, J>
where
    E: FabricEffectPort,
    J: NativeHostJournal,
{
    fn route(
        &mut self,
        binding: &CanonicalFabricPortBinding,
        effect: &TypedEffectRequest,
    ) -> FabricPortResult<PortEffectOutput> {
        let operation_ref = native_identity_ref(&[
            "native-effect-operation-v1",
            &effect.request_ref,
            &binding.binding_ref,
            &effect.generation.to_string(),
        ]);
        let current = self
            .instance
            .lock()
            .map_err(|_| crate::fabric::FabricPortError::storage("native effect instance state is unavailable"))?;
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
        let next = commit_native_operation_intent(self.profile, &current, operation)
            .map_err(|_| crate::fabric::FabricPortError::storage("native effect intent admission failed"))?;
        drop(current);
        save_shared(&self.journal, &next)
            .map_err(|_| crate::fabric::FabricPortError::storage("native effect intent persistence failed"))?;
        *self
            .instance
            .lock()
            .map_err(|_| crate::fabric::FabricPortError::storage("native effect instance state is unavailable"))? =
            next;
        match self.delegate.route(binding, effect) {
            Ok(output) => {
                let current = self
                    .instance
                    .lock()
                    .map_err(|_| {
                        crate::fabric::FabricPortError::storage("native effect instance state is unavailable")
                    })?
                    .clone();
                let terminal = observe_native_operation(
                    &current,
                    &operation_ref,
                    NativeOperationState::Terminal,
                    Some(output.output_ref.clone()),
                )
                .map_err(|_| crate::fabric::FabricPortError::storage("native effect terminal admission failed"))?;
                save_shared(&self.journal, &terminal).map_err(|_| {
                    crate::fabric::FabricPortError::storage("native effect terminal persistence failed")
                })?;
                *self.instance.lock().map_err(|_| {
                    crate::fabric::FabricPortError::storage("native effect instance state is unavailable")
                })? = terminal;
                Ok(output)
            }
            Err(error) => {
                let current = self
                    .instance
                    .lock()
                    .map_err(|_| {
                        crate::fabric::FabricPortError::storage("native effect instance state is unavailable")
                    })?
                    .clone();
                let unknown = observe_native_operation(&current, &operation_ref, NativeOperationState::Unknown, None)
                    .map_err(|_| {
                    crate::fabric::FabricPortError::storage("native effect uncertainty admission failed")
                })?;
                save_shared(&self.journal, &unknown).map_err(|_| {
                    crate::fabric::FabricPortError::storage("native effect uncertainty persistence failed")
                })?;
                *self.instance.lock().map_err(|_| {
                    crate::fabric::FabricPortError::storage("native effect instance state is unavailable")
                })? = unknown;
                Err(error)
            }
        }
    }
}

fn initial_instance(
    profile: &AdmittedNativeHostProfile,
    executable: &AdmittedNativeExecutable,
    admitted: &CanonicalAdmittedSystemExtensionManifest,
) -> NativeInstanceRecord {
    NativeInstanceRecord {
        schema: NATIVE_INSTANCE_STATE_SCHEMA.to_string(),
        instance_id: native_identity_ref(&[
            "native-instance-v1",
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
            phase: LifecyclePhase::Installed,
            restart_attempts: 0,
            health: HealthState::Unknown,
            checkpoint_ref: None,
        },
        usage: ResourceUsage::default(),
        callback_sequence: 0,
        event_sequence: 0,
        checkpoint_ref: None,
        unresolved: Vec::new(),
        completed_operations: Vec::new(),
        completed_operation_refs: Vec::new(),
        evidence_refs: Vec::new(),
        is_accepting_ingress: false,
    }
}

fn lock_instance(
    instance: &Arc<Mutex<NativeInstanceRecord>>,
) -> std::result::Result<std::sync::MutexGuard<'_, NativeInstanceRecord>, NativeServiceError> {
    instance.lock().map_err(|_| NativeServiceError::StatePoisoned)
}

fn save_shared<J: NativeHostJournal>(
    journal: &Arc<Mutex<J>>,
    instance: &NativeInstanceRecord,
) -> std::result::Result<(), NativeServiceError> {
    journal
        .lock()
        .map_err(|_| NativeServiceError::StatePoisoned)?
        .save_instance(instance)
        .map(|_| ())
        .map_err(NativeServiceError::Journal)
}
