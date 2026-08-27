use preserves::IOValue;
use preserves::Value;

use super::super::*;
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

const CALLBACK_ENVELOPE_RECORD: &str = "native-callback-envelope-v1";
const CALLBACK_OUTCOME_RECORD: &str = "native-callback-outcome-v1";
const EFFECT_RECORD: &str = "native-callback-effect-v1";
const PORT_TARGET_RECORD: &str = "native-callback-port-target-v1";
const NONE_RECORD: &str = "none";
const SOME_RECORD: &str = "some";
const ENVELOPE_FIELD_COUNT: usize = 18;
const OUTCOME_FIELD_COUNT: usize = 6;
const EFFECT_FIELD_COUNT: usize = 8;
const PORT_TARGET_FIELD_COUNT: usize = 2;

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
pub struct CanonicalNativeCallbackEnvelope {
    pub envelope_ref: String,
    pub invocation: CallbackInvocation,
    pub context: NativeCallbackContext,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedNativeCallbackEnvelope {
    pub invocation: CallbackInvocation,
    pub context: NativeCallbackContext,
}

// r[impl molten.system_extension.native_host.callback_protocol]
pub fn canonical_native_callback_envelope(
    context: &NativeCallbackContext,
    invocation: &CallbackInvocation,
) -> Result<CanonicalNativeCallbackEnvelope> {
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
        optional_ref_value(invocation.payload_ref.as_deref()),
        u64_value(invocation.logical_tick),
        u64_value(invocation.deadline_tick),
        optional_ref_value(context.state_ref.as_deref()),
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
        value,
        bytes,
    })
}

// r[impl molten.system_extension.native_host.callback_protocol]
pub fn decode_native_callback_envelope(
    bytes: &[u8],
    maximum_bytes: u64,
    maximum_items: usize,
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
    Ok(DecodedNativeCallbackEnvelope {
        invocation: CallbackInvocation {
            callback,
            generation: required_u64(&fields[6], "callback generation")?,
            sequence: required_u64(&fields[8], "callback sequence")?,
            event_ref: required_content_ref_string(&fields[9], "callback event ref")?,
            payload_ref: parse_optional_ref(&fields[10], "callback payload ref")?,
            logical_tick: required_u64(&fields[11], "callback logical tick")?,
            deadline_tick: required_u64(&fields[12], "callback deadline tick")?,
        },
        context: NativeCallbackContext {
            manifest_ref: required_content_ref_string(&fields[1], "callback manifest ref")?,
            executable_ref: required_content_ref_string(&fields[2], "callback executable ref")?,
            instance_id: required_string_field(&fields[3], "callback instance id")?,
            extension_id: required_string_field(&fields[4], "callback extension id")?,
            service_id: required_string_field(&fields[5], "callback service id")?,
            state_ref: parse_optional_ref(&fields[13], "callback state ref")?,
            policy_refs: parse_ref_sequence(&fields[14], "callback policy refs", maximum_items)?,
            resource_ref: required_content_ref_string(&fields[15], "callback resource ref")?,
            port_binding_refs: parse_ref_sequence(&fields[16], "callback port refs", maximum_items)?,
        },
    })
}

// r[impl molten.system_extension.native_host.callback_protocol]
pub fn encode_native_callback_outcome(outcome: &CallbackOutcome) -> Result<Vec<u8>> {
    let value = record(CALLBACK_OUTCOME_RECORD, vec![
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
        ref_sequence(&outcome.output_refs),
        sequence(outcome.effects.iter().map(effect_value).collect()),
        optional_ref_value(outcome.state_ref.as_deref()),
        optional_ref_value(outcome.checkpoint_ref.as_deref()),
        string(outcome.health.as_str()),
    ]);
    canonical_bytes(&value)
}

// r[impl molten.system_extension.native_host.callback_protocol]
pub fn decode_native_callback_outcome(
    bytes: &[u8],
    maximum_bytes: u64,
    maximum_items: usize,
) -> Result<CallbackOutcome> {
    require_byte_bound(bytes, maximum_bytes, "native callback output")?;
    let decoded = strict_canonical_decode(bytes)?;
    let fields = simple_record_fields(&decoded.value, CALLBACK_OUTCOME_RECORD, OUTCOME_FIELD_COUNT)?;
    let schema = required_string_field(&fields[0], "callback outcome schema")?;
    if schema != NATIVE_CALLBACK_OUTCOME_SCHEMA {
        return Err(MoltenError::invalid_harness("native callback outcome schema mismatch"));
    }
    let effect_values = required_sequence_field(&fields[2], "callback effects")?;
    if effect_values.len() > maximum_items {
        return Err(MoltenError::invalid_harness("native callback effect count exceeds its bound"));
    }
    let effects = effect_values.iter().map(parse_effect).collect::<Result<Vec<_>>>()?;
    Ok(CallbackOutcome {
        output_refs: parse_ref_sequence(&fields[1], "callback output refs", maximum_items)?,
        effects,
        state_ref: parse_optional_ref(&fields[3], "callback state ref")?,
        checkpoint_ref: parse_optional_ref(&fields[4], "callback checkpoint ref")?,
        health: parse_health(&required_string_field(&fields[5], "callback health")?)?,
    })
}

fn effect_value(effect: &TypedEffectRequest) -> IOValue {
    let target = match &effect.target {
        EffectTarget::FabricPort(key) => record(PORT_TARGET_RECORD, vec![string(&key.port_id), string(&key.version)]),
        EffectTarget::Ambient(ambient) => record("native-callback-ambient-target-v1", vec![string(ambient.as_str())]),
    };
    record(EFFECT_RECORD, vec![
        target,
        string(&effect.operation),
        string(&effect.input_schema_ref),
        string(&effect.output_schema_ref),
        string(&effect.request_ref),
        u64_value(effect.generation),
        u64_value(effect.accounted_bytes),
        string(NATIVE_CALLBACK_OUTCOME_SCHEMA),
    ])
}

fn parse_effect(value: &Value<IOValue>) -> Result<TypedEffectRequest> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = simple_record_fields(&value, EFFECT_RECORD, EFFECT_FIELD_COUNT)?;
    let target_value = crate::preserves_rail::value_to_iovalue(&fields[0]);
    let target_fields = simple_record_fields(&target_value, PORT_TARGET_RECORD, PORT_TARGET_FIELD_COUNT)
        .map_err(|_| MoltenError::invalid_harness("native callback effect target must be an exact fabric port"))?;
    let schema = required_string_field(&fields[7], "effect schema")?;
    if schema != NATIVE_CALLBACK_OUTCOME_SCHEMA {
        return Err(MoltenError::invalid_harness("native callback effect schema mismatch"));
    }
    Ok(TypedEffectRequest {
        target: EffectTarget::FabricPort(FabricPortKey {
            port_id: required_string_field(&target_fields[0], "effect port id")?,
            version: required_string_field(&target_fields[1], "effect port version")?,
        }),
        operation: required_string_field(&fields[1], "effect operation")?,
        input_schema_ref: required_string_field(&fields[2], "effect input schema")?,
        output_schema_ref: required_string_field(&fields[3], "effect output schema")?,
        request_ref: required_content_ref_string(&fields[4], "effect request ref")?,
        generation: required_u64(&fields[5], "effect generation")?,
        accounted_bytes: required_u64(&fields[6], "effect accounted bytes")?,
    })
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record(NONE_RECORD, Vec::new()), |reference| record(SOME_RECORD, vec![string(reference)]))
}

fn parse_optional_ref(value: &Value<IOValue>, field: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    if value.collect_simple_record(NONE_RECORD, Some(0)).is_some() {
        return Ok(None);
    }
    let fields = simple_record_fields(&value, SOME_RECORD, 1)?;
    required_content_ref_string(&fields[0], field).map(Some)
}

fn ref_sequence(references: &[String]) -> IOValue {
    sequence(references.iter().map(string).collect())
}

fn parse_ref_sequence(value: &Value<IOValue>, field: &str, maximum: usize) -> Result<Vec<String>> {
    let values = required_sequence_field(value, field)?;
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!("{field} exceeds its item bound")));
    }
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
    let actual = u64::try_from(bytes.len())
        .map_err(|_| MoltenError::invalid_harness(format!("{label} length does not fit u64")))?;
    if actual > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds {maximum} bytes")));
    }
    Ok(())
}
