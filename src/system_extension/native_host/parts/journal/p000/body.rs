
const INSTANCE_RECORD: &str = "native-instance-state-v2";
const LIFECYCLE_RECORD: &str = "native-instance-lifecycle-v2";
const USAGE_RECORD: &str = "native-instance-usage-v2";
const OPERATION_RECORD: &str = "native-instance-operation-v2";
const NONE_RECORD: &str = "none";
const SOME_RECORD: &str = "some";
const INSTANCE_FIELD_COUNT: u64 = 19;
const LIFECYCLE_FIELD_COUNT: u64 = 5;
const USAGE_FIELD_COUNT: u64 = 6;
const OPERATION_FIELD_COUNT: u64 = 8;
const MAX_INSTANCE_COLLECTION_ITEMS: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNativeInstanceRecord {
    pub record_ref: String,
    pub record: NativeInstanceRecord,
    pub value: preserves::IOValue,
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

#[derive(Debug, Clone)]
pub struct InMemoryNativeHostJournal {
    records: Vec<CanonicalNativeInstanceRecord>,
}

impl InMemoryNativeHostJournal {
    /// A journal with no saved instance records.
    pub const fn empty() -> Self {
        Self { records: Vec::new() }
    }
}

impl Default for InMemoryNativeHostJournal {
    fn default() -> Self {
        Self::empty()
    }
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
    adapter: crate::fabric_durability::RedbDurableStateAdapter,
}

impl DurableNativeHostJournal {
    pub fn new(adapter: crate::fabric_durability::RedbDurableStateAdapter) -> Self {
        Self { adapter }
    }

    pub fn adapter(&self) -> &crate::fabric_durability::RedbDurableStateAdapter {
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
            .append(&crate::fabric_durability::AppendRequest {
                adapter_id: descriptor.adapter_id.clone(),
                namespace_id: descriptor.namespace_id.clone(),
                generation: descriptor.generation,
                expected_sequence,
                value: canonical.bytes.clone(),
                value_ref: canonical.record_ref.clone(),
                durability: crate::fabric_durability::DurabilityLevel::MachineLoss,
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
        self.adapter
            .state()
            .durable_log
            .iter()
            .filter_map(|record| match decode_native_instance_record(&record.value) {
                Ok(decoded) if decoded.instance_id != instance_id => None,
                decoded => Some(decoded.map_err(journal_invalid)),
            })
            .collect()
    }
}

// r[impl molten.system_extension.native_host.durability]
pub fn canonical_native_instance_record(
    record_input: &NativeInstanceRecord,
) -> crate::error::Result<CanonicalNativeInstanceRecord> {
    let value = native_instance_value(record_input);
    let record_ref = crate::preserves_rail::canonical_hash(&value)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalNativeInstanceRecord {
        record_ref,
        record: record_input.clone(),
        value,
        bytes,
    })
}

// r[impl molten.system_extension.native_host.durability]
pub fn decode_native_instance_record(bytes: &[u8]) -> crate::error::Result<NativeInstanceRecord> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(&decoded.value, INSTANCE_RECORD, INSTANCE_FIELD_COUNT)?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "native instance schema")?;
    if schema != NATIVE_INSTANCE_STATE_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness("native instance schema mismatch"));
    }
    Ok(NativeInstanceRecord {
        schema,
        instance_id: crate::preserves_rail::required_string_field(&fields[1], "native instance id")?,
        extension_id: crate::preserves_rail::required_string_field(&fields[2], "native extension id")?,
        service_id: crate::preserves_rail::required_string_field(&fields[3], "native service id")?,
        manifest_ref: crate::preserves_rail::required_content_ref_string(&fields[4], "native manifest ref")?,
        executable_ref: crate::preserves_rail::required_content_ref_string(&fields[5], "native executable ref")?,
        profile_ref: crate::preserves_rail::required_content_ref_string(&fields[6], "native profile ref")?,
        state_schema_ref: crate::preserves_rail::required_content_ref_string(&fields[7], "native state schema ref")?,
        lifecycle: parse_lifecycle(&fields[8])?,
        usage: parse_usage(&fields[9])?,
        callback_sequence: required_u64(&fields[10], "native callback sequence")?,
        event_sequence: required_u64(&fields[11], "native event sequence")?,
        state_ref: parse_optional_ref(&fields[12], "native state ref")?,
        checkpoint_ref: parse_optional_ref(&fields[13], "native checkpoint ref")?,
        unresolved: parse_operations(&fields[14])?,
        completed_operations: parse_operations(&fields[15])?,
        completed_operation_refs: parse_refs(&fields[16], "completed operation refs")?,
        evidence_refs: parse_refs(&fields[17], "native evidence refs")?,
        is_accepting_ingress: required_bool(&fields[18], "native ingress state")?,
    })
}

fn native_instance_value(instance: &NativeInstanceRecord) -> preserves::IOValue {
    crate::preserves_rail::record(INSTANCE_RECORD, vec![
        crate::preserves_rail::string(&instance.schema),
        crate::preserves_rail::string(&instance.instance_id),
        crate::preserves_rail::string(&instance.extension_id),
        crate::preserves_rail::string(&instance.service_id),
        crate::preserves_rail::string(&instance.manifest_ref),
        crate::preserves_rail::string(&instance.executable_ref),
        crate::preserves_rail::string(&instance.profile_ref),
        crate::preserves_rail::string(&instance.state_schema_ref),
        lifecycle_value(&instance.lifecycle),
        usage_value(instance.usage),
        crate::preserves_rail::u64_value(instance.callback_sequence),
        crate::preserves_rail::u64_value(instance.event_sequence),
        optional_ref_value(instance.state_ref.as_deref()),
        optional_ref_value(instance.checkpoint_ref.as_deref()),
        crate::preserves_rail::sequence(instance.unresolved.iter().map(operation_value).collect()),
        crate::preserves_rail::sequence(instance.completed_operations.iter().map(operation_value).collect()),
        ref_sequence(&instance.completed_operation_refs),
        ref_sequence(&instance.evidence_refs),
        crate::preserves_rail::bool_value(instance.is_accepting_ingress),
    ])
}

fn lifecycle_value(state: &LifecycleState) -> preserves::IOValue {
    crate::preserves_rail::record(LIFECYCLE_RECORD, vec![
        crate::preserves_rail::u64_value(state.generation),
        crate::preserves_rail::string(state.phase.as_str()),
        crate::preserves_rail::u64_value(state.restart_attempts),
        crate::preserves_rail::string(state.health.as_str()),
        optional_ref_value(state.checkpoint_ref.as_deref()),
    ])
}

fn usage_value(usage: ResourceUsage) -> preserves::IOValue {
    crate::preserves_rail::record(USAGE_RECORD, vec![
        crate::preserves_rail::u64_value(usage.concurrent_callbacks),
        crate::preserves_rail::u64_value(usage.queued_events),
        crate::preserves_rail::u64_value(usage.inflight_bytes),
        crate::preserves_rail::u64_value(usage.open_streams),
        crate::preserves_rail::u64_value(usage.timers),
        crate::preserves_rail::u64_value(usage.effect_requests),
    ])
}

fn operation_value(operation: &NativeOperationRecord) -> preserves::IOValue {
    crate::preserves_rail::record(OPERATION_RECORD, vec![
        crate::preserves_rail::string(&operation.schema),
        crate::preserves_rail::string(&operation.operation_ref),
        crate::preserves_rail::string(&operation.parent_ref),
        crate::preserves_rail::string(operation.kind.as_str()),
        crate::preserves_rail::u64_value(operation.generation),
        crate::preserves_rail::string(operation.state.as_str()),
        optional_ref_value(operation.terminal_ref.as_deref()),
        crate::preserves_rail::bool_value(operation.is_retry_permitted),
    ])
}

fn parse_lifecycle(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<LifecycleState> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, LIFECYCLE_RECORD, LIFECYCLE_FIELD_COUNT)?;
    Ok(LifecycleState {
        generation: required_u64(&fields[0], "lifecycle generation")?,
        phase: parse_phase(&crate::preserves_rail::required_string_field(&fields[1], "lifecycle phase")?)?,
        restart_attempts: required_u64(&fields[2], "lifecycle restart attempts")?,
        health: parse_health(&crate::preserves_rail::required_string_field(&fields[3], "lifecycle health")?)?,
        checkpoint_ref: parse_optional_ref(&fields[4], "lifecycle checkpoint ref")?,
    })
}

fn parse_usage(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<ResourceUsage> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = crate::preserves_rail::simple_record_fields(&value, USAGE_RECORD, USAGE_FIELD_COUNT)?;
    Ok(ResourceUsage {
        concurrent_callbacks: required_u64(&fields[0], "usage callbacks")?,
        queued_events: required_u64(&fields[1], "usage queue")?,
        inflight_bytes: required_u64(&fields[2], "usage bytes")?,
        open_streams: required_u64(&fields[3], "usage streams")?,
        timers: required_u64(&fields[4], "usage timers")?,
        effect_requests: required_u64(&fields[5], "usage effects")?,
    })
}

fn parse_operations(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<NativeOperationRecord>> {
    let values = crate::preserves_rail::required_sequence_field(value, "native operations")?;
    require_item_bound(values.len(), "native operations")?;
    values.iter().map(parse_operation).collect()
}
