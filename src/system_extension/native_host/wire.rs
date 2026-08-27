#![allow(
    tigerstyle::excessive_file_length,
    reason = "the v2 wire codec keeps every canonical value field and strict decoder in one auditable protocol surface"
)]

use preserves::IOValue;
use preserves::Value;

use super::super::*;
use super::admit_native_callback_value;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::FabricPortKey;
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

const CALLBACK_ENVELOPE_RECORD: &str = "native-callback-envelope-v2";
const CALLBACK_OUTCOME_RECORD: &str = "native-callback-outcome-v2";
const EFFECT_RECORD: &str = "native-callback-effect-v2";
const PORT_TARGET_RECORD: &str = "native-callback-port-target-v2";
const VALUE_RECORD: &str = "native-callback-value-v2";
const NONE_RECORD: &str = "none";
const SOME_RECORD: &str = "some";
const ENVELOPE_FIELD_COUNT: usize = 18;
const OUTCOME_FIELD_COUNT: usize = 6;
const EFFECT_FIELD_COUNT: usize = 8;
const PORT_TARGET_FIELD_COUNT: usize = 2;
const VALUE_FIELD_COUNT: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCallbackContext {
    pub manifest_ref: String,
    pub executable_ref: String,
    pub instance_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub state_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub resource_ref: String,
    pub port_binding_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCallbackInputs {
    pub payload: Option<NativeCallbackValue>,
    pub state: Option<NativeCallbackValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMaterializedEffectRequest {
    pub effect: TypedEffectRequest,
    pub request: NativeCallbackValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMaterializedCallbackOutcome {
    pub outputs: Vec<NativeCallbackValue>,
    pub effects: Vec<NativeMaterializedEffectRequest>,
    pub state: Option<NativeCallbackValue>,
    pub checkpoint: Option<NativeCallbackValue>,
    pub health: HealthState,
}

impl NativeMaterializedCallbackOutcome {
    pub fn project(&self) -> CallbackOutcome {
        CallbackOutcome {
            output_refs: self.outputs.iter().map(|value| value.value_ref.clone()).collect(),
            effects: self.effects.iter().map(|effect| effect.effect.clone()).collect(),
            state_ref: self.state.as_ref().map(|value| value.value_ref.clone()),
            checkpoint_ref: self.checkpoint.as_ref().map(|value| value.value_ref.clone()),
            health: self.health,
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &NativeCallbackValue> {
        self.outputs
            .iter()
            .chain(self.effects.iter().map(|effect| &effect.request))
            .chain(self.state.iter())
            .chain(self.checkpoint.iter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNativeCallbackEnvelope {
    pub envelope_ref: String,
    pub invocation: CallbackInvocation,
    pub context: NativeCallbackContext,
    pub inputs: NativeCallbackInputs,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedNativeCallbackEnvelope {
    pub invocation: CallbackInvocation,
    pub context: NativeCallbackContext,
    pub inputs: NativeCallbackInputs,
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_materialization]
pub fn canonical_native_callback_envelope(
    context: &NativeCallbackContext,
    invocation: &CallbackInvocation,
    inputs: &NativeCallbackInputs,
) -> Result<CanonicalNativeCallbackEnvelope> {
    require_input_links(context, invocation, inputs)?;
    let value = record(CALLBACK_ENVELOPE_RECORD, vec![
        string(NATIVE_CALLBACK_ENVELOPE_SCHEMA),
        string(&context.manifest_ref),
        string(&context.executable_ref),
        string(&context.instance_id),
        string(&context.extension_id),
        string(&context.service_id),
        u64_value(invocation.generation),
        string(invocation.callback.as_str()),
        u64_value(invocation.sequence),
        string(&invocation.event_ref),
        optional_value(inputs.payload.as_ref()),
        u64_value(invocation.logical_tick),
        u64_value(invocation.deadline_tick),
        optional_value(inputs.state.as_ref()),
        ref_sequence(&context.policy_refs),
        string(&context.resource_ref),
        ref_sequence(&context.port_binding_refs),
        string(NATIVE_FRAMING),
    ]);
    let envelope_ref = canonical_hash(&value)?;
    let bytes = canonical_bytes(&value)?;
    Ok(CanonicalNativeCallbackEnvelope {
        envelope_ref,
        invocation: invocation.clone(),
        context: context.clone(),
        inputs: inputs.clone(),
        value,
        bytes,
    })
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_materialization]
pub fn decode_native_callback_envelope(
    bytes: &[u8],
    maximum_bytes: u64,
    maximum_value_bytes: u64,
    maximum_items: u64,
) -> Result<DecodedNativeCallbackEnvelope> {
    require_byte_bound(bytes, maximum_bytes, "native callback input")?;
    let decoded = strict_canonical_decode(bytes)?;
    let fields = simple_record_fields(&decoded.value, CALLBACK_ENVELOPE_RECORD, ENVELOPE_FIELD_COUNT)?;
    let schema = required_string_field(&fields[0], "callback envelope schema")?;
    if schema != NATIVE_CALLBACK_ENVELOPE_SCHEMA {
        return Err(MoltenError::invalid_harness("native callback envelope schema mismatch"));
    }
    let callback_name = required_string_field(&fields[7], "callback kind")?;
    let callback = CallbackKind::parse(&callback_name)
        .ok_or_else(|| MoltenError::invalid_harness("native callback kind is unsupported"))?;
    let framing = required_string_field(&fields[17], "callback framing")?;
    if framing != NATIVE_FRAMING {
        return Err(MoltenError::invalid_harness("native callback framing mismatch"));
    }
    let payload = parse_optional_value(&fields[10], "callback payload", maximum_value_bytes)?;
    let state = parse_optional_value(&fields[13], "callback state", maximum_value_bytes)?;
    require_item_bound(payload.iter().count() + state.iter().count(), maximum_items, "callback input values")?;
    let invocation = CallbackInvocation {
        callback,
        generation: required_u64(&fields[6], "callback generation")?,
        sequence: required_u64(&fields[8], "callback sequence")?,
        event_ref: required_content_ref_string(&fields[9], "callback event ref")?,
        payload_ref: payload.as_ref().map(|value| value.value_ref.clone()),
        logical_tick: required_u64(&fields[11], "callback logical tick")?,
        deadline_tick: required_u64(&fields[12], "callback deadline tick")?,
    };
    let context = NativeCallbackContext {
        manifest_ref: required_content_ref_string(&fields[1], "callback manifest ref")?,
        executable_ref: required_content_ref_string(&fields[2], "callback executable ref")?,
        instance_id: required_string_field(&fields[3], "callback instance id")?,
        extension_id: required_string_field(&fields[4], "callback extension id")?,
        service_id: required_string_field(&fields[5], "callback service id")?,
        state_ref: state.as_ref().map(|value| value.value_ref.clone()),
        policy_refs: parse_ref_sequence(&fields[14], "callback policy refs", maximum_items)?,
        resource_ref: required_content_ref_string(&fields[15], "callback resource ref")?,
        port_binding_refs: parse_ref_sequence(&fields[16], "callback port refs", maximum_items)?,
    };
    let inputs = NativeCallbackInputs { payload, state };
    require_input_links(&context, &invocation, &inputs)?;
    Ok(DecodedNativeCallbackEnvelope {
        invocation,
        context,
        inputs,
    })
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_publication]
pub fn encode_native_callback_outcome(outcome: &NativeMaterializedCallbackOutcome) -> Result<Vec<u8>> {
    for value in outcome.values() {
        admit_native_callback_value(value, u64::MAX).map_err(value_error)?;
    }
    let value = record(CALLBACK_OUTCOME_RECORD, vec![
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
        sequence(outcome.outputs.iter().map(value_value).collect()),
        sequence(outcome.effects.iter().map(effect_value).collect()),
        optional_value(outcome.state.as_ref()),
        optional_value(outcome.checkpoint.as_ref()),
        string(outcome.health.as_str()),
    ]);
    canonical_bytes(&value)
}

// r[impl molten.system_extension.native_host.value_protocol]
// r[impl molten.system_extension.native_host.value_publication]
pub fn decode_native_callback_outcome(
    bytes: &[u8],
    maximum_bytes: u64,
    maximum_value_bytes: u64,
    maximum_items: u64,
) -> Result<NativeMaterializedCallbackOutcome> {
    require_byte_bound(bytes, maximum_bytes, "native callback output")?;
    let decoded = strict_canonical_decode(bytes)?;
    let fields = simple_record_fields(&decoded.value, CALLBACK_OUTCOME_RECORD, OUTCOME_FIELD_COUNT)?;
    let schema = required_string_field(&fields[0], "callback outcome schema")?;
    if schema != NATIVE_CALLBACK_OUTCOME_SCHEMA {
        return Err(MoltenError::invalid_harness("native callback outcome schema mismatch"));
    }
    let output_values = required_sequence_field(&fields[1], "callback outputs")?;
    let effect_values = required_sequence_field(&fields[2], "callback effects")?;
    let state = parse_optional_value(&fields[3], "callback state", maximum_value_bytes)?;
    let checkpoint = parse_optional_value(&fields[4], "callback checkpoint", maximum_value_bytes)?;
    let value_count = output_values
        .len()
        .checked_add(effect_values.len())
        .and_then(|count| count.checked_add(state.iter().count()))
        .and_then(|count| count.checked_add(checkpoint.iter().count()))
        .ok_or_else(|| MoltenError::invalid_harness("native callback value count overflow"))?;
    require_item_bound(value_count, maximum_items, "callback output values")?;
    let outputs = output_values
        .iter()
        .map(|value| parse_value(value, "callback output", maximum_value_bytes))
        .collect::<Result<Vec<_>>>()?;
    let effects = effect_values
        .iter()
        .map(|value| parse_effect(value, maximum_value_bytes))
        .collect::<Result<Vec<_>>>()?;
    Ok(NativeMaterializedCallbackOutcome {
        outputs,
        effects,
        state,
        checkpoint,
        health: parse_health(&required_string_field(&fields[5], "callback health")?)?,
    })
}

fn effect_value(effect: &NativeMaterializedEffectRequest) -> IOValue {
    let target = match &effect.effect.target {
        EffectTarget::FabricPort(key) => record(PORT_TARGET_RECORD, vec![string(&key.port_id), string(&key.version)]),
        EffectTarget::Ambient(ambient) => record("native-callback-ambient-target-v2", vec![string(ambient.as_str())]),
    };
    record(EFFECT_RECORD, vec![
        target,
        string(&effect.effect.operation),
        string(&effect.effect.input_schema_ref),
        string(&effect.effect.output_schema_ref),
        value_value(&effect.request),
        u64_value(effect.effect.generation),
        u64_value(effect.effect.accounted_bytes),
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
    ])
}

fn parse_effect(value: &Value<IOValue>, maximum_value_bytes: u64) -> Result<NativeMaterializedEffectRequest> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record_fields(&value, EFFECT_RECORD, EFFECT_FIELD_COUNT)?;
    let target_value = crate::preserves_rail::value_to_iovalue(&fields[0]);
    let target_fields = simple_record_fields(&target_value, PORT_TARGET_RECORD, PORT_TARGET_FIELD_COUNT)
        .map_err(|_| MoltenError::invalid_harness("native callback effect target must be an exact fabric port"))?;
    let schema = required_string_field(&fields[7], "effect schema")?;
    if schema != NATIVE_CALLBACK_OUTCOME_SCHEMA {
        return Err(MoltenError::invalid_harness("native callback effect schema mismatch"));
    }
    let request = parse_value(&fields[4], "effect request", maximum_value_bytes)?;
    Ok(NativeMaterializedEffectRequest {
        effect: TypedEffectRequest {
            target: EffectTarget::FabricPort(FabricPortKey {
                port_id: required_string_field(&target_fields[0], "effect port id")?,
                version: required_string_field(&target_fields[1], "effect port version")?,
            }),
            operation: required_string_field(&fields[1], "effect operation")?,
            input_schema_ref: required_string_field(&fields[2], "effect input schema")?,
            output_schema_ref: required_string_field(&fields[3], "effect output schema")?,
            request_ref: request.value_ref.clone(),
            generation: required_u64(&fields[5], "effect generation")?,
            accounted_bytes: required_u64(&fields[6], "effect accounted bytes")?,
        },
        request,
    })
}

fn optional_value(value: Option<&NativeCallbackValue>) -> IOValue {
    value.map_or_else(|| record(NONE_RECORD, Vec::new()), |value| record(SOME_RECORD, vec![value_value(value)]))
}

fn parse_optional_value(
    value: &Value<IOValue>,
    field: &str,
    maximum_value_bytes: u64,
) -> Result<Option<NativeCallbackValue>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    if value.collect_simple_record(NONE_RECORD, Some(0)).is_some() {
        return Ok(None);
    }
    let fields = simple_record_fields(&value, SOME_RECORD, 1)?;
    parse_value(&fields[0], field, maximum_value_bytes).map(Some)
}

fn value_value(value: &NativeCallbackValue) -> IOValue {
    record(VALUE_RECORD, vec![string(&value.value_ref), bytes_value(&value.bytes)])
}

fn parse_value(value: &Value<IOValue>, field: &str, maximum_bytes: u64) -> Result<NativeCallbackValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record_fields(&value, VALUE_RECORD, VALUE_FIELD_COUNT)?;
    let value = NativeCallbackValue {
        value_ref: required_content_ref_string(&fields[0], field)?,
        bytes: parse_bytes(&fields[1], field, maximum_bytes)?,
    };
    admit_native_callback_value(&value, maximum_bytes).map_err(value_error)?;
    Ok(value)
}

fn bytes_value(bytes: &[u8]) -> IOValue {
    sequence(bytes.iter().map(|byte| u64_value(u64::from(*byte))).collect())
}

fn parse_bytes(value: &Value<IOValue>, field: &str, maximum_bytes: u64) -> Result<Vec<u8>> {
    let values = required_sequence_field(value, field)?;
    require_byte_bound_len(values.len(), maximum_bytes, field)?;
    values
        .iter()
        .map(|value| {
            let number = required_u64(value, field)?;
            u8::try_from(number)
                .map_err(|_| MoltenError::invalid_harness(format!("{field} contains a value outside the byte range")))
        })
        .collect()
}

fn require_input_links(
    context: &NativeCallbackContext,
    invocation: &CallbackInvocation,
    inputs: &NativeCallbackInputs,
) -> Result<()> {
    if invocation.payload_ref.as_deref() != inputs.payload.as_ref().map(|value| value.value_ref.as_str()) {
        return Err(MoltenError::invalid_harness("native callback payload reference lacks exact bytes"));
    }
    if context.state_ref.as_deref() != inputs.state.as_ref().map(|value| value.value_ref.as_str()) {
        return Err(MoltenError::invalid_harness("native callback state reference lacks exact bytes"));
    }
    for value in inputs.payload.iter().chain(inputs.state.iter()) {
        admit_native_callback_value(value, u64::MAX).map_err(value_error)?;
    }
    Ok(())
}

fn ref_sequence(references: &[String]) -> IOValue {
    sequence(references.iter().map(string).collect())
}

fn parse_ref_sequence(value: &Value<IOValue>, field: &str, maximum: u64) -> Result<Vec<String>> {
    let values = required_sequence_field(value, field)?;
    require_item_bound(values.len(), maximum, field)?;
    values.iter().map(|value| required_content_ref_string(value, field)).collect()
}

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn parse_health(value: &str) -> Result<HealthState> {
    match value {
        "unknown" => Ok(HealthState::Unknown),
        "starting" => Ok(HealthState::Starting),
        "healthy" => Ok(HealthState::Healthy),
        "degraded" => Ok(HealthState::Degraded),
        "failed" => Ok(HealthState::Failed),
        "quarantined" => Ok(HealthState::Quarantined),
        "stopped" => Ok(HealthState::Stopped),
        _ => Err(MoltenError::invalid_harness("native callback health is unsupported")),
    }
}

fn require_byte_bound(bytes: &[u8], maximum: u64, label: &str) -> Result<()> {
    require_byte_bound_len(bytes.len(), maximum, label)
}

fn require_byte_bound_len(actual: usize, maximum: u64, label: &str) -> Result<()> {
    let actual =
        u64::try_from(actual).map_err(|_| MoltenError::invalid_harness(format!("{label} length does not fit u64")))?;
    if actual > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds {maximum} bytes")));
    }
    Ok(())
}

fn require_item_bound(actual: usize, maximum: u64, label: &str) -> Result<()> {
    let actual =
        u64::try_from(actual).map_err(|_| MoltenError::invalid_harness(format!("{label} count does not fit u64")))?;
    if actual > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds its item bound")));
    }
    Ok(())
}

fn value_error(error: super::NativeValuePortFailure) -> MoltenError {
    MoltenError::invalid_harness(error.message)
}
