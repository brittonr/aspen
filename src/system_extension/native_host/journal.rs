#![allow(
    tigerstyle::excessive_file_length,
    reason = "the journal keeps one canonical instance codec beside its memory and durability-port adapters"
)]

use preserves::IOValue;
use preserves::Value;

use super::super::*;
use crate::error::MoltenError;
use crate::fabric_durability::AppendRequest;
use crate::fabric_durability::DurabilityLevel;
use crate::fabric_durability::RedbDurableStateAdapter;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::required_content_ref_string;
use crate::preserves_rail::required_sequence_field;
use crate::preserves_rail::required_string_field;
use crate::preserves_rail::sequence;
use crate::preserves_rail::simple_record_fields;
use crate::preserves_rail::strict_canonical_decode;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

const INSTANCE_RECORD: &str = "native-instance-state-v1";
const LIFECYCLE_RECORD: &str = "native-instance-lifecycle-v1";
const USAGE_RECORD: &str = "native-instance-usage-v1";
const OPERATION_RECORD: &str = "native-instance-operation-v1";
const NONE_RECORD: &str = "none";
const SOME_RECORD: &str = "some";
const INSTANCE_FIELD_COUNT: usize = 18;
const LIFECYCLE_FIELD_COUNT: usize = 5;
const USAGE_FIELD_COUNT: usize = 6;
const OPERATION_FIELD_COUNT: usize = 8;
const MAX_INSTANCE_COLLECTION_ITEMS: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNativeInstanceRecord {
    pub record_ref: String,
    pub record: NativeInstanceRecord,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeJournalError {
    InvalidRecord(String),
    Storage(String),
    Poisoned,
}

pub trait NativeHostJournal {
    fn save_instance(
        &mut self,
        record: &NativeInstanceRecord,
    ) -> Result<CanonicalNativeInstanceRecord, NativeJournalError>;

    fn latest_instance(&self, instance_id: &str) -> Result<Option<NativeInstanceRecord>, NativeJournalError>;

    fn history(&self, instance_id: &str) -> Result<Vec<NativeInstanceRecord>, NativeJournalError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryNativeHostJournal {
    records: Vec<CanonicalNativeInstanceRecord>,
}

impl NativeHostJournal for InMemoryNativeHostJournal {
    fn save_instance(
        &mut self,
        record: &NativeInstanceRecord,
    ) -> Result<CanonicalNativeInstanceRecord, NativeJournalError> {
        let canonical = canonical_native_instance_record(record).map_err(journal_invalid)?;
        self.records.push(canonical.clone());
        Ok(canonical)
    }

    fn latest_instance(&self, instance_id: &str) -> Result<Option<NativeInstanceRecord>, NativeJournalError> {
        Ok(self
            .records
            .iter()
            .rev()
            .find(|record| record.record.instance_id == instance_id)
            .map(|record| record.record.clone()))
    }

    fn history(&self, instance_id: &str) -> Result<Vec<NativeInstanceRecord>, NativeJournalError> {
        Ok(self
            .records
            .iter()
            .filter(|record| record.record.instance_id == instance_id)
            .map(|record| record.record.clone())
            .collect())
    }
}

pub struct DurableNativeHostJournal {
    adapter: RedbDurableStateAdapter,
}

impl DurableNativeHostJournal {
    pub fn new(adapter: RedbDurableStateAdapter) -> Self {
        Self { adapter }
    }

    pub fn adapter(&self) -> &RedbDurableStateAdapter {
        &self.adapter
    }
}

// r[impl molten.system_extension.native_host.durability]
impl NativeHostJournal for DurableNativeHostJournal {
    fn save_instance(
        &mut self,
        record: &NativeInstanceRecord,
    ) -> Result<CanonicalNativeInstanceRecord, NativeJournalError> {
        let canonical = canonical_native_instance_record(record).map_err(journal_invalid)?;
        let descriptor = &self.adapter.state().descriptor;
        let expected_sequence = self
            .adapter
            .state()
            .next_log_sequence()
            .map_err(|error| NativeJournalError::Storage(format!("native journal sequence: {error:?}")))?;
        self.adapter
            .append(&AppendRequest {
                adapter_id: descriptor.adapter_id.clone(),
                namespace_id: descriptor.namespace_id.clone(),
                generation: descriptor.generation,
                expected_sequence,
                value: canonical.bytes.clone(),
                value_ref: canonical.record_ref.clone(),
                durability: DurabilityLevel::MachineLoss,
            })
            .map_err(|error| NativeJournalError::Storage(error.to_string()))?;
        Ok(canonical)
    }

    fn latest_instance(&self, instance_id: &str) -> Result<Option<NativeInstanceRecord>, NativeJournalError> {
        for record in self.adapter.state().durable_log.iter().rev() {
            let decoded = decode_native_instance_record(&record.value).map_err(journal_invalid)?;
            if decoded.instance_id == instance_id {
                return Ok(Some(decoded));
            }
        }
        Ok(None)
    }

    fn history(&self, instance_id: &str) -> Result<Vec<NativeInstanceRecord>, NativeJournalError> {
        let mut history = Vec::new();
        for record in &self.adapter.state().durable_log {
            let decoded = decode_native_instance_record(&record.value).map_err(journal_invalid)?;
            if decoded.instance_id == instance_id {
                history.push(decoded);
            }
        }
        Ok(history)
    }
}

// r[impl molten.system_extension.native_host.durability]
pub fn canonical_native_instance_record(
    record_input: &NativeInstanceRecord,
) -> crate::error::Result<CanonicalNativeInstanceRecord> {
    let value = native_instance_value(record_input);
    let record_ref = canonical_hash(&value)?;
    let bytes = canonical_bytes(&value)?;
    Ok(CanonicalNativeInstanceRecord {
        record_ref,
        record: record_input.clone(),
        value,
        bytes,
    })
}

// r[impl molten.system_extension.native_host.durability]
pub fn decode_native_instance_record(bytes: &[u8]) -> crate::error::Result<NativeInstanceRecord> {
    let decoded = strict_canonical_decode(bytes)?;
    let fields = simple_record_fields(&decoded.value, INSTANCE_RECORD, INSTANCE_FIELD_COUNT)?;
    let schema = required_string_field(&fields[0], "native instance schema")?;
    if schema != NATIVE_INSTANCE_STATE_SCHEMA {
        return Err(MoltenError::invalid_harness("native instance schema mismatch"));
    }
    Ok(NativeInstanceRecord {
        schema,
        instance_id: required_string_field(&fields[1], "native instance id")?,
        extension_id: required_string_field(&fields[2], "native extension id")?,
        service_id: required_string_field(&fields[3], "native service id")?,
        manifest_ref: required_content_ref_string(&fields[4], "native manifest ref")?,
        executable_ref: required_content_ref_string(&fields[5], "native executable ref")?,
        profile_ref: required_content_ref_string(&fields[6], "native profile ref")?,
        state_schema_ref: required_content_ref_string(&fields[7], "native state schema ref")?,
        lifecycle: parse_lifecycle(&fields[8])?,
        usage: parse_usage(&fields[9])?,
        callback_sequence: required_u64(&fields[10], "native callback sequence")?,
        event_sequence: required_u64(&fields[11], "native event sequence")?,
        checkpoint_ref: parse_optional_ref(&fields[12], "native checkpoint ref")?,
        unresolved: parse_operations(&fields[13])?,
        completed_operations: parse_operations(&fields[14])?,
        completed_operation_refs: parse_refs(&fields[15], "completed operation refs")?,
        evidence_refs: parse_refs(&fields[16], "native evidence refs")?,
        is_accepting_ingress: required_bool(&fields[17], "native ingress state")?,
    })
}

fn native_instance_value(instance: &NativeInstanceRecord) -> IOValue {
    record(INSTANCE_RECORD, vec![
        string(&instance.schema),
        string(&instance.instance_id),
        string(&instance.extension_id),
        string(&instance.service_id),
        string(&instance.manifest_ref),
        string(&instance.executable_ref),
        string(&instance.profile_ref),
        string(&instance.state_schema_ref),
        lifecycle_value(&instance.lifecycle),
        usage_value(instance.usage),
        u64_value(instance.callback_sequence),
        u64_value(instance.event_sequence),
        optional_ref_value(instance.checkpoint_ref.as_deref()),
        sequence(instance.unresolved.iter().map(operation_value).collect()),
        sequence(instance.completed_operations.iter().map(operation_value).collect()),
        ref_sequence(&instance.completed_operation_refs),
        ref_sequence(&instance.evidence_refs),
        bool_value(instance.is_accepting_ingress),
    ])
}

fn lifecycle_value(state: &LifecycleState) -> IOValue {
    record(LIFECYCLE_RECORD, vec![
        u64_value(state.generation),
        string(state.phase.as_str()),
        u64_value(state.restart_attempts),
        string(state.health.as_str()),
        optional_ref_value(state.checkpoint_ref.as_deref()),
    ])
}

fn usage_value(usage: ResourceUsage) -> IOValue {
    record(USAGE_RECORD, vec![
        u64_value(usage.concurrent_callbacks),
        u64_value(usage.queued_events),
        u64_value(usage.inflight_bytes),
        u64_value(usage.open_streams),
        u64_value(usage.timers),
        u64_value(usage.effect_requests),
    ])
}

fn operation_value(operation: &NativeOperationRecord) -> IOValue {
    record(OPERATION_RECORD, vec![
        string(&operation.schema),
        string(&operation.operation_ref),
        string(&operation.parent_ref),
        string(operation.kind.as_str()),
        u64_value(operation.generation),
        string(operation.state.as_str()),
        optional_ref_value(operation.terminal_ref.as_deref()),
        bool_value(operation.is_retry_permitted),
    ])
}

fn parse_lifecycle(value: &Value<IOValue>) -> crate::error::Result<LifecycleState> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record_fields(&value, LIFECYCLE_RECORD, LIFECYCLE_FIELD_COUNT)?;
    Ok(LifecycleState {
        generation: required_u64(&fields[0], "lifecycle generation")?,
        phase: parse_phase(&required_string_field(&fields[1], "lifecycle phase")?)?,
        restart_attempts: required_u64(&fields[2], "lifecycle restart attempts")?,
        health: parse_health(&required_string_field(&fields[3], "lifecycle health")?)?,
        checkpoint_ref: parse_optional_ref(&fields[4], "lifecycle checkpoint ref")?,
    })
}

fn parse_usage(value: &Value<IOValue>) -> crate::error::Result<ResourceUsage> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record_fields(&value, USAGE_RECORD, USAGE_FIELD_COUNT)?;
    Ok(ResourceUsage {
        concurrent_callbacks: required_u64(&fields[0], "usage callbacks")?,
        queued_events: required_u64(&fields[1], "usage queue")?,
        inflight_bytes: required_u64(&fields[2], "usage bytes")?,
        open_streams: required_u64(&fields[3], "usage streams")?,
        timers: required_u64(&fields[4], "usage timers")?,
        effect_requests: required_u64(&fields[5], "usage effects")?,
    })
}

fn parse_operations(value: &Value<IOValue>) -> crate::error::Result<Vec<NativeOperationRecord>> {
    let values = required_sequence_field(value, "native operations")?;
    require_item_bound(values.len(), "native operations")?;
    values.iter().map(parse_operation).collect()
}

fn parse_operation(value: &Value<IOValue>) -> crate::error::Result<NativeOperationRecord> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record_fields(&value, OPERATION_RECORD, OPERATION_FIELD_COUNT)?;
    let schema = required_string_field(&fields[0], "native operation schema")?;
    if schema != NATIVE_OPERATION_SCHEMA {
        return Err(MoltenError::invalid_harness("native operation schema mismatch"));
    }
    Ok(NativeOperationRecord {
        schema,
        operation_ref: required_content_ref_string(&fields[1], "native operation ref")?,
        parent_ref: required_content_ref_string(&fields[2], "native operation parent ref")?,
        kind: parse_operation_kind(&required_string_field(&fields[3], "native operation kind")?)?,
        generation: required_u64(&fields[4], "native operation generation")?,
        state: parse_operation_state(&required_string_field(&fields[5], "native operation state")?)?,
        terminal_ref: parse_optional_ref(&fields[6], "native operation terminal ref")?,
        is_retry_permitted: required_bool(&fields[7], "native operation retry state")?,
    })
}

fn parse_refs(value: &Value<IOValue>, field: &str) -> crate::error::Result<Vec<String>> {
    let values = required_sequence_field(value, field)?;
    require_item_bound(values.len(), field)?;
    values.iter().map(|value| required_content_ref_string(value, field)).collect()
}

fn ref_sequence(references: &[String]) -> IOValue {
    sequence(references.iter().map(string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record(NONE_RECORD, Vec::new()), |reference| record(SOME_RECORD, vec![string(reference)]))
}

fn parse_optional_ref(value: &Value<IOValue>, field: &str) -> crate::error::Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    if value.collect_simple_record(NONE_RECORD, Some(0)).is_some() {
        return Ok(None);
    }
    let fields = simple_record_fields(&value, SOME_RECORD, 1)?;
    required_content_ref_string(&fields[0], field).map(Some)
}

fn required_u64(value: &Value<IOValue>, field: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn required_bool(value: &Value<IOValue>, field: &str) -> crate::error::Result<bool> {
    value
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected boolean for {field}")))
}

fn parse_phase(value: &str) -> crate::error::Result<LifecyclePhase> {
    match value {
        "absent" => Ok(LifecyclePhase::Absent),
        "installed" => Ok(LifecyclePhase::Installed),
        "admitted" => Ok(LifecyclePhase::Admitted),
        "initializing" => Ok(LifecyclePhase::Initializing),
        "initialized" => Ok(LifecyclePhase::Initialized),
        "starting" => Ok(LifecyclePhase::Starting),
        "running" => Ok(LifecyclePhase::Running),
        "checkpointing" => Ok(LifecyclePhase::Checkpointing),
        "recovering" => Ok(LifecyclePhase::Recovering),
        "draining" => Ok(LifecyclePhase::Draining),
        "drained" => Ok(LifecyclePhase::Drained),
        "failed" => Ok(LifecyclePhase::Failed),
        "restarting" => Ok(LifecyclePhase::Restarting),
        "upgrading" => Ok(LifecyclePhase::Upgrading),
        "rolling-back" => Ok(LifecyclePhase::RollingBack),
        "shutting-down" => Ok(LifecyclePhase::ShuttingDown),
        "quarantined" => Ok(LifecyclePhase::Quarantined),
        "stopped" => Ok(LifecyclePhase::Stopped),
        "removed" => Ok(LifecyclePhase::Removed),
        _ => Err(MoltenError::invalid_harness("native lifecycle phase is unsupported")),
    }
}

fn parse_health(value: &str) -> crate::error::Result<HealthState> {
    match value {
        "unknown" => Ok(HealthState::Unknown),
        "starting" => Ok(HealthState::Starting),
        "healthy" => Ok(HealthState::Healthy),
        "degraded" => Ok(HealthState::Degraded),
        "failed" => Ok(HealthState::Failed),
        "quarantined" => Ok(HealthState::Quarantined),
        "stopped" => Ok(HealthState::Stopped),
        _ => Err(MoltenError::invalid_harness("native health state is unsupported")),
    }
}

fn parse_operation_kind(value: &str) -> crate::error::Result<NativeOperationKind> {
    match value {
        "callback" => Ok(NativeOperationKind::Callback),
        "effect" => Ok(NativeOperationKind::Effect),
        "ingress" => Ok(NativeOperationKind::Ingress),
        _ => Err(MoltenError::invalid_harness("native operation kind is unsupported")),
    }
}

fn parse_operation_state(value: &str) -> crate::error::Result<NativeOperationState> {
    match value {
        "intent-committed" => Ok(NativeOperationState::IntentCommitted),
        "started" => Ok(NativeOperationState::Started),
        "terminal" => Ok(NativeOperationState::Terminal),
        "unknown" => Ok(NativeOperationState::Unknown),
        "stale" => Ok(NativeOperationState::Stale),
        _ => Err(MoltenError::invalid_harness("native operation state is unsupported")),
    }
}

fn require_item_bound(actual: usize, field: &str) -> crate::error::Result<()> {
    if actual > MAX_INSTANCE_COLLECTION_ITEMS {
        return Err(MoltenError::invalid_harness(format!("{field} exceeds its item bound")));
    }
    Ok(())
}

fn journal_invalid(error: crate::error::MoltenError) -> NativeJournalError {
    NativeJournalError::InvalidRecord(error.to_string())
}
